//! Persistent multi-turn session for CLI `ask` / `chat`.
//!
//! Default path: `memory/session.jsonl` or `$PERCI_SESSION`.
//! Survives process exits so `perci ask` has continuity.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Turns loaded into a new process.
const MAX_LOAD: usize = 48;
/// Turns kept on disk; older turns are dropped (A6).
pub const MAX_STORED: usize = 200;
/// Prune once the file holds this many turns beyond the cap, so appends
/// don't rewrite the file every time.
const PRUNE_SLACK: usize = 25;
/// Hard size cap regardless of turn count.
const MAX_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn discover() -> Self {
        let path = env::var_os("PERCI_SESSION")
            .map(PathBuf::from)
            .unwrap_or_else(|| crate::paths::data_file("memory/session.jsonl"));
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_recent(&self) -> io::Result<Vec<(String, String)>> {
        if !self.path.is_file() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&self.path)?;
        let mut turns = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                let u = v
                    .get("user")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let a = v
                    .get("assistant")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if !u.is_empty() && !a.is_empty() {
                    turns.push((u, a));
                }
            }
        }
        if turns.len() > MAX_LOAD {
            turns.drain(0..turns.len() - MAX_LOAD);
        }
        Ok(turns)
    }

    pub fn append(&self, user: &str, assistant: &str) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let line = serde_json::json!({
            "user": user.trim(),
            "assistant": assistant.trim(),
        });
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{line}")?;
        self.prune_if_needed()?;
        Ok(())
    }

    fn prune_if_needed(&self) -> io::Result<()> {
        let raw = fs::read_to_string(&self.path)?;
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        if lines.len() <= MAX_STORED + PRUNE_SLACK && (raw.len() as u64) < MAX_BYTES {
            return Ok(());
        }
        let mut keep = &lines[lines.len().saturating_sub(MAX_STORED)..];
        // Under the byte cap even when individual turns are huge.
        let mut total: u64 = keep.iter().map(|l| l.len() as u64 + 1).sum();
        while total >= MAX_BYTES && keep.len() > 1 {
            total -= keep[0].len() as u64 + 1;
            keep = &keep[1..];
        }
        let tmp = self.path.with_extension("jsonl.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            for line in keep {
                writeln!(f, "{line}")?;
            }
        }
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn clear(&self) -> io::Result<()> {
        if self.path.is_file() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn session_roundtrip() {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("perci_sess_{n}.jsonl"));
        let store = SessionStore { path: path.clone() };
        store.append("hello", "hi there").unwrap();
        store.append("what next", "plan step one").unwrap();
        let t = store.load_recent().unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].0, "hello");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn session_file_is_bounded() {
        // A6: the session file must not grow without bound.
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("perci_sess_cap_{n}.jsonl"));
        let store = SessionStore { path: path.clone() };
        for i in 0..(MAX_STORED + PRUNE_SLACK + 40) {
            store.append(&format!("question {i}"), "answer").unwrap();
        }
        let lines = fs::read_to_string(&path).unwrap().lines().count();
        assert!(lines <= MAX_STORED + PRUNE_SLACK, "{lines} lines kept");
        let recent = store.load_recent().unwrap();
        assert_eq!(recent.len(), MAX_LOAD);
        assert_eq!(
            recent.last().unwrap().0,
            format!("question {}", MAX_STORED + PRUNE_SLACK + 39)
        );
        store.clear().unwrap();
        assert!(store.load_recent().unwrap().is_empty());
    }
}
