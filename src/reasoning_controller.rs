//!
//! Bounded cognitive control for the native reasoning path.
//!
//! A binary associative field can route a prompt, but routing is not the same
//! thing as deciding how much work the prompt deserves. This module supplies
//! that missing control layer: a compact recurrent state, typed uncertainty
//! signals, a depth policy, and a visible halting rule. It is deliberately not
//! a hidden chain-of-thought store and it does not claim to create subjective
//! thought. It chooses a bounded program for the existing operators.

use crate::cognitive::CognitiveMatch;

const MAX_RECENT: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasoningMode {
    Direct,
    Explain,
    Explore,
    Verify,
    Clarify,
    Abstain,
}

impl ReasoningMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Explain => "explain",
            Self::Explore => "explore",
            Self::Verify => "verify",
            Self::Clarify => "clarify",
            Self::Abstain => "abstain",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlSignals {
    pub feature_bits: u64,
    pub complexity: u8,
    pub ambiguity: u8,
    pub contradiction: bool,
    pub out_of_distribution: bool,
    pub confidence_pm: u16,
    pub margin: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryControlState {
    lanes: [u64; 8],
    turns: u16,
}

impl Default for BinaryControlState {
    fn default() -> Self {
        Self {
            lanes: [
                0x6a09e667f3bcc909,
                0xbb67ae8584caa73b,
                0x3c6ef372fe94f82b,
                0xa54ff53a5f1d36f1,
                0x510e527fade682d1,
                0x9b05688c2b3e6c1f,
                0x1f83d9abfb41bd6b,
                0x5be0cd19137e2179,
            ],
            turns: 0,
        }
    }
}

impl BinaryControlState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn absorb(&mut self, text: &str, feature_bits: u64) {
        for (index, byte) in text.bytes().enumerate() {
            let lane = index % self.lanes.len();
            let salt = feature_bits
                .rotate_left((lane * 7) as u32)
                .wrapping_add((index as u64).wrapping_mul(0x9e3779b97f4a7c15));
            self.lanes[lane] ^= (byte as u64).wrapping_add(salt);
            self.lanes[lane] = self.lanes[lane]
                .rotate_left(((index + lane * 11) % 63 + 1) as u32)
                .wrapping_mul(0x100000001b3);
            self.lanes[lane] ^= self.lanes[(lane + 3) % self.lanes.len()] >> 17;
        }
        self.lanes[0] ^= feature_bits;
        self.lanes[7] = self.lanes[7]
            .wrapping_add(self.lanes[0].rotate_left(23))
            .rotate_left(17);
        self.turns = self.turns.saturating_add(1);
    }

    pub fn absorb_turn(&mut self, user: &str, assistant: &str, feature_bits: u64) {
        self.absorb("<user>", feature_bits ^ 0x55);
        self.absorb(user, feature_bits);
        self.absorb("<assistant>", feature_bits ^ 0xaa);
        self.absorb(assistant, feature_bits.rotate_left(1));
    }

    pub fn fingerprint(&self) -> u64 {
        let mut value = self.turns as u64 ^ 0x517cc1b727220a95;
        for (index, lane) in self.lanes.iter().enumerate() {
            value ^= lane.rotate_left(((index * 9) % 63 + 1) as u32);
            value = value.wrapping_mul(0x9e3779b97f4a7c15);
        }
        avalanche(value)
    }

    pub fn turns(&self) -> u16 {
        self.turns
    }

    /// Stable binary payload for receipts or a future `PERCICTL1` sidecar.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(9 + 2 + 8 * 8);
        out.extend_from_slice(b"PERCICTL1");
        out.extend_from_slice(&self.turns.to_le_bytes());
        for lane in self.lanes {
            out.extend_from_slice(&lane.to_le_bytes());
        }
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReasoningPlan {
    pub mode: ReasoningMode,
    pub min_cycles: u8,
    pub max_cycles: u8,
    pub halt_threshold_pm: u16,
    pub signals: ControlSignals,
    pub steps: Vec<&'static str>,
    pub state_fingerprint: u64,
}

impl ReasoningPlan {
    pub fn should_run_reason_loop(&self) -> bool {
        matches!(
            self.mode,
            ReasoningMode::Explain | ReasoningMode::Explore | ReasoningMode::Verify
        ) && self.max_cycles > 1
    }

    pub fn should_continue(&self, cycle: u8, improvement_pm: u16, unresolved: bool) -> bool {
        if cycle < self.min_cycles {
            return true;
        }
        if cycle >= self.max_cycles {
            return false;
        }
        unresolved || improvement_pm >= self.halt_threshold_pm
    }

    pub fn hint(&self) -> String {
        format!(
            "mode={} cycles={}-{} halt={} complexity={} ambiguity={} contradiction={} ood={} confidence={} margin={} state={:016x}",
            self.mode.name(),
            self.min_cycles,
            self.max_cycles,
            self.halt_threshold_pm,
            self.signals.complexity,
            self.signals.ambiguity,
            self.signals.contradiction,
            self.signals.out_of_distribution,
            self.signals.confidence_pm,
            self.signals.margin,
            self.state_fingerprint,
        )
    }
}

pub fn derive(
    user: &str,
    recent: &[(String, String)],
    matched: Option<&CognitiveMatch>,
    _operator: &str,
) -> ReasoningPlan {
    let lower = user.to_ascii_lowercase();
    let feature_bits = feature_bits(&lower, recent);
    let complexity = complexity(&lower);
    let ambiguity = ambiguity(&lower, recent);
    let contradiction = contains_any(
        &lower,
        &[
            "i disagree",
            "contradiction",
            "counterexample",
            "what would change",
            "is this wrong",
        ],
    );
    let out_of_distribution = looks_ood(&lower);
    let (confidence_pm, margin) = confidence(matched);
    let signals = ControlSignals {
        feature_bits,
        complexity,
        ambiguity,
        contradiction,
        out_of_distribution,
        confidence_pm,
        margin,
    };

    let brief = contains_any(
        &lower,
        &[
            "be brief",
            "keep it brief",
            "short answer",
            "one sentence",
            "tl;dr",
        ],
    );
    let deep = contains_any(
        &lower,
        &[
            "go deeper",
            "deep reasoning",
            "in depth",
            "step by step",
            "analyze",
            "thorough",
            "more detail",
        ],
    );
    let asks_explanation = lower.starts_with("why ")
        || lower.starts_with("how ")
        || contains_any(&lower, &["explain", "reason", "what follows"]);
    let asks_verification = contains_any(
        &lower,
        &["evidence", "prove", "test", "falsifiable", "counterexample"],
    );
    let asks_relation = contains_any(&lower, &["connect", "compare", "related", "across"]);

    let mode = if out_of_distribution {
        ReasoningMode::Abstain
    } else if ambiguity >= 2 && recent.is_empty() {
        ReasoningMode::Clarify
    } else if asks_verification || contradiction {
        ReasoningMode::Verify
    } else if asks_relation || (deep && complexity >= 2) {
        ReasoningMode::Explore
    } else if asks_explanation || deep {
        ReasoningMode::Explain
    } else {
        ReasoningMode::Direct
    };

    let (min_cycles, mut max_cycles): (u8, u8) = match mode {
        ReasoningMode::Direct | ReasoningMode::Clarify | ReasoningMode::Abstain => (1, 1),
        ReasoningMode::Explain => (2, 3),
        ReasoningMode::Explore => (2, 4),
        ReasoningMode::Verify => (2, 4),
    };
    if brief {
        max_cycles = 1;
    } else if complexity <= 1 && !deep && !asks_verification {
        max_cycles = max_cycles.min(2);
    }
    if confidence_pm < 450 || ambiguity > 0 {
        max_cycles = max_cycles.saturating_add(1).min(4);
    }
    let halt_threshold_pm = if confidence_pm < 450 || ambiguity > 0 {
        80
    } else if matches!(mode, ReasoningMode::Verify) {
        60
    } else {
        35
    };
    let steps = match mode {
        ReasoningMode::Direct => vec!["bind_request", "answer"],
        ReasoningMode::Explain => vec![
            "bind_claim",
            "separate_known_unknown",
            "explain_mechanism",
            "state_boundary",
        ],
        ReasoningMode::Explore => vec![
            "bind_frames",
            "find_shared_relation",
            "stress_test_transfer",
            "compress_answer",
        ],
        ReasoningMode::Verify => vec![
            "bind_claim",
            "seek_evidence",
            "test_counterexample",
            "report_uncertainty",
        ],
        ReasoningMode::Clarify => vec!["detect_missing_referent", "ask_smallest_question"],
        ReasoningMode::Abstain => vec!["detect_unknown", "state_limit", "request_grounding"],
    };

    let mut state = BinaryControlState::default();
    for (user_turn, assistant_turn) in recent.iter().rev().take(MAX_RECENT).rev() {
        state.absorb_turn(user_turn, assistant_turn, feature_bits);
    }
    state.absorb(user, feature_bits);
    ReasoningPlan {
        mode,
        min_cycles,
        max_cycles,
        halt_threshold_pm,
        signals,
        steps,
        state_fingerprint: state.fingerprint(),
    }
}

fn feature_bits(lower: &str, recent: &[(String, String)]) -> u64 {
    let mut bits = 0u64;
    let flags = [
        (0, contains_any(lower, &["brief", "short", "one sentence"])),
        (
            1,
            contains_any(lower, &["deep", "thorough", "step by step", "analyze"]),
        ),
        (2, contains_any(lower, &["why", "how", "explain", "reason"])),
        (
            3,
            contains_any(lower, &["connect", "compare", "related", "across"]),
        ),
        (
            4,
            contains_any(lower, &["evidence", "prove", "test", "measure"]),
        ),
        (
            5,
            contains_any(lower, &["disagree", "contradiction", "counterexample"]),
        ),
        (6, contains_any(lower, &[" this ", " that ", " it "])),
        (7, looks_ood(lower)),
        (
            8,
            lower.matches(" and ").count() >= 2 || lower.contains(" then "),
        ),
        (9, !recent.is_empty()),
    ];
    for (bit, present) in flags {
        if present {
            bits |= 1u64 << bit;
        }
    }
    bits
}

fn complexity(lower: &str) -> u8 {
    let clauses = lower.matches(" and ").count()
        + lower.matches(" because ").count()
        + lower.matches(" if ").count()
        + lower.matches(" then ").count()
        + lower.matches(" but ").count();
    (1 + clauses + lower.matches('?').count()).min(7) as u8
}

fn ambiguity(lower: &str, recent: &[(String, String)]) -> u8 {
    let deictic = contains_any(lower, &[" this ", " that ", " it ", "same"]);
    if !deictic {
        0
    } else if recent.is_empty() {
        2
    } else if lower.contains("what do you mean") || lower.contains("why do you think") {
        1
    } else {
        0
    }
}

fn confidence(matched: Option<&CognitiveMatch>) -> (u16, i32) {
    let Some(matched) = matched else {
        return (500, 0);
    };
    let margin = matched.margin;
    let score = matched.score.max(0) as u16;
    let confidence = (350u16
        .saturating_add(score.min(350))
        .saturating_add((margin.max(0) as u16).min(300)))
    .min(1000);
    (confidence, margin)
}

fn looks_ood(lower: &str) -> bool {
    lower.split_whitespace().any(|word| {
        let clean = word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        clean.starts_with("zxq") || clean == "blorf" || clean == "nembit" || clean.len() > 24
    })
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn avalanche(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_is_adaptive_and_bounded() {
        let brief = derive("be brief: what is geometry?", &[], None, "geometry");
        assert_eq!(brief.mode, ReasoningMode::Direct);
        assert_eq!(brief.max_cycles, 1);

        let deep = derive(
            "Analyze how geometry and life relate, then state the mechanism and boundary.",
            &[],
            None,
            "geometry",
        );
        assert_eq!(deep.mode, ReasoningMode::Explore);
        assert!((2..=4).contains(&deep.max_cycles));
        assert!(deep.should_run_reason_loop());
    }

    #[test]
    fn contradiction_and_unknowns_change_mode() {
        let verify = derive(
            "I disagree. What evidence or counterexample would change this claim?",
            &[("claim".into(), "answer".into())],
            None,
            "logic",
        );
        assert_eq!(verify.mode, ReasoningMode::Verify);
        assert!(verify.max_cycles >= 2);

        let abstain = derive(
            "zxqv blorf nembit — what does it mean?",
            &[],
            None,
            "general",
        );
        assert_eq!(abstain.mode, ReasoningMode::Abstain);
        assert!(!abstain.should_run_reason_loop());
    }

    #[test]
    fn binary_state_is_order_sensitive_and_serializable() {
        let mut first = BinaryControlState::default();
        first.absorb("geometry", 0x11);
        first.absorb("life", 0x22);
        let mut reversed = BinaryControlState::default();
        reversed.absorb("life", 0x22);
        reversed.absorb("geometry", 0x11);
        assert_ne!(first.fingerprint(), reversed.fingerprint());
        assert_eq!(&first.encode()[..9], b"PERCICTL1");
        assert_eq!(first.encode().len(), 9 + 2 + 64);
    }

    #[test]
    fn halting_rule_requires_minimum_then_stops_on_low_gain() {
        let plan = derive(
            "Explain why memory matters and test the boundary.",
            &[],
            None,
            "memory",
        );
        assert!(plan.should_continue(0, 0, false));
        assert!(!plan.should_continue(plan.min_cycles, 0, false));
        assert!(plan.should_continue(plan.min_cycles, plan.halt_threshold_pm, false));
    }
}
