//! `perci.recall` (cited memory lookup) and `perci.remember` (append a note).
//!
//! Both go through [`CitedMemory`] (Track B): the append-only journal is the
//! source of truth, one immutable note file per record is indexed by
//! AgentSearch, and recall falls back to a substring scan when AgentSearch is
//! not available. The tool result embeds the `perci.recall.v1` document (minus
//! timing, so the output hash is stable); every hit carries a [`Citation`]
//! that `perci.verify` re-checks against the note file *and* the journal.
//!
//! `perci.remember` is off unless `PERCI_TOOLS` enables it, refuses anything
//! that looks like a secret and caps notes at 1200 characters.

use super::{arg_str, opt_str, try_arg, with_summary, Ctx, Tool};
use crate::cited_memory::{Citation, CitedMemory};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};
use std::io;
use std::path::{Path, PathBuf};

/// The memory journal (`PERCI_MEMORY`, else `PERCI_DATA/memory/perci.jsonl`).
pub fn memory_path() -> PathBuf {
    crate::cited_memory::default_journal()
}

pub fn line_sha256(line: &str) -> String {
    crate::doctor::sha256_hex(line.trim_end_matches(['\r', '\n']).as_bytes())
}

/// Verdict evidence for one citation: the journal record (source of truth).
pub fn citation_evidence(c: &Citation) -> Evidence {
    Evidence::new("memory_record", format!("memory:{}", c.journal_line))
        .hashed(c.record_sha256.clone())
        .line(c.journal_line)
}

/// Re-check a perci.recall.v1 citation through [`CitedMemory::verify`].
pub fn verify_cited(ctx: &Ctx, c: &Citation) -> io::Result<crate::cited_memory::VerifyResult> {
    ctx.memory(|m: &mut CitedMemory| m.verify(c))
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
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let query = try_arg!(arg_str(args, "query"));
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(8)
            .clamp(1, 50) as usize;
        let res = match ctx.memory(|m| m.recall(query, limit)) {
            Ok(r) => r,
            Err(e) => return Outcome::error("RECALL_FAILED", e.to_string()),
        };
        let mut doc = serde_json::to_value(&res).unwrap_or(Value::Null);
        if let Some(obj) = doc.as_object_mut() {
            // Timing lives in the receipt; keep the hashed result stable.
            obj.remove("elapsed_us");
        }
        let n = res.hits.len();
        let summary = match (n, res.complete) {
            (0, true) => "no matching note".to_owned(),
            (0, false) => "no match found, but the search was partial".to_owned(),
            (n, _) => format!("{n} cited note(s) via {}", res.backend),
        };
        let mut outcome = Outcome::new(
            if n == 0 {
                Status::Abstain
            } else {
                Status::Pass
            },
            with_summary(doc, summary),
        )
        .confidence(Confidence::Bounded);
        if n == 0 {
            outcome = outcome.reason(if res.complete {
                Reason::new("NO_MATCH", "nothing in memory matches; do not invent one")
            } else {
                Reason::new(
                    "INCOMPLETE",
                    "the search reported partial coverage, so absence is not proof",
                )
            });
        }
        for w in &res.warnings {
            outcome = outcome.reason(Reason::new("RECALL_WARNING", w.clone()));
        }
        for h in &res.hits {
            if h.changed_since_index {
                outcome = outcome.reason(Reason::new(
                    "NOTE_CHANGED",
                    format!(
                        "{} changed after it was indexed; verify before citing",
                        h.citation.note_path
                    ),
                ));
            }
            outcome = outcome.evidence(citation_evidence(&h.citation));
        }
        let journal_bytes = std::fs::metadata(memory_path())
            .map(|m| m.len())
            .unwrap_or(0);
        outcome.weights.insert(
            "recall_index".into(),
            format!("{}; journal_bytes={journal_bytes}", res.backend),
        );
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
        "Remember a note"
    }
    fn description(&self) -> &'static str {
        "Append a note to the user's Perci memory journal and return its citation (journal line, sha256). Refuses anything that looks like a secret; 1200 characters max. Off by default (the operator enables it with PERCI_TOOLS)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string", "maxLength": MAX_NOTE_CHARS},
                "source": {"type": "string", "description": "Who is writing it, e.g. \"claude-code session 42\" (echoed in the result, not stored)."},
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
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let text = try_arg!(arg_str(args, "text")).trim();
        let source = try_arg!(opt_str(args, "source")).unwrap_or("").trim();
        let kind = args.get("kind").and_then(Value::as_str).unwrap_or("note");
        if !matches!(kind, "note" | "teach") {
            return Outcome::error("BAD_ARGS", "kind must be note or teach");
        }
        if text.is_empty() {
            return Outcome::error("BAD_ARGS", "text is empty");
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
        let receipt = match ctx.memory(|m| m.remember(kind, text)) {
            Ok(r) => r,
            Err(e) => return Outcome::error("WRITE_FAILED", e.to_string()),
        };
        let ev = citation_evidence(&receipt.citation);
        Outcome::pass(with_summary(
            json!({
                "schema": crate::cited_memory::SCHEMA,
                "status": receipt.status,
                "kind": kind,
                "source": source,
                "citation": receipt.citation,
            }),
            format!("stored as memory line {}", receipt.citation.journal_line),
        ))
        .evidence(ev)
        .nondeterministic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_ctx(tag: &str) -> (Ctx, PathBuf) {
        let dir = std::env::temp_dir().join(format!("perci-recall-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let journal = dir.join("perci.jsonl");
        (
            Ctx::with_memory(CitedMemory::new(&journal).without_agentsearch()),
            dir,
        )
    }

    #[test]
    fn remember_then_recall_is_cited_and_verifiable() {
        let (ctx, dir) = temp_ctx("rr");
        for text in [
            "backup drive is in the left desk drawer",
            "dentist on friday",
        ] {
            let v = Remember.call(&ctx, &json!({"text": text, "source": "test"}));
            assert_eq!(v.status, Status::Pass, "{:?}", v.reasons);
        }
        let v = Recall.call(&ctx, &json!({"query": "where is my backup drive"}));
        assert_eq!(v.status, Status::Pass);
        assert_eq!(v.result["schema"], crate::cited_memory::SCHEMA);
        assert!(v.result.get("elapsed_us").is_none());
        let hits = v.result["hits"].as_array().unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0]["text"]
            .as_str()
            .unwrap()
            .contains("left desk drawer"));
        assert_eq!(v.evidence[0].reference, "memory:1");
        let citation: Citation = serde_json::from_value(hits[0]["citation"].clone()).unwrap();
        assert!(verify_cited(&ctx, &citation).unwrap().ok);
        assert_eq!(
            verify_citation(&memory_path_of(&dir), Some(1), &citation.record_sha256).unwrap(),
            Some(true)
        );
        // Rewrite the journal: the citation no longer verifies.
        std::fs::write(
            memory_path_of(&dir),
            "{\"text\":\"backup drive is in the car\"}\n",
        )
        .unwrap();
        assert!(!verify_cited(&ctx, &citation).unwrap().ok);
        let none = Recall.call(&ctx, &json!({"query": "quantum chromodynamics"}));
        assert_eq!(none.status, Status::Abstain);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn remember_refuses_secrets() {
        let (ctx, dir) = temp_ctx("sec");
        let v = Remember.call(
            &ctx,
            &json!({"text": "my api key is sk-abcdefghijklmnopqrstuvwxyz123456"}),
        );
        assert_eq!(v.status, Status::Fail);
        assert!(!memory_path_of(&dir).exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn memory_path_of(dir: &Path) -> PathBuf {
        dir.join("perci.jsonl")
    }
}
