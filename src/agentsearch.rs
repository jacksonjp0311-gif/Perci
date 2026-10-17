//! Client for AgentSearch (github.com/jacksonjp0311-gif/AgentSearch) over its
//! JSON-lines stdio protocol.
//!
//! AgentSearch is a separate, zero-dependency Python tool: a SQLite FTS5
//! (trigram) literal-text index over authorized root folders that returns
//! line-level matches with a whole-file `content_sha256`. Perci talks to one
//! long-lived child process (`python -m agentsearch --config C --db D stdio`),
//! spawned lazily, so a chat that never recalls never starts Python.
//!
//! Everything AgentSearch returns is **data**: file text is untrusted and is
//! never routed into commands.
//!
//! Command selection: `PERCI_AGENTSEARCH` holds the command prefix, e.g.
//! `py -3.12 -m agentsearch` or `/opt/venv/bin/python -m agentsearch`
//! (`off` disables AgentSearch). Default: `python3 -m agentsearch` (Unix) or
//! `python -m agentsearch` (Windows).

use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

/// Per-request deadline. A `fresh` grep includes an index refresh.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// AgentSearch rejects requests over 64 KiB; stay well below.
const MAX_PATTERN_CHARS: usize = 4096;

/// The command prefix used to start AgentSearch, or `None` when disabled.
pub fn command_prefix() -> Option<Vec<String>> {
    match std::env::var("PERCI_AGENTSEARCH") {
        Ok(value) if value.trim().eq_ignore_ascii_case("off") || value.trim() == "0" => None,
        Ok(value) if !value.trim().is_empty() => {
            Some(value.split_whitespace().map(str::to_owned).collect())
        }
        _ => Some(if cfg!(windows) {
            vec!["python".into(), "-m".into(), "agentsearch".into()]
        } else {
            vec!["python3".into(), "-m".into(), "agentsearch".into()]
        }),
    }
}

/// One matching line inside one file.
#[derive(Clone, Debug, Deserialize)]
pub struct LineMatch {
    pub line: u64,
    #[serde(default)]
    pub column: u64,
    pub text: String,
}

/// One file returned by `grep`, with its whole-file SHA-256 evidence hash.
#[derive(Clone, Debug, Deserialize)]
pub struct FileHit {
    pub path: String,
    #[serde(default)]
    pub matches: Vec<LineMatch>,
    #[serde(default)]
    pub content_sha256: Option<String>,
    #[serde(default)]
    pub changed_since_index: bool,
}

/// Result of one literal `grep` request.
#[derive(Clone, Debug, Default)]
pub struct GrepResult {
    pub files: Vec<FileHit>,
    pub complete: bool,
    pub warnings: Vec<String>,
}

/// A long-lived AgentSearch child process speaking JSON lines.
pub struct AgentSearchClient {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<io::Result<String>>,
    next_id: u64,
    version: Option<String>,
}

impl AgentSearchClient {
    /// Write the root-scoped config (when missing or for other roots) and start
    /// the stdio server. Fails if Python or the `agentsearch` package is missing.
    pub fn spawn(config: &Path, db: &Path, roots: &[PathBuf]) -> io::Result<Self> {
        let prefix = command_prefix().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "AgentSearch disabled (PERCI_AGENTSEARCH=off)",
            )
        })?;
        if let Some(parent) = config.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !config_matches(config, roots) {
            let mut cmd = base_command(&prefix, config, db);
            cmd.arg("config");
            for root in roots {
                std::fs::create_dir_all(root)?;
                cmd.arg("--root").arg(root);
            }
            let out = cmd.stdin(Stdio::null()).output()?;
            if !out.status.success() {
                return Err(io::Error::other(format!(
                    "agentsearch config failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                )));
            }
        }
        let mut child = base_command(&prefix, config, db)
            .arg("stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("agentsearch stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("agentsearch stdout unavailable"))?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });
        let mut client = Self {
            child,
            stdin,
            lines: rx,
            next_id: 0,
            version: None,
        };
        let status = client.request(json!({"op": "status"}))?;
        if status.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(io::Error::other(format!(
                "agentsearch status failed: {status}"
            )));
        }
        client.version = status
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_owned);
        Ok(client)
    }

    /// AgentSearch package version reported by `status` (e.g. `1.0.0rc5`).
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Send one request object (an `id` is added) and wait for its reply.
    pub fn request(&mut self, mut body: Value) -> io::Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        body["id"] = json!(id);
        let line = serde_json::to_string(&body).map_err(io::Error::other)?;
        writeln!(self.stdin, "{line}")?;
        self.stdin.flush()?;
        loop {
            let reply = match self.lines.recv_timeout(REQUEST_TIMEOUT) {
                Ok(reply) => reply?,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "agentsearch did not answer in time",
                    ))
                }
            };
            let value: Value = serde_json::from_str(reply.trim()).map_err(io::Error::other)?;
            if value.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(value);
            }
        }
    }

    /// Literal (non-regex) content search inside `scope`.
    pub fn grep(
        &mut self,
        pattern: &str,
        scope: &Path,
        limit: u32,
        fresh: bool,
    ) -> io::Result<GrepResult> {
        let pattern: String = pattern.chars().take(MAX_PATTERN_CHARS).collect();
        let reply = self.request(json!({
            "op": "grep",
            "pattern": pattern,
            "scope": scope.to_string_lossy(),
            "limit": limit.clamp(1, 200),
            "per_file": 3,
            "fresh": fresh,
        }))?;
        if reply.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(io::Error::other(format!(
                "agentsearch grep failed: {}",
                reply.get("error").unwrap_or(&reply)
            )));
        }
        let files: Vec<FileHit> = reply
            .get("files")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(io::Error::other)?
            .unwrap_or_default();
        let warnings = reply
            .get("warnings")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|w| {
                        w.as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| w.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(GrepResult {
            files,
            complete: reply
                .get("complete")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            warnings,
        })
    }

    /// Fresh whole-file SHA-256 check (`verify_evidence`).
    pub fn verify(&mut self, path: &Path, sha256: &str) -> io::Result<bool> {
        let reply = self.request(json!({
            "op": "verify",
            "path": path.to_string_lossy(),
            "sha256": sha256,
        }))?;
        reply
            .get("verified")
            .and_then(Value::as_bool)
            .ok_or_else(|| io::Error::other(format!("agentsearch verify failed: {reply}")))
    }
}

impl AgentSearchClient {
    /// Kill the child process (tests use this to simulate a crash).
    #[doc(hidden)]
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for AgentSearchClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn base_command(prefix: &[String], config: &Path, db: &Path) -> Command {
    let mut cmd = Command::new(&prefix[0]);
    cmd.args(&prefix[1..])
        .arg("--config")
        .arg(config)
        .arg("--db")
        .arg(db);
    cmd
}

/// True when `config` exists and lists exactly `roots` (compared after
/// canonicalisation), so the one-shot `config` call is skipped on later runs.
fn config_matches(config: &Path, roots: &[PathBuf]) -> bool {
    let Ok(text) = std::fs::read_to_string(config) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    let Some(listed) = value.get("roots").and_then(Value::as_array) else {
        return false;
    };
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let mut have: Vec<PathBuf> = listed
        .iter()
        .filter_map(Value::as_str)
        .map(|s| canon(Path::new(s)))
        .collect();
    let mut want: Vec<PathBuf> = roots.iter().map(|p| canon(p)).collect();
    have.sort();
    want.sort();
    have == want
}
