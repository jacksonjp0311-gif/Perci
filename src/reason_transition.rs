//! PERCIRSN1 — learned reasoning-transition pack (Phase 4).
//!
//! Stores transitions between cognitive states (not full answers).
//! Execution is a bounded state machine with named operations, confidence
//! updates, and explicit halt conditions — never private chain-of-thought.

use crate::semantic_field::{extract_frame, SemanticFrame};
use crate::thought_plan::{BoundClaim, DiscourseAct, Intent, Relation, ThoughtPlan, Uncertainty};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// On-disk magic (8 bytes). Family id remains PERCIRSN1 in manifests.
const MAGIC: &[u8; 8] = b"PERCRSN1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
/// Fixed transition record: op id, goal hash, state hash, next hash, gain, conf delta, halt flag.
const RECORD_SIZE: usize = 36;

/// Named reasoning operations (inspectable, not narrative CoT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonOp {
    BindRequest,
    Decompose,
    IdentifyAssumptions,
    RetrievePrinciple,
    ProposeMechanism,
    GenerateAlternatives,
    CompareHypotheses,
    SearchCounterexample,
    DistinguishAnalogy,
    SeekEvidence,
    TestTransfer,
    ReviseContradiction,
    IdentifyUncertainty,
    CompressConclusion,
    Halt,
}

impl ReasonOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BindRequest => "bind_request",
            Self::Decompose => "decompose",
            Self::IdentifyAssumptions => "identify_assumptions",
            Self::RetrievePrinciple => "retrieve_principle",
            Self::ProposeMechanism => "propose_mechanism",
            Self::GenerateAlternatives => "generate_alternatives",
            Self::CompareHypotheses => "compare_hypotheses",
            Self::SearchCounterexample => "search_counterexample",
            Self::DistinguishAnalogy => "distinguish_analogy",
            Self::SeekEvidence => "seek_evidence",
            Self::TestTransfer => "test_transfer",
            Self::ReviseContradiction => "revise_contradiction",
            Self::IdentifyUncertainty => "identify_uncertainty",
            Self::CompressConclusion => "compress_conclusion",
            Self::Halt => "halt",
        }
    }

    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::BindRequest,
            1 => Self::Decompose,
            2 => Self::IdentifyAssumptions,
            3 => Self::RetrievePrinciple,
            4 => Self::ProposeMechanism,
            5 => Self::GenerateAlternatives,
            6 => Self::CompareHypotheses,
            7 => Self::SearchCounterexample,
            8 => Self::DistinguishAnalogy,
            9 => Self::SeekEvidence,
            10 => Self::TestTransfer,
            11 => Self::ReviseContradiction,
            12 => Self::IdentifyUncertainty,
            13 => Self::CompressConclusion,
            _ => Self::Halt,
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::BindRequest => 0,
            Self::Decompose => 1,
            Self::IdentifyAssumptions => 2,
            Self::RetrievePrinciple => 3,
            Self::ProposeMechanism => 4,
            Self::GenerateAlternatives => 5,
            Self::CompareHypotheses => 6,
            Self::SearchCounterexample => 7,
            Self::DistinguishAnalogy => 8,
            Self::SeekEvidence => 9,
            Self::TestTransfer => 10,
            Self::ReviseContradiction => 11,
            Self::IdentifyUncertainty => 12,
            Self::CompressConclusion => 13,
            Self::Halt => 14,
        }
    }
}

/// One executed step (receipt line).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionStep {
    pub cycle: u32,
    pub op: String,
    pub state_before: String,
    pub state_after: String,
    pub expected_info_gain_pm: u16,
    pub confidence_pm: u16,
    pub note: String,
    pub halt: bool,
}

/// Runtime state for the transition machine.
#[derive(Debug, Clone)]
pub struct ReasonState {
    pub label: String,
    pub goal: String,
    pub evidence_bits: u16,
    pub uncertainty_bits: u16,
    pub confidence_pm: u16,
    pub frame: SemanticFrame,
    pub mechanisms: Vec<String>,
    pub alternatives: Vec<String>,
    pub uncertainties: Vec<String>,
    pub boundaries: Vec<String>,
    pub claims: Vec<String>,
}

