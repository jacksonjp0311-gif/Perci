//! `perci.recall` (cited memory lookup) and `perci.remember` (staged notes).
//!
//! Every hit carries the file, 1-based line and the SHA-256 of that line's
//! exact bytes, so a harness can later prove the note it cited has not been
//! edited (`perci.verify` with `evidence`). The store behind recall is a
//! [`RecallBackend`]; the default reads Perci's append-only memory log.
//! AgentSearch (FTS5 + evidence hashes) plugs in via [`set_backend`].
//!
//! `perci.remember` never writes into memory directly. It appends to
//! `PERCI_DATA/memory/agent-staged.jsonl` with status `pending_review`;
//! promotion stays human-only. It is off unless `PERCI_TOOLS` enables it.

use super::{arg_str, try_arg, with_summary, Ctx, Tool};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// One cited hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub source: String,
    pub line: u64,
    pub text: String,
    pub sha256: String,
    pub score: u32,
}

/// A searchable store with stable line-level citations.
pub trait RecallBackend: Send + Sync {
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> io::Result<Vec<Hit>>;
    /// Identity of the index state (e.g. sha256 of the store) for receipts.
    fn generation(&self) -> String;
}

fn backend_slot() -> &'static Mutex<Option<Arc<dyn RecallBackend>>> {
    static SLOT: OnceLock<Mutex<Option<Arc<dyn RecallBackend>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Replace the recall store (e.g. with an AgentSearch client).
pub fn set_backend(backend: Arc<dyn RecallBackend>) {
    *backend_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(backend);
}

pub fn backend() -> Arc<dyn RecallBackend> {
    backend_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(|| Arc::new(MemoryLog::discover()))
}

pub fn memory_path() -> PathBuf {
    std::env::var_os("PERCI_MEMORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::paths::data_file("memory/perci.jsonl"))
}

pub fn staged_path() -> PathBuf {
    crate::paths::data_file("memory/agent-staged.jsonl")
}

/// Default backend: Perci's JSONL memory log, term-overlap scoring.
pub struct MemoryLog {
    pub path: PathBuf,
}

impl MemoryLog {
    pub fn discover() -> Self {
        Self {
            path: memory_path(),
        }
    }
}

const STOP: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "of", "to", "in", "on", "and", "or", "my", "i", "me",
    "what", "where", "did", "do", "you", "that", "this", "it", "for", "with", "recall", "remember",
    "memory", "note", "notes",
];

fn terms(q: &str) -> Vec<String> {
    let mut t: Vec<String> = q
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 2 && !STOP.contains(w))
        .map(str::to_owned)
        .collect();
    t.sort();
    t.dedup();
    t
}

/// Text of one JSONL memory record (or the raw line for legacy entries).
fn record_text(line: &str) -> String {
    serde_json::from_str::<Value>(line)
        .ok()
        .and_then(|v| v.get("text").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| line.to_owned())
}

pub fn line_sha256(line: &str) -> String {
    crate::doctor::sha256_hex(line.trim_end_matches(['\r', '\n']).as_bytes())
}

impl RecallBackend for MemoryLog {
    fn name(&self) -> &str {
        "perci-memory-log"
    }
    fn search(&self, query: &str, limit: usize) -> io::Result<Vec<Hit>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let q = terms(query);
        let mut hits: Vec<Hit> = text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .filter_map(|(i, l)| {
                let body = record_text(l);
                let words = terms(&body);
                let score = q
                    .iter()
                    .filter(|t| {
                        words
                            .iter()
                            .any(|w| w == *t || (t.len() >= 4 && w.starts_with(t.as_str())))
                    })
                    .count() as u32;
                (score > 0).then(|| Hit {
                    source: self.path.display().to_string(),
                    line: i as u64 + 1,
                    text: body,
                    sha256: line_sha256(l),
                    score,
                })
            })
            .collect();
        // Most terms first, then newest (higher line) first: deterministic.
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(b.line.cmp(&a.line)));
        hits.truncate(limit);
        Ok(hits)
    }
    fn generation(&self) -> String {
        match std::fs::read(&self.path) {
            Ok(b) => format!("sha256:{}", crate::doctor::sha256_hex(&b)),
            Err(_) => "empty".into(),
        }
    }
}

/// Re-hash a cited line (or a whole file when `line` is `None`).
/// `Ok(Some(true))` = unchanged, `Ok(Some(false))` = changed, `Ok(None)` = gone.
pub fn verify_citation(path: &Path, line: Option<u64>, sha256: &str) -> io::Result<Option<bool>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let actual = match line {
        None => crate::doctor::sha256_hex(&bytes),
        Some(n) => {
            let text = String::from_utf8_lossy(&bytes);
            match text.lines().nth(n.saturating_sub(1) as usize) {
                Some(l) if n >= 1 => line_sha256(l),
                _ => return Ok(None),
            }
        }
    };
    Ok(Some(actual.eq_ignore_ascii_case(sha256.trim())))
}

pub struct Recall;

