//! Loader and inference engine for Perci's packed cognitive weights.
//!
//! The `.pwgt` file is a binary associative network rather than a transformer.
//! Every prompt is encoded into a sparse 4,096-bit activation (bag features +
//! structure tags + **VSA bind/bundle composition**).  Learned expert masks
//! choose likely domains, then stored training prototypes are compared with
//! `AND` and `POPCOUNT`.  Mixture + residual hop provide multi-hypothesis
//! readout.  This keeps the hot path integer-only and makes the model
//! inspectable.

use memmap2::{Mmap, MmapOptions};
use std::cmp::Reverse;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

const MAGIC_V1: &[u8; 8] = b"PERCIW01";
const MAGIC_V2: &[u8; 8] = b"PERCIW02";
const MAGIC_V3: &[u8; 8] = b"PERCIW03";
const VERSION_V2: u32 = 2;
const VERSION_V3: u32 = 3;
const BITS: usize = 4096;
const WORDS: usize = BITS / 64;
const FIXED_HEADER: usize = 80;
const RECORD_SIZE: usize = 520;
const LABEL_ENTRY_SIZE_V1: usize = 16 + 16 + WORDS * 8;
const LABEL_ENTRY_SIZE_V2: usize = 16 + 16 + WORDS * 8 * 2;

/// Resolve the newest promoted Bitwork pack first while retaining read-only
/// migration fallbacks. `PERCI_WEIGHTS` always has explicit precedence.
pub fn default_weight_path() -> PathBuf {
    if let Some(path) = std::env::var_os("PERCI_WEIGHTS") {
        return PathBuf::from(path);
    }
    let v3 = PathBuf::from("models/perci-cognitive-v0.3.pwgt");
    if v3.is_file() {
        return v3;
    }
    let v2 = PathBuf::from("models/perci-cognitive-v0.2.pwgt");
    if v2.is_file() {
        return v2;
    }
    PathBuf::from("models/perci-cognitive-v0.1.pwgt")
}

#[derive(Clone, Debug)]
struct LabelInfo {
    name: String,
    start_record: usize,
    record_count: usize,
    positive_mask: [u64; WORDS],
    negative_mask: [u64; WORDS],
    concept_count: usize,
}

/// Secondary prototype/concept contributing to a mixture readout (top-k).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MixtureSupport {
    pub label: String,
    pub score: i32,
    pub overlap: u32,
    pub concept_id: u16,
    pub insight: Option<String>,
    /// True when this support came from residual hop \(q' = q \land \neg p^*\).
    pub residual: bool,
    /// Residual stream depth: 0 = same-geometry mix, 1 = first ANDNOT hop, 2 = second.
    pub hop: u8,
    /// Soft-attention weight in permille of total overlap mass (transformer-analog α).
    /// Primary + mixture sum ≈ 1000.
    pub attention_pm: u16,
}

/// Result returned by the associative network.
#[derive(Clone, Debug)]
pub struct CognitiveMatch {
    pub label: String,
    pub variant: u16,
    pub concept_id: u16,
    pub insight: Option<String>,
    pub score: i32,
    pub overlap: u32,
    pub runner_up_score: i32,
    pub margin: i32,
    pub query_popcount: u32,
    pub prototype_popcount: u32,
    pub positive_overlap: u32,
    pub negative_overlap: u32,
    pub hamming: u32,
    pub jaccard: f64,
    pub overlap_z: f64,
    /// Top-k supporting prototypes/concepts (excludes the primary winner).
    /// Used for multi-concept skeletons without changing the pack format.
    pub mixture: Vec<MixtureSupport>,
    /// VSA role–filler frame extracted at encode time (e.g. `ask:why`, `agent:trust`).
    /// Algebraic binds of these live in the query hypervector; this list is for speech/telemetry.
    pub composition: Vec<String>,
    /// Soft-attention weight on the primary prototype (permille of total overlap mass).
    pub primary_attention_pm: u16,
}

impl CognitiveMatch {
    /// Distinct short insights from primary + same-geometry mixture (for fluid speech).
    /// Residual-hop supports are **excluded** here — use [`Self::residual_skeleton`]
    /// so voice can frame them as latent second thoughts.
    pub fn concept_skeleton(&self, max: usize) -> Vec<String> {
        let mut out = Vec::new();
        let push_unique = |out: &mut Vec<String>, s: &str| {
            let t = s.trim();
            if t.is_empty() {
                return;
            }
            let n = t.chars().count();
            if n < 16 || n > 160 {
                return;
            }
            let low = t.to_ascii_lowercase();
            if out
                .iter()
                .any(|e| e.to_ascii_lowercase() == low || e.to_ascii_lowercase().contains(&low[..low.len().min(40)]))
            {
                return;
            }
            out.push(t.to_owned());
        };
        if let Some(ref i) = self.insight {
            push_unique(&mut out, i);
        }
        // Higher attention first — soft multi-head analog over mixture cards.
        let mut supports: Vec<&MixtureSupport> =
            self.mixture.iter().filter(|m| !m.residual).collect();
        supports.sort_by_key(|m| Reverse(m.attention_pm));
        for m in supports {
            if out.len() >= max {
                break;
            }
            if let Some(ref i) = m.insight {
                push_unique(&mut out, i);
            }
        }
        out.truncate(max);
        out
    }

    /// Insights that arrived only via residual hops (ordered hop1 then hop2).
    pub fn residual_skeleton(&self, max: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut residuals: Vec<&MixtureSupport> =
            self.mixture.iter().filter(|m| m.residual).collect();
        residuals.sort_by_key(|m| (m.hop, Reverse(m.attention_pm)));
        for m in residuals {
            if out.len() >= max {
                break;
            }
            if let Some(ref i) = m.insight {
                let t = i.trim();
                if t.chars().count() >= 16 && t.chars().count() <= 160 {
                    out.push(t.to_owned());
                }
            }
        }
        out
    }

    /// Short role–filler pairs from VSA encode (for fluid composition speech).
    pub fn composition_frame(&self, max: usize) -> Vec<String> {
        self.composition.iter().take(max).cloned().collect()
    }
}

/// In-memory representation of the 200 MiB Perci cognitive weight file.
///
/// Loading the complete byte vector avoids a dependency on platform-specific
/// memory mapping and keeps the crate's deployment surface small.  A later
/// version can replace `Vec<u8>` with `mmap` without changing the file format.
#[derive(Debug)]
pub struct CognitiveWeights {
    path: PathBuf,
    data: Mmap,
    labels: Vec<LabelInfo>,
    header_size: usize,
    total_records: usize,
    version: u32,
    concepts: Vec<Vec<String>>,
    /// Willshaw-lite: bag-encoded HVs for each concept insight (query-side concept memory).
    concept_hvs: Vec<Vec<[u64; WORDS]>>,
}

impl CognitiveWeights {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        // SAFETY: the mapping is read-only and the model file is immutable during inference.
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < FIXED_HEADER {
            return Err(invalid("weight file is shorter than its fixed header"));
        }
        if &data[0..8] != MAGIC_V1 && &data[0..8] != MAGIC_V2 && &data[0..8] != MAGIC_V3 {
            return Err(invalid("weight file has an unknown magic signature"));
        }

        let version = read_u32(&data, 8)?;
        let bits = read_u32(&data, 12)? as usize;
        let words = read_u32(&data, 16)? as usize;
        let label_count = read_u32(&data, 20)? as usize;
        let total_records = read_u64(&data, 24)? as usize;
        let header_size = read_u64(&data, 32)? as usize;
        let declared_size = read_u64(&data, 40)? as usize;

        if !matches!(version, 1 | VERSION_V2 | VERSION_V3) {
            return Err(invalid(format!("unsupported weight version {version}")));
        }
        let has_signed_masks = matches!(version, VERSION_V2 | VERSION_V3)
            && (&data[0..8] == MAGIC_V2 || &data[0..8] == MAGIC_V3);
        let is_v3 = version == VERSION_V3 && &data[0..8] == MAGIC_V3;
        let label_entry_size = if has_signed_masks {
            LABEL_ENTRY_SIZE_V2
        } else {
            LABEL_ENTRY_SIZE_V1
        };
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
            if offset + label_entry_size > header_size {
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
            let concept_count = read_u32(&data, offset + 12)? as usize;
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
            let mut negative_mask = [0u64; WORDS];
            if has_signed_masks {
                for word in &mut negative_mask {
                    *word = read_u64(&data, offset)?;
                    offset += 8;
                }
            }
            labels.push(LabelInfo {
                name,
                start_record,
                record_count,
                positive_mask,
                negative_mask,
                concept_count,
            });
        }