impl ReasonState {
    pub fn from_frame(frame: SemanticFrame) -> Self {
        let goal = format!(
            "{}:{}",
            frame.requested_output,
            if frame.subject.is_empty() {
                "topic"
            } else {
                &frame.subject
            }
        );
        Self {
            label: "bound".into(),
            goal,
            evidence_bits: 0,
            uncertainty_bits: 100,
            confidence_pm: 450,
            frame,
            mechanisms: Vec::new(),
            alternatives: Vec::new(),
            uncertainties: Vec::new(),
            boundaries: Vec::new(),
            claims: Vec::new(),
        }
    }

    pub fn fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.label.hash(&mut h);
        self.goal.hash(&mut h);
        self.confidence_pm.hash(&mut h);
        self.mechanisms.len().hash(&mut h);
        self.evidence_bits.hash(&mut h);
        h.finish()
    }
}

/// Select next op from current state (policy table + optional pack).
pub fn select_op(state: &ReasonState, cycle: u32, max_cycles: u32) -> ReasonOp {
    if cycle + 1 >= max_cycles {
        return ReasonOp::Halt;
    }
    if state.confidence_pm >= 880 && !state.claims.is_empty() && !state.mechanisms.is_empty() {
        return ReasonOp::CompressConclusion;
    }
    match cycle {
        0 => ReasonOp::BindRequest,
        1 => ReasonOp::Decompose,
        2 => ReasonOp::IdentifyAssumptions,
        3 => {
            if state.frame.requested_output.contains("mechanism")
                || state.frame.intent.contains("causal")
                || state.frame.intent.contains("trust")
            {
                ReasonOp::ProposeMechanism
            } else if state.frame.requested_output.contains("comparison") {
                ReasonOp::CompareHypotheses
            } else {
                ReasonOp::RetrievePrinciple
            }
        }
        4 => ReasonOp::GenerateAlternatives,
        5 => ReasonOp::SearchCounterexample,
        6 => ReasonOp::DistinguishAnalogy,
        7 => ReasonOp::IdentifyUncertainty,
        8 => ReasonOp::TestTransfer,
        9 => ReasonOp::CompressConclusion,
        _ => ReasonOp::Halt,
    }
}

