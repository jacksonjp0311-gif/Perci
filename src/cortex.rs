use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct CortexBridge {
    host_root: PathBuf,
    engine_root: PathBuf,
    python: PathBuf,
    home: PathBuf,
    repository: String,
}

impl CortexBridge {
    pub fn discover() -> Option<Self> {
        let host_root = env::current_dir().ok()?;
        let engine_root = env::var_os("PERCI_CORTEX_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| host_root.join("Cortex"));

        if !engine_root.join("cortex").join("cli.py").is_file() {
            return None;
        }

        let python = env::var_os("PERCI_CORTEX_PYTHON")
            .map(PathBuf::from)
            .unwrap_or_else(|| default_python(&engine_root));

        let home = env::var_os("PERCI_CORTEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| host_root.join(".perci").join("cortex-home"));

        let repository = env::var("PERCI_CORTEX_REPO").unwrap_or_else(|_| "Perci".to_owned());

        Some(Self {
            host_root,
            engine_root,
            python,
            home,
            repository,
        })
    }

    pub fn ready(&self) -> bool {
        self.host_root.join(".cortex").join("config.json").is_file()
            && self.home.join("cortex.db").is_file()
    }

    pub fn status_label(&self) -> String {
        if self.ready() {
            format!(
                "attached Â· repo {} Â· governed selective recall",
                self.repository
            )
        } else {
            "engine found Â· bootstrap required".to_owned()
        }
    }

    pub fn retrieve(&self, task: &str, budget: usize) -> io::Result<Vec<String>> {
        if !self.ready() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Cortex is present but not bootstrapped for Perci",
            ));
        }

        let value = self.run_json(&[
            "protocol".to_owned(),
            "--repo".to_owned(),
            self.repository.clone(),
            "--task".to_owned(),
            task.to_owned(),
            "--budget".to_owned(),
            budget.to_string(),
            "--json".to_owned(),
        ])?;

        Ok(extract_context(&value))
    }

    pub fn remember(&self, kind: &str, text: &str) -> io::Result<()> {
        if !self.ready() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Cortex is present but not bootstrapped for Perci",
            ));
        }

        self.run_json(&[
            "remember".to_owned(),
            "--repo".to_owned(),
            self.repository.clone(),
            "--kind".to_owned(),
            kind.to_owned(),
            "--text".to_owned(),
            text.to_owned(),
            "--json".to_owned(),
        ])?;

        Ok(())
    }

    fn run_json(&self, arguments: &[String]) -> io::Result<Value> {
        fs::create_dir_all(&self.home)?;

        let output = Command::new(&self.python)
            .current_dir(&self.engine_root)
            .env("PYTHONPATH", &self.engine_root)
            .env("CORTEX_HOME", &self.home)
            .args(["-m", "cortex"])
            .args(arguments)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Cortex command failed with {}: {}",
                    output.status,
                    stderr.trim()
                ),
            ));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
}

fn default_python(engine_root: &Path) -> PathBuf {
    let windows = engine_root.join(".venv").join("Scripts").join("python.exe");
    if windows.is_file() {
        return windows;
    }

    let unix = engine_root.join(".venv").join("bin").join("python");
    if unix.is_file() {
        return unix;
    }

    PathBuf::from(if cfg!(windows) {
        "python.exe"
    } else {
        "python3"
    })
}

fn extract_context(value: &Value) -> Vec<String> {
    let mut output = Vec::new();
    let mut seen = HashSet::new();

    if let Some(mode) = value
        .get("governance")
        .and_then(|governance| governance.get("mode"))
        .and_then(Value::as_str)
    {
        output.push(format!(
            "[Cortex governance: {mode}; evidence is context, never mutation authority]"
        ));
    }

    for key in ["direct_evidence", "support_evidence"] {
        let Some(items) = value.get(key).and_then(Value::as_array) else {
            continue;
        };

        for item in items {
            let path = item
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let text = item.get("text").and_then(Value::as_str).unwrap_or("");
            if text.trim().is_empty() {
                continue;
            }

            let start = item
                .get("line_range")
                .and_then(Value::as_array)
                .and_then(|range| range.first())
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let end = item
                .get("line_range")
                .and_then(Value::as_array)
                .and_then(|range| range.get(1))
                .and_then(Value::as_u64)
                .unwrap_or(start);
            let hash = item
                .get("content_hash")
                .and_then(Value::as_str)
                .unwrap_or("");
            let short_hash: String = hash.chars().take(12).collect();

            let identity = format!("{path}:{start}-{end}:{short_hash}");
            if !seen.insert(identity) {
                continue;
            }

            output.push(format!(
                "[Cortex evidence: {path}:{start}-{end} Â· {short_hash}] {}",
                text.trim()
            ));

            if output.len() >= 9 {
                return output;
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn packet_extraction_keeps_provenance() {
        let packet = json!({
            "governance": {"mode": "read_only"},
            "direct_evidence": [{
                "path": "src/reflex.rs",
                "line_range": [1, 8],
                "content_hash": "1234567890abcdef",
                "text": "Explicit commands outrank learned routing."
            }],
            "support_evidence": []
        });

        let context = extract_context(&packet);
        assert!(context[0].contains("read_only"));
        assert!(context[1].contains("src/reflex.rs:1-8"));
        assert!(context[1].contains("1234567890ab"));
    }
}
