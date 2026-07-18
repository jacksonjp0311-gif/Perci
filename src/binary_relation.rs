//! Compact binary topic/relation memory for native response selection.
//!
//! The relation field is intentionally smaller than a language model. It
//! stores hashed co-occurrence edges between content words in a user prompt
//! and content words in the reviewed response. At inference it scores a
//! candidate continuation; it does not synthesize text or claim a semantic
//! world model. The artifact is versioned, mmap-able, and integer-only.

use memmap2::{Mmap, MmapOptions};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCREL1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 48;
const RECORD_SIZE: usize = 20;
const MAX_COUNT: u16 = 255;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RelationKey {
    source: u64,
    target: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BinaryRelationStats {
    pub records: usize,
    pub source_bytes: u64,
    pub file_bytes: u64,
}

pub fn default_weight_path() -> PathBuf {
    env::var_os("PERCI_RELATION_WEIGHTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/perci-relations-v0.1.brel"))
}

#[derive(Debug)]
pub struct BinaryRelationField {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
    source_bytes: u64,
}

impl BinaryRelationField {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(invalid("binary relation file has an invalid header"));
        }
        if read_u32(&data, 8)? != VERSION {
            return Err(invalid("unsupported binary relation version"));
        }
        let record_count = read_u32(&data, 12)? as usize;
        let source_bytes = read_u64(&data, 16)?;
        let records_offset = read_u64(&data, 24)? as usize;
        if records_offset != HEADER_SIZE
            || records_offset
                .checked_add(record_count.saturating_mul(RECORD_SIZE))
                != Some(data.len())
        {
            return Err(invalid("binary relation section offsets are inconsistent"));
        }
        Ok(Self {
            path,
            data,
            record_count,
            source_bytes,
        })
    }

    pub fn discover() -> io::Result<Option<Self>> {
        let path = default_weight_path();
        if !path.is_file() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn file_bytes(&self) -> usize {
        self.data.len()
    }

    pub fn stats(&self) -> BinaryRelationStats {
        BinaryRelationStats {
            records: self.record_count,
            source_bytes: self.source_bytes,
            file_bytes: self.data.len() as u64,
        }
    }

    /// Score prompt/response topic relations. The cap keeps this a bounded
    /// reranking signal and prevents dense prompts from dominating generation.
    pub fn score(&self, user: &str, response: &str) -> i64 {
        let sources = content_tokens(user);
        let targets = content_tokens(response);
        let mut score = 0u32;
        for source in sources {
            for target in &targets {
                if source == *target {
                    continue;
                }
                if let Some(count) = self.lookup(RelationKey {
                    source: stable_hash(&source),
                    target: stable_hash(target),
                }) {
                    score = score.saturating_add(count.min(8) as u32);
                }
            }
        }
        score.min(255) as i64
    }

    fn lookup(&self, wanted: RelationKey) -> Option<u16> {
        let mut low = 0usize;
        let mut high = self.record_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let offset = HEADER_SIZE + mid * RECORD_SIZE;
            let actual = RelationKey {
                source: u64::from_le_bytes(self.data[offset..offset + 8].try_into().ok()?),
                target: u64::from_le_bytes(self.data[offset + 8..offset + 16].try_into().ok()?),
            };
            match actual.cmp(&wanted) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => {
                    return Some(u16::from_le_bytes(
                        self.data[offset + 16..offset + 18].try_into().ok()?,
                    ));
                }
            }
        }
        None
    }
}

#[derive(Debug, Default)]
pub struct BinaryRelationTrainer {
    edges: BTreeMap<RelationKey, u16>,
    source_bytes: u64,
}

impl BinaryRelationTrainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn train_pair(&mut self, user: &str, response: &str) {
        self.source_bytes = self
            .source_bytes
            .saturating_add(user.len() as u64 + response.len() as u64);
        let sources = content_tokens(user);
        let targets = content_tokens(response);
        for source in sources {
            for target in &targets {
                if source == *target {
                    continue;
                }
                let key = RelationKey {
                    source: stable_hash(&source),
                    target: stable_hash(target),
                };
                let value = self.edges.entry(key).or_default();
                *value = value.saturating_add(1).min(MAX_COUNT);
            }
        }
    }

    pub fn write(&self, path: &Path) -> io::Result<BinaryRelationStats> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let mut header = [0u8; HEADER_SIZE];
        header[..8].copy_from_slice(MAGIC);
        header[8..12].copy_from_slice(&VERSION.to_le_bytes());
        header[12..16].copy_from_slice(&(self.edges.len() as u32).to_le_bytes());
        header[16..24].copy_from_slice(&self.source_bytes.to_le_bytes());
        header[24..32].copy_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
        file.write_all(&header)?;
        for (key, count) in &self.edges {
            file.write_all(&key.source.to_le_bytes())?;
            file.write_all(&key.target.to_le_bytes())?;
            file.write_all(&count.to_le_bytes())?;
            file.write_all(&0u16.to_le_bytes())?;
        }
        file.flush()?;
        let file_bytes = file.metadata()?.len();
        Ok(BinaryRelationStats {
            records: self.edges.len(),
            source_bytes: self.source_bytes,
            file_bytes,
        })
    }
}