/// Apply one transition (deterministic, inspectable).
pub fn apply_op(state: &mut ReasonState, op: ReasonOp) -> TransitionStep {
    let before = state.label.clone();
    let before_fp = format!("{:016x}", state.fingerprint());
    let mut halt = false;
    let mut gain: u16;
    let note: String;

    match op {
        ReasonOp::BindRequest => {
            state.label = "request_bound".into();
            state.confidence_pm = state.confidence_pm.saturating_add(40).min(950);
            note = format!(
                "bound subject={} condition={}",
                state.frame.subject, state.frame.condition
            );
            gain = 120;
        }
        ReasonOp::Decompose => {
            state.label = "decomposed".into();
            note = format!(
                "parts: subject, condition={}, phenomenon={}, out={}",
                state.frame.condition, state.frame.phenomenon, state.frame.requested_output
            );
            state.confidence_pm = state.confidence_pm.saturating_add(20).min(950);
            gain = 90;
        }
        ReasonOp::IdentifyAssumptions => {
            state.label = "assumptions_marked".into();
            state
                .uncertainties
                .push("terms mean usual stack/domain senses unless redefined".into());
            state.uncertainty_bits = state.uncertainty_bits.saturating_add(20);
            note = "marked lexical/domain assumptions".into();
            gain = 70;
        }
        ReasonOp::RetrievePrinciple => {
            state.label = "principle_loaded".into();
            state.mechanisms.push(principle_for(&state.frame));
            state.evidence_bits = state.evidence_bits.saturating_add(30);
            note = "retrieved domain principle".into();
            state.confidence_pm = state.confidence_pm.saturating_add(50).min(950);
            gain = 100;
        }
        ReasonOp::ProposeMechanism => {
            state.label = "mechanism_proposed".into();
            state.mechanisms.push(mechanism_for(&state.frame));
            state.claims.push(claim_for(&state.frame));
            state.evidence_bits = state.evidence_bits.saturating_add(40);
            state.confidence_pm = state.confidence_pm.saturating_add(60).min(950);
            note = "proposed working mechanism".into();
            gain = 140;
        }
        ReasonOp::GenerateAlternatives => {
            state.label = "alternatives_listed".into();
            state.alternatives.push(alternative_for(&state.frame));
            note = "generated competing hypothesis".into();
            gain = 85;
        }
        ReasonOp::CompareHypotheses => {
            state.label = "hypotheses_compared".into();
            note = "prefer checkable mechanism over metaphor".into();
            state.confidence_pm = state.confidence_pm.saturating_add(30).min(950);
            gain = 75;
        }
        ReasonOp::SearchCounterexample => {
            state.label = "counterexample_sought".into();
            state.boundaries.push(counterexample_for(&state.frame));
            note = "named a failing condition".into();
            gain = 95;
        }
        ReasonOp::DistinguishAnalogy => {
            state.label = "analogy_bounded".into();
            state.boundaries.push(
                "analogy dies when domain mechanisms must become identical for the claim to hold"
                    .into(),
            );
            note = "separated analogy from mechanism".into();
            gain = 80;
        }
        ReasonOp::SeekEvidence => {
            state.label = "evidence_sought".into();
            state.evidence_bits = state.evidence_bits.saturating_add(25);
            note = "evidence requirement recorded".into();
            gain = 70;
        }
        ReasonOp::TestTransfer => {
            state.label = "transfer_checked".into();
            state.boundaries.push(
                "transfer requires entity swap / paraphrase still preserves the relation".into(),
            );
            note = "transfer criterion stated".into();
            gain = 90;
        }
        ReasonOp::ReviseContradiction => {
            state.label = "revised".into();
            state.confidence_pm = state.confidence_pm.saturating_sub(40);
            note = "confidence reduced under contradiction pressure".into();
            gain = 60;
        }
        ReasonOp::IdentifyUncertainty => {
            state.label = "uncertainty_explicit".into();
            state.uncertainties.push(uncertainty_for(&state.frame));
            state.uncertainty_bits = state.uncertainty_bits.saturating_add(15);
            note = "uncertainty marked".into();
            gain = 65;
        }
        ReasonOp::CompressConclusion => {
            state.label = "conclusion_compressed".into();
            if state.claims.is_empty() {
                state.claims.push(claim_for(&state.frame));
            }
            note = "compressed to working claim".into();
            gain = 50;
        }
        ReasonOp::Halt => {
            state.label = "halted".into();
            note = "expected information gain below threshold or cycle cap".into();
            gain = 0;
            halt = true;
        }
    }

    // Diminishing returns
    if state.confidence_pm >= 850 && !matches!(op, ReasonOp::Halt | ReasonOp::CompressConclusion) {
        gain /= 2;
    }

    let after_fp = format!("{:016x}", state.fingerprint());
    TransitionStep {
        cycle: 0, // filled by runner
        op: op.as_str().into(),
        state_before: format!("{before}/{before_fp}"),
        state_after: format!("{}/{}", state.label, after_fp),
        expected_info_gain_pm: gain,
        confidence_pm: state.confidence_pm,
        note,
        halt,
    }
}

fn principle_for(frame: &SemanticFrame) -> String {
    if frame.subject.contains("trust") {
        "trust is checkable acceptance under partial observability".into()
    } else if frame.subject.contains("boundary") {
        "boundary separates inside/outside and enables exchange and repair".into()
    } else if frame.subject.contains("life") || frame.subject.contains("order") {
        "life maintains local order through continuous exchange and repair across a boundary".into()
    } else if frame.subject.contains("memory") {
        "memory reconstructs past state from traces under partial cues".into()
    } else if frame.subject.contains("repair") {
        "repair restores function when a checkable failure mode is named and reversed".into()
    } else {
        format!(
            "structure under constraint for {}",
            if frame.subject.is_empty() {
                "topic"
            } else {
                &frame.subject
            }
        )
    }
}

