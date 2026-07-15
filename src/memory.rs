use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub schema_version: u8,
    pub timestamp_unix: u64,
    pub kind: String,
    pub text: String,
}

/// Append-only JSONL memory.
///
/// Legacy escaped-line entries remain readable so existing local memory is not
/// discarded during the v0.1.1 migration.
#[derive(Clone, Debug)]
pub struct MemoryStore {
    path: PathBuf,
}

impl MemoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn append(&self, text: &str) -> io::Result<()> {
        self.append_kind("note", text)
    }

    pub fn append_kind(&self, kind: &str, text: &str) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let record = MemoryRecord {
            schema_version: 1,
            timestamp_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            kind: kind.trim().to_owned(),
            text: text.trim().to_owned(),
        };

        let encoded = serde_json::to_string(&record).map_err(json_error)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{encoded}")
    }

    pub fn search(&self, query: &str, limit: usize) -> io::Result<Vec<String>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };

        let terms = search_terms(query);
        let mut scored: Vec<(usize, u64, String)> = text
            .lines()
            .filter_map(parse_record)
            .filter_map(|record| {
                let lower = record.text.to_ascii_lowercase();
                let score = if terms.is_empty() {
                    1
                } else {
                    terms
                        .iter()
                        .filter(|term| lower.contains(term.as_str()))
                        .count()
                };

                (score > 0).then_some((score, record.timestamp_unix, record.text))
            })
            .collect();

        scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(_, _, value)| value)
            .collect())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn parse_record(line: &str) -> Option<MemoryRecord> {
    if line.trim().is_empty() {
        return None;
    }

    if let Ok(record) = serde_json::from_str::<MemoryRecord>(line) {
        return Some(record);
    }

    // Backward-compatible reader for v0.1 escaped-line memory.
    let decoded = line.replace("\\n", "\n").replace("\\\\", "\\");
    Some(MemoryRecord {
        schema_version: 0,
        timestamp_unix: 0,
        kind: "legacy".to_owned(),
        text: decoded,
    })
}

fn search_terms(query: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "recall", "remember", "memory", "search", "find", "the", "that", "what", "did", "you",
    ];

    query
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|value| value.len() >= 2)
        .map(str::to_ascii_lowercase)
        .filter(|value| !STOP_WORDS.contains(&value.as_str()))
        .collect()
}

fn json_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip_preserves_literal_backslash_n() {
        let record = MemoryRecord {
            schema_version: 1,
            timestamp_unix: 1,
            kind: "note".to_owned(),
            text: r"literal \n remains literal".to_owned(),
        };
        let encoded = serde_json::to_string(&record).unwrap();
        let decoded: MemoryRecord = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.text, record.text);
    }

    #[test]
    fn recall_prefix_terms_are_removed() {
        assert_eq!(
            search_terms("recall governed local memory"),
            vec!["governed".to_owned(), "local".to_owned()]
        );
    }
}