impl Tool for Recall {
    fn name(&self) -> &'static str {
        "perci.recall"
    }
    fn title(&self) -> &'static str {
        "Cited memory lookup"
    }
    fn description(&self) -> &'static str {
        "Search the user's Perci memory and return matching notes, each with file, line and SHA-256 of the exact line, so answers can cite them and later prove they were not edited. pass = hits found, abstain = nothing relevant. Never invents notes."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 50, "default": 8}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let query = try_arg!(arg_str(args, "query"));
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(8)
            .clamp(1, 50) as usize;
        let be = backend();
        let hits = match be.search(query, limit) {
            Ok(h) => h,
            Err(e) => return Outcome::error("RECALL_FAILED", e.to_string()),
        };
        let mut outcome = Outcome::new(
            if hits.is_empty() { Status::Abstain } else { Status::Pass },
            with_summary(
                json!({
                    "backend": be.name(),
                    "hits": hits.iter().map(|h| json!({
                        "source": h.source, "line": h.line, "text": h.text, "sha256": h.sha256, "score": h.score
                    })).collect::<Vec<_>>(),
                    "complete": true,
                }),
                if hits.is_empty() { "no matching note".to_owned() } else { format!("{} cited note(s)", hits.len()) },
            ),
        )
        .confidence(Confidence::Bounded);
        if hits.is_empty() {
            outcome = outcome.reason(Reason::new(
                "NO_MATCH",
                "nothing in memory matches; do not invent one",
            ));
        }
        for h in &hits {
            outcome = outcome.evidence(
                Evidence::new("memory_note", h.source.clone())
                    .hashed(h.sha256.clone())
                    .line(h.line),
            );
        }
        outcome
            .weights
            .insert("recall_index".into(), be.generation());
        outcome
    }
}

pub struct Remember;

const MAX_NOTE_CHARS: usize = 1200;

impl Tool for Remember {
    fn name(&self) -> &'static str {
        "perci.remember"
    }
    fn title(&self) -> &'static str {
        "Stage a note for review"
    }
    fn description(&self) -> &'static str {
        "Stage a note for the user to review. Never written into memory directly: it goes to a pending-review file and only the user can promote it. Refuses anything that looks like a secret. Off by default (enable with PERCI_TOOLS)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text", "source"],
            "properties": {
                "text": {"type": "string", "maxLength": MAX_NOTE_CHARS},
                "source": {"type": "string", "description": "Who is staging it, e.g. \"claude-code session 42\"."},
                "kind": {"enum": ["note", "teach"], "default": "note"}
            },
            "additionalProperties": false
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn idempotent(&self) -> bool {
        false
    }
    fn default_enabled(&self) -> bool {
        false
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let text = try_arg!(arg_str(args, "text")).trim();
        let source = try_arg!(arg_str(args, "source")).trim();
        let kind = args.get("kind").and_then(Value::as_str).unwrap_or("note");
        if !matches!(kind, "note" | "teach") {
            return Outcome::error("BAD_ARGS", "kind must be note or teach");
        }
        if text.chars().count() > MAX_NOTE_CHARS {
            return Outcome::fail(json!({"status": "refused_too_long"})).reason(Reason::new(
                "TOO_LONG",
                format!("notes are capped at {MAX_NOTE_CHARS} characters"),
            ));
        }
        if crate::learning::looks_sensitive(text) {
            return Outcome::fail(json!({"status": "refused_sensitive"})).reason(Reason::new(
                "SENSITIVE",
                "looks like a credential or secret; not stored",
            ));
        }
        let path = staged_path();
        let record = json!({
            "schema": "perci.staged-note.v1",
            "ts": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            "kind": kind,
            "source": source,
            "text": text,
            "status": "pending_review",
        });
        let line = record.to_string();
        let write = (|| -> io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            f.write_all(format!("{line}\n").as_bytes())
        })();
        if let Err(e) = write {
            return Outcome::error("WRITE_FAILED", e.to_string());
        }
        let sha = line_sha256(&line);
        Outcome::pass(with_summary(
            json!({"status": "pending_review", "staged_path": path.display().to_string(), "sha256": sha}),
            "staged for the user's review",
        ))
        .evidence(Evidence::new("memory_note", path.display().to_string()).hashed(sha))
        .nondeterministic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_log_hits_are_cited_and_verifiable() {
        let dir = std::env::temp_dir().join(format!("perci-recall-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("perci.jsonl");
        std::fs::write(
            &path,
            "{\"schema_version\":1,\"timestamp_unix\":1,\"kind\":\"note\",\"text\":\"backup drive is in the left desk drawer\"}\n\
             {\"schema_version\":1,\"timestamp_unix\":2,\"kind\":\"note\",\"text\":\"dentist on friday\"}\n",
        )
        .unwrap();
        let log = MemoryLog { path: path.clone() };
        let hits = log.search("where is my backup drive", 8).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 1);
        assert!(hits[0].text.contains("left desk drawer"));
        assert_eq!(
            verify_citation(&path, Some(1), &hits[0].sha256).unwrap(),
            Some(true)
        );
        // Edit the note: the citation no longer verifies.
        std::fs::write(&path, "{\"text\":\"backup drive is in the car\"}\n").unwrap();
        assert_eq!(
            verify_citation(&path, Some(1), &hits[0].sha256).unwrap(),
            Some(false)
        );
        assert_eq!(
            verify_citation(&path, Some(9), &hits[0].sha256).unwrap(),
            None
        );
        assert!(log.search("quantum chromodynamics", 8).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