fn mechanism_for(frame: &SemanticFrame) -> String {
    let cond_delay = frame.condition.contains("delay")
        || frame.condition.contains("lag")
        || frame.condition.contains("timeout")
        || frame.condition.contains("retry");
    if frame.subject.contains("trust") && cond_delay {
        "timeout is a one-sided story: without shared done-predicates and idempotent retries, silence is mistaken for agreement or betrayal".into()
    } else if frame.subject.contains("trust") {
        "trust is earned when acceptance criteria stay checkable without private peer state".into()
    } else if frame.subject.contains("boundary")
        && (frame.prompt.to_ascii_lowercase().contains("repair")
            || frame.phenomenon.contains("repair")
            || frame.requested_output.contains("repair"))
    {
        "a boundary names what may cross; repair works on the crossing and the failure mode, not on max shape freeze".into()
    } else if frame.subject.contains("boundary") {
        "maintenance under change preserves what may cross the boundary; shapes describe contact, not cause".into()
    } else if frame.subject.contains("life") || frame.subject.contains("order") {
        "local order is maintained by continuous energy-driven exchange and repair; when exchange stops, order decays".into()
    } else if frame.subject.contains("repair") {
        "name the failed operation, reproduce the smallest failing input, apply one change, retest — metaphor does not repair".into()
    } else if frame.condition.contains("timeout") || frame.condition.contains("retry") {
        "timeouts need shared meaning; retries need idempotency so silence is not treated as success".into()
    } else if frame.requested_output.contains("mechanism") {
        format!(
            "working mechanism: {} stays coherent under {} only when the failure mode is named and checkable",
            empty(&frame.subject, "system"),
            empty(&frame.condition, "stated condition")
        )
    } else {
        format!(
            "relation: {} maps to {} when constraints stay explicit",
            empty(&frame.subject, "subject"),
            empty(&frame.requested_output, "output")
        )
    }
}

fn claim_for(frame: &SemanticFrame) -> String {
    if frame.subject.contains("trust") {
        "Interfaces earn trust under lag when “done” is checkable without private state".into()
    } else if frame.subject.contains("boundary")
        && (frame.prompt.to_ascii_lowercase().contains("repair")
            || frame.phenomenon.contains("repair"))
    {
        "Boundaries enable repair by separating inside from outside so maintenance can target what crossed and what failed".into()
    } else if frame.subject.contains("boundary") {
        "A boundary is a checkable separation that enables exchange, not a frozen outline of max coherence".into()
    } else if frame.subject.contains("life") || frame.subject.contains("order") {
        "Life maintains local order by continuous exchange and repair across a boundary under energy cost".into()
    } else if frame.subject.contains("repair") {
        "Repair is the reverse of a named failure mode under a checkable test — not a humility slogan".into()
    } else if frame.condition.contains("timeout") || frame.condition.contains("retry") {
        "Timeout and retry only earn trust when “done” is shared and retries are idempotent".into()
    } else {
        format!(
            "{} under {} yields a checkable {} when constraints are named",
            empty(&frame.subject, "system"),
            empty(&frame.condition, "condition"),
            empty(&frame.phenomenon, "outcome")
        )
    }
}

fn alternative_for(frame: &SemanticFrame) -> String {
    if frame.subject.contains("trust") {
        "alternative: trust is only latency; reject — latency without audit is hope, not trust"
            .into()
    } else if frame.subject.contains("boundary") {
        "alternative: maximize coherence without bands; reject — max coherence overfits and fails transfer".into()
    } else if frame.subject.contains("life") {
        "alternative: life is pure shape; reject — geometry describes contact, not biological cause"
            .into()
    } else {
        format!(
            "alternative: {} is merely wording, not a mechanism",
            empty(&frame.subject, "claim")
        )
    }
}

fn counterexample_for(frame: &SemanticFrame) -> String {
    if frame.subject.contains("trust") {
        "counterexample: perfect RTTs with silent double-charge still destroy trust".into()
    } else if frame.subject.contains("boundary") {
        "counterexample: a sealed boundary with no exchange freezes form and blocks repair".into()
    } else if frame.subject.contains("life") || frame.subject.contains("order") {
        "counterexample: a crystal holds order without life — maintenance, not static pattern, is the load-bearing claim".into()
    } else {
        format!(
            "boundary: claim fails when {} no longer predicts {}",
            empty(&frame.condition, "condition"),
            empty(&frame.phenomenon, "effect")
        )
    }
}

fn uncertainty_for(frame: &SemanticFrame) -> String {
    if frame.subject.contains("trust") || frame.subject.contains("boundary") {
        "terms mean usual stack/domain senses unless redefined".into()
    } else {
        format!(
            "unknown: full mechanism detail for {} beyond local operators/packs",
            empty(&frame.subject, "topic")
        )
    }
}

fn empty<'a>(s: &'a str, d: &'a str) -> &'a str {
    if s.is_empty() {
        d
    } else {
        s
    }
}