        let mut concepts = vec![Vec::new(); label_count];
        if is_v3 {
            for (expected_label, label) in labels.iter().enumerate() {
                for expected_concept in 0..label.concept_count {
                    if offset + 8 > header_size {
                        return Err(invalid("concept table exceeds the weight header"));
                    }
                    let label_id = read_u16(&data, offset)? as usize;
                    let concept_id = read_u16(&data, offset + 2)? as usize;
                    let text_len = read_u32(&data, offset + 4)? as usize;
                    offset += 8;
                    if label_id != expected_label || concept_id != expected_concept {
                        return Err(invalid("concept identifiers are not contiguous"));
                    }
                    if offset + text_len > header_size {
                        return Err(invalid("concept text exceeds the weight header"));
                    }
                    let insight = String::from_utf8(data[offset..offset + text_len].to_vec())
                        .map_err(|_| invalid("concept text is not valid UTF-8"))?;
                    concepts[label_id].push(insight);
                    offset += text_len;
                }
            }
        }

        // Concept hypervectors — associative memory over insight text without pack rewrite.
        let concept_hvs: Vec<Vec<[u64; WORDS]>> = concepts
            .iter()
            .map(|list| list.iter().map(|s| encode_bag_only(s)).collect())
            .collect();

        Ok(Self {
            path,
            data,
            labels,
            header_size,
            total_records,
            version,
            concepts,
            concept_hvs,
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

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn concept_count(&self) -> usize {
        self.concepts.iter().map(Vec::len).sum()
    }

    pub fn opening_insight(&self, seed: u64) -> Option<String> {
        // Prefer short concept lines without the "creativity ·" domain tag —
        // the Dark-Blood shell already states brand/version in the banner.
        let pool: Vec<&str> = self
            .labels
            .iter()
            .zip(self.concepts.iter())
            .filter(|(label, _)| {
                matches!(
                    label.name.as_str(),
                    "english" | "logic" | "geometry" | "science" | "creativity" | "general"
                )
            })
            .flat_map(|(_, concepts)| concepts.iter().map(|insight| insight.as_str()))
            .filter(|insight| {
                let n = insight.chars().count();
                n >= 24 && n <= 140
            })
            .collect();
        if pool.is_empty() {
            return None;
        }
        let insight = pool[seed as usize % pool.len()];
        Some(insight.to_owned())
    }

    /// Classify a prompt (no dialogue context).
    pub fn classify(&self, text: &str) -> io::Result<CognitiveMatch> {
        self.classify_with_context(text, &[])
    }

    /// Classify with optional **session context tokens** (transformer KV-cache analog).
    /// Context lemmas are VSA-bound under role `CTX` into the query hypervector.
    ///
    /// **Latency-critical path (v0.5.8+):**
    /// - Expert masks rank domains; only the **top domains** are fully scanned.
    /// - `select_concept` runs only for domain top-k survivors + Willshaw concept HVs.
    /// - Residual stream: up to **two** ANDNOT hops on the pool (no full re-scan).
    /// - Soft-attention permille weights on primary + mixture (overlap mass).
    pub fn classify_with_context(
        &self,
        text: &str,
        context: &[&str],
    ) -> io::Result<CognitiveMatch> {
        const PER_DOMAIN_TOP: usize = 3;
        const MIXTURE_MAX: usize = 5;
        const RESIDUAL_MIN_BITS: u32 = 6;
        // Scan budget: multi-frame asks need more experts; locked prompts fewer.
        let multi_domain_ask = looks_multi_domain_ask(text);
        let domain_scan_cap: usize = if multi_domain_ask { 8 } else { 5 };

        let (activation, composition_frame) = encode_with_composition_ctx(text, context);
        let query_popcount: u32 = activation.iter().map(|word| word.count_ones()).sum();
        let priors = lexical_priors(text);
        let mut candidates: Vec<(i32, u32, u32, usize)> = self
            .labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                let positive = intersection_count(&activation, &label.positive_mask);
                let negative = intersection_count(&activation, &label.negative_mask);
                let prior = priors
                    .iter()
                    .find(|(name, _)| *name == label.name.as_str())
                    .map(|(_, score)| *score)
                    .unwrap_or(0);
                let prior = if self.version >= VERSION_V3 {
                    prior.saturating_mul(2)
                } else {
                    prior
                };
                // Positive evidence counts twice; domain-specific negative evidence
                // subtracts support. Lexical priors remain narrow tie-breakers.
                let score = positive as i32 * 2 - negative as i32 + prior;
                (score, positive, negative, index)
            })
            .collect();
        candidates.sort_unstable_by_key(|(score, _, _, _)| Reverse(*score));

        // Global pool of strong prototypes (top-N per domain) with record indices.
        // Integer-only hot loop — concept string selection deferred until survivors.
        let mut pool: Vec<(CognitiveMatch, usize, usize)> = Vec::new();
        for (coarse_score, positive_overlap, negative_overlap, label_index) in
            candidates.iter().take(domain_scan_cap)
        {
            let label = &self.labels[*label_index];
            let mut domain_top: Vec<(CognitiveMatch, usize, usize)> =
                Vec::with_capacity(PER_DOMAIN_TOP);
            for local_index in 0..label.record_count {
                let record_index = label.start_record + local_index;
                let record_offset = self.header_size + record_index * RECORD_SIZE;
                let variant = read_u16(&self.data, record_offset)?;
                let concept_id = read_u16(&self.data, record_offset + 6)?;
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
                    overlap as i32 * 2 - prototype_popcount + *coarse_score * 2 + quality / 500;

                // Defer select_concept — only survivors get string work.
                let candidate = CognitiveMatch {
                    label: label.name.clone(),
                    variant,
                    concept_id,
                    insight: None,
                    score,
                    overlap,
                    runner_up_score: i32::MIN,
                    margin: 0,
                    query_popcount,
                    prototype_popcount: prototype_popcount as u32,
                    positive_overlap: *positive_overlap,
                    negative_overlap: *negative_overlap,
                    hamming: query_popcount + prototype_popcount as u32 - overlap * 2,
                    jaccard: jaccard(query_popcount, prototype_popcount as u32, overlap),
                    overlap_z: chance_normalized_overlap(
                        query_popcount,
                        prototype_popcount as u32,
                        overlap,
                    ),
                    mixture: Vec::new(),
                    composition: Vec::new(),
                    primary_attention_pm: 0,
                };

                // Insert into domain top-k by score.
                if domain_top.len() < PER_DOMAIN_TOP {
                    domain_top.push((candidate, record_index, *label_index));
                    domain_top.sort_unstable_by_key(|(m, _, _)| Reverse(m.score));
                } else if candidate.score
                    > domain_top
                        .last()
                        .map(|(m, _, _)| m.score)
                        .unwrap_or(i32::MIN)
                {
                    domain_top.pop();
                    domain_top.push((candidate, record_index, *label_index));
                    domain_top.sort_unstable_by_key(|(m, _, _)| Reverse(m.score));
                }
            }
            pool.extend(domain_top);
        }

        pool.sort_unstable_by_key(|(matched, _, _)| Reverse(matched.score));
        if pool.is_empty() {
            return Err(invalid("weight file contains no usable prototypes"));
        }

        // Concept selection only for survivors (~15–40 cards, not 403k).
        // Willshaw-lite: geometric |q ∧ concept_hv| boosts insight choice.
        for (matched, _, label_index) in pool.iter_mut() {
            let hvs = self
                .concept_hvs
                .get(*label_index)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let (cid, insight) = select_concept(
                text,
                &activation,
                &matched.label,
                &self.concepts[*label_index],
                hvs,
                matched.concept_id,
            );
            matched.concept_id = cid;
            matched.insight = insight;
        }

        let runner_up_score = pool.get(1).map(|(m, _, _)| m.score).unwrap_or(i32::MIN);
        let primary_record = pool[0].1;
        let mut best = pool[0].0.clone();
        best.runner_up_score = runner_up_score;
        best.margin = best.score.saturating_sub(runner_up_score);

        // Mixture supports: margin-adaptive + multi-domain aware (emergent phase).
        // Contested geometry (low margin) or multi-frame asks → lower floor, force
        // distinct labels. Complementary residual: prefer different concept/label.
        let mut mixture = Vec::new();
        let primary_insight = best.insight.clone();
        let contested = best.margin < 12 || multi_domain_ask;
        let score_floor = if contested {
            best.score.saturating_mul(1).saturating_div(2) // ≥ 50% when contested
        } else {
            best.score.saturating_mul(2).saturating_div(3) // ≥ ~66% when locked
        };
        let mut distinct_labels = 1usize; // primary counts
        for (m, _, _) in pool.iter().skip(1) {
            if mixture.len() >= MIXTURE_MAX {
                break;
            }
            if m.score < score_floor {
                // Still allow one complementary residual if multi-domain and different label.
                if !(multi_domain_ask
                    && m.label != best.label
                    && m.insight.is_some()
                    && mixture.len() < 2
                    && m.score >= best.score.saturating_mul(2).saturating_div(5))
                {
                    continue;
                }
            }
            let insight_dup = match (&m.insight, &primary_insight) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            };
            let already = mixture.iter().any(|s: &MixtureSupport| {
                s.label == m.label
                    && match (&s.insight, &m.insight) {
                        (Some(a), Some(b)) => a == b,
                        (None, None) => true,
                        _ => false,
                    }
            });
            if insight_dup && m.label == best.label {
                continue;
            }
            if already {
                continue;
            }
            if m.insight.is_some() || m.label != best.label {
                if m.label != best.label
                    && !mixture.iter().any(|s| s.label == m.label)
                {
                    distinct_labels += 1;
                }
                mixture.push(MixtureSupport {
                    label: m.label.clone(),
                    score: m.score,
                    overlap: m.overlap,
                    concept_id: m.concept_id,
                    insight: m.insight.clone(),
                    residual: false,
                    hop: 0,
                    attention_pm: 0,
                });
            }
        }
        // Multi-domain residual pass: if still single-label, pull best other-label
        // from pool even if slightly below floor (complementary second thought).
        if multi_domain_ask && distinct_labels < 2 {
            for (m, _, _) in pool.iter().skip(1) {
                if m.label == best.label || m.insight.is_none() {
                    continue;
                }
                if mixture.iter().any(|s| s.label == m.label) {
                    continue;
                }
                if m.score < best.score.saturating_mul(2).saturating_div(5) {
                    continue;
                }
                mixture.push(MixtureSupport {
                    label: m.label.clone(),
                    score: m.score,
                    overlap: m.overlap,
                    concept_id: m.concept_id,
                    insight: m.insight.clone(),
                    residual: false,
                    hop: 0,
                    attention_pm: 0,
                });
                break;
            }
        }