pub fn train_source(source: &str, output: &Path) -> io::Result<BinaryRelationStats> {
    let mut trainer = BinaryRelationTrainer::new();
    let source_path = Path::new(source);
    if source == "--repo" || source == "--all" {
        train_directory(Path::new("."), &mut trainer)?;
    } else if source_path.is_dir() {
        train_directory(source_path, &mut trainer)?;
    } else {
        train_file(source_path, &mut trainer)?;
    }
    trainer.write(output)
}

pub fn status_report() -> String {
    let path = default_weight_path();
    match BinaryRelationField::load(&path) {
        Ok(field) => format!(
            "native binary relation field\n  path: {}\n  records: {}\n  source bytes: {}\n  mapped: {:.2} MiB",
            path.display(),
            field.record_count(),
            field.source_bytes,
            field.file_bytes() as f64 / (1024.0 * 1024.0),
        ),
        Err(_) => format!(
            "native binary relation field\n  state: not trained\n  path: {}\n  next: perci language train --repo",
            path.display()
        ),
    }
}

fn train_directory(path: &Path, trainer: &mut BinaryRelationTrainer) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            if child.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            train_directory(&child, trainer)?;
        } else if is_text_source(&child) {
            train_file(&child, trainer)?;
        }
    }
    Ok(())
}

fn train_file(path: &Path, trainer: &mut BinaryRelationTrainer) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(&line) {
            let user = first_string(&value, &["prompt", "user", "input", "question"]);
            let response = first_string(&value, &["response", "assistant", "output", "answer"]);
            if let (Some(user), Some(response)) = (user, response) {
                trainer.train_pair(user, response);
            }
        }
    }
    Ok(())
}

fn first_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    let object = value.as_object()?;
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
}

fn is_text_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("jsonl") | Some("json") | Some("txt")
    )
}

fn content_tokens(text: &str) -> HashSet<String> {
    const STOP: &[&str] = &[
        "a", "about", "an", "and", "answer", "as", "at", "can", "connect", "does",
        "for", "from", "give", "how", "i", "if", "imagine", "in", "is", "it", "me",
        "of", "one", "or", "reflect", "the", "then", "this", "to", "what", "when",
        "which", "why", "with", "without", "you", "your",
    ];
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|token| token.len() > 2 && !STOP.contains(&token.as_str()))
        .collect()
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn read_u32(data: &[u8], offset: usize) -> io::Result<u32> {
    data.get(offset..offset + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| invalid("binary relation header is truncated"))
}

fn read_u64(data: &[u8], offset: usize) -> io::Result<u64> {
    data.get(offset..offset + 8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| invalid("binary relation header is truncated"))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::{BinaryRelationField, BinaryRelationTrainer};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn relation_round_trip_scores_grounded_pair() {
        let mut trainer = BinaryRelationTrainer::new();
        trainer.train_pair(
            "What does geometry teach about boundaries?",
            "Geometry makes the boundary measurable through exchange.",
        );
        let path = std::env::temp_dir().join(format!(
            "perci-relation-{}-{}.brel",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        trainer.write(&path).unwrap();
        let field = BinaryRelationField::load(&path).unwrap();
        assert!(field.score("Explain geometry boundaries", "The boundary exchanges state") > 0);
        assert_eq!(field.score("Explain music", "The boundary exchanges state"), 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn relation_hashing_is_deterministic() {
        let mut left = BinaryRelationTrainer::new();
        let mut right = BinaryRelationTrainer::new();
        left.train_pair("geometry boundary", "exchange structure");
        right.train_pair("geometry boundary", "exchange structure");
        assert_eq!(left.edges, right.edges);
    }
}
