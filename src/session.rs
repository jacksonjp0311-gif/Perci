//! Persistent multi-turn session for CLI `ask` / `chat`.
//!
//! Default path: `memory/session.jsonl` or `$PERCI_SESSION`.
//! Survives process exits so `perci ask` has continuity.

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const MAX_LOAD: usize = 48;

#[derive(Debug, Clone)]
pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn discover() -> Self {
        let path = env::var_os("PERCI_SESSION")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("memory/session.jsonl"));
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
        let meta = fs::metadata(&self.path)?;
        if meta.len() < 200_000 {
            return Ok(());
        }
        let turns = self.load_recent()?;
        let mut f = fs::File::create(&self.path)?;
        let keep: Vec<_> = if turns.len() > MAX_LOAD {
            turns[turns.len() - MAX_LOAD..].to_vec()
        } else {
            turns
        };
        for (u, a) in keep {
            let line = serde_json::json!({"user": u, "assistant": a});
            writeln!(f, "{line}")?;
        }
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
}
