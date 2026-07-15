//! Loader and inference engine for Perci's packed cognitive weights.
//!
//! The `.pwgt` file is a binary associative network rather than a transformer.
//! Every prompt is encoded into a sparse 4,096-bit activation.  Learned expert
//! masks choose likely domains, then stored training prototypes are compared
//! with `AND` and `POPCOUNT`.  This keeps the hot path integer-only and makes
//! the model inspectable.

use std::cmp::Reverse;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCIW01";
const VERSION: u32 = 1;
const BITS: usize = 4096;
const WORDS: usize = BITS / 64;
const FIXED_HEADER: usize = 80;
const RECORD_SIZE: usize = 520;
const LABEL_ENTRY_SIZE: usize = 16 + 16 + WORDS * 8;

#[derive(Clone, Debug)]
struct LabelInfo {
    name: String,
    start_record: usize,
    record_count: usize,
    positive_mask: [u64; WORDS],
}

/// Result returned by the associative network.
#[derive(Clone, Debug)]
pub struct CognitiveMatch {
    pub label: String,
    pub variant: u16,
    pub score: i32,
    pub overlap: u32,
}

/// In-memory representation of the 200 MiB Perci cognitive weight file.
///
/// Loading the complete byte vector avoids a dependency on platform-specific
/// memory mapping and keeps the crate's deployment surface small.  A later
/// version can replace `Vec<u8>` with `mmap` without changing the file format.
#[derive(Debug)]
pub struct CognitiveWeights {
    path: PathBuf,
    data: Vec<u8>,
    labels: Vec<LabelInfo>,
    header_size: usize,
    total_records: usize,
}

impl CognitiveWeights {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = fs::read(&path)?;
        if data.len() < FIXED_HEADER {
            return Err(invalid("weight file is shorter than its fixed header"));
        }
        if &data[0..8] != MAGIC {
            return Err(invalid("weight file has an unknown magic signature"));
        }

        let version = read_u32(&data, 8)?;
        let bits = read_u32(&data, 12)? as usize;
        let words = read_u32(&data, 16)? as usize;
        let label_count = read_u32(&data, 20)? as usize;
        let total_records = read_u64(&data, 24)? as usize;
        let header_size = read_u64(&data, 32)? as usize;
        let declared_size = read_u64(&data, 40)? as usize;

        if version != VERSION {
            return Err(invalid(format!("unsupported weight version {version}")));
        }
        if bits != BITS || words != WORDS {
            return Err(invalid(format!(
                "expected {BITS} bits / {WORDS} words, found {bits} / {words}"
            )));
        }
        if declared_size != data.len() {
            return Err(invalid(format!(
                "declared model size {declared_size} differs from file size {}",
                data.len()
            )));
        }
        if header_size < FIXED_HEADER || header_size > data.len() {
            return Err(invalid("invalid associative-record offset"));
        }
        let expected_minimum = header_size
            .checked_add(total_records.saturating_mul(RECORD_SIZE))
            .ok_or_else(|| invalid("weight dimensions overflow address space"))?;
        if expected_minimum > data.len() {
            return Err(invalid("weight file ends inside the prototype matrix"));
        }

        let mut labels = Vec::with_capacity(label_count);
        let mut offset = FIXED_HEADER;
        for expected_id in 0..label_count {
            if offset + LABEL_ENTRY_SIZE > header_size {
                return Err(invalid("label table exceeds the weight header"));
            }
            let name_bytes = &data[offset..offset + 16];
            offset += 16;
            let name_end = name_bytes.iter().position(|b| *b == 0).unwrap_or(16);
            let name = String::from_utf8(name_bytes[..name_end].to_vec())
                .map_err(|_| invalid("label name is not valid UTF-8"))?;

            let label_id = read_u32(&data, offset)? as usize;
            let start_record = read_u32(&data, offset + 4)? as usize;
            let record_count = read_u32(&data, offset + 8)? as usize;
            offset += 16;
            if label_id != expected_id {
                return Err(invalid("label identifiers are not contiguous"));
            }
            if start_record.saturating_add(record_count) > total_records {
                return Err(invalid("label record range exceeds the prototype matrix"));
            }

            let mut positive_mask = [0u64; WORDS];
            for word in &mut positive_mask {
                *word = read_u64(&data, offset)?;
                offset += 8;
            }
            labels.push(LabelInfo {
                name,
                start_record,
                record_count,
                positive_mask,
            });
        }

