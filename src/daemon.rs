//! Long-lived Perci daemon — TCP JSONL on localhost for sub-ms routing after warm.
//!
//! Protocol (one JSON object per line):
//!   {"op":"ping"}
//!   {"op":"ask","text":"..."}
//!   {"op":"classify","text":"..."}
//!   {"op":"shutdown"}
//!
//! Responses:
//!   {"ok":true,"text":"..."}
//!   {"ok":true,"result":{...}}
//!   {"ok":false,"error":"..."}
//!
//! Default: 127.0.0.1:17865  (override with PERCI_DAEMON_PORT / PERCI_DAEMON_HOST)

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

pub fn default_host() -> String {
    env::var("PERCI_DAEMON_HOST").unwrap_or_else(|_| "127.0.0.1".into())
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
    // Keep one classifier mapping for the daemon lifetime. The chat backend has
    // its own immutable mapping; both mappings share OS pages and never mutate.
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
    eprintln!("ops: ping | ask | classify | shutdown");

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
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
    stream.set_read_timeout(Some(Duration::from_secs(300)))?;
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
        match op.as_str() {
            "ping" => {
                writeln!(
                    writer,
                    "{}",
                    json!({"ok": true, "service": "perci-daemon", "version": env!("CARGO_PKG_VERSION")})
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
                writeln!(writer, "{}", json!({"ok": true, "shutdown": true}))?;
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

/// Client: send one request, read one response line.
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
