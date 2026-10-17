//! `perci mcp`: Model Context Protocol server over stdio.
//!
//! JSON-RPC 2.0, one message per line on stdin/stdout; logs go to stderr
//! only. Lifecycle: `initialize` → `notifications/initialized` →
//! `tools/list` / `tools/call` (plus `ping`). Each tool is listed with
//! `inputSchema`, `outputSchema` (the verdict schema) and annotations;
//! `tools/call` returns the verdict as `structuredContent` and a one-line
//! summary in `content[0].text` for clients that only read text.
//!
//! Requests are handled in order on one thread. Weights and the policy load
//! once (prewarmed at start), so every call after the first is warm.

use super::{enabled_from_env, resolve, run_recorded, wire_name, Ctx};
use crate::verdict::Status;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const MAX_MESSAGE_BYTES: usize = 256 * 1024;

/// Server state across one session.
pub struct Server {
    ctx: Ctx,
    tools: Vec<std::sync::Arc<dyn super::Tool>>,
    initialized: std::sync::atomic::AtomicBool,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self {
            ctx: Ctx::new(),
            tools: enabled_from_env(),
            initialized: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    fn tool_list(&self) -> Value {
        let schema = crate::verdict::json_schema();
        json!({
            "tools": self.tools.iter().map(|t| json!({
                "name": wire_name(t.name()),
                "title": t.title(),
                "description": format!("{} [{}]", t.description(), t.name()),
                "inputSchema": t.input_schema(),
                "outputSchema": schema,
                "annotations": {
                    "title": t.title(),
                    "readOnlyHint": t.read_only(),
                    "idempotentHint": t.idempotent(),
                    "destructiveHint": t.destructive(),
                    "openWorldHint": false
                }
            })).collect::<Vec<_>>()
        })
    }

    /// Handle one parsed message. `None` for notifications (no reply).
    pub fn handle(&self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        if msg.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return id.map(|id| error(id, -32600, "invalid request: jsonrpc must be \"2.0\""));
        }
        // Notifications carry no id and never get a reply.
        let Some(id) = id else {
            if method == "notifications/initialized" {
                self.initialized
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            return None;
        };
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        Some(match method {
            "initialize" => {
                let requested = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let version = if PROTOCOL_VERSIONS.contains(&requested) {
                    requested
                } else {
                    PROTOCOL_VERSIONS[0]
                };
                ok(
                    id,
                    json!({
                        "protocolVersion": version,
                        "capabilities": {"tools": {"listChanged": false}},
                        "serverInfo": {"name": "perci", "title": "Perci", "version": env!("CARGO_PKG_VERSION")},
                        "instructions": "Perci is a fast, deterministic checker. Call verify (or check_math) on any answer with numbers before trusting it; policy_check or agent_gate_step before running shell commands or writing files; exact for arithmetic; relation for what-follows-from-these-premises; recall for the user's notes with citations. This session's calls are recorded in a hash-chained step ledger: after running tests yourself, report them with agent_gate_step phase post (or ledger_append) so sequence rules such as deploy-after-tests can see them; next_step suggests what usually comes next. Every result is a perci.verdict.v1: pass / fail / abstain with reasons and a hashed receipt. On fail, fix what the reasons say. On abstain, Perci could not check it: do not present it as verified."
                    }),
                )
            }
            "ping" => ok(id, json!({})),
            "tools/list" => ok(id, self.tool_list()),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                match resolve(&self.tools, name) {
                    None => error(id, -32602, &format!("unknown tool: {name}")),
                    Some(tool) => {
                        let verdict = run_recorded(&self.ctx, tool.as_ref(), &args);
                        ok(
                            id,
                            json!({
                                "content": [{"type": "text", "text": verdict.summary()}],
                                "structuredContent": verdict.to_value(),
                                "isError": verdict.status == Status::Error,
                            }),
                        )
                    }
                }
            }
            "resources/list" => ok(id, json!({"resources": []})),
            "prompts/list" => ok(id, json!({"prompts": []})),
            other => error(id, -32601, &format!("method not found: {other}")),
        })
    }
}

fn ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// Serve on the given reader/writer until EOF.
pub fn serve<R: BufRead, W: Write>(reader: &mut R, writer: &mut W) -> std::io::Result<()> {
    serve_with(Server::new(), reader, writer)
}