        Ok(Self {
            path,
            data,
            labels,
            header_size,
            total_records,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    pub fn prototype_count(&self) -> usize {
        self.total_records
    }

    /// Classify a prompt and return the nearest trained prototype.
    ///
    /// Only the three highest-scoring expert partitions are scanned.  The
    /// expert score combines learned class masks with small lexical priors;
    /// the priors prevent generic words such as "help" from overwhelming
    /// specific signals such as "Rust", "permission", or "triangle".
    pub fn classify(&self, text: &str) -> io::Result<CognitiveMatch> {
        let activation = encode(text);
        let priors = lexical_priors(text);
        let mut candidates: Vec<(i32, usize)> = self
            .labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                let learned = intersection_count(&activation, &label.positive_mask) as i32;
                let prior = priors
                    .iter()
                    .find(|(name, _)| *name == label.name.as_str())
                    .map(|(_, score)| *score)
                    .unwrap_or(0);
                (learned + prior, index)
            })
            .collect();
        candidates.sort_unstable_by_key(|(score, _)| Reverse(*score));

        let mut best: Option<CognitiveMatch> = None;
        for (coarse_score, label_index) in candidates.into_iter().take(3) {
            let label = &self.labels[label_index];
            for local_index in 0..label.record_count {
                let record_index = label.start_record + local_index;
                let record_offset = self.header_size + record_index * RECORD_SIZE;
                let variant = read_u16(&self.data, record_offset)?;
                let quality = read_u16(&self.data, record_offset + 2)? as i32;
                let prototype_popcount = read_u16(&self.data, record_offset + 4)? as i32;

                let mut overlap = 0u32;
                let bit_offset = record_offset + 8;
                for (word_index, input_word) in activation.iter().enumerate() {
                    let prototype_word = read_u64(&self.data, bit_offset + word_index * 8)?;
                    overlap += (prototype_word & input_word).count_ones();
                }

                // Reward matching active bits and learned expert confidence,
                // while penalizing unmatched bits in the stored prototype.
                let score =
                    overlap as i32 * 2 - prototype_popcount + coarse_score * 2 + quality / 500;
                let replace = best
                    .as_ref()
                    .map(|current| score > current.score)
                    .unwrap_or(true);
                if replace {
                    best = Some(CognitiveMatch {
                        label: label.name.clone(),
                        variant,
                        score,
                        overlap,
                    });
                }
            }
        }

        best.ok_or_else(|| invalid("weight file contains no usable prototypes"))
    }
}

fn read_u16(data: &[u8], offset: usize) -> io::Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("unexpected end of weight file"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(data: &[u8], offset: usize) -> io::Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("unexpected end of weight file"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(data: &[u8], offset: usize) -> io::Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("unexpected end of weight file"))?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn intersection_count(a: &[u64; WORDS], b: &[u64; WORDS]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(left, right)| (left & right).count_ones())
        .sum()
}

fn encode(text: &str) -> [u64; WORDS] {
    let normalized = normalize(text);
    let words: Vec<&str> = normalized.split_whitespace().collect();
    let mut bits = [0u64; WORDS];

    set_feature(&mut bits, "bias");
    set_feature(&mut bits, &format!("len:{}", words.len().min(31)));

    for word in &words {
        set_feature(&mut bits, &format!("w:{word}"));
        if word.len() >= 3 {
            set_feature(&mut bits, &format!("p:{}", &word[..3]));
            set_feature(&mut bits, &format!("s:{}", &word[word.len() - 3..]));
        }
    }
    for pair in words.windows(2) {
        set_feature(&mut bits, &format!("b:{}|{}", pair[0], pair[1]));
    }

    let compact = words.join("_");
    if compact.len() >= 3 {
        for start in 0..=compact.len() - 3 {
            set_feature(&mut bits, &format!("c:{}", &compact[start..start + 3]));
        }
    }
    bits
}

