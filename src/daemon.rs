//! Long-lived Perci daemon — TCP JSONL on **loopback** for sub-ms routing after warm.
//!
//! Protocol (one JSON object per line, max 256 KiB):
//!   {"op":"ping","token":"..."}
//!   {"op":"ask","text":"...","token":"..."}
//!   {"op":"classify","text":"...","token":"..."}
//!   {"op":"shutdown","token":"..."}
//!
//! Security (v0.7.0):
//! - Default bind **127.0.0.1 only**. Non-loopback requires PERCI_DAEMON_ALLOW_NON_LOOPBACK=1.
//! - Optional session token: set PERCI_DAEMON_TOKEN; all ops except ping-without-token
//!   require matching `token` when the env var is set. Shutdown **always** requires token
//!   when PERCI_DAEMON_TOKEN is set.
//! - Read timeout 60s, write 30s, max line 256 KiB.
//!
//! Default: 127.0.0.1:17865  (PERCI_DAEMON_PORT / PERCI_DAEMON_HOST)

use crate::backend::{CompositeBackend, LanguageBackend};
use crate::chat::ChatEngine;
use crate::cortex::CortexBridge;
use crate::memory::MemoryStore;
use crate::personality::Personality;
use crate::session::SessionStore;
use serde_json::{json, Value};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

const MAX_LINE_BYTES: usize = 256 * 1024;

pub fn default_host() -> String {
    let host = env::var("PERCI_DAEMON_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let allow = env::var("PERCI_DAEMON_ALLOW_NON_LOOPBACK").ok().as_deref() == Some("1");
    if !allow && host != "127.0.0.1" && host != "localhost" && host != "::1" {
        eprintln!(
            "perci daemon: refusing non-loopback host {host:?}; set PERCI_DAEMON_ALLOW_NON_LOOPBACK=1 for secure mode override"
        );
        return "127.0.0.1".into();
    }
    host
}

pub fn default_port() -> u16 {
    env::var("PERCI_DAEMON_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(17865)
}

pub fn addr() -> String {
    format!("{}:{}", default_host(), default_port())
}

fn expected_token() -> Option<String> {
    env::var("PERCI_DAEMON_TOKEN").ok().filter(|s| !s.is_empty())
}

fn check_token(req: &Value, op: &str) -> Result<(), String> {
    let Some(expected) = expected_token() else {
        return Ok(());
    };
    // ping may omit token for liveness probes when PERCI_DAEMON_ALLOW_OPEN_PING=1
    if op == "ping" && env::var("PERCI_DAEMON_ALLOW_OPEN_PING").ok().as_deref() == Some("1") {
        return Ok(());
    }
    let got = req.get("token").and_then(|x| x.as_str()).unwrap_or("");
    if got == expected {
        Ok(())
    } else {
        Err("unauthorized: missing or invalid token".into())
    }
}

/// Run blocking daemon (single client at a time for shared ChatEngine safety).
pub fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let personality = {
        let path =
            env::var("PERCI_PERSONALITY").unwrap_or_else(|_| "config/personality.prompt".into());
        Personality::load(path).unwrap_or_else(|_| Personality::default_perci())
    };
    let memory_path = env::var_os("PERCI_MEMORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("memory/perci.jsonl"));
    let backend: Box<dyn LanguageBackend> = Box::new(CompositeBackend::discover()?);
    let classifier = {
        let path = crate::cognitive::default_weight_path();
        if path.is_file() {
            Some(crate::cognitive::CognitiveWeights::load(path)?)
        } else {
            None
        }
    };
    let cortex = CortexBridge::discover();
    let session = SessionStore::discover();
    let learner = crate::learning::InteractionLearner::discover();
    let mut engine = ChatEngine::new(personality, MemoryStore::new(memory_path), backend, cortex)
        .with_session(session)
        .with_learning(learner);

    let bind = addr();
    let listener = TcpListener::bind(&bind)?;
    listener.set_nonblocking(false)?;
    eprintln!("perci daemon listening on {bind}");
    eprintln!(
        "ops: ping | ask | classify | shutdown · token_required={}",
        expected_token().is_some()
    );
    eprintln!("security: loopback-default · max_line={MAX_LINE_BYTES} · read_timeout=60s");

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
        // Prefer peer on loopback when possible.
        if let Ok(peer) = stream.peer_addr() {
            let ip = peer.ip();
            if !ip.is_loopback() && env::var("PERCI_DAEMON_ALLOW_NON_LOOPBACK").ok().as_deref() != Some("1")
            {
                eprintln!("reject non-loopback peer {ip}");
                continue;
            }
        }
        if let Err(e) = handle_client(stream, &mut engine, classifier.as_ref()) {
            eprintln!("client error: {e}");
        }
    }
    Ok(())
}

fn handle_client(
    stream: TcpStream,
    engine: &mut ChatEngine,
    classifier: Option<&crate::cognitive::CognitiveWeights>,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        if line.len() > MAX_LINE_BYTES {
            writeln!(
                writer,
                "{}",
                json!({"ok": false, "error": "payload too large"})
            )?;
            break;
        }
        let req: Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(e) => {
                writeln!(
                    writer,
                    "{}",
                    json!({"ok": false, "error": format!("bad json: {e}")})
                )?;
                continue;
            }
        };
        let op = req
            .get("op")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if let Err(e) = check_token(&req, &op) {
            writeln!(writer, "{}", json!({"ok": false, "error": e}))?;
            writer.flush()?;
            continue;
        }

        match op.as_str() {
            "ping" => {
                writeln!(
                    writer,
                    "{}",
                    json!({
                        "ok": true,
                        "service": "perci-daemon",
                        "version": env!("CARGO_PKG_VERSION"),
                        "fabric": "v0.7.0"
                    })
                )?;
            }
            "ask" => {
                let text = req
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                if text.is_empty() {
                    writeln!(
                        writer,
                        "{}",
                        json!({"ok": false, "error": "ask requires text"})
                    )?;
                    continue;
                }
                if text.len() > 32 * 1024 {
                    writeln!(
                        writer,
                        "{}",
                        json!({"ok": false, "error": "ask text too long"})
                    )?;
                    continue;
                }
                match engine.respond(text) {
                    Ok(r) => {
                        writeln!(
                            writer,
                            "{}",
                            json!({"ok": true, "text": r.text, "route": format!("{:?}", r.route)})
                        )?;
                    }
                    Err(e) => {
                        writeln!(writer, "{}", json!({"ok": false, "error": e.to_string()}))?;
                    }
                }
            }
            "classify" => {
                let text = req
                    .get("text")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                if text.is_empty() {
                    writeln!(
                        writer,
                        "{}",
                        json!({"ok": false, "error": "classify requires text"})
                    )?;
                    continue;
                }
                match classify_json_value(text, classifier) {
                    Ok(v) => {
                        writeln!(writer, "{}", json!({"ok": true, "result": v}))?;
                    }
                    Err(e) => {
                        writeln!(writer, "{}", json!({"ok": false, "error": e}))?;
                    }
                }
            }
            "shutdown" => {
                // Fail closed: if token env is set, check_token already required it.
                // If no token configured, refuse shutdown over network for safety.
                if expected_token().is_none() {
                    writeln!(
                        writer,
                        "{}",
                        json!({"ok": false, "error": "shutdown disabled without PERCI_DAEMON_TOKEN"})
                    )?;
                    continue;
                }
                writeln!(writer, "{}", json!({"ok": true, "shutdown": true}))?;
                writer.flush()?;
                std::process::exit(0);
            }
            "" => {
                writeln!(writer, "{}", json!({"ok": false, "error": "missing op"}))?;
            }
            other => {
                writeln!(
                    writer,
                    "{}",
                    json!({"ok": false, "error": format!("unknown op: {other}")})
                )?;
            }
        }
        writer.flush()?;
    }
    Ok(())
}

