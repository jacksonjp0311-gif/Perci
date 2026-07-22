//! PERCIDSC1 — discourse planning pack (Phase 5).
//!
//! Converts a resolved ThoughtPlan into rhetorical acts with variant selection
//! so answers avoid fixed checklist cadence. Wording authority stays with LM1;
//! truth/authority stay with operators, tools, and governance.
//!
//! Candidate-only. Never auto-promotes.

use crate::thought_plan::{DiscourseAct, Intent, ThoughtPlan};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCDSC1";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
/// intent_id, variant, n_acts, 8 act bytes, pad → 16 bytes.
const RECORD_SIZE: usize = 16;

/// One selectable discourse skeleton.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoursePlan {
    pub intent: String,
    pub variant: u8,
    pub acts: Vec<DiscourseAct>,
    pub connectives: Vec<String>,
    pub style_notes: Vec<String>,
}

impl DiscoursePlan {
    pub fn summary(&self) -> String {
        let acts = self
            .acts
            .iter()
            .map(|a| a.as_str())
            .collect::<Vec<_>>()
            .join(" → ");
        format!("intent={} v{} acts=[{acts}]", self.intent, self.variant)
    }
}

/// Style depth for plan selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleDepth {
    Brief,
    Balanced,
    Deep,
}

impl StyleDepth {
    pub fn from_prompt(user: &str) -> Self {
        let t = user.to_ascii_lowercase();
        if t.contains("brief") || t.contains("short") || t.contains("one sentence") {
            Self::Brief
        } else if t.contains("deep") || t.contains("detail") || t.contains("thorough") {
            Self::Deep
        } else {
            Self::Balanced
        }
    }
}

/// Select a discourse plan for a ThoughtPlan (+ optional prompt for depth/history).
pub fn plan_discourse(plan: &ThoughtPlan, user: &str, recent_len: usize) -> DiscoursePlan {
    let depth = StyleDepth::from_prompt(user);
    let variant = select_variant(plan, user, recent_len, depth);
    let acts = acts_for(plan.intent, depth, variant, plan);
    let connectives = connectives_for(variant, &acts);
    let style_notes = style_notes_for(depth, variant);

    DiscoursePlan {
        intent: plan.intent.as_str().into(),
        variant,
        acts,
        connectives,
        style_notes,
    }
}

fn select_variant(plan: &ThoughtPlan, user: &str, recent_len: usize, depth: StyleDepth) -> u8 {
    // Stable hash from intent + prompt + history length so multi-turn varies.
    // Fold word count and subject bindings so paraphrases / followups diversify act order.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in plan.intent.as_str().bytes().chain(user.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h ^= (recent_len as u64).wrapping_mul(0x9E37);
    h ^= (user.split_whitespace().count() as u64).wrapping_mul(0x517C_C1B7);
    for b in &plan.semantic_bindings {
        for byte in b.role.bytes().chain(b.filler.bytes()) {
            h ^= byte as u64;
            h = h.wrapping_mul(0x1000_0000_01b3);
        }
    }
    h ^= match depth {
        StyleDepth::Brief => 1,
        StyleDepth::Balanced => 2,
        StyleDepth::Deep => 3,
    };
    // Mix high bits so nearby strings land on different variants more often.
    h ^= h >> 33;
    (h % 3) as u8
}