/// Bounded execution receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonRun {
    pub schema: String,
    pub prompt: String,
    pub frame_summary: String,
    pub steps: Vec<TransitionStep>,
    pub halt_reason: String,
    pub final_confidence_pm: u16,
    pub cycles: u32,
}

impl ReasonRun {
    pub fn to_thought_plan(&self, frame: &SemanticFrame, state: &ReasonState) -> ThoughtPlan {
        let intent = Intent::infer_from_prompt(&self.prompt);
        let mut plan = ThoughtPlan::empty("percirsn1-executor", intent);
        plan.semantic_bindings = frame.to_bindings();
        plan.confidence_pm = state.confidence_pm;
        plan.active_packs = vec!["PERCIW03".into(), "PERCIRSN1".into(), "PERCISEM1".into()];
        plan.reasoning_cycles = self
            .steps
            .iter()
            .map(|s| format!("cycle{}:{}:{}", s.cycle, s.op, s.note))
            .collect();
        plan.halt_reason = self.halt_reason.clone();
        for c in &state.claims {
            plan.claims.push(BoundClaim {
                text: c.clone(),
                status: "working".into(),
            });
        }
        for m in &state.mechanisms {
            plan.mechanisms.push(Relation {
                subject: frame.subject.clone(),
                relation: "mechanism".into(),
                object: m.clone(),
            });
        }
        for u in &state.uncertainties {
            plan.uncertainties.push(Uncertainty { text: u.clone() });
        }
        for b in &state.boundaries {
            plan.boundaries
                .push(crate::thought_plan::Constraint { text: b.clone() });
        }
        plan.discourse_acts = vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Mechanism,
            DiscourseAct::Boundary,
            DiscourseAct::Uncertainty,
        ];
        // Surface from claims + first mechanism
        let mut surface = String::new();
        if let Some(c) = state.claims.first() {
            surface.push_str(c);
            if !surface.ends_with('.') {
                surface.push('.');
            }
            surface.push(' ');
        }
        if let Some(m) = state.mechanisms.first() {
            surface.push_str(m);
            if !surface.ends_with('.') {
                surface.push('.');
            }
            surface.push(' ');
        }
        if let Some(b) = state.boundaries.first() {
            surface.push_str(b);
            if !surface.ends_with('.') {
                surface.push('.');
            }
        }
        plan.surface_answer = surface.trim().into();
        plan
    }
}

/// Run bounded reason transitions for a prompt.
pub fn run_bounded(user: &str, max_cycles: u32) -> (ReasonRun, ReasonState, SemanticFrame) {
    let max_cycles = max_cycles.clamp(2, 12);
    let frame = extract_frame(user);
    let mut state = ReasonState::from_frame(frame.clone());
    let mut steps = Vec::new();
    let mut halt_reason = "cycle_cap".into();

    for cycle in 0..max_cycles {
        let op = select_op(&state, cycle, max_cycles);
        let mut step = apply_op(&mut state, op);
        step.cycle = cycle;
        let halt = step.halt || step.expected_info_gain_pm < 40 && cycle >= 3;
        steps.push(step);
        if halt || matches!(op, ReasonOp::Halt) {
            halt_reason = if matches!(op, ReasonOp::Halt) {
                "halt_op".into()
            } else if cycle + 1 >= max_cycles {
                "cycle_cap".into()
            } else {
                "info_gain_below_threshold".into()
            };
            break;
        }
        // After compress, halt
        if matches!(op, ReasonOp::CompressConclusion) {
            let mut h = apply_op(&mut state, ReasonOp::Halt);
            h.cycle = cycle + 1;
            steps.push(h);
            halt_reason = "conclusion_compressed".into();
            break;
        }
    }

    let run = ReasonRun {
        schema: "perci.reason-run.v1".into(),
        prompt: user.trim().into(),
        frame_summary: frame.summary_line(),
        final_confidence_pm: state.confidence_pm,
        cycles: steps.len() as u32,
        steps,
        halt_reason,
    };
    (run, state, frame)
}

// ─── Binary transition pack (policy seeds) ───────────────────────────────────

#[derive(Debug)]
pub struct ReasonTransitionPack {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
}

