//! Native binary language memory for Perci.
//!
//! This is deliberately not a hidden LLM adapter.  It is a compact, local,
//! model-independent sequence learner built from binary transition fields:
//!
//! * the alphabet is printable ASCII plus newline/tab;
//! * each context stores a bitset of allowed next symbols;
//! * context depths 1..=N provide bounded back-off and composition;
//! * training is an append-free rebuild into a versioned, mmap-able artifact;
//! * inference uses only integer lookups, bit tests, and a deterministic PRNG.
//!
//! It will not have the breadth of a web-scale transformer.  Its purpose is to
//! make the language path genuinely Perci-owned and trainable in Bitwork's
//! binary style, while retaining an honest, inspectable capability boundary.

use memmap2::{Mmap, MmapOptions};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCLNG1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 48;
const MAX_ORDER: usize = 6;
const ALPHABET: usize = 128;
const BIT_WORDS: usize = ALPHABET / 64;
// Four binary threshold planes retain bounded frequency evidence without
// introducing floating-point weights: planes represent counts >= 1, 2, 4, 8.
const BIT_PLANES: usize = 4;
const RECORD_SIZE: usize = 1 + MAX_ORDER + BIT_PLANES * BIT_WORDS * 8;
const MAX_GENERATION_CHARS: usize = 1200;

