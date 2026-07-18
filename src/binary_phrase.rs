//! A compact word/phrase transition field for Perci.
//!
//! PERCLNG1 is a useful character-level memory, but character transitions are
//! the wrong abstraction for long, readable language.  This module adds a
//! second native layer: a bounded vocabulary and binary thresholded transitions
//! between words.  The file is still a plain, versioned binary artifact and
//! inference uses mmap, integer scores, and a deterministic PRNG.  There is no
//! model server, tensor runtime, gradient engine, or external LLM involved.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};

const MAGIC: &[u8; 8] = b"PERCPHR1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
const MAX_ORDER: usize = 4;
const MAX_VOCAB: usize = 4095;
const MAX_TOKEN_BYTES: usize = 63;
const INDEX_RECORD_SIZE: usize = 1 + MAX_ORDER * 2 + 4 + 2;
const ENTRY_SIZE: usize = 3;
const MAX_GENERATION_TOKENS: usize = 120;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RecordKey {
    depth: u8,
    context: [u16; MAX_ORDER],
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BinaryPhraseStats {
    pub order: usize,
    pub vocabulary: usize,
    pub records: usize,
    pub entries: usize,
    pub source_bytes: u64,
    pub file_bytes: u64,
}

/// The current native phrase artifact.  The environment override makes
/// experiments reproducible without replacing the active local field.
pub fn default_weight_path() -> PathBuf {
    env::var_os("PERCI_PHRASE_WEIGHTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/perci-language-v0.2.bphr"))
}

#[derive(Debug)]
pub struct BinaryPhraseModel {
    path: PathBuf,
    data: Mmap,
    order: usize,
    vocabulary: Vec<String>,
    ids: HashMap<String, u16>,
    index_offset: usize,
    entries_offset: usize,
    record_count: usize,
    entry_count: usize,
    source_bytes: u64,
}

impl BinaryPhraseModel {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE {
            return Err(invalid("binary phrase file is shorter than its header"));
        }
        if &data[..8] != MAGIC {
            return Err(invalid("binary phrase file has an unknown signature"));
        }
        if read_u32(&data, 8)? != VERSION {
            return Err(invalid("unsupported binary phrase version"));
        }
        let order = data[12] as usize;
        if !(1..=MAX_ORDER).contains(&order) {
            return Err(invalid(
                "binary phrase order is outside the supported range",
            ));
        }
        let vocabulary_count = read_u32(&data, 16)? as usize;
        if !(1..=MAX_VOCAB).contains(&vocabulary_count) {
            return Err(invalid(
                "binary phrase vocabulary is outside the supported range",
            ));
        }
        let record_count = read_u32(&data, 20)? as usize;
        let source_bytes = read_u64(&data, 24)?;
        let vocabulary_offset = read_u64(&data, 32)? as usize;
        let index_offset = read_u64(&data, 40)? as usize;
        let entries_offset = read_u64(&data, 48)? as usize;
        let entry_count = read_u64(&data, 56)? as usize;
        if vocabulary_offset != HEADER_SIZE
            || index_offset < vocabulary_offset
            || entries_offset < index_offset
            || entries_offset > data.len()
            || index_offset + record_count.saturating_mul(INDEX_RECORD_SIZE) != entries_offset
            || entries_offset + entry_count.saturating_mul(ENTRY_SIZE) != data.len()
        {
            return Err(invalid("binary phrase section offsets are inconsistent"));
        }