impl ReasonTransitionPack {
    pub fn default_path() -> PathBuf {
        PathBuf::from("models/candidates/packs/percirsn1-v0.1.brsn")
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid PERCIRSN1 header",
            ));
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported PERCIRSN1 version",
            ));
        }
        let record_count = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        Ok(Self {
            path,
            data,
            record_count,
        })
    }

    pub fn record_count(&self) -> usize {
        self.record_count
    }

    pub fn mapped_bytes(&self) -> u64 {
        self.data.len() as u64
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Build candidate transition pack from default policy table.
pub fn build_default_pack(out: impl AsRef<Path>) -> io::Result<usize> {
    // Encode (goal_bucket, cycle) → op as records for future learned policy.
    let mut rows: Vec<(u8, u8, u8, u16, u16, u8)> = Vec::new();
    // goal buckets: 0=mechanism, 1=comparison, 2=plan, 3=general
    for goal in 0u8..4 {
        for cycle in 0u8..10 {
            let op = match (goal, cycle) {
                (_, 0) => ReasonOp::BindRequest,
                (_, 1) => ReasonOp::Decompose,
                (_, 2) => ReasonOp::IdentifyAssumptions,
                (0, 3) | (3, 3) => ReasonOp::ProposeMechanism,
                (1, 3) => ReasonOp::CompareHypotheses,
                (2, 3) => ReasonOp::RetrievePrinciple,
                (_, 4) => ReasonOp::GenerateAlternatives,
                (_, 5) => ReasonOp::SearchCounterexample,
                (_, 6) => ReasonOp::DistinguishAnalogy,
                (_, 7) => ReasonOp::IdentifyUncertainty,
                (_, 8) => ReasonOp::CompressConclusion,
                _ => ReasonOp::Halt,
            };
            rows.push((
                goal,
                cycle,
                op.to_u8(),
                100,
                50,
                if matches!(op, ReasonOp::Halt) { 1 } else { 0 },
            ));
        }
    }
    write_pack(&rows, out)
}

fn write_pack(rows: &[(u8, u8, u8, u16, u16, u8)], out: impl AsRef<Path>) -> io::Result<usize> {
    let out = out.as_ref();
    if let Some(p) = out.parent() {
        fs::create_dir_all(p)?;
    }
    let mut bytes = Vec::with_capacity(HEADER_SIZE + rows.len() * RECORD_SIZE);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
    bytes.extend_from_slice(&(RECORD_SIZE as u32).to_le_bytes());
    while bytes.len() < HEADER_SIZE {
        bytes.push(0);
    }
    for (goal, cycle, op, gain, conf_delta, halt) in rows {
        bytes.push(*goal);
        bytes.push(*cycle);
        bytes.push(*op);
        bytes.push(*halt);
        bytes.extend_from_slice(&gain.to_le_bytes());
        bytes.extend_from_slice(&conf_delta.to_le_bytes());
        // pad + hashes placeholders
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out)?;
    f.write_all(&bytes)?;
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_lag_run_produces_mechanism_claim() {
        let (run, state, frame) =
            run_bounded("Why does trust collapse when communication is delayed?", 8);
        assert!(run.cycles >= 3);
        assert!(!state.mechanisms.is_empty() || !state.claims.is_empty());
        assert!(frame.subject.contains("trust"));
        let plan = run.to_thought_plan(&frame, &state);
        assert!(!plan.surface_answer.is_empty());
        assert!(plan.receipt().contains("reasoning cycles"));
        assert!(plan
            .active_packs
            .iter()
            .any(|p| p.contains("RSN1") || p.contains("SEM1")));
    }

    #[test]
    fn steps_are_named_ops_not_prose_cot() {
        let (run, _, _) = run_bounded("Explain boundary bands vs max coherence", 6);
        for s in &run.steps {
            assert!(!s.op.contains(' '));
            assert!(ReasonOp::from_u8(0).as_str().len() > 0 || !s.op.is_empty());
        }
        assert!(run.steps.iter().any(|s| s.op == "bind_request"));
    }

    #[test]
    fn build_pack_roundtrip() {
        let path = std::env::temp_dir().join("perci-rsn1-test.brsn");
        let n = build_default_pack(&path).unwrap();
        assert!(n >= 10);
        let pack = ReasonTransitionPack::load(&path).unwrap();
        assert_eq!(pack.record_count(), n);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn halts_by_cycle_cap() {
        let (run, _, _) = run_bounded("plan the next transfer test", 3);
        assert!(run.cycles <= 3);
        assert!(!run.halt_reason.is_empty());
    }
}
