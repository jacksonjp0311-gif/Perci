//! Decision-trace ledger (context-graph lite).
//!
//! Append-only JSONL of high-salience cognitive decisions for agent planning
//! and human audit. Never mutates weights.

use crate::deliberation::Deliberation;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default path under models/candidates (agent-writable, not weight pack).
pub fn default_path() -> PathBuf {
    env::var_os("PERCI_DECISION_TRACE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/decision-trace.jsonl"))
}

/// High-salience if a program ran, or operator is not pure social/greeting.
pub fn is_high_salience(d: &Deliberation) -> bool {
    if d.program_id.is_some() {
        return true;
    }
    !matches!(
        d.operator,
        "social-reflex" | "dialogue-act" | "associative-response"
    ) || d.operator.contains("synthesis")
        || d.operator.contains("plan")
        || d.operator.contains("code")
        || d.operator.contains("math")
        || d.operator.contains("exact")
        || d.operator.contains("impasse")
        || d.operator.contains("agent")
}

/// Append one decision line. Best-effort; never fails the chat path.
pub fn append(user: &str, d: &Deliberation) {
    if !is_high_salience(d) {
        return;
    }
    let path = default_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let program = d.program_id.unwrap_or("");
    let steps = d.program_steps.join("→");
    let critic = match d.critic_ok {
        Some(true) => "pass",
        Some(false) => "flags",
        None => "n/a",
    };
    let user_esc = json_escape(&truncate(user, 200));
    let op_esc = json_escape(d.operator);
    let line = format!(
        "{{\"ts\":{ts},\"operator\":{op_esc},\"program_id\":{},\"steps\":{},\"critic\":{},\"confidence\":{:.3},\"user\":{user_esc}}}\n",
        json_escape(program),
        json_escape(&steps),
        json_escape(critic),
        d.confidence,
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Read last N traces for agent lab / CLI.
pub fn recent(limit: usize) -> io::Result<Vec<String>> {
    let path = default_path();
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_owned())
        .collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    Ok(lines)
}

pub fn status_label() -> String {
    let path = default_path();
    match recent(500) {
        Ok(rows) => format!("{} events · {}", rows.len(), path.display()),
        Err(_) => format!("unavailable · {}", path.display()),
    }
}

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_owned()
    } else {
        t.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::Deliberation;

    #[test]
    fn high_salience_for_programs() {
        let mut d = Deliberation::new("open-domain-synthesis", "x");
        d.program_id = Some("cross_domain_synthesis");
        assert!(is_high_salience(&d));
        let social = Deliberation::new("social-reflex", "hi");
        assert!(!is_high_salience(&social));
    }
}
