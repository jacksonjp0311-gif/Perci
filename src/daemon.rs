//! Long-lived Perci daemon — TCP JSONL on **loopback** for sub-ms routing after warm.
//!
//! Protocol (one JSON object per line, max 256 KiB):
//!   {"op":"ping","token":"..."}
//!   {"op":"ask","text":"...","token":"..."}
//!   {"op":"ask","text":"...","stateless":true}   (no session/learning; deterministic)
//!   {"op":"classify","text":"...","token":"..."}
//!   {"op":"shutdown","token":"..."}
//!
//! Security (v0.7.0):
//! - Default bind **127.0.0.1 only**. Non-loopback requires PERCI_DAEMON_ALLOW_NON_LOOPBACK=1.
//! - Optional session token: set PERCI_DAEMON_TOKEN; all ops except ping-without-token
//!   require matching `token` when the env var is set. Shutdown **always** requires token
//!   when PERCI_DAEMON_TOKEN is set.
//! - Read timeout 60s, write 30s, max line 256 KiB.
//! - TCP_NODELAY on both ends and one `write_all` per reply, so a warm `ping`
//!   costs well under a millisecond instead of the ~40 ms Nagle floor.
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
    env::var("PERCI_DAEMON_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())
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
        let path = env::var_os("PERCI_PERSONALITY")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::paths::asset("config/personality.prompt"));
        Personality::load(path).unwrap_or_else(|_| Personality::default_perci())
    };
    let memory_path = env::var_os("PERCI_MEMORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::data_file("memory/perci.jsonl"));
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
    write_marker(&bind);
    eprintln!("perci daemon listening on {bind}");
    eprintln!(
        "ops: ping | ask | classify | session_reset | shutdown · token_required={}",
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
            if !ip.is_loopback()
                && env::var("PERCI_DAEMON_ALLOW_NON_LOOPBACK").ok().as_deref() != Some("1")
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

/// Read one line but never buffer more than `max + 1` bytes, so a client that
/// streams data without a newline cannot exhaust memory. A return value
/// greater than `max` means "too large" (the caller closes the connection).
pub fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    line: &mut String,
    max: usize,
) -> std::io::Result<usize> {
    use std::io::Read;
    let limit = u64::try_from(max).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    let n = reader.by_ref().take(limit).read_until(b'\n', &mut bytes)?;
    line.push_str(&String::from_utf8_lossy(&bytes));
    Ok(n)
}

/// Serialize `value` and send it as **one** `write_all` (payload + newline).
///
/// Two small writes (`writeln!` on a raw `TcpStream`) hit the Nagle /
/// delayed-ACK interaction and cost ~40 ms per reply (B2E-PERCI-012).
pub fn send_line<W: Write>(writer: &mut W, value: &Value) -> std::io::Result<()> {
    let mut buf = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    buf.push(b'\n');
    writer.write_all(&buf)?;
    writer.flush()
}

/// Stateless chat engine for `{"op":"ask","stateless":true}`: no session,
/// no learning observer, no memory file. Same input gives the same output.
pub fn stateless_engine() -> Result<ChatEngine, Box<dyn std::error::Error>> {
    let personality = {
        let path = env::var_os("PERCI_PERSONALITY")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::paths::asset("config/personality.prompt"));
        Personality::load(path).unwrap_or_else(|_| Personality::default_perci())
    };
    let backend: Box<dyn LanguageBackend> = Box::new(CompositeBackend::discover()?);
    Ok(ChatEngine::new(
        personality,
        MemoryStore::new(PathBuf::new()),
        backend,
        None,
    ))
}

