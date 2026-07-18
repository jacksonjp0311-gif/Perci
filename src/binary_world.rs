//! Typed binary world-model field for native Perci cognition.
//!
//! `PERCIWM1` stores compact subject/relation/object edges with a domain,
//! polarity, confidence, and evidence bin.  It is deliberately a bounded
//! reranking memory: it cannot invent text, execute tools, or promote facts by
//! itself.  At inference it scores whether a generated continuation preserves
//! a learned typed relation from the current prompt.

use memmap2::{Mmap, MmapOptions};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCIWM1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
const RECORD_SIZE: usize = 32;
const MAX_COUNT: u16 = 255;

const RELATION_CUES: &[&str] = &[
    "causes", "supports", "maintains", "measures", "exchanges", "separates",
    "changes", "predicts", "tests", "preserves", "teaches", "connects", "relates",
    "defines", "contains", "requires", "repairs", "observes", "organizes",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct WorldKey {
    subject: u64,
    relation: u64,
    object: u64,
    domain: u8,
    polarity: u8,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BinaryWorldStats {
    pub records: usize,
    pub source_bytes: u64,
    pub file_bytes: u64,
}

pub fn default_weight_path() -> PathBuf {
    env::var_os("PERCI_WORLD_WEIGHTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/perci-world-v0.1.bwm"))
}

#[derive(Debug)]
pub struct BinaryWorldModel {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
    source_bytes: u64,
}

impl BinaryWorldModel {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(invalid("binary world-model file has an invalid header"));
        }
        if read_u32(&data, 8)? != VERSION {
            return Err(invalid("unsupported binary world-model version"));
        }
        let record_count = read_u32(&data, 12)? as usize;
        let source_bytes = read_u64(&data, 16)?;
        let records_offset = read_u64(&data, 24)? as usize;
        if records_offset != HEADER_SIZE
            || records_offset
                .checked_add(record_count.saturating_mul(RECORD_SIZE))
                != Some(data.len())
        {
            return Err(invalid("binary world-model section offsets are inconsistent"));
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

    pub fn stats(&self) -> BinaryWorldStats {
        BinaryWorldStats {
            records: self.record_count,
            source_bytes: self.source_bytes,
            file_bytes: self.data.len() as u64,
        }
    }

    /// Score a candidate against prompt-derived typed edges.
    ///
    /// Exact subject/relation/object matches receive the strongest signal;
    /// domain agreement doubles it.  The score is capped so the optional
    /// field cannot overpower direct topic binding or exact operators.
    ///
    /// Entity-slot bonus: when the prompt binds invented surface names to
    /// known motif slots, reward responses that preserve **both** slots
    /// (relation transfer) without requiring the invented name to be parroted.
    pub fn score(&self, user: &str, response: &str) -> i64 {
        let keys = pair_keys(user, response);
        let mut score = 0u32;
        for key in keys {
            let Some((count, confidence, evidence)) = self.lookup(key) else {
                continue;
            };
            let typed = 1u32 + u32::from(key.domain != 0);
            let quality = 1u32 + u32::from(confidence >= 2) + u32::from(evidence >= 2);
            score = score.saturating_add(u32::from(count.min(8)) * typed * quality);
        }
        score = score.saturating_add(entity_slot_bonus(user, response));
        score.min(255) as i64
    }

    fn lookup(&self, wanted: WorldKey) -> Option<(u16, u8, u8)> {
        let mut low = 0usize;
        let mut high = self.record_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let offset = HEADER_SIZE + mid * RECORD_SIZE;
            let actual = WorldKey {
                subject: read_u64_at(&self.data, offset).ok()?,
                relation: read_u64_at(&self.data, offset + 8).ok()?,
                object: read_u64_at(&self.data, offset + 16).ok()?,
                domain: *self.data.get(offset + 24)?,
                polarity: *self.data.get(offset + 25)?,
            };
            match actual.cmp(&wanted) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => {
                    let confidence = *self.data.get(offset + 26)?;
                    let evidence = *self.data.get(offset + 27)?;
                    let count = u16::from_le_bytes(
                        self.data[offset + 28..offset + 30].try_into().ok()?,
                    );
                    return Some((count, confidence, evidence));
                }
            }
        }
        None
    }
}

#[derive(Debug, Default)]
pub struct BinaryWorldTrainer {
    edges: BTreeMap<WorldKey, (u16, u8, u8)>,
    source_bytes: u64,
}