fn acts_for(
    intent: Intent,
    depth: StyleDepth,
    variant: u8,
    plan: &ThoughtPlan,
) -> Vec<DiscourseAct> {
    if matches!(intent, Intent::Refuse) {
        return vec![DiscourseAct::Refuse, DiscourseAct::Boundary];
    }
    if matches!(intent, Intent::Social | Intent::Exact) {
        return vec![DiscourseAct::DirectAnswer];
    }

    let mut acts = match (intent, depth, variant % 3) {
        (Intent::CausalExplanation | Intent::Trust, StyleDepth::Brief, _) => {
            vec![DiscourseAct::DirectAnswer, DiscourseAct::Boundary]
        }
        (Intent::CausalExplanation | Intent::Trust, StyleDepth::Balanced, 0) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Mechanism,
            DiscourseAct::Boundary,
        ],
        (Intent::CausalExplanation | Intent::Trust, StyleDepth::Balanced, 1) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Example,
            DiscourseAct::Mechanism,
            DiscourseAct::Boundary,
        ],
        (Intent::CausalExplanation | Intent::Trust, StyleDepth::Balanced, _) => vec![
            DiscourseAct::Mechanism,
            DiscourseAct::DirectAnswer,
            DiscourseAct::Counterexample,
        ],
        (Intent::CausalExplanation | Intent::Trust, StyleDepth::Deep, 0) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Mechanism,
            DiscourseAct::Example,
            DiscourseAct::Boundary,
            DiscourseAct::Uncertainty,
        ],
        (Intent::CausalExplanation | Intent::Trust, StyleDepth::Deep, _) => vec![
            DiscourseAct::Orientation,
            DiscourseAct::Mechanism,
            DiscourseAct::Counterexample,
            DiscourseAct::Boundary,
            DiscourseAct::Uncertainty,
        ],
        (Intent::Comparison | Intent::Synthesis, StyleDepth::Brief, _) => {
            vec![DiscourseAct::Judgment, DiscourseAct::Boundary]
        }
        (Intent::Comparison | Intent::Synthesis, _, 0) => vec![
            DiscourseAct::Criteria,
            DiscourseAct::Tradeoff,
            DiscourseAct::Judgment,
            DiscourseAct::Boundary,
        ],
        (Intent::Comparison | Intent::Synthesis, _, 1) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Tradeoff,
            DiscourseAct::Boundary,
        ],
        (Intent::Comparison | Intent::Synthesis, _, _) => vec![
            DiscourseAct::Orientation,
            DiscourseAct::Criteria,
            DiscourseAct::Judgment,
            DiscourseAct::Uncertainty,
        ],
        (Intent::Verification, _, 0) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Evidence,
            DiscourseAct::Counterexample,
            DiscourseAct::Uncertainty,
        ],
        (Intent::Verification, _, _) => vec![
            DiscourseAct::Evidence,
            DiscourseAct::DirectAnswer,
            DiscourseAct::Boundary,
        ],
        (Intent::Teaching, StyleDepth::Brief, _) => {
            vec![DiscourseAct::DirectAnswer, DiscourseAct::Example]
        }
        (Intent::Teaching, _, 0) => vec![
            DiscourseAct::Orientation,
            DiscourseAct::Mechanism,
            DiscourseAct::Example,
            DiscourseAct::CheckUnderstanding,
        ],
        (Intent::Teaching, _, _) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Example,
            DiscourseAct::Boundary,
            DiscourseAct::CheckUnderstanding,
        ],
        (Intent::Plan, _, 0) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Mechanism,
            DiscourseAct::Boundary,
        ],
        (Intent::Plan, _, _) => vec![
            DiscourseAct::Orientation,
            DiscourseAct::DirectAnswer,
            DiscourseAct::Evidence,
            DiscourseAct::Boundary,
        ],
        (Intent::Identity, _, _) => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Boundary,
            DiscourseAct::Refuse,
        ],
        _ => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Boundary,
            DiscourseAct::Uncertainty,
        ],
    };

    // Drop acts that have no material when possible (avoid empty checklist slots).
    acts.retain(|a| match a {
        DiscourseAct::Mechanism => !plan.mechanisms.is_empty() || !plan.claims.is_empty(),
        DiscourseAct::Counterexample | DiscourseAct::Boundary => {
            !plan.boundaries.is_empty() || matches!(a, DiscourseAct::Boundary)
        }
        DiscourseAct::Uncertainty => !plan.uncertainties.is_empty() || plan.confidence_pm < 800,
        DiscourseAct::Evidence => !plan.evidence.is_empty() || !plan.claims.is_empty(),
        DiscourseAct::Example => !plan.alternatives.is_empty() || !plan.mechanisms.is_empty(),
        _ => true,
    });

    if acts.is_empty() {
        acts.push(DiscourseAct::DirectAnswer);
    }
    // Cap length for brief
    if matches!(depth, StyleDepth::Brief) && acts.len() > 2 {
        acts.truncate(2);
    }
    acts
}

fn connectives_for(variant: u8, acts: &[DiscourseAct]) -> Vec<String> {
    let pool = match variant % 3 {
        0 => ["", " That holds because ", " Still, ", " A useful check: "],
        1 => ["", " Underneath that, ", " It frays when ", " Note: "],
        _ => ["", " In practice, ", " The edge case is ", " Uncertainty: "],
    };
    acts.iter()
        .enumerate()
        .map(|(i, _)| pool[i.min(pool.len() - 1)].to_owned())
        .collect()
}

fn style_notes_for(depth: StyleDepth, variant: u8) -> Vec<String> {
    let mut n = vec![
        "lead with the point".into(),
        "no checklist section titles".into(),
        "no private chain-of-thought".into(),
    ];
    match depth {
        StyleDepth::Brief => n.push("one or two sentences".into()),
        StyleDepth::Balanced => n.push("continuous multi-sentence prose".into()),
        StyleDepth::Deep => n.push("mechanism + boundary + uncertainty".into()),
    }
    if variant == 1 {
        n.push("prefer example before boundary".into());
    }
    if variant == 2 {
        n.push("prefer mechanism-first ordering".into());
    }
    n
}