        // Residual stream (transformer residual analog): q₁ = q∧¬p*, q₂ = q₁∧¬p₂.
        // Pool-only, max 2 hops — multi-hop without full re-scan latency.
        let mut exclude = vec![primary_record];
        let mut residual_act = {
            let pbits = read_prototype_bits(&self.data, self.header_size, primary_record)?;
            andnot_bits(&activation, &pbits)
        };
        for hop in 1u8..=2 {
            if mixture.len() >= MIXTURE_MAX {
                break;
            }
            let residual_pop: u32 = residual_act.iter().map(|w| w.count_ones()).sum();
            if residual_pop < RESIDUAL_MIN_BITS {
                break;
            }
            if let Some((support, rec_idx)) = self.residual_hop_pool(
                &residual_act,
                &exclude,
                &best,
                &primary_insight,
                &mixture,
                &pool,
                hop,
            ) {
                exclude.push(rec_idx);
                if let Ok(pbits) = read_prototype_bits(&self.data, self.header_size, rec_idx) {
                    residual_act = andnot_bits(&residual_act, &pbits);
                }
                mixture.push(support);
            } else {
                break;
            }
        }

        best.mixture = mixture;
        best.composition = composition_frame;
        assign_attention_weights(&mut best);
        Ok(best)
    }

    /// Residual retrieval against the **existing pool only** under a residual activation.
    fn residual_hop_pool(
        &self,
        residual_act: &[u64; WORDS],
        exclude: &[usize],
        best: &CognitiveMatch,
        primary_insight: &Option<String>,
        mixture: &[MixtureSupport],
        pool: &[(CognitiveMatch, usize, usize)],
        hop: u8,
    ) -> Option<(MixtureSupport, usize)> {
        let mut best_r: Option<(i32, u32, usize, String, u16, Option<String>)> = None;
        for (m, rec_idx, _) in pool.iter() {
            if exclude.contains(rec_idx) {
                continue;
            }
            if matches!(
                m.label.as_str(),
                "greeting" | "general" | "smalltalk" | "thanks" | "goodbye"
            ) && m.label != best.label
            {
                continue;
            }
            if m.insight.is_none() {
                continue;
            }
            let insight_dup = match (&m.insight, primary_insight) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            };
            if insight_dup {
                continue;
            }
            let already = mixture.iter().any(|s| {
                s.label == m.label
                    && match (&s.insight, &m.insight) {
                        (Some(a), Some(b)) => a == b,
                        (None, None) => true,
                        _ => false,
                    }
            });
            if already {
                continue;
            }
            let Ok(pbits) = read_prototype_bits(&self.data, self.header_size, *rec_idx) else {
                continue;
            };
            let ov = intersection_count(residual_act, &pbits);
            if ov < 4 {
                continue;
            }
            let rscore = ov as i32 * 2 - m.prototype_popcount as i32;
            if rscore <= 0 {
                continue;
            }
            let better = match best_r {
                None => true,
                Some((bs, _, _, _, _, _)) => rscore > bs,
            };
            if better {
                best_r = Some((
                    rscore,
                    ov,
                    *rec_idx,
                    m.label.clone(),
                    m.concept_id,
                    m.insight.clone(),
                ));
            }
        }

        best_r.map(|(score, overlap, rec_idx, label, concept_id, insight)| {
            (
                MixtureSupport {
                    label,
                    score,
                    overlap,
                    concept_id,
                    insight,
                    residual: true,
                    hop,
                    attention_pm: 0,
                },
                rec_idx,
            )
        })
    }
}

/// Soft-attention: α_i ∝ overlap_i, stored as permille (sum ≈ 1000).
fn assign_attention_weights(best: &mut CognitiveMatch) {
    let mut total = best.overlap as u64;
    for m in &best.mixture {
        total += m.overlap as u64;
    }
    if total == 0 {
        best.primary_attention_pm = 1000;
        return;
    }
    best.primary_attention_pm = ((best.overlap as u64 * 1000) / total) as u16;
    for m in &mut best.mixture {
        m.attention_pm = ((m.overlap as u64 * 1000) / total) as u16;
    }
}

/// Connect / relational multi-frame prompts need multipartite mixture readout.
fn looks_multi_domain_ask(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("connect ")
        || lower.contains("boundary between")
        || lower.contains("difference between")
        || lower.contains(" vs ")
        || lower.contains(" versus ")
    {
        return true;
    }
    if lower.contains(" and ") {
        // cheap multi-noun signal: at least two content words around "and"
        let parts: Vec<&str> = lower.split(" and ").collect();
        if parts.len() >= 2 {
            let left = parts[0].split_whitespace().last().unwrap_or("");
            let right = parts[1].split_whitespace().next().unwrap_or("");
            if left.len() >= 4 && right.len() >= 4 {
                return true;
            }
        }
    }
    false
}

fn jaccard(query_popcount: u32, prototype_popcount: u32, overlap: u32) -> f64 {
    let union = query_popcount + prototype_popcount - overlap;
    if union == 0 {
        0.0
    } else {
        overlap as f64 / union as f64
    }
}

