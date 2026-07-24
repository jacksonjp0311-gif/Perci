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
    pub fn generate_reply(&self, user: &str, domain: &str, max_chars: usize, state: u64) -> String {
        self.generate_reply_with_context(user, domain, max_chars, state, None)
    }

    /// Generate a continuation while binding a bounded prior-turn context.
    /// The context is injected into hidden transition history and never
    /// rendered, so paired-turn candidates can distinguish a follow-up from a
    /// fresh question without leaking the prompt into the answer.
    pub fn generate_reply_with_context(
        &self,
        user: &str,
        domain: &str,
        max_chars: usize,
        mut state: u64,
        context: Option<&str>,
    ) -> String {
        let topic = humanize_topic(&salient_topic(user));
        let intent = salient_intent(user);
        let operation = dialogue_operation(user);
        state ^= stable_hash(user.as_bytes());
        if state == 0 {
            state = 1;
        }
        // Intent is a learned control signal, not a second response engine:
        // these primers give the transition field a discourse state before it
        // walks the binary vocabulary.  Domain primers remain the fallback.
        let primers: &[&str] = match intent {
            "improvement" => &[
                "<intent> a measurable improvement in <topic> is",
                "<intent> the useful change in <topic> is the one that",
                "<intent> to improve <topic>, first observe whether",
            ],
            "repair" => &[
                "<intent> you are pointing to a dialogue failure: the missing link is",
                "<intent> the repair is to connect your meaning to the answer by",
                "<intent> I should not guess past your point; the direct issue is",
            ],
            "social" => &[
                "<intent> I am with you; the point worth carrying forward is",
                "<intent> that reaction matters because it notices",
                "<intent> I hear the opening; we can follow it toward",
            ],
            "capability" => &[
                "<intent> the language gap around <topic> is coverage and discourse state: the next test is",
                "<intent> a learned sequence can sound natural when it preserves <topic> across turns",
                "<intent> the honest boundary for <topic> is that this field learns transitions, not",
            ],
            _ => match domain {
            "geometry" => &[
                "<intent> geometry makes the relation visible: <topic> is",
                "<intent> a geometric reading of <topic> begins with the boundary that",
            ],
            "science" => &[
                "<intent> the mechanism to examine is <topic>: it",
                "<intent> a scientific question about <topic> asks which observation would",
            ],
            "logic" => &[
                "<intent> the key distinction is <topic>: it",
                "<intent> to reason about <topic>, first state the premise that",
            ],
            "code" => &[
                "<intent> start with the smallest reproducible case for <topic>: it",
                "<intent> a reliable implementation of <topic> begins by isolating the state that",
            ],
            "identity" => &[
                "<intent> i am perci, a local binary system. my boundary is",
                "<intent> i can describe <topic> operationally: the evidence I have is",
            ],
            "greeting" => &[
                "<intent> hello, i am here locally and attentively. a useful question is",
                "<intent> hello, i am online. we can examine <topic> by asking",
            ],
            _ => &[
                "<intent> a useful way to think about <topic> is",
                "<intent> one useful distinction for <topic> is",
                "<intent> a practical way to approach <topic> is",
                "<intent> when we examine <topic>, the mechanism is",
                "<intent> a deeper connection in <topic> is",
            ],
            },
        };
        let primer = primers[(state as usize) % primers.len()];
        let unknown = self.id_for("<unk>");
        let mut history = vec![unknown; self.order];
        let mut output = Vec::new();
        for token in tokenize(primer) {
            if token == "<topic>" {
                // `<topic>` is a rendering marker, but the learned transition
                // field must see the actual salient words or every prompt
                // collapses onto one global continuation distribution.
                for topic_token in tokenize(&topic) {
                    history_shift(&mut history, self.id_for(&topic_token));
                }
                output.push(token);
            } else if token == "<intent>" {
                for intent_token in tokenize(intent) {
                    history_shift(&mut history, self.id_for(&intent_token));
                }
                output.push(token);
            } else {
                let id = self.id_for(&token);
                history_shift(&mut history, id);
                output.push(token);
            }
        }
        // Keep discourse operation, semantic topic, and conversational
        // continuity as separate experts. Appending raw context to one history
        // used to erase the operation primer and made prompt/answer learning
        // impossible. The factorized histories preserve each signal and mix
        // their threshold-coded votes during generation.
        let mut operation_history = vec![unknown; self.order];
        history_shift(
            &mut operation_history,
            self.id_for(&format!("<op:{operation}>")),
        );
        let mut topic_history = vec![unknown; self.order];
        for token in tokenize(&salient_topic(user)) {
            history_shift(&mut topic_history, self.id_for(&token));
        }
        history_shift(&mut topic_history, self.id_for("<answer>"));
        let mut continuity_history = vec![unknown; self.order];
        if let Some(context) = context {
            // The tail carries the nearest referent, but it is only one expert;
            // it can no longer overwrite the requested discourse operation.
            let context_tokens = tokenize(context);
            let start = context_tokens.len().saturating_sub(18);
            for token in &context_tokens[start..] {
                history_shift(&mut continuity_history, self.id_for(token));
            }
            history_shift(&mut continuity_history, self.id_for("<context>"));
        }
        let target = max_chars.clamp(120, 1200);
        let mut generated_tokens = 0usize;
        let operation_weight = self
            .ids
            .contains_key(&format!("<op:{operation}>"))
            .then_some(8)
            .unwrap_or(0);
        let topic_weight = self.ids.contains_key("<answer>").then_some(7).unwrap_or(0);
        let continuity_weight = self.ids.contains_key("<context>").then_some(3).unwrap_or(0);
        for _ in 0..MAX_GENERATION_TOKENS {
            let expert_histories = [
                (&history[..], 5i32),
                (&operation_history[..], operation_weight),
                (&topic_history[..], topic_weight),
                (&continuity_history[..], continuity_weight),
            ];
            let Some(next) =
                self.next_token_factorized(&expert_histories, &mut state, generated_tokens)
            else {
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
            history_shift(&mut operation_history, next);
            history_shift(&mut topic_history, next);
            history_shift(&mut continuity_history, next);
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

    /// Mix several sparse binary transition experts without converting the
    /// field into a dense neural runtime. Each expert contributes
    /// `depth² × bit-plane magnitude × declared weight`; the result is an
    /// inspectable product-of-contexts approximation:
    ///
    /// score(token) = Σ_e λ_e Σ_d d² q_e,d(token)
    ///
    /// Operation, topic, local syntax, and recent continuity therefore retain
    /// independent influence instead of being destructively flattened into one
    /// four-token history.
    fn next_token_factorized(
        &self,
        histories: &[(&[u16], i32)],
        state: &mut u64,
        generated_tokens: usize,
    ) -> Option<u16> {
        let mut scores: HashMap<u16, i32> = HashMap::new();
        for (history, expert_weight) in histories {
            if *expert_weight <= 0 {
                continue;
            }
            for depth in 1..=self.order.min(history.len()) {
                let mut key = RecordKey {
                    depth: depth as u8,
                    context: [0; MAX_ORDER],
                };
                key.context[MAX_ORDER - depth..].copy_from_slice(&history[history.len() - depth..]);
                for (id, bits) in self.lookup(&key)? {
                    if id == self.id_for("<unk>")
                        || self
                            .vocabulary
                            .get(id as usize)
                            .map(|token| is_hidden_control(token))
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    if generated_tokens < 3
                        && self
                            .vocabulary
                            .get(id as usize)
                            .map(|token| {
                                matches!(token.as_str(), "." | "," | "?" | "!" | ":" | ";")
                            })
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    let level = (0..4)
                        .filter(|plane| bits & (1 << plane) != 0)
                        .map(|plane| 1 << plane)
                        .sum::<i32>();
                    *scores.entry(id).or_default() +=
                        (depth * depth) as i32 * level * *expert_weight;
                }
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

    /// Preserve the supervision that ordinary prose training discards: a
    /// reviewed prompt is paired with its reviewed answer. Three sparse views
    /// are recorded so inference can factor operation, topic, and ordinary
    /// syntax without storing a dense model or copying the prompt into output.
    pub fn train_dialogue_pair(&mut self, prompt: &str, response: &str) {
        let response = response.trim();
        if !looks_like_prose(response) {
            return;
        }
        let operation = dialogue_operation(prompt);
        let topic = salient_topic(prompt);
        let response_tokens = {
            let mut tokens = tokenize(response);
            tokens.retain(|token| !token.chars().all(|ch| ch.is_ascii_digit()));
            if !matches!(tokens.last().map(String::as_str), Some("." | "?" | "!")) {
                tokens.push(".".to_owned());
            }
            tokens.push("<eos>".to_owned());
            tokens
        };
        if response_tokens.len() <= 1 {
            return;
        }
        self.source_bytes = self
            .source_bytes
            .saturating_add((prompt.len() + response.len()) as u64);

        let mut operation_view = vec![format!("<op:{operation}>")];
        operation_view.extend(response_tokens.clone());
        self.documents.push(operation_view);

        let mut topic_view = tokenize(&topic);
        topic_view.push("<answer>".to_owned());
        topic_view.extend(response_tokens.clone());
        self.documents.push(topic_view);

        let mut combined_view = vec![format!("<op:{operation}>"), "<answer>".to_owned()];
        combined_view.extend(response_tokens);
        self.documents.push(combined_view);
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
                let mut pending_user: Option<&str> = None;
                for item in messages {
                    if let Some(content) = item.get("content").and_then(|value| value.as_str()) {
                        match item.get("role").and_then(|value| value.as_str()) {
                            Some("user") => pending_user = Some(content),
                            Some("assistant") if pending_user.is_some() => {
                                trainer.train_dialogue_pair(
                                    pending_user.take().unwrap_or_default(),
                                    content,
                                );
                                trainer.train_text(content);
                            }
                            _ => trainer.train_text(content),
                        }
                    }
                }
            }
            let prompt = map.get("prompt").and_then(|value| value.as_str());
            let response = ["response", "assistant", "answer"]
                .iter()
                .find_map(|key| map.get(*key).and_then(|value| value.as_str()));
            if let (Some(prompt), Some(response)) = (prompt, response) {
                trainer.train_dialogue_pair(prompt, response);
            }
            for key in [
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
        if is_hidden_control(token) {
            continue;
        }
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

fn is_hidden_control(token: &str) -> bool {
    token == "<intent>" || token == "<answer>" || token == "<context>" || token.starts_with("<op:")
}

/// Small, inspectable intent vocabulary used to condition native continuation.
/// It is deliberately narrower than the dialogue operator catalog: the voice
/// layer remains authoritative for final response shape.
fn salient_intent(user: &str) -> &'static str {
    let lower = crate::text_normalize::normalize_for_routing(user);
    if lower.contains("improving")
        || lower.contains("improvement")
        || (lower.contains("evolve") && lower.contains("system"))
    {
        "improvement"
    } else if (lower.contains("why dont you") || lower.contains("why don't you"))
        && (lower.contains("say")
            || lower.contains("saying")
            || lower.contains("think")
            || lower.contains("mean"))
    {
        "repair"
    } else if lower == "interesting"
        || lower == "thats interesting"
        || lower == "that's interesting"
        || lower == "that is interesting"
        || lower == "wow"
    {
        "social"
    } else if lower.contains("frontier")
        && (lower.contains("response")
            || lower.contains("language")
            || lower.contains("natural")
            || lower.contains("like"))
    {
        "capability"
    } else if lower.contains("dont agree")
        || lower.contains("don't agree")
        || lower.contains("disagree")
        || lower.contains("seems wrong")
    {
        "disagreement"
    } else if lower.contains("change your mind")
        || lower.contains("revise")
        || lower.contains("counterexample")
    {
        "revision"
    } else if lower.contains("what do you mean")
        || lower.contains("explain") && lower.contains("differently")
        || lower.contains("clarif")
    {
        "clarification"
    } else if lower.contains("creative")
        || lower.contains("original thought")
        || lower.contains("fresh angle")
        || lower.contains("new idea")
    {
        "creative"
    } else if lower.contains("one sentence")
        || lower.contains("short answer")
        || lower.contains("brief answer")
    {
        "brief"
    } else if lower.contains("go deeper")
        || lower.contains("one level deeper")
        || lower.contains("step by step")
    {
        "depth"
    } else if lower.contains("what should we test")
        || lower.contains("next test")
        || lower.contains("what do we test")
    {
        "test"
    } else if lower.contains("learn") || lower.contains("teach") || lower.contains("remember") {
        "learning"
    } else if lower.contains("evidence")
        || lower.contains("proof")
        || lower.contains("reproducible")
    {
        "evidence"
    } else {
        "general"
    }
}

/// Coarser than the public dialogue-act catalog by design. These values are
/// binary training controls: they describe what an answer must *do*, not the
/// subject it discusses.
fn dialogue_operation(user: &str) -> &'static str {
    let lower = crate::text_normalize::normalize_for_routing(user);
    if lower.contains("compare")
        || lower.contains("difference")
        || lower.contains("unlike")
        || lower.contains("versus")
    {
        "compare"
    } else if lower.contains("connect")
        || lower.contains("shared structure")
        || lower.contains("relationship between")
    {
        "connect"
    } else if lower.contains("what next")
        || lower.contains("next move")
        || lower.contains("what should")
        || lower.contains("plan")
    {
        "plan"
    } else if lower.contains("counterexample")
        || lower.contains("falsif")
        || lower.contains("what would change")
        || lower.contains("test this")
    {
        "test"
    } else if lower.contains("why")
        || lower.contains("explain")
        || lower.contains("how does")
        || lower.contains("how do ")
    {
        "explain"
    } else if lower.contains("go deeper")
        || lower.contains("one level deeper")
        || lower.contains("elaborate")
        || lower.contains("tell me more")
    {
        "deepen"
    } else if lower.contains("creative")
        || lower.contains("original")
        || lower.contains("imagine")
        || lower.contains("fresh")
    {
        "create"
    } else if lower.contains("evidence")
        || lower.contains("proof")
        || lower.contains("how do you know")
        || lower.contains("trust the result")
    {
        "evidence"
    } else if lower.contains("dont agree")
        || lower.contains("don't agree")
        || lower.contains("seems wrong")
        || lower.contains("conclusion follows")
    {
        "challenge"
    } else if lower.contains("rephrase")
        || lower.contains("differently")
        || lower.contains("plain language")
        || lower.contains("what do you mean")
    {
        "clarify"
    } else if lower.contains("learn") || lower.contains("remember") || lower.contains("teach") {
        "learn"
    } else if lower.contains("improv") || lower.contains("evolv") || lower.contains("repair") {
        "repair"
    } else if lower.trim() == "interesting"
        || lower.trim() == "wow"
        || lower.starts_with("hello")
        || lower.starts_with("hi ")
    {
        "social"
    } else {
        "answer"
    }
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
        "express",
        "new",
        "say",
        "differently",
        "more",
        "next",
        "evolution",
        "beautiful",
        "way",
        "understand",
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
        "hard",
        "easy",
        "important",
        "matter",
        "mean",
        "like",
        "difference",
        "between",
        "agree",
        "disagree",
        "wrong",
        "mind",
        "revise",
        "clarify",
        "creative",
        "original",
        "fresh",
        "angle",
        "preserve",
        "support",
        "think",
        "describe",
        "forms",
        "changes",
        "time",
        "lived",
        "dont",
        "don't",
        "thats",
        "that's",
        "saying",
        "instead",
        "im",
        "i'm",
        "why",
        "like",
    ];
    crate::text_normalize::repair_typos(user)
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| word.len() >= 3 && !stop.contains(&word.to_ascii_lowercase().as_str()))
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
        .if_empty("the question")
}

/// Turn a compact topic list into a readable coordination for a primer. This
/// is presentation only; the binary route still scores the individual words.
fn humanize_topic(topic: &str) -> String {
    let words = topic.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        [] => "the question".to_owned(),
        [one] => (*one).to_owned(),
        [one, two] => format!("the relationship between {one} and {two}"),
        [rest @ .., last] => {
            format!("the relationship among {}, and {last}", rest.join(", "))
        }
    }
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
    fn contextual_generation_keeps_prior_turn_hidden() {
        let mut trainer = BinaryPhraseTrainer::new(4);
        trainer.train_text(PRIMER_CORPUS);
        let path = env::temp_dir().join(format!(
            "perci-binary-phrase-context-{}-{}.bphr",
            std::process::id(),
            now_millis()
        ));
        trainer.write(&path).unwrap();
        let model = BinaryPhraseModel::load(&path).unwrap();
        let out = model.generate_reply_with_context(
            "why does that matter",
            "general",
            280,
            11,
            Some("memory carries state across time; why does that matter"),
        );
        assert!(out.len() > 12);
        assert!(!out.contains("memory carries state across time"));
        assert!(!out.contains("<intent>"));
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
        assert_eq!(trainer.documents.len(), 4);
        assert!(trainer
            .documents
            .iter()
            .any(|document| document.first().map(String::as_str) == Some("<op:answer>")));
    }

    #[test]
    fn topic_extraction_discards_prompt_scaffolding() {
        assert_eq!(
            salient_topic("express a new thought about code and music"),
            "code music"
        );
        assert_eq!(
            salient_topic("say it differently: why does memory matter"),
            "memory"
        );
        assert_eq!(
            humanize_topic("geometry memory language"),
            "the relationship among geometry, memory, and language"
        );
    }

    #[test]
    fn learned_sequence_conditions_on_dialogue_intent() {
        let mut trainer = BinaryPhraseTrainer::new(4);
        trainer.train_text(&format!(
            "{PRIMER_CORPUS}\n\
                a measurable improvement in <topic> is a change that survives a held-out check.\n\
                you are pointing to a dialogue failure: the missing link is the user's meaning.\n\
                I am with you; the point worth carrying forward is the new observation.\n\
                the language gap is coverage and discourse state: the next test is transfer."
        ));
        let path = env::temp_dir().join(format!(
            "perci-binary-phrase-intent-{}-{}.bphr",
            std::process::id(),
            now_millis()
        ));
        trainer.write(&path).unwrap();
        let model = BinaryPhraseModel::load(&path).unwrap();
        let improvement = model.generate_reply("improving your system", "general", 280, 13);
        let repair = model.generate_reply(
            "why dont you think about what im saying instead",
            "general",
            280,
            17,
        );
        assert!(improvement.len() > 12);
        assert!(repair.len() > 12);
        assert!(!improvement.to_ascii_lowercase().contains("<intent>"));
        assert!(!repair.to_ascii_lowercase().contains("<topic>"));
        let _ = fs::remove_file(path);
    }

    fn now_millis() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    }
}