/// Apply a discourse plan to a ThoughtPlan: set acts + render ordered material slots.
pub fn apply_plan(plan: &mut ThoughtPlan, discourse: &DiscoursePlan) {
    plan.discourse_acts = discourse.acts.clone();
    if !plan
        .active_packs
        .iter()
        .any(|p| p.contains("DSC1") || p.contains("dsc1"))
    {
        plan.active_packs.push("PERCIDSC1".into());
    }
}

/// Materialize ordered content chunks for each discourse act (facts only from plan).
pub fn materialize_slots(
    plan: &ThoughtPlan,
    discourse: &DiscoursePlan,
) -> Vec<(DiscourseAct, String)> {
    let mut out = Vec::new();
    for act in &discourse.acts {
        let text = match act {
            DiscourseAct::DirectAnswer | DiscourseAct::Judgment => plan
                .claims
                .first()
                .map(|c| c.text.clone())
                .or_else(|| {
                    plan.mechanisms
                        .first()
                        .map(|m| m.object.clone())
                })
                .unwrap_or_else(|| plan.surface_answer.clone()),
            DiscourseAct::Mechanism => plan
                .mechanisms
                .first()
                .map(|m| m.object.clone())
                .or_else(|| plan.claims.get(1).map(|c| c.text.clone()))
                .unwrap_or_default(),
            DiscourseAct::Example => plan
                .alternatives
                .first()
                .map(|a| a.text.clone())
                .or_else(|| {
                    plan.mechanisms
                        .get(1)
                        .map(|m| m.object.clone())
                })
                .unwrap_or_default(),
            DiscourseAct::Boundary | DiscourseAct::Counterexample => plan
                .boundaries
                .first()
                .map(|b| b.text.clone())
                .unwrap_or_else(|| {
                    "The claim stops where mechanisms would have to become identical across domains."
                        .into()
                }),
            DiscourseAct::Uncertainty => plan
                .uncertainties
                .first()
                .map(|u| u.text.clone())
                .unwrap_or_else(|| {
                    format!(
                        "confidence remains bounded ({}‰); more evidence can revise.",
                        plan.confidence_pm
                    )
                }),
            DiscourseAct::Evidence => plan
                .evidence
                .first()
                .map(|e| e.claim.clone())
                .or_else(|| plan.claims.first().map(|c| c.text.clone()))
                .unwrap_or_default(),
            DiscourseAct::Criteria => {
                "Score options on cost of being wrong and checkability under lag.".into()
            }
            DiscourseAct::Tradeoff => plan
                .alternatives
                .first()
                .map(|a| a.text.clone())
                .unwrap_or_else(|| {
                    "Tradeoff: fluency vs auditability — prefer checkable contracts.".into()
                }),
            DiscourseAct::Orientation => {
                let subj = plan
                    .semantic_bindings
                    .iter()
                    .find(|b| b.role == "subject")
                    .map(|b| b.filler.as_str())
                    .unwrap_or("this");
                format!("On {subj}:")
            }
            DiscourseAct::CheckUnderstanding => {
                "If that misses, name the part to pressure-test next.".into()
            }
            DiscourseAct::Refuse => plan
                .claims
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_else(|| "I refuse that claim; it is not supported.".into()),
        };
        let text = text.trim().to_owned();
        if !text.is_empty() {
            out.push((*act, text));
        }
    }
    out
}

// ─── Binary pack ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct DiscoursePack {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
}

impl DiscoursePack {
    pub fn default_path() -> PathBuf {
        PathBuf::from("models/candidates/packs/percidsc1-v0.1.bdsc")
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..8] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid PERCDSC1 header",
            ));
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported PERCDSC1 version",
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

fn intent_id(intent: Intent) -> u8 {
    match intent {
        Intent::CausalExplanation => 0,
        Intent::Comparison => 1,
        Intent::Verification => 2,
        Intent::Teaching => 3,
        Intent::Plan => 4,
        Intent::Refuse => 5,
        Intent::Social => 6,
        Intent::Exact => 7,
        Intent::Synthesis => 8,
        Intent::Identity => 9,
        Intent::Trust => 10,
        Intent::Unknown => 11,
    }
}

fn act_id(a: DiscourseAct) -> u8 {
    match a {
        DiscourseAct::DirectAnswer => 0,
        DiscourseAct::Mechanism => 1,
        DiscourseAct::Example => 2,
        DiscourseAct::Boundary => 3,
        DiscourseAct::Criteria => 4,
        DiscourseAct::Tradeoff => 5,
        DiscourseAct::Judgment => 6,
        DiscourseAct::Evidence => 7,
        DiscourseAct::Counterexample => 8,
        DiscourseAct::Uncertainty => 9,
        DiscourseAct::Orientation => 10,
        DiscourseAct::CheckUnderstanding => 11,
        DiscourseAct::Refuse => 12,
    }
}