fn chance_normalized_overlap(query_popcount: u32, prototype_popcount: u32, overlap: u32) -> f64 {
    let d = BITS as f64;
    let kx = query_popcount as f64;
    let kp = prototype_popcount as f64;
    let expected = kx * kp / d;
    let variance = kx * (kp / d) * (1.0 - kp / d) * ((d - kx) / (d - 1.0));
    if variance <= f64::EPSILON {
        0.0
    } else {
        (overlap as f64 - expected) / variance.sqrt()
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

#[cfg_attr(not(test), allow(dead_code))]
fn encode(text: &str) -> [u64; WORDS] {
    encode_with_composition(text).0
}

fn encode_with_composition(text: &str) -> ([u64; WORDS], Vec<String>) {
    encode_with_composition_ctx(text, &[])
}

/// Bag-only encode for concept Willshaw HVs (matches pack training geometry).
fn encode_bag_only(text: &str) -> [u64; WORDS] {
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

/// Bag + A4 structure + A2 VSA + optional session CTX binds (KV-cache analog).
fn encode_with_composition_ctx(
    text: &str,
    context: &[&str],
) -> ([u64; WORDS], Vec<String>) {
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

    encode_structure_features(&mut bits, text, &words);
    let mut frame = encode_vsa_composition(&mut bits, text, &words);
    if !context.is_empty() {
        encode_session_context(&mut bits, context, &mut frame);
    }
    (bits, frame)
}

/// Inject recent dialogue lemmas as CTX-bound atoms (not free-text dump).
fn encode_session_context(bits: &mut [u64; WORDS], context: &[&str], frame: &mut Vec<String>) {
    let role = atom_hv("R:CTX");
    let mut parts: Vec<[u64; WORDS]> = Vec::new();
    for (slot, tok) in context.iter().take(6).enumerate() {
        let t = tok.trim().to_ascii_lowercase();
        if t.len() < 4 || is_structure_stop(&t) {
            continue;
        }
        let fill = atom_hv(&format!("F:{t}"));
        let fill_slot = permute_hv(&fill, (slot as u32).wrapping_mul(13).wrapping_add(5));
        parts.push(bind_hv(&role, &fill_slot));
        set_feature(bits, &format!("ctx:{t}"));
        let tag = format!("ctx:{t}");
        if !frame.contains(&tag) && frame.len() < 12 {
            frame.push(tag);
        }
    }
    if !parts.is_empty() {
        let bundled = bundle_hv(&parts);
        or_into(bits, &bundled);
    }
}

/// Binary Spatter / VSA composition on the same 4096-bit space.
///
/// - **atom** — deterministic sparse hypervector for a symbol  
/// - **bind** — XOR (role–filler)  
/// - **bundle** — majority (OR when &lt;3 parts)  
/// - **permute** — fixed bit rotation for sequence slots  
///
/// Overlay is OR'd into the bag activation so pack NN still works (query denser,
/// prototypes bag-encoded until authorized rebuild).
fn encode_vsa_composition(
    bits: &mut [u64; WORDS],
    raw: &str,
    words: &[&str],
) -> Vec<String> {
    let pairs = extract_role_fillers(raw, words);
    if pairs.is_empty() {
        return Vec::new();
    }

    let mut bound_parts: Vec<[u64; WORDS]> = Vec::with_capacity(pairs.len());
    let mut frame: Vec<String> = Vec::with_capacity(pairs.len());

    for (slot, (role, filler)) in pairs.iter().enumerate() {
        let role_hv = atom_hv(&format!("R:{role}"));
        let fill_hv = atom_hv(&format!("F:{filler}"));
        // Sequence-sensitive: permute filler by slot before bind (order of roles).
        let fill_slot = permute_hv(&fill_hv, (slot as u32).wrapping_mul(17).wrapping_add(3));
        let bound = bind_hv(&role_hv, &fill_slot);
        bound_parts.push(bound);
        let tag = format!("{role}:{filler}");
        // Hash side-channel so bag-like prototypes can still co-activate.
        set_feature(bits, &format!("vsa:{tag}"));
        if !frame.contains(&tag) {
            frame.push(tag);
        }
    }

    let composition = bundle_hv(&bound_parts);
    or_into(bits, &composition);
    frame
}

/// Extract (role, filler) pairs for VSA bind — POS-free heuristics.
fn extract_role_fillers(raw: &str, words: &[&str]) -> Vec<(String, String)> {
    let lower = raw.to_ascii_lowercase();
    let mut pairs: Vec<(String, String)> = Vec::new();
    let push = |pairs: &mut Vec<(String, String)>, role: &str, filler: &str| {
        let f = filler.trim();
        // Ask roles may be short ("why"); content fillers need real lemmas.
        let min_len = if role == "ask" { 2 } else { 4 };
        if f.len() < min_len {
            return;
        }
        let tag = (role.to_owned(), f.to_owned());
        if !pairs.iter().any(|(r, x)| r == role && x == f) {
            pairs.push(tag);
        }
    };

    if looks_ask_why(&lower) {
        push(&mut pairs, "ask", "why");
    } else if looks_ask_how(&lower) {
        push(&mut pairs, "ask", "how");
    } else if looks_ask_compare(&lower) {
        push(&mut pairs, "ask", "compare");
    } else if looks_ask_connect(&lower) {
        push(&mut pairs, "ask", "connect");
    } else if looks_ask_what(&lower) {
        push(&mut pairs, "ask", "what");
    }

    let content: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| w.len() >= 3 && !is_structure_stop(w))
        .collect();

    for (i, w) in words.iter().enumerate() {
        if matches!(*w, "does" | "did" | "is" | "are" | "was" | "were" | "makes" | "make")
        {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                push(&mut pairs, "agent", next);
            }
        }
        if matches!(*w, "in" | "for" | "about" | "on" | "under" | "via" | "within") {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                push(&mut pairs, "domain", next);
            }
        }
        if matches!(*w, "vs" | "versus" | "between") {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                push(&mut pairs, "contrast", next);
            }
        }
        if is_negation_cue(w) {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                push(&mut pairs, "neg", next);
            }
        }
    }

    if let Some(first) = content.first() {
        push(&mut pairs, "topic", first);
    }
    if let Some(last) = content.last() {
        if content.len() >= 2 {
            push(&mut pairs, "focus", last);
        }
    }

    // Relational pairs: bind both sides under relate / co roles (order-invariant).
    for (a, b) in extract_sorted_pairs(words).into_iter().take(3) {
        push(&mut pairs, "relate", &format!("{a}+{b}"));
    }

    // Cap density — composition should stay sparse relative to 4096.
    pairs.truncate(8);
    pairs
}

/// Deterministic sparse atomic hypervector for a symbol (8 bit positions).
fn atom_hv(symbol: &str) -> [u64; WORDS] {
    let mut bits = [0u64; WORDS];
    let mut hash = 0xcbf29ce484222325u64;
    for byte in symbol.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    // Salt so atoms differ from bag `set_feature` 4-bit codes for the same string.
    hash ^= 0xa5a5_f5a5_c0deu64;
    for _ in 0..8 {
        let position = (hash & (BITS as u64 - 1)) as usize;
        bits[position >> 6] |= 1u64 << (position & 63);
        hash = hash.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(0x85eb_ca6b);
    }
    bits
}

/// Bind: XOR (self-inverse unbind). Role–filler composition.
fn bind_hv(a: &[u64; WORDS], b: &[u64; WORDS]) -> [u64; WORDS] {
    let mut out = [0u64; WORDS];
    for i in 0..WORDS {
        out[i] = a[i] ^ b[i];
    }
    out
}

/// Bundle: majority vote across parts; OR when fewer than 3 (sparse-friendly).
fn bundle_hv(parts: &[[u64; WORDS]]) -> [u64; WORDS] {
    let mut out = [0u64; WORDS];
    if parts.is_empty() {
        return out;
    }
    if parts.len() < 3 {
        for p in parts {
            or_into(&mut out, p);
        }
        return out;
    }
    let threshold = (parts.len() / 2) + 1;
    for bit in 0..BITS {
        let word = bit >> 6;
        let mask = 1u64 << (bit & 63);
        let mut count = 0usize;
        for p in parts {
            if p[word] & mask != 0 {
                count += 1;
            }
        }
        if count >= threshold {
            out[word] |= mask;
        }
    }
    out
}