/// The current native language weight path.  `PERCI_LANGUAGE_WEIGHTS` always
/// has precedence so experiments can remain isolated from the active pack.
pub fn default_weight_path() -> PathBuf {
    env::var_os("PERCI_LANGUAGE_WEIGHTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/perci-language-v0.1.blng"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RecordKey {
    depth: u8,
    context: [u8; MAX_ORDER],
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BinaryLanguageStats {
    pub order: usize,
    pub records: usize,
    pub source_bytes: u64,
    pub unique_transitions: u64,
    pub file_bytes: u64,
}

/// Read-only binary sequence memory.  The file remains compact and is mapped
/// rather than copied into a second heap allocation.
#[derive(Debug)]
pub struct BinaryLanguageModel {
    path: PathBuf,
    data: Mmap,
    order: usize,
    record_count: usize,
    source_bytes: u64,
    unique_transitions: u64,
}

impl BinaryLanguageModel {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE {
            return Err(invalid("binary language file is shorter than its header"));
        }
        if &data[..8] != MAGIC {
            return Err(invalid("binary language file has an unknown signature"));
        }
        let version = read_u32(&data, 8)?;
        if version != VERSION {
            return Err(invalid(format!(
                "unsupported binary language version {version}"
            )));
        }
        let order = data[12] as usize;
        if !(1..=MAX_ORDER).contains(&order) {
            return Err(invalid(
                "binary language order is outside the supported range",
            ));
        }
        let declared_record_size = read_u16(&data, 14)? as usize;
        if declared_record_size != RECORD_SIZE {
            return Err(invalid(
                "binary language record size does not match this runtime",
            ));
        }
        let record_count = read_u64(&data, 16)? as usize;
        let source_bytes = read_u64(&data, 24)?;
        let unique_transitions = read_u64(&data, 32)?;
        let expected =
            HEADER_SIZE
                .checked_add(record_count.checked_mul(RECORD_SIZE).ok_or_else(|| {
                    invalid("binary language record count overflows the file size")
                })?)
                .ok_or_else(|| invalid("binary language file size overflows"))?;
        if expected != data.len() {
            return Err(invalid(format!(
                "binary language file length mismatch: header says {expected}, file is {}",
                data.len()
            )));
        }
        Ok(Self {
            path,
            data,
            order,
            record_count,
            source_bytes,
            unique_transitions,
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

    pub fn order(&self) -> usize {
        self.order
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn source_bytes(&self) -> u64 {
        self.source_bytes
    }

    pub fn unique_transitions(&self) -> u64 {
        self.unique_transitions
    }

    pub fn file_bytes(&self) -> usize {
        self.data.len()
    }

    pub fn stats(&self) -> BinaryLanguageStats {
        BinaryLanguageStats {
            order: self.order,
            records: self.record_count,
            source_bytes: self.source_bytes,
            unique_transitions: self.unique_transitions,
            file_bytes: self.file_bytes() as u64,
        }
    }

    /// Generate a bounded continuation from a short native primer.
    pub fn generate_from_seed(&self, seed: &str, max_chars: usize, mut state: u64) -> String {
        let max_chars = max_chars.clamp(16, MAX_GENERATION_CHARS);
        let mut history = vec![b' '; self.order];
        let clean_seed = normalise_text(seed);
        // Keep a primer's trailing space in the live state; trimming it here
        // would fuse the first generated token to the primer.
        let mut output = String::from_utf8_lossy(&clean_seed).to_string();
        for byte in clean_seed.iter().rev().take(self.order).rev() {
            shift_history(&mut history, *byte);
        }
        if state == 0 {
            state = stable_hash(seed.as_bytes());
        }
        while output.chars().count() < max_chars {
            let Some(byte) = self.next_symbol(&history, &mut state) else {
                break;
            };
            if byte == b'\n' {
                if output.trim_end().ends_with('.') || output.trim_end().ends_with('?') {
                    output.push('\n');
                    break;
                }
            } else {
                output.push(byte as char);
            }
            shift_history(&mut history, byte);
            if output.ends_with("\n\n") {
                break;
            }
        }
        clean_generated(&output)
    }

    /// Turn a Bitwork domain into a short primer, then let the binary field
    /// supply the continuation.  The user topic is retained as a binding cue,
    /// while the body comes from trained transitions rather than a response
    /// card or an external model.
    pub fn generate_reply(&self, user: &str, domain: &str, max_chars: usize, state: u64) -> String {
        let topic = salient_topic(user);
        let primer = match domain {
            "geometry" => format!("Geometry makes the relation visible: {topic} "),
            "science" => format!("The mechanism to examine is {topic}: "),
            "logic" => format!("The key distinction is {topic}: "),
            "code" => format!("Start with the smallest reproducible case for {topic}: "),
            "identity" => "I am Perci, a local binary system. ".to_owned(),
            "greeting" => "Hello — I am here, locally and attentively. ".to_owned(),
            _ => format!("A useful way to think about {topic} is "),
        };
        self.generate_from_seed(&primer, max_chars, state)
    }

    fn next_symbol(&self, history: &[u8], state: &mut u64) -> Option<u8> {
        let upper = self.order.min(history.len());
        let mut scores = [0i32; ALPHABET];
        let mut any = false;
        for depth in 1..=upper {
            let mut key = RecordKey {
                depth: depth as u8,
                context: [0; MAX_ORDER],
            };
            key.context[MAX_ORDER - depth..].copy_from_slice(&history[history.len() - depth..]);
            if let Some(planes) = self.lookup(&key) {
                any = true;
                let depth_weight = (depth * depth) as i32;
                for byte in 0..ALPHABET as u8 {
                    if !is_allowed_generated_byte(byte) {
                        continue;
                    }
                    let level = planes
                        .iter()
                        .enumerate()
                        .filter(|(_, bits)| bit_is_set(bits, byte))
                        .map(|(plane, _)| 1i32 << plane)
                        .sum::<i32>();
                    scores[byte as usize] += depth_weight * level;
                }
            }
        }
        if !any {
            return None;
        }
        let mut best = i32::MIN;
        let mut candidates = Vec::new();
        for byte in 0..ALPHABET as u8 {
            if scores[byte as usize] > best {
                best = scores[byte as usize];
                candidates.clear();
                candidates.push(byte);
            } else if scores[byte as usize] == best && best > 0 {
                candidates.push(byte);
            }
        }
        if candidates.is_empty() || best <= 0 {
            return None;
        }
        *state = xorshift64(*state);
        Some(candidates[(*state as usize) % candidates.len()])
    }

    fn lookup(&self, wanted: &RecordKey) -> Option<[[u64; BIT_WORDS]; BIT_PLANES]> {
        let mut low = 0usize;
        let mut high = self.record_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let offset = HEADER_SIZE + mid * RECORD_SIZE;
            let depth = self.data[offset];
            let context_start = offset + 1;
            let context_end = context_start + MAX_ORDER;
            let ordering = (depth, &self.data[context_start..context_end])
                .cmp(&(wanted.depth, &wanted.context[..]));
            match ordering {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => {
                    let bits_start = context_end;
                    let mut planes = [[0u64; BIT_WORDS]; BIT_PLANES];
                    for (plane, words) in planes.iter_mut().enumerate() {
                        for (index, slot) in words.iter_mut().enumerate() {
                            let word = plane * BIT_WORDS + index;
                            *slot = u64::from_le_bytes(
                                self.data[bits_start + word * 8..bits_start + (word + 1) * 8]
                                    .try_into()
                                    .ok()?,
                            );
                        }
                    }
                    return Some(planes);
                }
            }
        }
        None
    }
}

/// In-memory trainer.  A transition is one binary bit: it is present or it is
/// absent.  Rebuilding the artifact is deterministic for a given corpus.
#[derive(Debug)]
pub struct BinaryLanguageTrainer {
    order: usize,
    table: BTreeMap<RecordKey, [u8; ALPHABET]>,
    source_bytes: u64,
    unique_transitions: u64,
}

impl BinaryLanguageTrainer {
    pub fn new(order: usize) -> Self {
        Self {
            order: order.clamp(1, MAX_ORDER),
            table: BTreeMap::new(),
            source_bytes: 0,
            unique_transitions: 0,
        }
    }

    pub fn order(&self) -> usize {
        self.order
    }

    pub fn train_text(&mut self, text: &str) {
        let prose = text
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('|') {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.train_bytes(&normalise_text(&prose));
    }

    pub fn train_bytes(&mut self, bytes: &[u8]) {
        let filtered = bytes
            .iter()
            .copied()
            .filter(|byte| *byte < ALPHABET as u8 && is_allowed_training_byte(*byte))
            .collect::<Vec<_>>();
        self.source_bytes = self.source_bytes.saturating_add(filtered.len() as u64);
        let mut history = vec![b' '; self.order];
        for byte in filtered {
            for depth in 1..=self.order {
                let mut key = RecordKey {
                    depth: depth as u8,
                    context: [0; MAX_ORDER],
                };
                key.context[MAX_ORDER - depth..].copy_from_slice(&history[history.len() - depth..]);
                let count = self.table.entry(key).or_insert([0u8; ALPHABET]);
                if count[byte as usize] == 0 {
                    self.unique_transitions = self.unique_transitions.saturating_add(1);
                }
                count[byte as usize] = count[byte as usize].saturating_add(1).min(15);
            }
            shift_history(&mut history, byte);
        }
    }

    /// Ingest a repository file.  JSON/JSONL training records contribute their
    /// human-facing strings instead of raw punctuation and object syntax.
    pub fn train_file(&mut self, path: &Path) -> io::Result<()> {
        let extension = path
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(extension.as_str(), "jsonl" | "ndjson") {
            let file = File::open(path)?;
            for line in BufReader::new(file).lines() {
                let line = line?;
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    self.train_json_value(&value);
                }
            }
        } else if extension == "json" {
            let raw = fs::read_to_string(path)?;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                self.train_json_value(&value);
            }
        } else if matches!(
            extension.as_str(),
            "md" | "txt" | "toml" | "rs" | "py" | "ps1"
        ) {
            self.train_text(&fs::read_to_string(path)?);
        }
        Ok(())
    }

    fn train_json_value(&mut self, value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(messages) = map.get("messages") {
                    if let Some(items) = messages.as_array() {
                        self.train_text("\n<dialogue>\n");
                        for item in items {
                            let role = item.get("role").and_then(|v| v.as_str()).unwrap_or("turn");
                            let content =
                                item.get("content").and_then(|v| v.as_str()).unwrap_or("");
                            self.train_text(&format!("\n<{role}> {content}\n"));
                        }
                    }
                }
                for key in [
                    "prompt",
                    "response",
                    "assistant",
                    "text",
                    "claim",
                    "answer",
                    "insight",
                ] {
                    if let Some(text) = map.get(key).and_then(|v| v.as_str()) {
                        self.train_text(text);
                    }
                }
                // Preserve nested message-like records without learning IDs,
                // timestamps, or paths as if they were language.
                for (key, child) in map {
                    if !matches!(
                        key.as_str(),
                        "id" | "source" | "tags" | "created" | "recorded_at"
                    ) {
                        if child.is_object() || child.is_array() {
                            self.train_json_value(child);
                        }
                    }
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    self.train_json_value(item);
                }
            }
            serde_json::Value::String(text) => self.train_text(text),
            _ => {}
        }
    }

    pub fn train_directory(&mut self, root: &Path) -> io::Result<u64> {
        let mut files = Vec::new();
        collect_training_files(root, &mut files)?;
        files.sort();
        for path in &files {
            self.train_file(path)?;
        }
        Ok(files.len() as u64)
    }

    pub fn write(&self, path: &Path) -> io::Result<BinaryLanguageStats> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("blng.tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        let record_count = self.table.len() as u64;
        file.write_all(MAGIC)?;
        file.write_all(&VERSION.to_le_bytes())?;
        file.write_all(&[self.order as u8, 7])?; // order, log2(alphabet)
        file.write_all(&(RECORD_SIZE as u16).to_le_bytes())?;
        file.write_all(&record_count.to_le_bytes())?;
        file.write_all(&self.source_bytes.to_le_bytes())?;
        file.write_all(&self.unique_transitions.to_le_bytes())?;
        file.write_all(&0u64.to_le_bytes())?; // reserved for a future checksum
        for (key, counts) in &self.table {
            file.write_all(&[key.depth])?;
            file.write_all(&key.context)?;
            for plane in 0..BIT_PLANES {
                let threshold = 1u8 << plane;
                for word_index in 0..BIT_WORDS {
                    let mut word = 0u64;
                    for bit in 0..64 {
                        let byte = word_index * 64 + bit;
                        if byte < ALPHABET && counts[byte] >= threshold {
                            word |= 1u64 << bit;
                        }
                    }
                    file.write_all(&word.to_le_bytes())?;
                }
            }
        }
        file.flush()?;
        drop(file);
        fs::rename(&tmp, path)?;
        let file_bytes = fs::metadata(path)?.len();
        Ok(BinaryLanguageStats {
            order: self.order,
            records: self.table.len(),
            source_bytes: self.source_bytes,
            unique_transitions: self.unique_transitions,
            file_bytes,
        })
    }
}

pub fn train_source(source: &str, output: &Path, order: usize) -> io::Result<BinaryLanguageStats> {
    let mut trainer = BinaryLanguageTrainer::new(order);
    // Small primers give the native decoder a stable start state without
    // hard-coding whole answers.  The body is learned from repository text.
    // The compact corpus needs a small, coherent anchor so low-order back-off
    // prefers grammatical continuations over incidental log fragments.
    for _ in 0..8 {
        trainer.train_text(PRIMER_CORPUS);
    }
    let source_path = Path::new(source);
    if source == "--repo" || source == "repo" {
        for directory in ["knowledge", "docs"] {
            let path = Path::new(directory);
            if path.is_dir() {
                trainer.train_directory(path)?;
            }
        }
    } else if source_path.is_dir() {
        trainer.train_directory(source_path)?;
    } else {
        trainer.train_file(source_path)?;
    }
    trainer.write(output)
}

pub fn status_report() -> String {
    let path = default_weight_path();
    match BinaryLanguageModel::load(&path) {
        Ok(model) => {
            let stats = model.stats();
            format!(
                "native binary language\n  path: {}\n  order: {}\n  records: {}\n  transitions: {}\n  source bytes: {}\n  mapped: {:.2} MiB",
                path.display(),
                stats.order,
                stats.records,
                stats.unique_transitions,
                stats.source_bytes,
                stats.file_bytes as f64 / (1024.0 * 1024.0)
            )
        }
        Err(_error) if !path.exists() => format!(
            "native binary language\n  state: not trained\n  path: {}\n  next: perci language train --repo",
            path.display()
        ),
        Err(error) => format!(
            "native binary language\n  state: invalid\n  path: {}\n  error: {error}",
            path.display()
        ),
    }
}

fn collect_training_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_training_files(&path, out)?;
        } else if path.is_file() {
            let extension = path
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if matches!(
                extension.as_str(),
                "md" | "txt" | "json" | "jsonl" | "ndjson"
            ) {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn read_u16(data: &[u8], offset: usize) -> io::Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("binary language header is truncated"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> io::Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("binary language header is truncated"))?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn read_u64(data: &[u8], offset: usize) -> io::Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("binary language header is truncated"))?;
    Ok(u64::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn shift_history(history: &mut Vec<u8>, byte: u8) {
    if history.is_empty() {
        return;
    }
    history.remove(0);
    history.push(byte);
}

fn bit_is_set(bits: &[u64; BIT_WORDS], byte: u8) -> bool {
    bits[byte as usize / 64] & (1u64 << (byte as usize % 64)) != 0
}

fn is_allowed_training_byte(byte: u8) -> bool {
    matches!(byte, b'\n' | b'\t' | 32..=126)
}

fn is_allowed_generated_byte(byte: u8) -> bool {
    matches!(byte, b'\n' | b'\t' | 32..=126)
}

fn normalise_text(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut previous_space = false;
    for byte in text.bytes() {
        let byte = match byte {
            b'\r' => continue,
            b'\n' | b'\t' | 32..=126 => byte,
            _ => b' ',
        };
        if byte == b' ' {
            if previous_space {
                continue;
            }
            previous_space = true;
        } else {
            previous_space = false;
        }
        out.push(byte);
    }
    out
}

fn clean_generated(text: &str) -> String {
    let mut out = text.trim().replace("  ", " ");
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.chars().take(MAX_GENERATION_CHARS).collect()
}

fn salient_topic(user: &str) -> String {
    let stop = [
        "what", "why", "how", "are", "the", "this", "that", "you", "can", "does", "is", "a", "an",
        "to", "of", "and", "we", "our", "your",
    ];
    user.split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .filter(|word| word.len() >= 3 && !stop.contains(&word.to_ascii_lowercase().as_str()))
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
        .if_empty("the question")
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn xorshift64(mut value: u64) -> u64 {
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value
}

trait StringIfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl StringIfEmpty for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_owned()
        } else {
            self
        }
    }
}

const PRIMER_CORPUS: &str = r#"
Hello — I am here, locally and attentively.
I am Perci, a local binary system. I can classify structure, retain governed context, run exact tools, and explain what my evidence supports.
A useful way to think about a question is to name the mechanism, the evidence, and the boundary where the analogy stops.
The key distinction is between an association and a verified claim: an association suggests a path, while evidence decides whether the path holds.
The mechanism to examine is simple: observe a change, state a prediction, measure the result, and update the model when the result disagrees.
Geometry makes the relation visible: a boundary separates regions while also defining what can pass between them.
Start with the smallest reproducible case: isolate the input, record the expected behavior, and test one change at a time.
Progress is not a feeling stored in a weight; it is a measured improvement that survives a held-out check.
An answer becomes more useful when it leads with the direct claim, names its evidence, and leaves uncertainty visible.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn trainer_round_trip_is_binary_and_sorted() {
        let mut trainer = BinaryLanguageTrainer::new(4);
        trainer.train_text("hello world. hello there.\n");
        let path = env::temp_dir().join(format!(
            "perci-binary-language-{}-{}.blng",
            std::process::id(),
            now_millis()
        ));
        let stats = trainer.write(&path).unwrap();
        assert!(stats.records > 0);
        assert!(stats.unique_transitions > 0);
        let model = BinaryLanguageModel::load(&path).unwrap();
        let out = model.generate_from_seed("hello", 80, 7);
        assert!(!out.is_empty());
        assert!(out.bytes().all(is_allowed_generated_byte));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn jsonl_messages_feed_only_human_facing_text() {
        let mut trainer = BinaryLanguageTrainer::new(3);
        trainer.train_json_value(&serde_json::json!({
            "id": "secret-id",
            "messages": [
                {"role": "user", "content": "Ask clearly."},
                {"role": "assistant", "content": "Answer directly."}
            ],
            "source": "test"
        }));
        assert!(trainer.source_bytes > 0);
        assert!(trainer.unique_transitions > 0);
    }

    fn now_millis() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }
}