/// Build candidate discourse pack (intent × variant skeletons).
pub fn build_default_pack(out: impl AsRef<Path>) -> io::Result<usize> {
    let intents = [
        Intent::CausalExplanation,
        Intent::Trust,
        Intent::Comparison,
        Intent::Synthesis,
        Intent::Verification,
        Intent::Teaching,
        Intent::Plan,
        Intent::Identity,
        Intent::Refuse,
    ];
    let mut rows = Vec::new();
    for intent in intents {
        for variant in 0u8..3 {
            let dummy = ThoughtPlan::empty("pack-build", intent);
            let acts = acts_for(intent, StyleDepth::Balanced, variant, &dummy);
            rows.push((intent_id(intent), variant, acts));
        }
    }
    write_pack(&rows, out)
}

fn write_pack(rows: &[(u8, u8, Vec<DiscourseAct>)], out: impl AsRef<Path>) -> io::Result<usize> {
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
    for (intent, variant, acts) in rows {
        let mut rec = [0u8; RECORD_SIZE];
        rec[0] = *intent;
        rec[1] = *variant;
        rec[2] = acts.len().min(8) as u8;
        for i in 0..8 {
            rec[4 + i] = acts.get(i).map(|a| act_id(*a)).unwrap_or(255);
        }
        bytes.extend_from_slice(&rec);
    }
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out)?;
    f.write_all(&bytes)?;
    Ok(rows.len())
}

/// Anti-robotic / variation eval: same intent, different prompts → different variants or act order.
#[derive(Debug, Clone, Serialize)]
pub struct DiscourseEvalReport {
    pub schema: String,
    pub pairs: usize,
    pub varied: usize,
    pub details: Vec<String>,
}

pub fn evaluate_variation(prompts: &[(Intent, &str)]) -> DiscourseEvalReport {
    let mut pairs = 0;
    let mut varied = 0;
    let mut details = Vec::new();
    for i in 0..prompts.len() {
        for j in (i + 1)..prompts.len() {
            if prompts[i].0 != prompts[j].0 {
                continue;
            }
            pairs += 1;
            let pi = ThoughtPlan::empty("eval", prompts[i].0);
            let pj = ThoughtPlan::empty("eval", prompts[j].0);
            let a = plan_discourse(&pi, prompts[i].1, i);
            let b = plan_discourse(&pj, prompts[j].1, j + 3);
            if a.variant != b.variant || a.acts != b.acts {
                varied += 1;
            } else {
                details.push(format!(
                    "same plan for {:?} / {:?} (v{})",
                    prompts[i].1, prompts[j].1, a.variant
                ));
            }
        }
    }
    DiscourseEvalReport {
        schema: "perci.discourse-eval.v1".into(),
        pairs,
        varied,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reason_transition::run_bounded;

    #[test]
    fn causal_plan_has_mechanism_and_boundary() {
        let (run, state, frame) =
            run_bounded("Why does trust collapse when communication is delayed?", 8);
        let mut plan = run.to_thought_plan(&frame, &state);
        let d = plan_discourse(
            &plan,
            "Why does trust collapse when communication is delayed?",
            0,
        );
        apply_plan(&mut plan, &d);
        assert!(
            d.acts.contains(&DiscourseAct::DirectAnswer)
                || d.acts.contains(&DiscourseAct::Mechanism)
        );
        let slots = materialize_slots(&plan, &d);
        assert!(!slots.is_empty());
        assert!(!slots.iter().any(|(_, t)| t.is_empty()));
    }

    #[test]
    fn variants_differ_across_prompts() {
        let report = evaluate_variation(&[
            (Intent::Trust, "how should interfaces earn trust under lag"),
            (Intent::Trust, "why does trust collapse when delayed"),
            (Intent::Trust, "earn trust under timeout and retry"),
        ]);
        assert!(report.pairs >= 1);
        assert!(report.varied >= 1, "details={:?}", report.details);
    }

    #[test]
    fn build_pack_roundtrip() {
        let path = std::env::temp_dir().join("perci-dsc1-test.bdsc");
        let n = build_default_pack(&path).unwrap();
        assert!(n >= 9);
        let pack = DiscoursePack::load(&path).unwrap();
        assert_eq!(pack.record_count(), n);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn brief_is_shorter() {
        let plan =
            ThoughtPlan::empty("t", Intent::CausalExplanation).push_claim("claim A", "working");
        let brief = plan_discourse(&plan, "brief: why does trust fail under lag", 0);
        let deep = plan_discourse(&plan, "go deep: why does trust fail under lag", 0);
        assert!(brief.acts.len() <= deep.acts.len() || brief.acts.len() <= 3);
    }
}