fn classify_json_value(
    input: &str,
    classifier: Option<&crate::cognitive::CognitiveWeights>,
) -> Result<Value, String> {
    if let Some(weights) = classifier {
        let matched = weights.classify(input).map_err(|e| e.to_string())?;
        let mixture: Vec<Value> = matched
            .mixture
            .iter()
            .map(|m| {
                json!({
                    "label": m.label,
                    "score": m.score,
                    "overlap": m.overlap,
                    "concept_id": m.concept_id,
                    "insight": m.insight,
                    "residual": m.residual,
                    "hop": m.hop,
                    "attention_pm": m.attention_pm,
                })
            })
            .collect();
        return Ok(json!({
            "schema": "perci.classify.v5-attn",
            "label": matched.label,
            "variant": matched.variant,
            "concept_id": matched.concept_id,
            "insight": matched.insight,
            "skeleton": matched.concept_skeleton(3),
            "composition": matched.composition_frame(8),
            "primary_attention_pm": matched.primary_attention_pm,
            "mixture": mixture,
            "score": matched.score,
            "overlap": matched.overlap,
            "runner_up_score": matched.runner_up_score,
            "margin": matched.margin,
            "query_popcount": matched.query_popcount,
            "prototype_popcount": matched.prototype_popcount,
            "positive_overlap": matched.positive_overlap,
            "negative_overlap": matched.negative_overlap,
            "hamming": matched.hamming,
            "jaccard": matched.jaccard,
            "overlap_z": matched.overlap_z,
        }));
    }
    Ok(json!({
        "schema": "perci.classify.v1-fallback",
        "label": "general",
        "variant": 0,
        "score": 10,
        "overlap": 2,
    }))
}

/// Client: send one request, read one response line (includes token if configured).
pub fn request(op: &str, text: Option<&str>) -> Result<Value, String> {
    let addr = addr();
    let mut stream =
        TcpStream::connect(&addr).map_err(|e| format!("daemon connect {addr}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();
    let mut req = json!({"op": op});
    if let Some(t) = text {
        req["text"] = json!(t);
    }
    if let Some(tok) = expected_token() {
        req["token"] = json!(tok);
    }
    let line = format!("{req}\n");
    stream
        .write_all(line.as_bytes())
        .map_err(|e| format!("daemon write: {e}"))?;
    stream.flush().ok();
    let mut reader = BufReader::new(stream);
    let mut resp = String::new();
    reader
        .read_line(&mut resp)
        .map_err(|e| format!("daemon read: {e}"))?;
    serde_json::from_str(resp.trim()).map_err(|e| format!("daemon json: {e} · {resp}"))
}

pub fn ping() -> bool {
    request("ping", None)
        .ok()
        .and_then(|v| v.get("ok").and_then(|x| x.as_bool()))
        .unwrap_or(false)
}

pub fn ask_daemon(text: &str) -> Result<String, String> {
    let v = request("ask", Some(text))?;
    if v.get("ok").and_then(|x| x.as_bool()) != Some(true) {
        return Err(v
            .get("error")
            .and_then(|x| x.as_str())
            .unwrap_or("ask failed")
            .to_string());
    }
    v.get("text")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "no text in daemon response".into())
}

pub fn classify_daemon(text: &str) -> Result<Value, String> {
    let v = request("classify", Some(text))?;
    if v.get("ok").and_then(|x| x.as_bool()) != Some(true) {
        return Err(v
            .get("error")
            .and_then(|x| x.as_str())
            .unwrap_or("classify failed")
            .to_string());
    }
    v.get("result")
        .cloned()
        .ok_or_else(|| "no result in daemon response".into())
}