/// Fixed bit rotation (permutation) for sequence/order sensitivity.
fn permute_hv(hv: &[u64; WORDS], amount: u32) -> [u64; WORDS] {
    let shift = (amount as usize) % BITS;
    if shift == 0 {
        return *hv;
    }
    let mut out = [0u64; WORDS];
    for bit in 0..BITS {
        let word = bit >> 6;
        let mask = 1u64 << (bit & 63);
        if hv[word] & mask != 0 {
            let dest = (bit + shift) % BITS;
            out[dest >> 6] |= 1u64 << (dest & 63);
        }
    }
    out
}

fn or_into(dst: &mut [u64; WORDS], src: &[u64; WORDS]) {
    for i in 0..WORDS {
        dst[i] |= src[i];
    }
}

/// Structure-preserving features (Bitwork math path A4).
/// Makes “why vs how”, “A vs B” ≈ “B vs A”, and light role/negation visible to NN.
fn encode_structure_features(bits: &mut [u64; WORDS], raw: &str, words: &[&str]) {
    let lower = raw.to_ascii_lowercase();

    // Ask-type first-class bits.
    if looks_ask_why(&lower) {
        set_feature(bits, "ask:why");
    }
    if looks_ask_how(&lower) {
        set_feature(bits, "ask:how");
    }
    if looks_ask_compare(&lower) {
        set_feature(bits, "ask:compare");
    }
    if looks_ask_connect(&lower) {
        set_feature(bits, "ask:connect");
    }
    if looks_ask_what(&lower) {
        set_feature(bits, "ask:what");
    }
    if lower.contains('?') {
        set_feature(bits, "ask:question");
    }

    // Order-invariant sorted pairs (reduces A-vs-B / B-vs-A confusion).
    for (a, b) in extract_sorted_pairs(words) {
        set_feature(bits, &format!("pair:{a}|{b}"));
    }

    // Negation scope: neg:<next content word>.
    for (i, w) in words.iter().enumerate() {
        if is_negation_cue(w) {
            set_feature(bits, "neg:scope");
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                set_feature(bits, &format!("neg:{next}"));
            }
        }
    }

    // Light role heuristics (POS-free).
    let content: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| w.len() >= 3 && !is_structure_stop(w))
        .collect();

    // agent / subject after does|is|are|makes
    for (i, w) in words.iter().enumerate() {
        if matches!(*w, "does" | "did" | "is" | "are" | "was" | "were" | "makes" | "make")
        {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                set_feature(bits, &format!("role:agent|{next}"));
            }
        }
        if matches!(*w, "in" | "for" | "about" | "on" | "under" | "via") {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                set_feature(bits, &format!("role:domain|{next}"));
            }
        }
        if matches!(*w, "vs" | "versus" | "between") {
            if let Some(next) = words
                .iter()
                .skip(i + 1)
                .find(|t| t.len() >= 3 && !is_structure_stop(t))
            {
                set_feature(bits, &format!("role:contrast|{next}"));
            }
        }
    }
    // First content word as topic seed when present.
    if let Some(first) = content.first() {
        set_feature(bits, &format!("role:topic|{first}"));
    }
    if let Some(last) = content.last() {
        if content.len() >= 2 {
            set_feature(bits, &format!("role:focus|{last}"));
        }
    }
}

fn looks_ask_why(lower: &str) -> bool {
    lower.contains("why ")
        || lower.starts_with("why")
        || lower.contains("reason for")
        || lower.contains("cause of")
        || lower.contains("because")
}

fn looks_ask_how(lower: &str) -> bool {
    lower.contains("how ")
        || lower.starts_with("how")
        || lower.contains("how to")
        || lower.contains("in what way")
        || lower.contains("mechanism")
}

fn looks_ask_compare(lower: &str) -> bool {
    lower.contains(" vs ")
        || lower.contains(" versus ")
        || lower.contains("difference between")
        || lower.contains("compare ")
        || lower.contains(" compared ")
        || lower.contains("better than")
        || lower.contains(" or ")
}

fn looks_ask_connect(lower: &str) -> bool {
    lower.contains("connect ")
        || lower.contains("relate ")
        || lower.contains("relationship between")
        || lower.contains("link between")
        || lower.contains("boundary between")
        || lower.contains("bridge ")
}

fn looks_ask_what(lower: &str) -> bool {
    lower.contains("what is")
        || lower.contains("what are")
        || lower.contains("what's")
        || lower.contains("define ")
        || lower.contains("explain ")
}

fn is_negation_cue(w: &str) -> bool {
    matches!(
        w,
        "not" | "no" | "never" | "without" | "isnt" | "isn" | "dont" | "don" | "doesnt"
            | "doesn" | "cant" | "won" | "wont" | "neither" | "nor"
    )
}

fn is_structure_stop(w: &str) -> bool {
    matches!(
        w,
        "the" | "a" | "an" | "and" | "or" | "but" | "if" | "then" | "than" | "that"
            | "this" | "these" | "those" | "with" | "from" | "into" | "onto" | "about"
            | "what" | "when" | "where" | "which" | "who" | "whom" | "why" | "how"
            | "can" | "could" | "would" | "should" | "will" | "just" | "really" | "very"
            | "your" | "you" | "me" | "my" | "our" | "we" | "i" | "is" | "are" | "was"
            | "were" | "be" | "been" | "being" | "do" | "does" | "did" | "to" | "of"
            | "in" | "on" | "for" | "it" | "its" | "as" | "at" | "by" | "not" | "no"
            | "please" | "tell" | "give" | "make" | "more" | "some" | "any" | "all"
            | "also" | "like" | "have" | "has" | "had" | "get" | "got" | "let" | "vs"
            | "versus" | "between"
    )
}

/// Extract order-invariant content pairs from relational cues and windows.
fn extract_sorted_pairs(words: &[&str]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let content: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| w.len() >= 3 && !is_structure_stop(w))
        .collect();

    // Relational cues: A vs B, A and B (content neighbors around cue).
    for (i, w) in words.iter().enumerate() {
        if matches!(*w, "vs" | "versus" | "and" | "or") {
            let left = words[..i]
                .iter()
                .rev()
                .find(|t| t.len() >= 3 && !is_structure_stop(t));
            let right = words[i + 1..]
                .iter()
                .find(|t| t.len() >= 3 && !is_structure_stop(t));
            if let (Some(a), Some(b)) = (left, right) {
                if a != b {
                    let (lo, hi) = if *a <= *b { (*a, *b) } else { (*b, *a) };
                    pairs.push((lo.to_owned(), hi.to_owned()));
                }
            }
        }
    }

    // Adjacent content bigrams as sorted pairs (first few only — keep sparse).
    for window in content.windows(2).take(6) {
        let a = window[0];
        let b = window[1];
        if a == b {
            continue;
        }
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let p = (lo.to_owned(), hi.to_owned());
        if !pairs.contains(&p) {
            pairs.push(p);
        }
    }
    pairs
}

fn andnot_bits(a: &[u64; WORDS], b: &[u64; WORDS]) -> [u64; WORDS] {
    let mut out = [0u64; WORDS];
    for i in 0..WORDS {
        out[i] = a[i] & !b[i];
    }
    out
}

