use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Simple append-only local memory. Each memory is one escaped line so a failed
/// write cannot corrupt earlier entries.
#[derive(Clone, Debug)]
pub struct MemoryStore { path: PathBuf }

impl MemoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self { Self { path: path.into() } }

    pub fn append(&self, text: &str) -> io::Result<()> {
        if let Some(parent) = self.path.parent() { fs::create_dir_all(parent)?; }
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let safe = text.replace('\\', "\\\\").replace('\n', "\\n");
        writeln!(file, "{safe}")
    }

    pub fn search(&self, query: &str, limit: usize) -> io::Result<Vec<String>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(v) => v,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let terms: Vec<String> = query.split_whitespace().map(|s| s.to_ascii_lowercase()).collect();
        let mut scored: Vec<(usize, String)> = text.lines().filter_map(|line| {
            let decoded = line.replace("\\n", "\n").replace("\\\\", "\\");
            let lower = decoded.to_ascii_lowercase();
            let score = terms.iter().filter(|term| lower.contains(term.as_str())).count();
            (score > 0).then_some((score, decoded))
        }).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(scored.into_iter().take(limit).map(|(_, value)| value).collect())
    }

    pub fn path(&self) -> &Path { &self.path }
}