impl BinaryWorldTrainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn train_pair(&mut self, user: &str, response: &str) {
        self.source_bytes = self
            .source_bytes
            .saturating_add(user.len() as u64 + response.len() as u64);
        let domain = domain_id(user, response);
        let polarity = polarity_bin(user, response);
        let confidence = confidence_bin(user, response);
        let evidence = evidence_bin(user, response);
        for key in pair_keys_with_type(user, response, domain, polarity) {
            let entry = self.edges.entry(key).or_insert((0, confidence, evidence));
            entry.0 = entry.0.saturating_add(1).min(MAX_COUNT);
            entry.1 = entry.1.max(confidence);
            entry.2 = entry.2.max(evidence);
        }
    }

    pub fn write(&self, path: &Path) -> io::Result<BinaryWorldStats> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::with_capacity(HEADER_SIZE + self.edges.len() * RECORD_SIZE);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&(self.edges.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.source_bytes.to_le_bytes());
        bytes.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
        bytes.resize(HEADER_SIZE, 0);
        for (key, (count, confidence, evidence)) in &self.edges {
            bytes.extend_from_slice(&key.subject.to_le_bytes());
            bytes.extend_from_slice(&key.relation.to_le_bytes());
            bytes.extend_from_slice(&key.object.to_le_bytes());
            bytes.push(key.domain);
            bytes.push(key.polarity);
            bytes.push(*confidence);
            bytes.push(*evidence);
            bytes.extend_from_slice(&count.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
        }
        let tmp = path.with_extension("bwm.tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        file.write_all(&bytes)?;
        file.flush()?;
        drop(file);
        fs::rename(&tmp, path)?;
        Ok(BinaryWorldStats {
            records: self.edges.len(),
            source_bytes: self.source_bytes,
            file_bytes: bytes.len() as u64,
        })
    }
}

pub fn train_source(source: &str, output: &Path) -> io::Result<BinaryWorldStats> {
    let mut trainer = BinaryWorldTrainer::new();
    let source_path = Path::new(source);
    if source == "--repo" || source == "--all" || source == "repo" {
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
    match BinaryWorldModel::load(&path) {
        Ok(model) => format!(
            "native typed world-model field\n  path: {}\n  records: {}\n  source bytes: {}\n  mapped: {:.2} MiB",
            path.display(),
            model.record_count(),
            model.source_bytes,
            model.file_bytes() as f64 / (1024.0 * 1024.0),
        ),
        Err(_) => format!(
            "native typed world-model field\n  state: not trained\n  path: {}\n  next: perci language train --repo",
            path.display()
        ),
    }
}

fn train_directory(path: &Path, trainer: &mut BinaryWorldTrainer) -> io::Result<()> {
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        if child.is_dir() {
            if child.file_name().and_then(|name| name.to_str()) != Some("target") {
                train_directory(&child, trainer)?;
            }
        } else if is_text_source(&child) {
            train_file(&child, trainer)?;
        }
    }
    Ok(())
}

fn train_file(path: &Path, trainer: &mut BinaryWorldTrainer) -> io::Result<()> {
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default();
    if matches!(extension, "jsonl" | "ndjson") {
        for line in BufReader::new(File::open(path)?).lines() {
            let line = line?;
            if let Ok(value) = serde_json::from_str::<Value>(&line) {
                train_json_value(&value, trainer);
            }
        }
    } else if extension == "json" {
        if let Ok(value) = serde_json::from_str::<Value>(&fs::read_to_string(path)?) {
            train_json_value(&value, trainer);
        }
    } else if matches!(extension, "md" | "txt" | "rs" | "py" | "ps1") {
        let text = fs::read_to_string(path)?;
        for line in text.lines().filter(|line| looks_like_prose(line)) {
            trainer.train_pair(line, line);
        }
    }
    Ok(())
}

fn train_json_value(value: &Value, trainer: &mut BinaryWorldTrainer) {
    match value {
        Value::Object(map) => {
            let user = first_string(value, &["prompt", "user", "input", "question"]);
            let response = first_string(value, &["response", "assistant", "output", "answer"]);
            if let (Some(user), Some(response)) = (user, response) {
                trainer.train_pair(user, response);
            }
            for (key, child) in map {
                if !matches!(key.as_str(), "id" | "source" | "tags" | "created" | "recorded_at")
                    && (child.is_object() || child.is_array())
                {
                    train_json_value(child, trainer);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                train_json_value(item, trainer);
            }
        }
        _ => {}
    }
}

fn first_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    let object = value.as_object()?;
    keys.iter().find_map(|key| object.get(*key).and_then(Value::as_str))
}

fn is_text_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|value| value.to_str()), Some("jsonl") | Some("ndjson") | Some("json") | Some("md") | Some("txt") | Some("rs") | Some("py") | Some("ps1"))
}

/// Reward relation survival under invented entity names (no name-parrot required).
fn entity_slot_bonus(user: &str, response: &str) -> u32 {
    if !crate::entity_slot::looks_entity_slot_transfer(user) {
        return 0;
    }
    let Some(frame) = crate::entity_slot::extract_entity_slot_frame(user) else {
        return 0;
    };
    if crate::entity_slot::slots_bound_in_speech(response, &frame.slot_a, &frame.slot_b) {
        // Strong but capped: cannot dominate exact tools / operator speech alone.
        48
    } else {
        0
    }
}

fn pair_keys(user: &str, response: &str) -> Vec<WorldKey> {
    pair_keys_with_type(user, response, domain_id(user, response), polarity_bin(user, response))
}