/// Serve with a prepared server (e.g. one whose memory is warming up).
pub fn serve_with<R: BufRead, W: Write>(
    server: Server,
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<()> {
    // Verifying the routing pack hashes every byte of it (~65 ms for v0.2),
    // so do it beside the request loop: `initialize` and `tools/list` answer
    // at once, and a `route` call waits on the pack lock until it is ready.
    std::thread::scope(|s| {
        s.spawn(|| server.ctx.prewarm());
        serve_loop(&server, reader, writer)
    })
}

fn serve_loop<R: BufRead, W: Write>(
    server: &Server,
    reader: &mut R,
    writer: &mut W,
) -> std::io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = crate::daemon::read_bounded_line(reader, &mut line, MAX_MESSAGE_BYTES)?;
        if n == 0 {
            return Ok(());
        }
        if line.len() > MAX_MESSAGE_BYTES {
            crate::daemon::send_line(
                writer,
                &error(Value::Null, -32600, "message too large (max 256 KiB)"),
            )?;
            // Skip the rest of the oversized line.
            let mut sink = Vec::new();
            reader.read_until(b'\n', &mut sink)?;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                crate::daemon::send_line(
                    writer,
                    &error(Value::Null, -32700, &format!("parse error: {e}")),
                )?;
                continue;
            }
        };
        let replies: Vec<Value> = match &msg {
            // JSON-RPC batch (older protocol revisions).
            Value::Array(items) => items.iter().filter_map(|m| server.handle(m)).collect(),
            m => server.handle(m).into_iter().collect(),
        };
        if msg.is_array() && !replies.is_empty() {
            crate::daemon::send_line(writer, &Value::Array(replies))?;
        } else {
            for r in replies {
                crate::daemon::send_line(writer, &r)?;
            }
        }
    }
}

/// `perci mcp` entry point.
pub fn run_stdio() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    eprintln!(
        "perci mcp: ready ({} tools, verdict {})",
        enabled_from_env().len(),
        crate::verdict::SCHEMA
    );
    let server = Server::new();
    // Record every verdict of this session in a hash-chained run ledger:
    // PERCI_RUN_ID, else a fresh id. PERCI_LEDGER=off disables it.
    let run = server.ctx.set_run_from_env().or_else(|| {
        if std::env::var("PERCI_LEDGER").is_ok_and(|v| v == "off" || v == "0") {
            return None;
        }
        let id = format!(
            "mcp-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            std::process::id()
        );
        server.ctx.set_run(&id).ok().map(|_| id)
    });
    if let Some(r) = &run {
        eprintln!(
            "perci mcp: run ledger {}",
            super::ledger::run_path(r).display()
        );
    }
    // Sync memory notes and start AgentSearch in the background so the first
    // recall is warm. Derived files only (notes/, agentsearch/ next to the
    // journal); PERCI_MCP_NO_PREWARM=1 skips it.
    if server.tools.iter().any(|t| t.name() == "perci.recall")
        && std::env::var_os("PERCI_MCP_NO_PREWARM").is_none()
    {
        server.ctx.prewarm_memory();
    }
    serve_with(server, &mut reader, &mut writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(lines: &[Value]) -> Vec<Value> {
        let input: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let mut reader = std::io::BufReader::new(input.as_bytes());
        let mut out = Vec::new();
        serve(&mut reader, &mut out).unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn lifecycle_list_and_call() {
        let replies = session(&[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "exact", "arguments": {"expr": "17% of 240", "claimed": "41"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "nope"}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "ping"}),
        ]);
        assert_eq!(
            replies.len(),
            5,
            "notification must not be answered: {replies:?}"
        );
        assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
        let tools = replies[1]["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        for n in [
            "exact",
            "check_math",
            "relation",
            "route",
            "policy_check",
            "recall",
            "verify",
            "status",
            "agent_check_claims",
            "agent_gate_step",
        ] {
            assert!(names.contains(&n), "{n} missing from {names:?}");
        }
        for t in tools {
            assert!(t["inputSchema"]["type"] == "object");
            assert!(t["outputSchema"]["required"].is_array());
        }
        let call = &replies[2]["result"];
        assert_eq!(call["structuredContent"]["status"], "fail");
        assert_eq!(call["isError"], false);
        assert!(call["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("EXACT_MISMATCH"));
        crate::verdict::validate(&call["structuredContent"]).unwrap();
        assert_eq!(replies[3]["error"]["code"], -32601);
        assert_eq!(replies[4]["result"], json!({}));
    }

    #[test]
    fn bad_input_gets_errors_not_crashes() {
        let input = "not json\n{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/call\",\"params\":{\"name\":\"missing\"}}\n";
        let mut reader = std::io::BufReader::new(input.as_bytes());
        let mut out = Vec::new();
        serve(&mut reader, &mut out).unwrap();
        let replies: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(replies[0]["error"]["code"], -32700);
        assert_eq!(replies[1]["error"]["code"], -32602);
    }
}