        let mut vocabulary = Vec::with_capacity(vocabulary_count);
        let mut cursor = vocabulary_offset;
        for _ in 0..vocabulary_count {
            let len = *data
                .get(cursor)
                .ok_or_else(|| invalid("binary phrase vocabulary is truncated"))?
                as usize;
            cursor += 1;
            let bytes = data
                .get(cursor..cursor + len)
                .ok_or_else(|| invalid("binary phrase token is truncated"))?;
            vocabulary.push(String::from_utf8_lossy(bytes).into_owned());
            cursor += len;
        }
        if cursor != index_offset {
            return Err(invalid("binary phrase vocabulary offset is incorrect"));
        }
        let ids = vocabulary
            .iter()
            .enumerate()
            .map(|(id, token)| (token.clone(), id as u16))
            .collect();
        Ok(Self {
            path,
            data,
            order,
            vocabulary,
            ids,
            index_offset,
            entries_offset,
            record_count,
            entry_count,
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

    pub fn order(&self) -> usize {
        self.order
    }

    pub fn vocabulary_count(&self) -> usize {
        self.vocabulary.len()
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub fn file_bytes(&self) -> usize {
        self.data.len()
    }

    pub fn stats(&self) -> BinaryPhraseStats {
        BinaryPhraseStats {
            order: self.order,
            vocabulary: self.vocabulary_count(),
            records: self.record_count,
            entries: self.entry_count,
            source_bytes: self.source_bytes,
            file_bytes: self.data.len() as u64,
        }
    }

    /// Generate a topic-bound response from learned word transitions.
    pub fn generate_reply(
        &self,
        user: &str,
        domain: &str,
        max_chars: usize,
        mut state: u64,
    ) -> String {
        let topic = salient_topic(user);
        state ^= stable_hash(user.as_bytes());
        if state == 0 {
            state = 1;
        }
        let primers: &[&str] = match domain {
            "geometry" => &[
                "geometry makes the relation visible: <topic> is",
                "a geometric reading of <topic> begins with the boundary that",
            ],
            "science" => &[
                "the mechanism to examine is <topic>: it",
                "a scientific question about <topic> asks which observation would",
            ],
            "logic" => &[
                "the key distinction is <topic>: it",
                "to reason about <topic>, first state the premise that",
            ],
            "code" => &[
                "start with the smallest reproducible case for <topic>: it",
                "a reliable implementation of <topic> begins by isolating the state that",
            ],
            "identity" => &[
                "i am perci, a local binary system. my boundary is",
                "i can describe <topic> operationally: the evidence I have is",
            ],
            "greeting" => &[
                "hello, i am here locally and attentively. a useful question is",
                "hello, i am online. we can examine <topic> by asking",
            ],
            _ => &[
                "a useful way to think about <topic> is",
                "one useful distinction for <topic> is",
                "a practical way to approach <topic> is",
                "when we examine <topic>, the mechanism is",
                "a deeper connection in <topic> is",
            ],
        };
        let primer = primers[(state as usize) % primers.len()];
        let mut history = vec![self.id_for("<unk>"); self.order];
        let mut output = Vec::new();
        for token in tokenize(primer) {
            let id = self.id_for(&token);
            history_shift(&mut history, id);
            output.push(token);
        }
        let target = max_chars.clamp(120, 1200);
        let mut generated_tokens = 0usize;
        for _ in 0..MAX_GENERATION_TOKENS {
            let Some(next) = self.next_token(&history, &mut state, generated_tokens) else {
                break;
            };
            let token = self
                .vocabulary
                .get(next as usize)
                .map(String::as_str)
                .unwrap_or("<unk>");
            if token == "<eos>" || token == "<unk>" {
                break;
            }
            history_shift(&mut history, next);
            output.push(token.to_owned());
            generated_tokens += 1;
            let rendered = render_tokens(&output, &topic);
            if rendered.chars().count() >= target.min(MAX_GENERATION_TOKENS * 12)
                || (generated_tokens >= 8 && matches!(token, "." | "?" | "!"))
            {
                break;
            }
        }
        let rendered = render_tokens(&output, &topic);
        if rendered.len() < 12 {
            format!(
                "The question concerns {topic}; I need more learned context to continue reliably."
            )
        } else {
            rendered
        }
    }

    fn id_for(&self, token: &str) -> u16 {
        self.ids
            .get(token)
            .copied()
            .or_else(|| self.ids.get("<unk>").copied())
            .unwrap_or(0)
    }

    fn next_token(&self, history: &[u16], state: &mut u64, generated_tokens: usize) -> Option<u16> {
        let mut scores: HashMap<u16, i32> = HashMap::new();
        for depth in 1..=self.order.min(history.len()) {
            let mut key = RecordKey {
                depth: depth as u8,
                context: [0; MAX_ORDER],
            };
            key.context[MAX_ORDER - depth..].copy_from_slice(&history[history.len() - depth..]);
            for (id, bits) in self.lookup(&key)? {
                if id == self.id_for("<unk>") {
                    continue;
                }
                if generated_tokens < 3
                    && self
                        .vocabulary
                        .get(id as usize)
                        .map(|token| matches!(token.as_str(), "." | "," | "?" | "!" | ":" | ";"))
                        .unwrap_or(false)
                {
                    continue;
                }
                let level = (0..4)
                    .filter(|plane| bits & (1 << plane) != 0)
                    .map(|plane| 1 << plane)
                    .sum::<i32>();
                *scores.entry(id).or_default() += (depth * depth) as i32 * level;
            }
        }
        // An unseen multi-word context should not terminate the response.  A
        // depth-one unknown context is the learned sentence-start/global
        // distribution and provides an explicit, inspectable back-off path.
        if scores.is_empty() {
            let key = RecordKey {
                depth: 1,
                context: [0, 0, 0, 0],
            };
            for (id, bits) in self.lookup(&key)? {
                if id == self.id_for("<unk>") {
                    continue;
                }
                let level = (0..4)
                    .filter(|plane| bits & (1 << plane) != 0)
                    .map(|plane| 1 << plane)
                    .sum::<i32>();
                *scores.entry(id).or_default() += level;
            }
        }
        let best = scores.values().copied().max()?;
        let mut candidates = scores
            .into_iter()
            .filter(|(_, score)| *score == best)
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(id, _)| *id);
        if candidates.is_empty() {
            return None;
        }
        *state = xorshift64(*state);
        Some(candidates[(*state as usize) % candidates.len()].0)
    }

    fn lookup(&self, wanted: &RecordKey) -> Option<Vec<(u16, u8)>> {
        let mut low = 0usize;
        let mut high = self.record_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let offset = self.index_offset + mid * INDEX_RECORD_SIZE;
            let depth = self.data[offset];
            let context_start = offset + 1;
            let mut actual = RecordKey {
                depth,
                context: [0; MAX_ORDER],
            };
            for (slot, value) in actual.context.iter_mut().enumerate() {
                let start = context_start + slot * 2;
                *value = u16::from_le_bytes(self.data[start..start + 2].try_into().ok()?);
            }
            let ordering = actual.cmp(wanted);
            match ordering {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => {
                    let entry_offset = u32::from_le_bytes(
                        self.data[offset + 1 + MAX_ORDER * 2..offset + 1 + MAX_ORDER * 2 + 4]
                            .try_into()
                            .ok()?,
                    ) as usize;
                    let count = u16::from_le_bytes(
                        self.data[offset + 1 + MAX_ORDER * 2 + 4..offset + INDEX_RECORD_SIZE]
                            .try_into()
                            .ok()?,
                    ) as usize;
                    let start = self.entries_offset + entry_offset;
                    let mut out = Vec::with_capacity(count);
                    for index in 0..count {
                        let item = start + index * ENTRY_SIZE;
                        let id = u16::from_le_bytes(self.data[item..item + 2].try_into().ok()?);
                        out.push((id, self.data[item + 2]));
                    }
                    return Some(out);
                }
            }
        }
        Some(Vec::new())
    }
}

#[derive(Debug)]
pub struct BinaryPhraseTrainer {
    order: usize,
    documents: Vec<Vec<String>>,
    source_bytes: u64,
}

impl BinaryPhraseTrainer {
    pub fn new(order: usize) -> Self {
        Self {
            order: order.clamp(1, MAX_ORDER),
            documents: Vec::new(),
            source_bytes: 0,
        }
    }

