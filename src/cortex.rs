use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

struct CortexProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl CortexProcess {
    fn spawn(bridge: &CortexBridge) -> io::Result<Self> {
        let daemon = bridge
            .host_root
            .join("scripts")
            .join("perci_cortex_daemon.py");

        if !daemon.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Cortex daemon not found: {}", daemon.display()),
            ));
        }

        let mut child = Command::new(&bridge.python)
            .arg("-u")
            .arg(&daemon)
            .arg("--engine-root")
            .arg(&bridge.engine_root)
            .arg("--home")
            .arg(&bridge.home)
            .arg("--repo")
            .arg(&bridge.repository)
            .current_dir(&bridge.host_root)
            .env("PYTHONPATH", &bridge.engine_root)
            .env("CORTEX_HOME", &bridge.home)
            .env("PYTHONUTF8", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            io::Error::new(io::ErrorKind::BrokenPipe, "Cortex daemon stdin unavailable")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Cortex daemon stdout unavailable",
            )
        })?;

        let mut process = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };

        process.request(&json!({"operation": "ping"}))?;
        Ok(process)
    }

    fn request(&mut self, request: &Value) -> io::Result<Value> {
        serde_json::to_writer(&mut self.stdin, request).map_err(json_error)?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;

        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line)?;
        if bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Cortex daemon closed its output stream",
            ));
        }

        let response: Value = serde_json::from_str(&line).map_err(json_error)?;
        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            let message = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown Cortex daemon error");
            return Err(io::Error::new(io::ErrorKind::Other, message));
        }

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }
}

impl Drop for CortexProcess {
    fn drop(&mut self) {
        let _ = self.request(&json!({"operation": "shutdown"}));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Clone)]
struct CacheEntry {
    created: Instant,
    context: Vec<String>,
}

pub struct CortexBridge {
    host_root: PathBuf,
    engine_root: PathBuf,
    python: PathBuf,
    home: PathBuf,
    repository: String,
    process: Option<CortexProcess>,
    cache: HashMap<String, CacheEntry>,
    cache_order: VecDeque<String>,
    cache_ttl: Duration,
    cache_limit: usize,
}

impl std::fmt::Debug for CortexBridge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CortexBridge")
            .field("host_root", &self.host_root)
            .field("engine_root", &self.engine_root)
            .field("python", &self.python)
            .field("home", &self.home)
            .field("repository", &self.repository)
            .field("daemon_warm", &self.process.is_some())
            .field("cache_entries", &self.cache.len())
            .finish()
    }
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
        let cache_seconds = env::var("PERCI_CORTEX_CACHE_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(45);
        let cache_limit = env::var("PERCI_CORTEX_CACHE_ITEMS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(24)
            .clamp(1, 256);

        Some(Self {
            host_root,
            engine_root,
            python,
            home,
            repository,
            process: None,
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            cache_ttl: Duration::from_secs(cache_seconds),
            cache_limit,
        })
    }

    pub fn ready(&self) -> bool {
        self.host_root.join(".cortex").join("config.json").is_file()
            && self.home.join("cortex.db").is_file()
    }

    pub fn status_label(&self) -> String {
        if self.ready() {
            let runtime = if self.process.is_some() {
                "warm daemon"
            } else {
                "lazy daemon"
            };
            return format!(
                "attached | repo {} | {} | cache {}",
                self.repository,
                runtime,
                self.cache.len()
            );
        }

        "engine found | bootstrap required".to_owned()
    }

    pub fn retrieve(&mut self, task: &str, budget: usize) -> io::Result<Vec<String>> {
        if !self.ready() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Cortex is present but not bootstrapped for Perci",
            ));
        }

        let key = cache_key(task, budget);
        if let Some(entry) = self.cache.get(&key) {
            if entry.created.elapsed() <= self.cache_ttl {
                return Ok(entry.context.clone());
            }
        }

        self.cache.remove(&key);
        self.cache_order.retain(|item| item != &key);

        let request = json!({
            "operation": "protocol",
            "task": task,
            "budget": budget,
        });

        let value = self.daemon_request(&request).or_else(|_| {
            self.run_json(&[
                "protocol".to_owned(),
                "--repo".to_owned(),
                self.repository.clone(),
                "--task".to_owned(),
                task.to_owned(),
                "--budget".to_owned(),
                budget.to_string(),
                "--json".to_owned(),
            ])
        })?;

        let context = extract_context(&value);
        self.insert_cache(key, context.clone());
        Ok(context)
    }

    pub fn remember(&mut self, kind: &str, text: &str) -> io::Result<()> {
        if !self.ready() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Cortex is present but not bootstrapped for Perci",
            ));
        }

        self.clear_cache();
        let request = json!({
            "operation": "remember",
            "kind": kind,
            "text": text,
        });

        self.daemon_request(&request).or_else(|_| {
            self.run_json(&[
                "remember".to_owned(),
                "--repo".to_owned(),
                self.repository.clone(),
                "--kind".to_owned(),
                kind.to_owned(),
                "--text".to_owned(),
                text.to_owned(),
                "--json".to_owned(),
            ])
        })?;

        Ok(())
    }

    fn daemon_request(&mut self, request: &Value) -> io::Result<Value> {
        if self.process.is_none() {
            self.process = Some(CortexProcess::spawn(self)?);
        }

        let first = self
            .process
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Cortex daemon unavailable"))?
            .request(request);

        if first.is_ok() {
            return first;
        }

        self.process = None;
        self.process = Some(CortexProcess::spawn(self)?);
        self.process
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Cortex daemon unavailable"))?
            .request(request)
    }

    fn run_json(&self, arguments: &[String]) -> io::Result<Value> {
        fs::create_dir_all(&self.home)?;

        let output = Command::new(&self.python)
            .current_dir(&self.engine_root)
            .env("PYTHONPATH", &self.engine_root)
            .env("CORTEX_HOME", &self.home)
            .env("PYTHONUTF8", "1")
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

        serde_json::from_slice(&output.stdout).map_err(json_error)
    }

    fn insert_cache(&mut self, key: String, context: Vec<String>) {
        while self.cache_order.len() >= self.cache_limit {
            if let Some(oldest) = self.cache_order.pop_front() {
                self.cache.remove(&oldest);
            }
        }

        self.cache_order.push_back(key.clone());
        self.cache.insert(
            key,
            CacheEntry {
                created: Instant::now(),
                context,
            },
        );
    }

    fn clear_cache(&mut self) {
        self.cache.clear();
        self.cache_order.clear();
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

fn cache_key(task: &str, budget: usize) -> String {
    let normalized = task
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ");
    format!("{budget}:{normalized}")
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
                "[Cortex evidence: {path}:{start}-{end} | {short_hash}] {}",
                text.trim()
            ));

            if output.len() >= 9 {
                return output;
            }
        }
    }

    output
}

fn json_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
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
                "path": "knowledge/packs/perci-core-intelligence-v1/evidence.md",
                "line_range": [1, 8],
                "content_hash": "1234567890abcdef",
                "text": "Evidence outranks recollection."
            }],
            "support_evidence": []
        });

        let context = extract_context(&packet);
        assert!(context[0].contains("read_only"));
        assert!(context[1].contains("evidence.md:1-8"));
        assert!(context[1].contains("1234567890ab"));
    }

    #[test]
    fn cache_keys_are_normalized() {
        assert_eq!(
            cache_key("  Explain   Counterexamples ", 800),
            "800:explain counterexamples"
        );
    }
}