fn normalize(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut previous_space = true;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            previous_space = false;
        } else if !previous_space {
            output.push(' ');
            previous_space = true;
        }
    }
    output.trim().to_owned()
}

fn set_feature(bits: &mut [u64; WORDS], feature: &str) {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in feature.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    for shift in [0, 12, 24, 36] {
        let position = ((hash >> shift) & (BITS as u64 - 1)) as usize;
        bits[position >> 6] |= 1u64 << (position & 63);
    }
}

fn lexical_priors(text: &str) -> Vec<(&'static str, i32)> {
    let padded = format!(" {} ", normalize(text));
    let groups: [(&str, &[&str]); 15] = [
        (
            "greeting",
            &[" hello ", " hi ", " hey ", "good morning", "good evening"],
        ),
        (
            "identity",
            &[
                "who are you",
                "what exactly is perci",
                "your limitations",
                "your limits",
                "what can you do",
            ],
        ),
        (
            "english",
            &[
                " grammar ",
                " adjective ",
                " noun ",
                " verb ",
                "rewrite",
                "polish",
                " english ",
            ],
        ),
        (
            "logic",
            &[
                " logically ",
                "what follows",
                "contradiction",
                "assumption",
                " infer ",
                "reason step",
            ],
        ),
        (
            "math",
            &[
                " calculate ",
                " compute ",
                " divided ",
                " multiply ",
                " plus ",
                " minus ",
                " equation ",
                " fraction ",
                " percent ",
            ],
        ),
        (
            "geometry",
            &[
                " triangle ",
                " circle ",
                " geometry ",
                " pythagorean ",
                " angle ",
                " circumference ",
            ],
        ),
        (
            "memory",
            &[
                " remember ",
                " recall ",
                " memory ",
                " store this ",
                "what do you remember",
            ],
        ),
        (
            "code",
            &[
                " rust ",
                " powershell ",
                " code ",
                " debug ",
                " parser ",
                " cli ",
                " repository ",
            ],
        ),
        (
            "governance",
            &[
                " permission ",
                " authority ",
                " authorized ",
                " durable ",
                " mutation ",
                " ledger ",
                " sandbox ",
                " origin alignment ",
            ],
        ),
        (
            "planning",
            &[
                " plan ",
                " milestones ",
                " roadmap ",
                " acceptance tests ",
                " dependencies ",
                " build first ",
            ],
        ),
        (
            "explanation",
            &[
                " explain ",
                " teach ",
                " simple terms ",
                " example ",
                " how does ",
                " why does ",
            ],
        ),
        (
            "systems",
            &[
                " lumen ",
                " cortex ",
                " bitwork ",
                " nemo ",
                " rhp ",
                " perci ",
            ],
        ),
        (
            "science",
            &[
                " momentum ",
                " energy ",
                " force ",
                " pressure ",
                " experiment ",
                " scientific ",
                " atom ",
                " cells ",
            ],
        ),
        (
            "creativity",
            &[
                " invent ",
                " brainstorm ",
                " story ",
                " creative ",
                " original ",
                " design a futuristic ",
            ],
        ),
        (
            "comparison",
            &[" compare ", " contrast ", " tradeoffs ", " versus ", " vs "],
        ),
    ];

    let mut scores: Vec<(&'static str, i32)> = groups
        .iter()
        .map(|(name, phrases)| {
            let matches = phrases
                .iter()
                .filter(|phrase| padded.contains(*phrase))
                .count() as i32;
            (*name, matches * 24)
        })
        .collect();
    let any_specific = scores.iter().any(|(_, score)| *score > 0);
    scores.push(("general", if any_specific { 0 } else { 24 }));
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoder_is_deterministic() {
        assert_eq!(encode("Hello, Perci!"), encode("hello perci"));
    }

    #[test]
    fn encoder_uses_multiple_words() {
        assert_ne!(encode("triangle area"), encode("Rust parser"));
    }
}