    pub fn train_text(&mut self, text: &str) {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with('#')
                || trimmed.starts_with('|')
                || trimmed.starts_with('-')
                || trimmed.starts_with('*')
                || trimmed.starts_with('`')
                || trimmed
                    .chars()
                    .next()
                    .map(|ch| ch.is_ascii_digit())
                    .unwrap_or(false)
                || !looks_like_prose(trimmed)
            {
                continue;
            }
            let mut tokens = tokenize(trimmed);
            tokens.retain(|token| !token.chars().all(|ch| ch.is_ascii_digit()));
            if tokens.is_empty() {
                continue;
            }
            self.source_bytes = self.source_bytes.saturating_add(trimmed.len() as u64);
            if !matches!(tokens.last().map(String::as_str), Some("." | "?" | "!")) {
                tokens.push(".".to_owned());
            }
            tokens.push("<eos>".to_owned());
            self.documents.push(tokens);
        }
    }

    pub fn train_file(&mut self, path: &Path) -> io::Result<()> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if matches!(extension.as_str(), "jsonl" | "ndjson") {
            let file = File::open(path)?;
            for line in BufReader::new(file).lines() {
                let line = line?;
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    train_json_value(self, &value);
                }
            }
        } else if extension == "json" {
            let raw = fs::read_to_string(path)?;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) {
                train_json_value(self, &value);
            }
        } else if matches!(
            extension.as_str(),
            "md" | "txt" | "toml" | "rs" | "py" | "ps1"
        ) {
            self.train_text(&fs::read_to_string(path)?);
        }
        Ok(())
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

    pub fn write(&self, path: &Path) -> io::Result<BinaryPhraseStats> {
        let mut frequencies: HashMap<String, u64> = HashMap::new();
        for document in &self.documents {
            for token in document {
                *frequencies.entry(token.clone()).or_default() += 1;
            }
        }
        frequencies.entry("<unk>".to_owned()).or_default();
        frequencies.entry("<eos>".to_owned()).or_default();
        frequencies.entry("<topic>".to_owned()).or_default();
        let mut ranked = frequencies.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|(left, left_count), (right, right_count)| {
            right_count.cmp(left_count).then_with(|| left.cmp(right))
        });
        let mut vocabulary = vec!["<unk>".to_owned()];
        for (token, _) in ranked {
            if token != "<unk>" && vocabulary.len() < MAX_VOCAB && !vocabulary.contains(&token) {
                vocabulary.push(token);
            }
        }
        let ids = vocabulary
            .iter()
            .enumerate()
            .map(|(id, token)| (token.clone(), id as u16))
            .collect::<HashMap<_, _>>();
        let unknown = ids["<unk>"];
        let mut transitions: BTreeMap<RecordKey, BTreeMap<u16, u8>> = BTreeMap::new();
        for document in &self.documents {
            let mut history = vec![unknown; self.order];
            for token in document {
                let next = ids.get(token).copied().unwrap_or(unknown);
                for depth in 1..=self.order {
                    let mut key = RecordKey {
                        depth: depth as u8,
                        context: [0; MAX_ORDER],
                    };
                    key.context[MAX_ORDER - depth..]
                        .copy_from_slice(&history[history.len() - depth..]);
                    let count = transitions.entry(key).or_default().entry(next).or_default();
                    *count = count.saturating_add(1).min(15);
                }
                history_shift(&mut history, next);
            }
        }

        let mut vocabulary_bytes = Vec::new();
        for token in &vocabulary {
            let bytes = token.as_bytes();
            let len = bytes.len().min(MAX_TOKEN_BYTES);
            vocabulary_bytes.push(len as u8);
            vocabulary_bytes.extend_from_slice(&bytes[..len]);
        }
        let mut index_bytes = Vec::with_capacity(transitions.len() * INDEX_RECORD_SIZE);
        let mut entries_bytes = Vec::new();
        for (key, nexts) in &transitions {
            index_bytes.push(key.depth);
            for value in key.context {
                index_bytes.extend_from_slice(&value.to_le_bytes());
            }
            let entry_offset = entries_bytes.len() as u32;
            index_bytes.extend_from_slice(&entry_offset.to_le_bytes());
            index_bytes.extend_from_slice(&(nexts.len() as u16).to_le_bytes());
            for (token, count) in nexts {
                let mut planes = 0u8;
                for plane in 0..4 {
                    if *count >= (1u8 << plane) {
                        planes |= 1 << plane;
                    }
                }
                entries_bytes.extend_from_slice(&token.to_le_bytes());
                entries_bytes.push(planes);
            }
        }
        let index_offset = HEADER_SIZE + vocabulary_bytes.len();
        let entries_offset = index_offset + index_bytes.len();
        let mut file_bytes = Vec::with_capacity(entries_offset + entries_bytes.len());
        file_bytes.extend_from_slice(MAGIC);
        file_bytes.extend_from_slice(&VERSION.to_le_bytes());
        file_bytes.extend_from_slice(&[self.order as u8, 0]);
        file_bytes.extend_from_slice(&0u16.to_le_bytes());
        file_bytes.extend_from_slice(&(vocabulary.len() as u32).to_le_bytes());
        file_bytes.extend_from_slice(&(transitions.len() as u32).to_le_bytes());
        file_bytes.extend_from_slice(&self.source_bytes.to_le_bytes());
        file_bytes.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
        file_bytes.extend_from_slice(&(index_offset as u64).to_le_bytes());
        file_bytes.extend_from_slice(&(entries_offset as u64).to_le_bytes());
        file_bytes.extend_from_slice(&((entries_bytes.len() / ENTRY_SIZE) as u64).to_le_bytes());
        debug_assert_eq!(file_bytes.len(), HEADER_SIZE);
        file_bytes.extend_from_slice(&vocabulary_bytes);
        file_bytes.extend_from_slice(&index_bytes);
        file_bytes.extend_from_slice(&entries_bytes);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("bphr.tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)?;
        file.write_all(&file_bytes)?;
        file.flush()?;
        drop(file);
        fs::rename(&tmp, path)?;
        Ok(BinaryPhraseStats {
            order: self.order,
            vocabulary: vocabulary.len(),
            records: transitions.len(),
            entries: entries_bytes.len() / ENTRY_SIZE,
            source_bytes: self.source_bytes,
            file_bytes: file_bytes.len() as u64,
        })
    }
}

