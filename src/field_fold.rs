//! PERCIFLD1 field-fold experiment harness (Phase 7 prep / Phase 1 skeleton).
//!
//! Hypothesis (operational): recoverable structure depends on the fold operator
//! and a matched decoder. Not consciousness. Not quantum. Not lossless infinity.
//!
//! ```text
//! state₀ → fold → state₁ → fold → state₂ → …
//! generic decoder vs operator-matched vs mismatched
//! ```

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Fixed-width binary state for fold experiments (4096 bits = 64 × u64).
pub const FOLD_WORDS: usize = 64;
pub const FOLD_BITS: usize = FOLD_WORDS * 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoldOperator {
    XorBind,
    MajorityBundle,
    ThresholdBundle,
    RolePermute,
    ResidualFold,
}

impl FoldOperator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::XorBind => "xor_bind",
            Self::MajorityBundle => "majority_bundle",
            Self::ThresholdBundle => "threshold_bundle",
            Self::RolePermute => "role_permute",
            Self::ResidualFold => "residual_fold",
        }
    }

    pub fn all() -> &'static [FoldOperator] {
        &[
            Self::XorBind,
            Self::MajorityBundle,
            Self::ThresholdBundle,
            Self::RolePermute,
            Self::ResidualFold,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryState {
    pub words: [u64; FOLD_WORDS],
}

impl BinaryState {
    pub fn zero() -> Self {
        Self {
            words: [0u64; FOLD_WORDS],
        }
    }

    pub fn from_seed(seed: u64) -> Self {
        let mut words = [0u64; FOLD_WORDS];
        let mut s = seed | 1;
        for w in words.iter_mut() {
            // xorshift64*
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            *w = s;
        }
        Self { words }
    }

    pub fn hamming(&self, other: &Self) -> u32 {
        self.words
            .iter()
            .zip(other.words.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum()
    }

    pub fn popcount(&self) -> u32 {
        self.words.iter().map(|w| w.count_ones()).sum()
    }

    pub fn and_popcount(&self, other: &Self) -> u32 {
        self.words
            .iter()
            .zip(other.words.iter())
            .map(|(a, b)| (a & b).count_ones())
            .sum()
    }

    /// Similarity in parts-per-mille: agreement / bits.
    pub fn similarity_pm(&self, other: &Self) -> u16 {
        let agree = (FOLD_BITS as u32).saturating_sub(self.hamming(other));
        ((agree as u64 * 1000) / FOLD_BITS as u64) as u16
    }
}

/// Apply a fold operator to (state, key) → compact folded state.
pub fn fold(op: FoldOperator, state: &BinaryState, key: &BinaryState) -> BinaryState {
    match op {
        FoldOperator::XorBind => {
            let mut out = BinaryState::zero();
            for i in 0..FOLD_WORDS {
                out.words[i] = state.words[i] ^ key.words[i];
            }
            out
        }
        FoldOperator::MajorityBundle => {
            // Bundle state with key via majority of three: state, key, state&key
            let mut out = BinaryState::zero();
            for i in 0..FOLD_WORDS {
                let a = state.words[i];
                let b = key.words[i];
                let c = a & b;
                // Approximate majority per bit using (a&b)|(a&c)|(b&c) with c=a&b → a&b | …
                out.words[i] = (a & b) | (a & c) | (b & c);
            }
            out
        }
        FoldOperator::ThresholdBundle => {
            let mut out = BinaryState::zero();
            for i in 0..FOLD_WORDS {
                // Keep bits present in both or in state with key density.
                out.words[i] = (state.words[i] & key.words[i])
                    | (state.words[i] & key.words[i].rotate_left(1));
            }
            out
        }
        FoldOperator::RolePermute => {
            let mut out = BinaryState::zero();
            for i in 0..FOLD_WORDS {
                let j = (i * 7 + 3) % FOLD_WORDS;
                out.words[i] = state.words[j].rotate_left((key.words[i] % 63) as u32 + 1)
                    ^ key.words[i].rotate_right(3);
            }
            out
        }
        FoldOperator::ResidualFold => {
            // Residual: keep difference + thin shared skeleton.
            let mut out = BinaryState::zero();
            for i in 0..FOLD_WORDS {
                let shared = state.words[i] & key.words[i];
                let residual = state.words[i] ^ key.words[i];
                out.words[i] = shared | residual.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            }
            out
        }
    }
}

/// Decode / observe a folded state (matched vs generic vs mismatched).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecoderKind {
    Generic,
    Matched,
    Mismatched,
}

pub fn decode(
    kind: DecoderKind,
    fold_op: FoldOperator,
    folded: &BinaryState,
    key: &BinaryState,
) -> BinaryState {
    match kind {
        DecoderKind::Generic => {
            // Generic nearest-ish: XOR with key (assumes bind-like).
            fold(FoldOperator::XorBind, folded, key)
        }
        DecoderKind::Matched => match fold_op {
            FoldOperator::XorBind => fold(FoldOperator::XorBind, folded, key), // self-inverse
            FoldOperator::RolePermute => {
                // Approximate inverse: reverse rotation index.
                let mut out = BinaryState::zero();
                for i in 0..FOLD_WORDS {
                    let j = (i * 7 + 3) % FOLD_WORDS;
                    // Not perfect inverse; measures partial recovery.
                    out.words[j] = (folded.words[i] ^ key.words[i].rotate_right(3))
                        .rotate_right((key.words[i] % 63) as u32 + 1);
                }
                out
            }
            _ => {
                // Soft unbundle: re-intersect with key.
                let mut out = BinaryState::zero();
                for i in 0..FOLD_WORDS {
                    out.words[i] = folded.words[i] | (folded.words[i] & key.words[i]);
                }
                out
            }
        },
        DecoderKind::Mismatched => {
            // Wrong operator decode.
            let wrong = match fold_op {
                FoldOperator::XorBind => FoldOperator::MajorityBundle,
                _ => FoldOperator::XorBind,
            };
            fold(wrong, folded, key)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldTrialResult {
    pub operator: String,
    pub depth: u32,
    pub compression_ratio_pm: u16,
    pub reconstruction_similarity_pm: u16,
    pub matched_decode_pm: u16,
    pub mismatched_decode_pm: u16,
    pub generic_decode_pm: u16,
    pub collision_rate_pm: u16,
    pub latency_us: u64,
    pub bits: u32,
}

/// Run repeated-fold degradation for one operator.
pub fn run_fold_ladder(op: FoldOperator, seed: u64, max_depth: u32) -> Vec<FoldTrialResult> {
    let original = BinaryState::from_seed(seed);
    let key = BinaryState::from_seed(seed ^ 0xA5A5_5A5A_C3C3_3C3C);
    let mut state = original.clone();
    let mut out = Vec::new();
    for depth in 1..=max_depth {
        let t0 = Instant::now();
        state = fold(op, &state, &key);
        let matched = decode(DecoderKind::Matched, op, &state, &key);
        let mismatched = decode(DecoderKind::Mismatched, op, &state, &key);
        let generic = decode(DecoderKind::Generic, op, &state, &key);
        let latency = t0.elapsed().as_micros() as u64;

        // Collision proxy: density collapse toward 0 or full.
        let dens = state.popcount();
        let mid = FOLD_BITS as u32 / 2;
        let collision = dens.abs_diff(mid) * 1000 / mid;

        // Compression ratio proxy: shared bits with key / total (not storage shrink).
        let shared = state.and_popcount(&key);
        let ratio = (shared as u64 * 1000 / FOLD_BITS as u64) as u16;

        out.push(FoldTrialResult {
            operator: op.as_str().into(),
            depth,
            compression_ratio_pm: ratio,
            reconstruction_similarity_pm: state.similarity_pm(&original),
            matched_decode_pm: matched.similarity_pm(&original),
            mismatched_decode_pm: mismatched.similarity_pm(&original),
            generic_decode_pm: generic.similarity_pm(&original),
            collision_rate_pm: collision.min(1000) as u16,
            latency_us: latency,
            bits: FOLD_BITS as u32,
        });
    }
    out
}

/// Full multi-operator experiment receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldExperimentReport {
    pub schema: String,
    pub claim_boundary: Vec<String>,
    pub trials: Vec<FoldTrialResult>,
    pub findings: Vec<String>,
}

pub fn run_experiment(seed: u64, max_depth: u32) -> FoldExperimentReport {
    let mut trials = Vec::new();
    for op in FoldOperator::all() {
        trials.extend(run_fold_ladder(*op, seed, max_depth));
    }

    let mut findings = Vec::new();
    // Compare matched vs mismatched at depth 1 for xor (self-inverse should recover well).
    if let Some(xor1) = trials
        .iter()
        .find(|t| t.operator == "xor_bind" && t.depth == 1)
    {
        if xor1.matched_decode_pm > xor1.mismatched_decode_pm {
            findings.push(
                "xor_bind: operator-matched decode recovers more than mismatched (supports operator-conditioned retrieval)."
                    .into(),
            );
        } else {
            findings.push(
                "xor_bind: matched decode did not beat mismatched at depth 1 (investigate seed/metrics)."
                    .into(),
            );
        }
        if xor1.matched_decode_pm >= 900 {
            findings.push(
                "xor_bind: near-lossless unbind at depth 1 (self-inverse binding)."
                    .into(),
            );
        }
    }

    // Degradation: compare depth 1 vs max for residual.
    let d1 = trials
        .iter()
        .find(|t| t.operator == "residual_fold" && t.depth == 1)
        .map(|t| t.reconstruction_similarity_pm);
    let dn = trials
        .iter()
        .find(|t| t.operator == "residual_fold" && t.depth == max_depth)
        .map(|t| t.reconstruction_similarity_pm);
    if let (Some(a), Some(b)) = (d1, dn) {
        if b + 50 < a {
            findings.push(format!(
                "residual_fold: reconstruction similarity falls with depth ({a}→{b} pm) — fold is lossy under repetition."
            ));
        } else {
            findings.push(format!(
                "residual_fold: limited measured drop depth1→{max_depth} ({a}→{b} pm)."
            ));
        }
    }

    findings.push(
        "Folding is experimental telemetry, not a claim of infinite or lossless compression."
            .into(),
    );
    findings.push(
        "Observer = decode operator only; not consciousness or quantum observation.".into(),
    );

    FoldExperimentReport {
        schema: "perci.field-fold-experiment.v1".into(),
        claim_boundary: vec![
            "not consciousness".into(),
            "not quantum cognition".into(),
            "not infinite lossless fold".into(),
            "not auto-promote".into(),
        ],
        trials,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xor_bind_is_self_inverse() {
        let s = BinaryState::from_seed(42);
        let k = BinaryState::from_seed(99);
        let f = fold(FoldOperator::XorBind, &s, &k);
        let back = fold(FoldOperator::XorBind, &f, &k);
        assert_eq!(back.hamming(&s), 0);
    }

    #[test]
    fn matched_beats_mismatched_on_xor() {
        let report = run_experiment(7, 3);
        let t = report
            .trials
            .iter()
            .find(|t| t.operator == "xor_bind" && t.depth == 1)
            .expect("xor depth1");
        assert!(
            t.matched_decode_pm >= t.mismatched_decode_pm,
            "matched={} mismatched={}",
            t.matched_decode_pm,
            t.mismatched_decode_pm
        );
        assert!(!report.findings.is_empty());
    }

    #[test]
    fn ladder_emits_depths() {
        let ladder = run_fold_ladder(FoldOperator::RolePermute, 1, 3);
        assert_eq!(ladder.len(), 3);
        assert_eq!(ladder[0].depth, 1);
        assert_eq!(ladder[2].depth, 3);
    }
}
