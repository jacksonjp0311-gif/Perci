//! Bounded JSONL logs under `PERCI_DATA/logs`.
//!
//! Per-turn telemetry (decision trace, emergence geometry, curriculum
//! candidates, knowledge evidence) is **off by default**; set `PERCI_TRACE=1`
//! to record it. Every appender rotates at `PERCI_LOG_MAX_BYTES` (default
//! 10 MiB) and keeps 3 rotated files (`name.1` … `name.3`).

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024;
pub const KEEP_ROTATED: usize = 3;

/// True when per-turn tracing is enabled (`PERCI_TRACE=1|true|on`).
pub fn tracing_enabled() -> bool {
    matches!(
        env::var("PERCI_TRACE")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}

pub fn max_bytes() -> u64 {
    env::var("PERCI_LOG_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v >= 1024)
        .unwrap_or(DEFAULT_MAX_BYTES)
}

fn rotated(path: &Path, n: usize) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{n}"));
    PathBuf::from(name)
}

/// Shift `path` → `path.1` → … → `path.keep`, dropping the oldest.
pub fn rotate(path: &Path, keep: usize) -> io::Result<()> {
    if keep == 0 {
        return match fs::remove_file(path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let _ = fs::remove_file(rotated(path, keep));
    for n in (1..keep).rev() {
        let from = rotated(path, n);
        if from.exists() {
            fs::rename(&from, rotated(path, n + 1))?;
        }
    }
    if path.exists() {
        fs::rename(path, rotated(path, 1))?;
    }
    Ok(())
}

/// Append one line (a trailing newline is added if missing), rotating first
/// when the file would exceed `limit` bytes.
pub fn append_bounded_with(path: &Path, line: &str, limit: u64, keep: usize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if len > 0 && len + line.len() as u64 + 1 > limit {
        rotate(path, keep)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if line.ends_with('\n') {
        file.write_all(line.as_bytes())
    } else {
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")
    }
}

/// Bounded append with the default limits (for explicit lab/agent receipts).
pub fn append_bounded(path: &Path, line: &str) -> io::Result<()> {
    append_bounded_with(path, line, max_bytes(), KEEP_ROTATED)
}

/// Bounded append that only happens when per-turn tracing is enabled.
/// Best-effort: errors are ignored so logging never breaks a chat turn.
pub fn append_trace(path: &Path, line: &str) {
    if tracing_enabled() {
        let _ = append_bounded(path, line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_caps_size_and_keeps_three_files() {
        let dir = env::temp_dir().join(format!("perci-logs-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("trace.jsonl");
        let line = "x".repeat(99);
        for _ in 0..100 {
            append_bounded_with(&path, &line, 1_000, 3).unwrap();
        }
        let size = |p: &Path| fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        assert!(size(&path) <= 1_000);
        for n in 1..=3 {
            let r = rotated(&path, n);
            assert!(r.is_file(), "missing {}", r.display());
            assert!(size(&r) <= 1_000);
        }
        assert!(!rotated(&path, 4).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn newline_is_added_once() {
        let dir = env::temp_dir().join(format!("perci-logs-nl-{}", std::process::id()));
        let path = dir.join("a.jsonl");
        let _ = fs::remove_dir_all(&dir);
        append_bounded_with(&path, "{\"a\":1}", 10_000, 3).unwrap();
        append_bounded_with(&path, "{\"b\":2}\n", 10_000, 3).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}\n{\"b\":2}\n");
        let _ = fs::remove_dir_all(&dir);
    }
}