fn handle_client(
    stream: TcpStream,
    engine: &mut ChatEngine,
    classifier: Option<&crate::cognitive::CognitiveWeights>,
) -> Result<(), Box<dyn std::error::Error>> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    stream.set_nodelay(true)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;
    let mut line = String::new();
    loop {
        line.clear();
        let n = read_bounded_line(&mut reader, &mut line, MAX_LINE_BYTES)?;
        if n == 0 {
            break;
        }
        if line.len() > MAX_LINE_BYTES {
            send_line(
                &mut writer,
                &json!({"ok": false, "error": "payload too large"}),
            )?;
            break;
        }
        let req: Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(e) => {
                send_line(
                    &mut writer,
                    &json!({"ok": false, "error": format!("bad json: {e}")}),
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
            send_line(&mut writer, &json!({"ok": false, "error": e}))?;
            continue;
        }

        if op == "shutdown" {
            // Fail closed: if token env is set, check_token already required it.
            // If no token configured, refuse shutdown over network for safety.
            if expected_token().is_none() {
                send_line(
                    &mut writer,
                    &json!({"ok": false, "error": "shutdown disabled without PERCI_DAEMON_TOKEN"}),
                )?;
                continue;
            }
            send_line(&mut writer, &json!({"ok": true, "shutdown": true}))?;
            let _ = std::fs::remove_file(marker_path());
            std::process::exit(0);
        }
        let reply = respond(&op, &req, engine, classifier);
        send_line(&mut writer, &reply)?;
    }
    Ok(())
}