pub fn train_source(source: &str, output: &Path, order: usize) -> io::Result<BinaryPhraseStats> {
    let mut trainer = BinaryPhraseTrainer::new(order);
    for _ in 0..3 {
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
    match BinaryPhraseModel::load(&path) {
        Ok(model) => {
            let stats = model.stats();
            format!(
                "native binary phrase field\n  path: {}\n  order: {}\n  vocabulary: {}\n  records: {}\n  transitions: {}\n  source bytes: {}\n  mapped: {:.2} MiB",
                path.display(), stats.order, stats.vocabulary, stats.records, stats.entries,
                stats.source_bytes, stats.file_bytes as f64 / (1024.0 * 1024.0)
            )
        }
        Err(_) if !path.exists() => format!(
            "native binary phrase field\n  state: not trained\n  path: {}\n  next: perci language train --repo",
            path.display()
        ),
        Err(error) => format!(
            "native binary phrase field\n  state: invalid\n  path: {}\n  error: {error}",
            path.display()
        ),
    }
}

fn train_json_value(trainer: &mut BinaryPhraseTrainer, value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(messages) = map.get("messages").and_then(|value| value.as_array()) {
                for item in messages {
                    if let Some(content) = item.get("content").and_then(|value| value.as_str()) {
                        trainer.train_text(content);
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
                if let Some(text) = map.get(key).and_then(|value| value.as_str()) {
                    trainer.train_text(text);
                }
            }
            for (key, child) in map {
                if !matches!(
                    key.as_str(),
                    "id" | "source" | "tags" | "created" | "recorded_at"
                ) && (child.is_object() || child.is_array())
                {
                    train_json_value(trainer, child);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                train_json_value(trainer, item);
            }
        }
        serde_json::Value::String(text) => trainer.train_text(text),
        _ => {}
    }
}

fn collect_training_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_training_files(&path, out)?;
        } else if path.is_file() {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
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

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, tokens: &mut Vec<String>| {
        if !current.is_empty() {
            tokens.push(current.to_ascii_lowercase());
            current.clear();
        }
    };
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            let mut special = String::from("<");
            while let Some(next) = chars.next() {
                special.push(next);
                if next == '>' {
                    break;
                }
            }
            if special.ends_with('>') {
                flush(&mut current, &mut tokens);
                tokens.push(special.to_ascii_lowercase());
            } else {
                current.push_str(&special);
            }
        } else if ch.is_ascii_alphanumeric() || ch == '\'' || ch == '-' {
            current.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '.' | ',' | '?' | '!' | ':' | ';') {
            flush(&mut current, &mut tokens);
            tokens.push(ch.to_string());
        } else {
            flush(&mut current, &mut tokens);
        }
    }
    flush(&mut current, &mut tokens);
    tokens
}

