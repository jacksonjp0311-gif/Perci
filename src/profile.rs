//! External profile directory (`PERCI_PROFILE`, default `PERCI_DATA`).
//!
//! A profile holds everything that customizes one installation and must not
//! live in the public repository: a policy overlay, sequence rules, weight
//! packs, memory and run ledgers. `profile.json` (schema `perci.profile.v1`,
//! optional) names the pieces; every path in it is relative to the profile
//! directory unless absolute. Unknown keys are ignored. See `docs/PROFILES.md`.
//!
//! ```json
//! {"schema": "perci.profile.v1", "name": "example",
//!  "policy": "policy.json", "sequence_rules": "sequence-rules.json",
//!  "packs": ["packs/acme-router"], "packs_index": "packs/packs.manifest.json",
//!  "tool_pack": "acme-router", "pins": {"acme-router": "<sha256>"},
//!  "require_pins": true, "memory_journal": "memory/perci.jsonl",
//!  "route": {"min_confidence_permille": 800},
//!  "tools": {"enabled": ["exact", "route", "policy_check"]}}
//! ```

use serde_json::Value;
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "perci.profile.v1";
pub const FILE: &str = "profile.json";

pub fn dir() -> PathBuf {
    crate::paths::profile()
}

/// `profile.json` as written. `Ok(Null)` when absent; invalid JSON or a
/// foreign `schema` is an error.
pub fn config_checked() -> Result<Value, String> {
    let path = dir().join(FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Value::Null),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let v: Value = serde_json::from_str(&text)
        .map_err(|e| format!("{}: not valid JSON ({e})", path.display()))?;
    if !v.is_object() {
        return Err(format!("{}: must be a JSON object", path.display()));
    }
    match v.get("schema").and_then(Value::as_str) {
        None | Some(SCHEMA) => Ok(v),
        Some(other) => Err(format!(
            "{}: schema {other:?} is not {SCHEMA}",
            path.display()
        )),
    }
}

/// [`config_checked`], with errors reported as `Null` (callers that must not
/// fail use this; `perci status` and `perci packs list` show the error).
pub fn config() -> Value {
    config_checked().unwrap_or(Value::Null)
}

/// Resolve a profile-relative path (absolute paths stay as they are).
pub fn resolve(rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        dir().join(p)
    }
}

/// `cfg[key]` as a path, else `default_rel` inside the profile.
pub fn path_of(cfg: &Value, key: &str, default_rel: &str) -> PathBuf {
    match cfg.get(key).and_then(Value::as_str) {
        Some(p) if !p.trim().is_empty() => resolve(p.trim()),
        _ => dir().join(default_rel),
    }
}

/// Whether `key` is set explicitly (then a missing file is an error).
pub fn explicit(cfg: &Value, key: &str) -> bool {
    cfg.get(key)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
}

/// Warn when the profile sits inside a git checkout (private files risk
/// being committed).
pub fn inside_git_checkout() -> Option<PathBuf> {
    let d = dir();
    let mut cur: Option<&Path> = Some(d.as_path());
    while let Some(p) = cur {
        if p.join(".git").exists() {
            return Some(p.to_path_buf());
        }
        cur = p.parent();
    }
    None
}