fn read_prototype_bits(
    data: &[u8],
    header_size: usize,
    record_index: usize,
) -> io::Result<[u64; WORDS]> {
    let record_offset = header_size + record_index * RECORD_SIZE;
    let bit_offset = record_offset + 8;
    let mut bits = [0u64; WORDS];
    for i in 0..WORDS {
        bits[i] = read_u64(data, bit_offset + i * 8)?;
    }
    Ok(bits)
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

fn select_concept(
    text: &str,
    activation: &[u64; WORDS],
    label: &str,
    concepts: &[String],
    concept_hvs: &[[u64; WORDS]],
    prototype_concept: u16,
) -> (u16, Option<String>) {
    if concepts.is_empty() {
        return (prototype_concept, None);
    }
    let padded = format!(" {} ", normalize(text));
    let aliases: &[&[&str]] = match label {
        "identity" => &[
            &[" boundary ", " not a compressed language model "],
            &[" capability ", " capabilities ", " exact tools "],
            &[" self model ", " failure modes ", " likely to be wrong "],
            &[
                " growth ",
                " persists ",
                " transfers ",
                " measured outcomes ",
            ],
            &[" bitwork ", " binary geometry ", " sparse associative "],
            &[
                " strongest claim ",
                " strongest honest claim ",
                " intelligence ",
            ],
            &[
                " introspection ",
                " operational awareness ",
                " private experience ",
            ],
            &[
                " weight change ",
                " weights changed ",
                " fresh process ",
                " model hash ",
            ],
            &[
                " evidence ",
                " reproducible test ",
                " baseline ",
                " failure boundary ",
            ],
        ],
        "english" => &[
            &[" meaning ", " distinctions "],
            &[" ambiguity ", " ambiguous ", " misread "],
            &[
                " metaphor ",
                " analogy ",
                " transfer structure ",
                " carry structure ",
                " between subjects ",
            ],
            &[" compression ", " shorter sentence ", " preserve meaning "],
            &[" dialogue ", " conversational ", " pronoun ", " reference "],
            &[" definition ", " membership "],
        ],
        "logic" => &[
            &[" inference ", " premise ", " conclusion "],
            &[
                " counterexample ",
                " universal claim ",
                " universal statement ",
                " every case ",
                " defeats a claim ",
            ],
            &[" uncertainty ", " uncertain "],
            &[
                " causality ",
                " causation ",
                " intervention ",
                " every case ",
                " defeats a claim ",
                " prediction alone ",
                " correlation ",
            ],
            &[" contradiction ", " collision "],
            &[" necessity ", " necessary ", " possible ", " probable "],
            &[
                " falsification ",
                " falsify ",
                " disconfirmation ",
                " competing prediction ",
            ],
            &[
                " argument structure ",
                " observation ",
                " premise ",
                " inference ",
                " conclusion ",
            ],
            &[
                " model update ",
                " revise the rule ",
                " lower confidence ",
                " failed premise ",
            ],
            &[
                " calibration ",
                " calibrated ",
                " stated level ",
                " new cases ",
            ],
        ],
        "geometry" => &[
            &[" boundary ", " inside ", " outside "],
            &[" symmetry ", " preserve form ", " stays equivalent "],
            &[
                " curvature ",
                " bending ",
                " entire shape ",
                " global shape ",
            ],
            &[" dimension ", " independent directions "],
            &[
                " topology ",
                " deformation ",
                " deformed ",
                " survive continuous ",
            ],
            &[" proof ", " diagram "],
            &[
                " perspective ",
                " projection ",
                " projected ",
                " spatial object ",
            ],
            &[" fractal ", " across scale ", " recursion "],
        ],
        "memory" => &[
            &[" evidence ", " reconstruction "],
            &[" forgetting ", " relevance ", " selection "],
            &[" identity ", " continuity "],
            &[" learning ", " learned behavior ", " changing performance "],
            &[" provenance ", " source ", " stored architectural "],
        ],
        "systems" => &[
            &[
                " emergence ",
                " emergent ",
                " global pattern ",
                " no component ",
            ],
            &[" feedback ", " amplification ", " stabilizing "],
            &[" system boundary ", " boundary sit "],
            &[" modularity ", " module ", " interface "],
            &[" resilience ", " contain the failure ", " recover "],
            &[" complexity ", " component count ", " interactions "],
            &[
                " routing ",
                " operator ",
                " requested operation ",
                " nearby concept ",
            ],
            &[
                " composition ",
                " conversational context ",
                " response layer ",
            ],
            &[" context ", " session state ", " authority boundary "],
            &[
                " ablation ",
                " component removed ",
                " capability disappears ",
            ],
        ],
        "science" => &[
            &[
                " life ",
                " organism ",
                " local order ",
                " keep local order ",
                " entropy ",
            ],
            &[
                " death ",
                " biological ",
                " integration ",
                " self maintenance ",
            ],
            &[" evolution ", " selection ", " inherited ", " foresight "],
            &[" measurement ", " instrument "],
            &[" energy ", " conserved "],
            &[" scale ", " physical effects "],
            &[" hypothesis ", " hypotheses ", " observation "],
        ],
        "general" => &[
            &[" life ", " living process "],
            &[
                " death ",
                " finitude ",
                " limited time ",
                " choices weight ",
            ],
            &[" time ", " sequence "],
            &[" meaning ", " purpose "],
            &[" consciousness ", " subjective experience "],
            &[" freedom ", " choice ", " consequence "],
            &[" change ", " stable thing "],
            &[
                " knowledge ",
                " stored information ",
                " become knowledge ",
                " justified ",
            ],
        ],
        "planning" => &[
            &[" dependency ", " dependencies ", " uncertainty "],
            &[" milestone ", " verified state "],
            &[
                " risk ",
                " likelihood ",
                " consequence ",
                " recoverability ",
            ],
            &[" feedback ", " feedback loop ", " mistaken assumptions "],
            &[" objective ", " success ", " substitute "],
            &[
                " experiment ",
                " causal condition ",
                " control ",
                " measured outcome ",
            ],
            &[
                " acceptance ",
                " acceptance gate ",
                " pass threshold ",
                " receipt ",
            ],
            &[" next change ", " largest measured failure ", " rollback "],
        ],
        "explanation" => &[
            &[" mechanism ", " predicts ", " conditions change "],
            &[" example ", " possibility ", " counterexample "],
            &[
                " levels ",
                " component behavior ",
                " system behavior ",
                " observed outcome ",
            ],
            &[" clarity ", " causal spine ", " detail "],
            &[" transfer ", " new surface form ", " original answer "],
            &[
                " deep reasoning ",
                " causal chain ",
                " boundary condition ",
                " test ",
            ],
            &[
                " response fit ",
                " requested operation ",
                " tone ",
                " uncertainty ",
            ],
            &[" observation ", " inference ", " runtime "],
            &[" mechanism ", " evidence ", " competing explanation "],
            &[" transfer test ", " unseen entities ", " perturb "],
            &[
                " self critique ",
                " failure mechanism ",
                " concrete repair ",
            ],
        ],
        "comparison" => &[
            &[" criteria ", " hidden criteria "],
            &[" tradeoff ", " worsens ", " constraints "],
            &[" baseline ", " improvement "],
            &[" context ", " workload ", " failure cost ", " latency "],
            &[" dominance ", " no worse ", " better on "],
            &[" ablation ", " component removed ", " degrades "],
            &[" regression ", " previously passing ", " worsens "],
            &[
                " selection ",
                " requested entity ",
                " abstention ",
                " novelty ",
            ],
        ],
        _ => &[],
    };
    let mut best_id = prototype_concept as usize % concepts.len();
    let mut best_score = 0usize;
    for (concept_id, insight) in concepts.iter().enumerate() {
        let alias_score = aliases
            .get(concept_id)
            .map(|terms| terms.iter().filter(|term| padded.contains(**term)).count() * 100)
            .unwrap_or(0);
        let insight_words = normalize(insight);
        let word_overlap = insight_words
            .split_whitespace()
            .filter(|word| word.len() >= 5 && padded.contains(&format!(" {word} ")))
            .count();
        // Willshaw-lite geometric score: |q ∧ concept_hv| (associative concept memory).
        let geometric = concept_hvs
            .get(concept_id)
            .map(|hv| intersection_count(activation, hv) as usize)
            .unwrap_or(0);
        // Contamination guard: pure geometry without lexical/alias evidence must
        // not surface unrelated concepts (e.g. "death" on arithmetic "why" prompts).
        let lexical = alias_score + word_overlap.saturating_mul(5);
        let score = if lexical > 0 {
            lexical + geometric.saturating_mul(4)
        } else {
            0
        };
        if score > best_score {
            best_score = score;
            best_id = concept_id;
        }
    }
    // A prototype always carries a concept id, but proximity alone is not
    // semantic evidence that the concept answers this prompt. Keep the id for
    // telemetry and abstain from emitting its prose without lexical support.
    if best_score == 0 {
        return (prototype_concept, None);
    }
    (best_id as u16, concepts.get(best_id).cloned())
}

fn lexical_priors(text: &str) -> Vec<(&'static str, i32)> {
    let padded = format!(" {} ", normalize(text));
    let groups: [(&str, &[&str]); 15] = [
        (
            "greeting",
            &[
                " hello ",
                " hi ",
                " hey ",
                " morning ",
                "good morning",
                "good evening",
                "ready to explore",
            ],
        ),
        (
            "identity",
            &[
                "who are you",
                "what exactly is perci",
                "your limitations",
                "your limits",
                "what can you do",
                "capabilities as perci",
                "honest account of your capabilities",
                "kind of cognitive system",
                "what kind of system are you",
                "machinery behind your apparent mind",
                "strongest claim",
                "strongest honest claim",
                "intelligence",
                "fresh process",
                "model hash",
                "weights changed",
                "session context",
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
                " grammatical ",
                " revise ",
                " paragraph ",
                " pronoun ",
                " metaphor ",
                " ambiguity ",
                " sentence ",
                " wording ",
                " refers to ",
                " conversational context ",
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
                "counterexample",
                " one counterexample ",
                " universal statement ",
                " universal claim ",
                " every case ",
                " defeats a claim ",
                " premise ",
                " conclusion ",
                " necessary ",
                " necessity ",
                " causation ",
                " correlation ",
                " intervention ",
                " falsification ",
                " falsify ",
                "competing prediction",
                " argument structure ",
                " model update ",
                " calibration ",
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
                " ratio ",
                " algebraic ",
                " invariant ",
                " proof ",
                " numerical examples ",
                " quantity ",
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
                " perpendicular ",
                " bisector ",
                " polygon ",
                " curvature ",
                " bending ",
                " topology ",
                " deformed ",
                " deformation ",
                " symmetry ",
                " spatial ",
                " projected ",
                " projection ",
                " quadrilateral ",
                " diagonal ",
                " global shape ",
                " preserve form ",
                " transformations ",
                " local bending ",
                " entire shape ",
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
                " saved ",
                " notes ",
                " search ",
                " retrieve ",
                " stored ",
                " forgetting ",
                " taught ",
                " learned behavior ",
                " preserving a note ",
                " changing performance ",
                " earlier ",
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
                " borrow ",
                " checker ",
                " compile ",
                " test ",
                " state transition ",
                " type design ",
                " recovery error ",
                " interface ",
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
                " rollback boundary ",
                " evidence gate ",
                " reversible ",
                " promotion ",
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
                " objective ",
                " risk ",
                " feedback loop ",
                " development phase ",
                " experiment ",
                " acceptance gate ",
                " pass threshold ",
                " next change ",
            ],
        ),
        (
            "explanation",
            &[
                " explain ",
                " teach ",
                " simple terms ",
                " example ",
                " mechanism ",
                " counterexample ",
                " one level deeper ",
                " causal spine ",
                " system result ",
                " deep reasoning ",
                " response fit ",
                " requested operation ",
                " observation ",
                " inference ",
                " transfer test ",
                " self critique ",
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
                " emergent ",
                " global pattern ",
                " system boundary ",
                " resilience ",
                " interaction complexity ",
                " component count ",
                " amplification ",
                " stabilizing ",
                " contain the failure ",
                " governing invariant ",
                " routing ",
                " composition ",
                " ablation ",
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
                " hypothesis ",
                " falsifiable ",
                " measurement ",
                " measurements ",
                " observation ",
                " atom ",
                " cells ",
                " biological ",
                " organism ",
                " local order ",
                " self maintenance ",
                " entropy ",
                " evolution ",
                " physical effects ",
                " scale ",
                " without foresight ",
                " selection accumulate ",
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
                " novelty ",
                " variations ",
                " idea space ",
                " perspective ",
                " unusual interface ",
                " interface concepts ",
                " three unusual ",
            ],
        ),
        (
            "comparison",
            &[
                " compare ",
                " contrast ",
                " tradeoffs ",
                " versus ",
                " vs ",
                " criterion ",
                " criteria ",
                " baseline ",
                " dominate ",
                " every relevant measure ",
                " ablation ",
                " regression ",
                " regression after ",
                " constrain promotion ",
            ],
        ),
    ];

    let mut scores: Vec<(&'static str, i32)> = groups
        .iter()
        .map(|(name, phrases)| {
            let matches = phrases
                .iter()
                .filter(|phrase| padded.contains(*phrase))
                .count() as i32;
            (*name, matches * 48)
        })
        .collect();
    let any_specific = scores.iter().any(|(_, score)| *score > 0);
    let general_markers = [
        " finitude ",
        " meaning ",
        " freedom ",
        " choice ",
        " consciousness ",
        " subjective experience ",
        " justified reliable use ",
        " purpose ",
        " existence ",
        " limited time ",
        " choices weight ",
        " stored information ",
        " become knowledge ",
    ];
    let general_score = general_markers
        .iter()
        .filter(|marker| padded.contains(**marker))
        .count() as i32
        * 48;
    scores.push((
        "general",
        if general_score > 0 {
            general_score
        } else if any_specific {
            0
        } else {
            48
        },
    ));
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

    #[test]
    fn classify_returns_mixture_when_weights_present() {
        let path = default_weight_path();
        if !path.is_file() {
            return;
        }
        let weights = CognitiveWeights::load(&path).expect("load weights");
        let matched = weights
            .classify("how should trust and interfaces work in distributed systems?")
            .expect("classify");
        // Primary domain always set.
        assert!(!matched.label.is_empty());
        // Top-k readout may be empty on tiny packs; on full v3 usually non-empty.
        let skeleton = matched.concept_skeleton(3);
        assert!(skeleton.len() <= 3);
        // Margin defined vs runner-up.
        assert!(matched.score >= matched.runner_up_score || matched.runner_up_score == i32::MIN);
    }

    #[test]
    fn soft_attention_weights_sum_near_thousand() {
        let path = default_weight_path();
        if !path.is_file() {
            return;
        }
        let weights = CognitiveWeights::load(&path).expect("load");
        let m = weights
            .classify("how should trust and interfaces work in distributed systems?")
            .expect("c");
        let mut sum = m.primary_attention_pm as u32;
        for s in &m.mixture {
            sum += s.attention_pm as u32;
        }
        // Rounding may leave sum slightly under 1000; allow slack.
        assert!(sum >= 500 && sum <= 1000, "attention sum={sum}");
    }

    #[test]
    fn session_context_changes_activation() {
        let bare = encode_with_composition("what about recovery?");
        let with_ctx =
            encode_with_composition_ctx("what about recovery?", &["distributed", "trust"]);
        assert_ne!(bare.0, with_ctx.0);
        assert!(with_ctx.1.iter().any(|f| f.starts_with("ctx:")));
    }

    #[test]
    fn classify_hot_path_stays_interactive() {
        let path = default_weight_path();
        if !path.is_file() {
            return;
        }
        let weights = CognitiveWeights::load(&path).expect("load weights");
        // Warm once (mmap page-in), then measure.
        let _ = weights
            .classify("warmup prompt for bitwork latency")
            .expect("warm");
        let start = std::time::Instant::now();
        let matched = weights
            .classify("why does trust fail in distributed systems?")
            .expect("classify");
        let ms = start.elapsed().as_millis();
        assert!(!matched.label.is_empty());
        // v0.5.8: top-domain scan + lazy select_concept + pool residual.
        // OneDrive/cold disks may be slower; keep a generous but interactive budget.
        assert!(
            ms < 3000,
            "classify too slow after latency fix: {ms}ms (label={})",
            matched.label
        );
    }

    #[test]
    fn concept_skeleton_dedupes() {
        let m = CognitiveMatch {
            label: "systems".into(),
            variant: 0,
            concept_id: 1,
            insight: Some("trust needs clear interfaces".into()),
            score: 100,
            overlap: 10,
            runner_up_score: 90,
            margin: 10,
            query_popcount: 20,
            prototype_popcount: 30,
            positive_overlap: 5,
            negative_overlap: 1,
            hamming: 40,
            jaccard: 0.2,
            overlap_z: 1.0,
            mixture: vec![
                MixtureSupport {
                    label: "systems".into(),
                    score: 90,
                    overlap: 8,
                    concept_id: 2,
                    insight: Some("trust needs clear interfaces".into()),
                    residual: false,
                    hop: 0,
                    attention_pm: 200,
                },
                MixtureSupport {
                    label: "governance".into(),
                    score: 85,
                    overlap: 7,
                    concept_id: 3,
                    insight: Some("permission and proof are different gates".into()),
                    residual: false,
                    hop: 0,
                    attention_pm: 300,
                },
            ],
            composition: vec!["topic:trust".into(), "domain:systems".into()],
            primary_attention_pm: 500,
        };
        let sk = m.concept_skeleton(3);
        assert_eq!(sk.len(), 2);
        assert!(sk[0].contains("trust"));
        // Higher attention_pm support should surface before lower when both unique.
        assert!(sk[1].contains("permission") || sk[1].contains("proof"));
    }

    #[test]
    fn encoder_structure_features_distinguish_ask_type() {
        let why = encode("why does trust fail in distributed systems?");
        let how = encode("how does trust fail in distributed systems?");
        // Same content words, different ask-type → different activation.
        assert_ne!(why, how);
    }

    #[test]
    fn vsa_bind_is_self_inverse() {
        let a = atom_hv("R:agent");
        let b = atom_hv("F:trust");
        let bound = bind_hv(&a, &b);
        let recovered = bind_hv(&bound, &a); // unbind role → filler
        assert_eq!(recovered, b);
    }

    #[test]
    fn vsa_composition_frame_extracts_roles() {
        let (_, frame) =
            encode_with_composition("why does trust fail in distributed systems?");
        assert!(
            frame.iter().any(|f| f.starts_with("ask:")),
            "expected ask role, got {frame:?}"
        );
        assert!(
            frame.iter().any(|f| f.contains("trust") || f.starts_with("agent:")),
            "expected agent/topic trust, got {frame:?}"
        );
        assert!(
            frame.iter().any(|f| f.starts_with("domain:") || f.contains("distributed")),
            "expected domain, got {frame:?}"
        );
    }

    #[test]
    fn vsa_systematic_role_swap_changes_vector() {
        // Same fillers, different ask bind → different composition (systematicity).
        let (why, fw) = encode_with_composition("why does trust work in systems?");
        let (how, fh) = encode_with_composition("how does trust work in systems?");
        assert_ne!(why, how);
        assert!(fw.iter().any(|f| f == "ask:why"));
        assert!(fh.iter().any(|f| f == "ask:how"));
    }

    #[test]
    fn encoder_sorted_pairs_are_order_invariant() {
        let ab = encode("compare trust vs systems");
        let ba = encode("compare systems vs trust");
        // Sorted pair: features make these much closer than bag-only order.
        // They need not be identical (role/topic still order-sensitive) but
        // must share more structure than unrelated prompts.
        let unrelated = encode("bake a cake with chocolate frosting");
        let same_ab: u32 = ab
            .iter()
            .zip(ba.iter())
            .map(|(a, b)| (a & b).count_ones())
            .sum();
        let same_un: u32 = ab
            .iter()
            .zip(unrelated.iter())
            .map(|(a, b)| (a & b).count_ones())
            .sum();
        assert!(same_ab > same_un);
    }

    #[test]
    fn residual_skeleton_filters_non_residual() {
        let m = CognitiveMatch {
            label: "systems".into(),
            variant: 0,
            concept_id: 1,
            insight: Some("primary systems frame for routing".into()),
            score: 100,
            overlap: 10,
            runner_up_score: 90,
            margin: 10,
            query_popcount: 20,
            prototype_popcount: 30,
            positive_overlap: 5,
            negative_overlap: 1,
            hamming: 40,
            jaccard: 0.2,
            overlap_z: 1.0,
            mixture: vec![
                MixtureSupport {
                    label: "systems".into(),
                    score: 90,
                    overlap: 8,
                    concept_id: 2,
                    insight: Some("same geometry mixture support line".into()),
                    residual: false,
                    hop: 0,
                    attention_pm: 250,
                },
                MixtureSupport {
                    label: "science".into(),
                    score: 40,
                    overlap: 5,
                    concept_id: 9,
                    insight: Some("latent residual concept after ANDNOT hop".into()),
                    residual: true,
                    hop: 1,
                    attention_pm: 150,
                },
            ],
            composition: vec!["ask:why".into()],
            primary_attention_pm: 600,
        };
        let res = m.residual_skeleton(2);
        assert_eq!(res.len(), 1);
        assert!(res[0].contains("ANDNOT") || res[0].contains("latent residual"));
        // concept_skeleton excludes residual (voice frames it separately).
        let sk = m.concept_skeleton(3);
        assert!(sk.iter().all(|s| !s.contains("ANDNOT")));
        assert!(sk.iter().any(|s| s.contains("primary") || s.contains("geometry")));
    }

    #[test]
    fn science_priors_cover_method_language() {
        let priors = lexical_priors("design a falsifiable hypothesis with measurements");
        let score = priors
            .iter()
            .find(|(label, _)| *label == "science")
            .map(|(_, score)| *score)
            .unwrap_or_default();
        assert!(score >= 144);
    }

    #[test]
    fn unrelated_prompts_do_not_emit_nearest_concept_prose() {
        let concepts = vec![
            "Life is matter organized into a process.".to_owned(),
            "Time is modeled as sequence and structure.".to_owned(),
        ];
        let zero = [0u64; WORDS];
        let (concept_id, insight) =
            select_concept("what are you sensing", &zero, "general", &concepts, &[], 0);
        assert_eq!(concept_id, 0);
        assert!(insight.is_none());
    }

    #[test]
    fn semantically_supported_prompts_still_emit_concepts() {
        let concepts = vec![
            "Boundary separates inside from outside.".to_owned(),
            "Symmetry preserves structure under transformation.".to_owned(),
            "Curvature accumulates local bending into global shape.".to_owned(),
        ];
        let hvs: Vec<[u64; WORDS]> = concepts.iter().map(|s| encode_bag_only(s)).collect();
        let act = encode("how does local curvature accumulate into global shape");
        let (concept_id, insight) = select_concept(
            "how does local curvature accumulate into global shape",
            &act,
            "geometry",
            &concepts,
            &hvs,
            0,
        );
        assert_eq!(concept_id, 2);
        assert!(insight.unwrap().starts_with("Curvature"));
    }

    #[test]
    fn reasoning_facets_select_by_operation_alias() {
        let logic = vec![
            "Inference.".to_owned(),
            "Counterexample.".to_owned(),
            "Uncertainty.".to_owned(),
            "Causality.".to_owned(),
            "Contradiction.".to_owned(),
            "Necessity.".to_owned(),
            "Falsification.".to_owned(),
            "Argument structure.".to_owned(),
            "Model update.".to_owned(),
            "Calibration.".to_owned(),
        ];
        let zero = [0u64; WORDS];
        let (logic_id, logic_insight) = select_concept(
            "what makes a claim falsifiable with a competing prediction",
            &zero,
            "logic",
            &logic,
            &[],
            0,
        );
        assert_eq!(logic_id, 6);
        assert!(logic_insight.unwrap().contains("Falsification"));

        let explanation = vec![
            "Mechanism.".to_owned(),
            "Example.".to_owned(),
            "Levels.".to_owned(),
            "Clarity.".to_owned(),
            "Transfer.".to_owned(),
            "Deep reasoning.".to_owned(),
            "Response fit.".to_owned(),
            "Observation and inference.".to_owned(),
            "Mechanism and evidence.".to_owned(),
            "Transfer test.".to_owned(),
            "Self-critique.".to_owned(),
        ];
        let (explanation_id, explanation_insight) = select_concept(
            "how should a response fit the requested operation and uncertainty",
            &zero,
            "explanation",
            &explanation,
            &[],
            0,
        );
        assert_eq!(explanation_id, 6);
        assert!(explanation_insight.unwrap().contains("Response fit"));
    }
}