fn looks_like_prose(line: &str) -> bool {
    let alphabetic = line.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    let digits = line.chars().filter(|ch| ch.is_ascii_digit()).count();
    if alphabetic < 8 || digits * 3 > alphabetic {
        return false;
    }
    ![
        "```", "=>", "::", "\\", "$env:", "C:\\", "http://", "https://",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

fn history_shift(history: &mut [u16], value: u16) {
    if history.is_empty() {
        return;
    }
    history.rotate_left(1);
    *history.last_mut().expect("non-empty history") = value;
}

fn render_tokens(tokens: &[String], topic: &str) -> String {
    let mut out = String::new();
    for token in tokens {
        if token == "<topic>" {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push_str(topic);
            continue;
        }
        let punctuation = matches!(token.as_str(), "." | "," | "?" | "!" | ":" | ";");
        if !out.is_empty() && !punctuation && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(token);
    }
    let mut result = out.trim().to_owned();
    if let Some(first) = result.chars().next() {
        let upper = first.to_ascii_uppercase().to_string();
        result.replace_range(0..first.len_utf8(), &upper);
    }
    result
}

fn salient_topic(user: &str) -> String {
    let stop = [
        "what",
        "why",
        "how",
        "are",
        "the",
        "this",
        "that",
        "you",
        "can",
        "does",
        "is",
        "a",
        "an",
        "to",
        "of",
        "and",
        "we",
        "our",
        "your",
        "about",
        "tell",
        "explain",
        "connect",
        "give",
        "reflect",
        "creatively",
        "imagine",
        "original",
        "thought",
        "human",
        "then",
        "mark",
        "where",
        "without",
        "claiming",
        "which",
        "would",
        "distinguish",
        "nearby",
        "explanation",
        "could",
        "emerge",
        "when",
        "inside",
        "bounded",
        "system",
        "mechanism",
        "supports",
        "analogy",
        "one",
        "direct",
        "claim",
        "evidence",
        "test",
        "name",
        "first",
        "state",
        "relation",
        "question",
        "useful",
        "practical",
        "deeper",
    ];
    user.split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| word.len() >= 3 && !stop.contains(&word.to_ascii_lowercase().as_str()))
        .take(4)
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

fn read_u32(data: &[u8], offset: usize) -> io::Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("binary phrase header is truncated"))?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn read_u64(data: &[u8], offset: usize) -> io::Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("binary phrase header is truncated"))?;
    Ok(u64::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
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
a useful way to think about <topic> is a structure that relates parts while preserving the differences between them.
one useful distinction for <topic> is between a pattern we notice and a mechanism we can test.
a practical way to approach <topic> is to name the boundary, the exchange, and the evidence.
when we examine <topic>, the mechanism matters more than the metaphor.
a deeper connection in <topic> is a relation that survives a change of scale without erasing the domain differences.
geometry makes the relation visible: a boundary separates regions and also defines what can pass between them.
 a geometric reading of <topic> begins with the boundary that separates a system from its surroundings.
the mechanism to examine is <topic>: observe a change, state a prediction, measure the result, and update the model when the result disagrees.
a scientific question about <topic> asks which observation would distinguish the leading explanation from a nearby one.
the key distinction is <topic>: an association suggests a path, while evidence decides whether the path holds.
to reason about <topic>, first state the premise that would make the conclusion follow.
start with the smallest reproducible case for <topic>: isolate the input, record the expected behavior, and test one change at a time.
a reliable implementation of <topic> begins by isolating the state that can change and the invariant that must survive.
progress is not a feeling stored in a weight; it is a measured improvement that survives a held-out check.
an answer becomes useful when it leads with a direct claim, names its evidence, and leaves uncertainty visible.
language can bridge domains when the relation is explicit and the mechanism remains testable.
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn phrase_round_trip_is_binary_and_generates_text() {
        let mut trainer = BinaryPhraseTrainer::new(4);
        trainer.train_text(PRIMER_CORPUS);
        let path = env::temp_dir().join(format!(
            "perci-binary-phrase-{}-{}.bphr",
            std::process::id(),
            now_millis()
        ));
        let stats = trainer.write(&path).unwrap();
        assert!(stats.vocabulary > 4);
        assert!(stats.records > 0);
        let model = BinaryPhraseModel::load(&path).unwrap();
        let out = model.generate_reply(
            "what is geometry teaching us about life",
            "geometry",
            280,
            7,
        );
        assert!(out.len() > 12);
        assert!(!out.contains("<topic>"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn jsonl_training_keeps_human_facing_strings() {
        let mut trainer = BinaryPhraseTrainer::new(3);
        train_json_value(
            &mut trainer,
            &serde_json::json!({
                "id": "private-id",
                "messages": [{"role": "user", "content": "Ask clearly."}, {"role": "assistant", "content": "Answer directly."}],
                "source": "ignored"
            }),
        );
        assert!(trainer.source_bytes > 0);
        assert_eq!(trainer.documents.len(), 2);
    }

    fn now_millis() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }
}
