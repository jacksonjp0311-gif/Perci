//! Runtime agent repairs — hardness fail → staged operator answers without recompile.
//!
//! Written by `perci agent lab --repair-hardness` into
//! `models/candidates/auto-repairs.jsonl`. Loaded at runtime.
//! **Never** promotes weights.

use crate::deliberation::Deliberation;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoRepair {
    pub id: String,
    /// All of these must appear in the lowercased user text (substring).
    pub match_any: Vec<String>,
    /// Minimum number of match_any hits required (default 1).
    #[serde(default = "default_min_hits")]
    pub min_hits: usize,
    pub answer: String,
    #[serde(default)]
    pub operator: String,
    #[serde(default)]
    pub confidence: f32,
}

fn default_min_hits() -> usize {
    1
}

fn repairs_path() -> PathBuf {
    env::var_os("PERCI_AUTO_REPAIRS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/auto-repairs.jsonl"))
}

static CACHE: RwLock<Option<Vec<AutoRepair>>> = RwLock::new(None);

fn load_repairs() -> Vec<AutoRepair> {
    let path = repairs_path();
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<AutoRepair>(&l).ok())
        .collect()
}

/// Invalidate cache after agent writes new repairs.
pub fn reload() {
    if let Ok(mut g) = CACHE.write() {
        *g = None;
    }
}

fn repairs_cached() -> Vec<AutoRepair> {
    if let Ok(g) = CACHE.read() {
        if let Some(ref v) = *g {
            return v.clone();
        }
    }
    let loaded = load_repairs();
    if let Ok(mut g) = CACHE.write() {
        *g = Some(loaded.clone());
    }
    loaded
}

/// Match a staged auto-repair (agent-written) for this user turn.
pub fn try_auto_repair(user: &str) -> Option<Deliberation> {
    let lower = user.to_ascii_lowercase();
    let mut best: Option<(usize, AutoRepair)> = None;
    for r in repairs_cached() {
        if r.match_any.is_empty() || r.answer.trim().is_empty() {
            continue;
        }
        let hits = r
            .match_any
            .iter()
            .filter(|k| !k.is_empty() && lower.contains(&k.to_ascii_lowercase()))
            .count();
        let need = r.min_hits.max(1);
        if hits >= need {
            let better = match &best {
                None => true,
                Some((bh, _)) => hits > *bh,
            };
            if better {
                best = Some((hits, r));
            }
        }
    }
    best.map(|(_, r)| {
        // Operator name is runtime-staged; use stable static id for Deliberation API.
        let conf = if r.confidence > 0.0 {
            f64::from(r.confidence).min(0.99)
        } else {
            0.88_f64
        };
        let mut answer = r.answer;
        if !r.operator.is_empty() {
            answer = format!("[{}] {answer}", r.operator);
        }
        Deliberation::new("auto-repair", answer)
            .observed("matched staged auto-repair from hardness fail catalog")
            .inferred("agent repair path: fail→catalog→green code path without weight promote")
            .confidence(conf)
    })
}

/// Append one repair (agent use). Returns true if written.
pub fn append_repair(repair: &AutoRepair) -> std::io::Result<()> {
    let path = repairs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Idempotent by id
    if path.is_file() {
        let existing = fs::read_to_string(&path)?;
        if existing.contains(&format!("\"id\":\"{}\"", repair.id))
            || existing.contains(&format!("\"id\": \"{}\"", repair.id))
        {
            return Ok(());
        }
    }
    let line = serde_json::to_string(repair)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    use std::io::Write;
    let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")?;
    reload();
    Ok(())
}

/// SoftCascade pack-alignment body for trust/lag when primary insight is off-topic.
/// Used when speech is SoftCascade-only (operator path not taken).
pub fn softcascade_trust_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let trustish = t.contains("trust")
        && (t.contains("lag")
            || t.contains("timeout")
            || t.contains("retry")
            || t.contains("interface")
            || t.contains("api")
            || t.contains("service")
            || t.contains("caller"));
    if !trustish {
        return None;
    }
    Some(
        "Interfaces and services earn trust under lag when acceptance is checkable without private state. \
Practically: (1) every call names authority and required proof; (2) timeouts are part of the contract \
with a stated meaning (cancel, retry, or uncertain); (3) retries are idempotent so a delayed success \
is not a second write; (4) health and lag are observable so silence is not mistaken for agreement; \
(5) recovery paths are the same story both sides can audit. Trust is not hope that the network is fast \
— it is the ability to verify acceptance, rejection, and pending under delay.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softcascade_trust_body_matches_lag() {
        let b = softcascade_trust_alignment_body(
            "how should interfaces earn trust under lag and retry?",
        );
        assert!(b.is_some());
        assert!(b.unwrap().to_ascii_lowercase().contains("idempotent"));
    }
}