fn respond(
    op: &str,
    req: &Value,
    engine: &mut ChatEngine,
    classifier: Option<&crate::cognitive::CognitiveWeights>,
) -> Value {
    let text = req
        .get("text")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    match op {
        "ping" => json!({
            "ok": true,
            "service": "perci-daemon",
            "version": env!("CARGO_PKG_VERSION"),
            "build_id": crate::branding::build_id(),
            "fabric": "v0.7.0"
        }),
        "ask" => {
            if text.is_empty() {
                return json!({"ok": false, "error": "ask requires text"});
            }
            if text.len() > 32 * 1024 {
                return json!({"ok": false, "error": "ask text too long"});
            }
            let stateless = req.get("stateless").and_then(Value::as_bool) == Some(true);
            let result = if stateless {
                stateless_engine()
                    .map_err(|e| e.to_string())
                    .and_then(|mut fresh| fresh.respond(text).map_err(|e| e.to_string()))
            } else {
                engine.respond(text).map_err(|e| e.to_string())
            };
            match result {
                Ok(r) => json!({
                    "ok": true,
                    "text": r.text,
                    "route": format!("{:?}", r.route),
                    "stateless": stateless
                }),
                Err(e) => json!({"ok": false, "error": e}),
            }
        }
        "classify" => {
            if text.is_empty() {
                return json!({"ok": false, "error": "classify requires text"});
            }
            match classify_json_value(text, classifier) {
                Ok(v) => json!({"ok": true, "result": v}),
                Err(e) => json!({"ok": false, "error": e}),
            }
        }
        // Probe / surgical isolation: clear dialogue history only.
        "session_reset" | "session_clear" => match engine.clear_dialogue_history() {
            Ok(()) => json!({"ok": true, "session_reset": true}),
            Err(e) => json!({"ok": false, "error": e.to_string()}),
        },
        "" => json!({"ok": false, "error": "missing op"}),
        other => json!({"ok": false, "error": format!("unknown op: {other}")}),
    }
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

/// `PERCI_DATA/daemon.json`, written by a running daemon so clients know to try it.
pub fn marker_path() -> PathBuf {
    crate::paths::data_file("daemon.json")
}

fn write_marker(bind: &str) {
    let path = marker_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(
        &path,
        json!({
            "addr": bind,
            "pid": std::process::id(),
            "build_id": crate::branding::build_id(),
        })
        .to_string(),
    );
}

/// True when a daemon might be listening: its marker exists or an address
/// was configured explicitly. Avoids a TCP connect to a closed port on every
/// `perci ask` — on Windows that alone costs about 2 s per attempt.
pub fn may_be_running() -> bool {
    env::var_os("PERCI_DAEMON_PORT").is_some()
        || env::var_os("PERCI_DAEMON_HOST").is_some()
        || marker_path().is_file()
}

/// Daemon state for `perci ask`: `None` = not running, `Some(true)` = live and
/// built from this binary, `Some(false)` = live but stale. One connect at most.
pub fn ping_state() -> Option<bool> {
    if !may_be_running() {
        return None;
    }
    match request("ping", None) {
        Ok(v) if v.get("ok").and_then(|x| x.as_bool()) == Some(true) => {
            let remote = v.get("build_id").and_then(|x| x.as_str()).unwrap_or("");
            Some(!remote.is_empty() && remote == crate::branding::build_id())
        }
        Err(_) => {
            // A daemon that was killed leaves its marker behind; drop it so the
            // next call does not probe again.
            let _ = std::fs::remove_file(marker_path());
            None
        }
        _ => None,
    }
}

fn connect(addr: &str) -> Result<TcpStream, String> {
    use std::net::ToSocketAddrs;
    let timeout = Duration::from_millis(
        env::var("PERCI_DAEMON_CONNECT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300),
    );
    let mut last = format!("daemon connect {addr}: no address");
    for sock in addr
        .to_socket_addrs()
        .map_err(|e| format!("daemon connect {addr}: {e}"))?
    {
        match TcpStream::connect_timeout(&sock, timeout) {
            Ok(stream) => {
                let _ = stream.set_nodelay(true);
                return Ok(stream);
            }
            Err(e) => last = format!("daemon connect {addr}: {e}"),
        }
    }
    Err(last)
}

/// Client: send one request, read one response line (includes token if configured).
pub fn request(op: &str, text: Option<&str>) -> Result<Value, String> {
    let addr = addr();
    let mut stream = connect(&addr)?;
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

/// True when a daemon is live **and** its baked build_id matches this binary.
/// Prevents `perci ask` from silently using a stale warm process after code changes.
pub fn ping_current() -> bool {
    match request("ping", None) {
        Ok(v) if v.get("ok").and_then(|x| x.as_bool()) == Some(true) => {
            let remote = v.get("build_id").and_then(|x| x.as_str()).unwrap_or("");
            // Older daemons without build_id are treated as stale.
            !remote.is_empty() && remote == crate::branding::build_id()
        }
        _ => false,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_marker_means_no_connect_attempt() {
        if env::var_os("PERCI_DAEMON_PORT").is_some() || env::var_os("PERCI_DAEMON_HOST").is_some()
        {
            return;
        }
        let _ = std::fs::remove_file(marker_path());
        assert!(!may_be_running());
        assert_eq!(ping_state(), None);
    }

    #[test]
    fn replies_go_out_in_one_write() {
        // B2E-PERCI-012: `writeln!` on a raw TcpStream issued two writes per
        // reply, which with Nagle + delayed ACK cost ~40 ms per round trip.
        struct Counting(Vec<Vec<u8>>);
        impl Write for Counting {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.push(buf.to_vec());
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut w = Counting(Vec::new());
        send_line(&mut w, &json!({"ok": true, "n": 1})).unwrap();
        assert_eq!(w.0.len(), 1);
        assert_eq!(w.0[0], b"{\"n\":1,\"ok\":true}\n".to_vec());
    }

    #[test]
    fn bounded_line_read_stops_at_the_limit() {
        // B2E-PERCI-009: read_line buffered an unbounded line before the size check.
        let huge = vec![b'a'; 4 * 1024 * 1024]; // no newline
        let mut reader = std::io::BufReader::new(&huge[..]);
        let mut line = String::new();
        let n = read_bounded_line(&mut reader, &mut line, 1024).unwrap();
        assert_eq!(n, 1025);
        assert_eq!(line.len(), 1025);

        let mut ok = std::io::BufReader::new(&b"{\"op\":\"ping\"}\nrest"[..]);
        let mut line = String::new();
        assert_eq!(read_bounded_line(&mut ok, &mut line, 1024).unwrap(), 14);
        assert_eq!(line, "{\"op\":\"ping\"}\n");
    }
}