fn pair_keys_with_type(user: &str, response: &str, domain: u8, polarity: u8) -> Vec<WorldKey> {
    let relation = relation_token(user, response);
    let subjects = salient_tokens(user);
    let objects = salient_tokens(response);
    let mut keys = Vec::new();
    for subject in subjects.iter().take(8) {
        for object in objects.iter().take(10) {
            if subject == object {
                continue;
            }
            keys.push(WorldKey {
                subject: stable_hash(subject),
                relation: stable_hash(relation),
                object: stable_hash(object),
                domain,
                polarity,
            });
        }
    }
    keys
}

fn salient_tokens(text: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "and", "that", "this", "with", "from", "what", "when", "where", "which",
        "why", "how", "does", "about", "into", "onto", "then", "than", "their", "there",
        "your", "you", "are", "can", "could", "would", "should", "one", "some", "only",
        "between", "without", "while", "because", "have", "has", "for", "not", "is", "it",
    ];
    let mut seen = HashSet::new();
    text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
        .map(str::to_ascii_lowercase)
        .filter(|token| token.len() > 2 && !STOP.contains(&token.as_str()))
        .filter(|token| seen.insert(token.clone()))
        .take(12)
        .collect()
}

fn relation_token(user: &str, response: &str) -> &'static str {
    let lower = format!("{} {}", user, response).to_ascii_lowercase();
    RELATION_CUES
        .iter()
        .copied()
        .find(|cue| lower.split(|ch: char| !ch.is_ascii_alphabetic()).any(|word| word == *cue))
        .unwrap_or("relates")
}

fn domain_id(user: &str, response: &str) -> u8 {
    let lower = format!("{} {}", user, response).to_ascii_lowercase();
    if lower.contains("geometry") || lower.contains("boundary") || lower.contains("shape") {
        1
    } else if lower.contains("life") || lower.contains("death") || lower.contains("biology") {
        2
    } else if lower.contains("language") || lower.contains("word") || lower.contains("meaning") {
        3
    } else if lower.contains("code") || lower.contains("rust") || lower.contains("software") {
        4
    } else if lower.contains("logic") || lower.contains("premise") || lower.contains("contradiction") {
        5
    } else if lower.contains("system") || lower.contains("machine") || lower.contains("architecture") {
        6
    } else if lower.contains("identity") || lower.contains("aware") || lower.contains("conscious") {
        7
    } else {
        0
    }
}

fn polarity_bin(user: &str, response: &str) -> u8 {
    let lower = format!("{} {}", user, response).to_ascii_lowercase();
    if [" not ", "without", "fails", "cannot", "can't", "false"].iter().any(|term| lower.contains(term)) {
        1
    } else {
        0
    }
}

fn confidence_bin(user: &str, response: &str) -> u8 {
    let lower = format!("{} {}", user, response).to_ascii_lowercase();
    if ["evidence", "measure", "test", "reproduce", "observed"].iter().any(|term| lower.contains(term)) {
        3
    } else if ["mechanism", "because", "causes", "supports"].iter().any(|term| lower.contains(term)) {
        2
    } else {
        1
    }
}

fn evidence_bin(user: &str, response: &str) -> u8 {
    let lower = format!("{} {}", user, response).to_ascii_lowercase();
    if ["source", "provenance", "falsifiable", "reproduce"].iter().any(|term| lower.contains(term)) {
        3
    } else if ["evidence", "test", "measure", "observation"].iter().any(|term| lower.contains(term)) {
        2
    } else {
        0
    }
}

fn looks_like_prose(line: &str) -> bool {
    let alphabetic = line.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    alphabetic >= 24 && !line.trim_start().starts_with(['#', '|', '-', '*', '`'])
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
        .ok_or_else(|| invalid("binary world-model header is truncated"))
}

fn read_u64(data: &[u8], offset: usize) -> io::Result<u64> {
    data.get(offset..offset + 8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(|| invalid("binary world-model header is truncated"))
}

fn read_u64_at(data: &[u8], offset: usize) -> io::Result<u64> {
    read_u64(data, offset)
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::{BinaryWorldModel, BinaryWorldTrainer};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn world_model_round_trip_scores_typed_relation() {
        let mut trainer = BinaryWorldTrainer::new();
        trainer.train_pair(
            "What does geometry teach about boundaries?",
            "Geometry measures a boundary through evidence and exchange.",
        );
        let path = std::env::temp_dir().join(format!(
            "perci-world-{}-{}.bwm",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        let stats = trainer.write(&path).unwrap();
        assert!(stats.records > 0);
        let model = BinaryWorldModel::load(&path).unwrap();
        assert!(model.score("Explain geometry boundaries", "Geometry measures a boundary") > 0);
        assert_eq!(model.score("Explain music", "The boundary exchanges state"), 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn typed_edge_is_deterministic() {
        let mut left = BinaryWorldTrainer::new();
        let mut right = BinaryWorldTrainer::new();
        left.train_pair("geometry and life", "a boundary maintains exchange");
        right.train_pair("geometry and life", "a boundary maintains exchange");
        assert_eq!(left.edges, right.edges);
    }
}
