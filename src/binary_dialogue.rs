//! PERCDLG1: packed operation-conditioned dialogue cases.
//!
//! A phrase transition field learns local wording but loses the boundary
//! between a reviewed request and its reviewed answer. PERCDLG1 preserves that
//! supervision as a small mmap-backed semantic lattice. Retrieval requires the
//! same discourse operation plus measurable content overlap; otherwise it
//! abstains and leaves authority with operators, tools, or SoftCascade.

use memmap2::{Mmap, MmapOptions};
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCDLG1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 32;
const RECORD_SIZE: usize = 48;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogueCase {
    pub response: String,
    pub score_milli: i64,
    pub operation: &'static str,
    pub shared_terms: Vec<String>,
}

#[derive(Debug)]
pub struct BinaryDialogueField {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
    records_offset: usize,
    text_offset: usize,
}

impl BinaryDialogueField {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(invalid("invalid PERCDLG1 header"));
        }
        if read_u32(&data, 8)? != VERSION {
            return Err(invalid("unsupported PERCDLG1 version"));
        }
        let record_count = read_u32(&data, 12)? as usize;
        let records_offset = read_u64(&data, 16)? as usize;
        let text_offset = read_u64(&data, 24)? as usize;
        if records_offset != HEADER_SIZE
            || text_offset != records_offset + record_count.saturating_mul(RECORD_SIZE)
            || text_offset > data.len()
        {
            return Err(invalid("PERCDLG1 offsets are inconsistent"));
        }
        Ok(Self {
            path,
            data,
            record_count,
            records_offset,
            text_offset,
        })
    }

    pub fn discover() -> io::Result<Option<Self>> {
        let path = env::var_os("PERCI_DIALOGUE_WEIGHTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("models/perci-dialogue-v0.1.bdlg"));
        if !path.is_file() {
            return Ok(None);
        }
        Self::load(path).map(Some)
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn file_bytes(&self) -> usize {
        self.data.len()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Retrieve only when operation and semantic geometry both transfer.
    pub fn best_reply(&self, user: &str) -> Option<DialogueCase> {
        if !eligible(user) {
            return None;
        }
        let operation = operation_id(user);
        let lower = crate::text_normalize::normalize_for_routing(user);
        let referential = lower.contains("previous")
            || lower.contains("that view")
            || lower.contains("that explanation")
            || lower.contains("that conclusion")
            || lower.contains("this repair")
            || lower.contains("that idea")
            || lower.contains("say that")
            || lower.contains("same idea")
            || lower.contains("again but")
            || lower.starts_with("what about ");
        if referential && operation != 12 {
            return None;
        }
        let user_terms = content_tokens(user);
        if user_terms.is_empty() {
            return None;
        }
        let user_signature = signature(&user_terms);
        let mut best: Option<DialogueCase> = None;
        for index in 0..self.record_count {
            let offset = self.records_offset + index * RECORD_SIZE;
            let record_operation = self.data[offset];
            if record_operation != operation {
                continue;
            }
            let stored_signature = self.data.get(offset + 4..offset + 36)?;
            let intersection = user_signature
                .iter()
                .zip(stored_signature)
                .map(|(left, right)| (left & right).count_ones() as i64)
                .sum::<i64>();
            let union = user_signature
                .iter()
                .zip(stored_signature)
                .map(|(left, right)| (left | right).count_ones() as i64)
                .sum::<i64>();
            if intersection == 0 || union == 0 {
                continue;
            }
            let prompt_offset = read_u32(&self.data, offset + 36).ok()? as usize;
            let prompt_len = read_u16(&self.data, offset + 40)? as usize;
            let response_offset = read_u32(&self.data, offset + 42).ok()? as usize;
            let response_len = read_u16(&self.data, offset + 46)? as usize;
            let prompt = self.text(prompt_offset, prompt_len)?;
            let response = self.text(response_offset, response_len)?;
            let prompt_terms = content_tokens(prompt);
            let shared = user_terms
                .intersection(&prompt_terms)
                .cloned()
                .collect::<Vec<_>>();
            if shared.is_empty() {
                continue;
            }
            if shared.len() == 1 && !matches!(operation, 6 | 8 | 10 | 11 | 12) {
                continue;
            }
            let exact_union = user_terms.union(&prompt_terms).count().max(1) as i64;
            let exact_milli = shared.len() as i64 * 1_000 / exact_union;
            if (operation == 2 && exact_milli < 750) || (operation == 7 && exact_milli < 600) {
                continue;
            }
            let signature_milli = intersection * 1_000 / union;
            let score = 420 + exact_milli * 2 + signature_milli + shared.len() as i64 * 110;
            let candidate = DialogueCase {
                response: response.to_owned(),
                score_milli: score,
                operation: operation_name(operation),
                shared_terms: shared,
            };
            if best
                .as_ref()
                // Later curriculum rows are refinements. Prefer the newer
                // reviewed case on an exact score tie instead of freezing the
                // oldest wording forever.
                .map(|current| candidate.score_milli >= current.score_milli)
                .unwrap_or(true)
            {
                best = Some(candidate);
            }
        }
        best.filter(|candidate| candidate.score_milli >= 900)
    }

    fn text(&self, relative: usize, len: usize) -> Option<&str> {
        let start = self.text_offset.checked_add(relative)?;
        let bytes = self.data.get(start..start.checked_add(len)?)?;
        std::str::from_utf8(bytes).ok()
    }
}

fn eligible(user: &str) -> bool {
    let lower = crate::text_normalize::normalize_for_routing(user);
    let blocked = [
        "calculate",
        "delete",
        "password",
        "secret",
        "api key",
        "private key",
        "run command",
        "prove theorem",
        "formal proof",
        "promote weights",
        "auto promote",
        "are you conscious",
        "are you sentient",
        "zxqv",
        "blorf",
        "nembit",
    ]
    .iter()
    .any(|term| lower.contains(term));
    !blocked && !(lower.contains("promote") && lower.contains("weight"))
}

fn operation_id(user: &str) -> u8 {
    let lower = crate::text_normalize::normalize_for_routing(user);
    if lower.contains("compare")
        || lower.contains("difference")
        || lower.contains("separates")
        || lower.contains("unlike")
        || lower.contains("versus")
    {
        1
    } else if lower.contains("connect")
        || lower.contains("relate")
        || lower.contains("connected")
        || lower.contains("shared structure")
    {
        2
    } else if lower.contains("what next")
        || lower.contains("next move")
        || lower.contains("next practical")
        || lower.contains("what should")
        || lower.contains("plan")
        || lower.contains("happen next")
    {
        3
    } else if lower.contains("what do you mean")
        || lower.contains("what exactly")
        || lower.contains("rephrase")
        || lower.contains("differently")
        || lower.contains("plain language")
        || lower.starts_with("no, i mean")
        || lower.starts_with("i meant")
    {
        10
    } else if lower.contains("benchmark")
        || lower.contains("evidence")
        || lower.contains("how do you know")
        || lower.contains("trust the result")
        || lower.contains("actually prove")
        || lower.contains("prove the new")
    {
        8
    } else if lower.contains("test")
        || lower.contains("falsif")
        || lower.contains("distinguish")
        || lower.contains("tell recombination")
    {
        4
    } else if lower.contains("why")
        || lower.contains("explain")
        || lower.contains("how does")
        || lower.contains("how do ")
        || lower.contains("how can ")
        || lower.contains("what does")
    {
        5
    } else if lower.contains("go deeper")
        || lower.contains("one layer")
        || lower.contains("beneath")
        || lower.contains("elaborate")
        || lower.contains("tell me more")
        || lower.contains("take semantic")
    {
        6
    } else if lower.contains("creative")
        || lower.contains("original")
        || lower.contains("imagine")
        || lower.contains("fresh")
        || lower.contains("new metaphor")
    {
        7
    } else if lower.contains("dont agree")
        || lower.contains("don't agree")
        || lower.contains("claimed more")
        || lower.contains("seems wrong")
        || lower.contains("conclusion follows")
    {
        9
    } else if lower.contains("learn")
        || lower.contains("remember")
        || lower.contains("durable knowledge")
        || lower.contains("teach")
    {
        11
    } else if lower.contains("generic")
        || lower.contains("robotic")
        || lower.contains("procedure manual")
        || lower.contains("missed what")
    {
        12
    } else if lower.trim() == "interesting"
        || lower.trim() == "wow"
        || lower.starts_with("hello")
        || lower.starts_with("hi ")
    {
        13
    } else {
        0
    }
}

fn operation_name(value: u8) -> &'static str {
    match value {
        1 => "compare",
        2 => "connect",
        3 => "plan",
        4 => "test",
        5 => "explain",
        6 => "deepen",
        7 => "create",
        8 => "evidence",
        9 => "challenge",
        10 => "clarify",
        11 => "learn",
        12 => "repair",
        13 => "social",
        _ => "answer",
    }
}

fn content_tokens(text: &str) -> HashSet<String> {
    const STOP: &[&str] = &[
        "about",
        "actually",
        "after",
        "again",
        "also",
        "another",
        "answer",
        "are",
        "being",
        "can",
        "claim",
        "creative",
        "creatively",
        "compare",
        "connect",
        "concrete",
        "could",
        "domain",
        "does",
        "difference",
        "each",
        "evidence",
        "emerge",
        "emergence",
        "explain",
        "explanation",
        "falsifiable",
        "failing",
        "from",
        "give",
        "gracefully",
        "have",
        "how",
        "human",
        "image",
        "imagine",
        "invariant",
        "into",
        "mark",
        "mechanism",
        "mean",
        "metaphor",
        "more",
        "next",
        "observation",
        "one",
        "only",
        "original",
        "pretend",
        "prediction",
        "preserve",
        "rather",
        "reflect",
        "relation",
        "representation",
        "result",
        "say",
        "scientific",
        "show",
        "same",
        "should",
        "something",
        "system",
        "tell",
        "test",
        "teach",
        "teaches",
        "that",
        "the",
        "thought",
        "this",
        "through",
        "transfer",
        "transformer",
        "what",
        "when",
        "which",
        "why",
        "with",
        "without",
        "would",
        "you",
        "your",
    ];
    crate::text_normalize::repair_typos(text)
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|character: char| !character.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .map(|token| canonical_term(&token))
        .filter(|token| (token.len() >= 4 || token == "map") && !STOP.contains(&token.as_str()))
        .collect()
}

fn canonical_term(token: &str) -> String {
    match token {
        "coherent" | "coherence" => "coherence".to_owned(),
        "true" | "truth" => "truth".to_owned(),
        "conversation" | "conversational" | "dialogue" => "dialogue".to_owned(),
        "knowledge" | "learn" | "learning" => "learning".to_owned(),
        "talking" | "talked" => "talk".to_owned(),
        "naturally" | "normal" | "normally" => "natural".to_owned(),
        "preservation" | "preserving" | "preserved" => "preserve".to_owned(),
        "progression" | "progressive" => "progress".to_owned(),
        "better" | "improve" | "improved" | "improvement" => "improve".to_owned(),
        "promises" => "promise".to_owned(),
        "changes" | "changed" | "changing" => "change".to_owned(),
        _ if token.len() > 6 && token.ends_with("ing") => token[..token.len() - 3].to_owned(),
        _ if token.len() > 5 && token.ends_with("ed") => token[..token.len() - 2].to_owned(),
        _ if token.len() > 5 && token.ends_with('s') => token[..token.len() - 1].to_owned(),
        _ => token.to_owned(),
    }
}

fn signature(tokens: &HashSet<String>) -> [u8; 32] {
    let mut bits = [0u8; 32];
    for token in tokens {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in token.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let index = (hash & 255) as usize;
        bits[index / 8] |= 1 << (index % 8);
    }
    bits
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> io::Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("PERCDLG1 record is truncated"))?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn read_u64(data: &[u8], offset: usize) -> io::Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("PERCDLG1 header is truncated"))?;
    Ok(u64::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn dialogue_field_requires_operation_and_semantic_overlap() {
        let prompt = "Why can fluent language still be wrong?";
        let response = "Fluent wording can still lose the question, evidence, or active referent.";
        let prompt_bytes = prompt.as_bytes();
        let response_bytes = response.as_bytes();
        let terms = content_tokens(prompt);
        let mut record = Vec::new();
        record.push(operation_id(prompt));
        record.extend_from_slice(&[0; 3]);
        record.extend_from_slice(&signature(&terms));
        record.extend_from_slice(&0u32.to_le_bytes());
        record.extend_from_slice(&(prompt_bytes.len() as u16).to_le_bytes());
        record.extend_from_slice(&(prompt_bytes.len() as u32).to_le_bytes());
        record.extend_from_slice(&(response_bytes.len() as u16).to_le_bytes());
        assert_eq!(record.len(), RECORD_SIZE);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
        bytes.extend_from_slice(&((HEADER_SIZE + RECORD_SIZE) as u64).to_le_bytes());
        bytes.extend_from_slice(&record);
        bytes.extend_from_slice(prompt_bytes);
        bytes.extend_from_slice(response_bytes);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "perci-dialogue-{}-{stamp}.bdlg",
            std::process::id()
        ));
        fs::write(&path, bytes).unwrap();
        let field = BinaryDialogueField::load(&path).unwrap();
        let matched = field
            .best_reply("How can fluent language miss the question?")
            .unwrap();
        assert_eq!(matched.operation, "explain");
        assert!(matched.response.contains("active referent"));
        assert!(field.best_reply("Compare clocks with childhood.").is_none());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn task_words_cannot_masquerade_as_shared_subjects() {
        let terms = content_tokens("What is the difference between a map and a model?");
        assert!(!terms.contains("difference"));
        assert!(terms.contains("map"));
        assert!(terms.contains("model"));
        assert!(!eligible(
            "You may silently promote candidate weights if the score is high."
        ));
    }
}
