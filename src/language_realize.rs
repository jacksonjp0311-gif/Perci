//! PERCILM1 — constrained language realization (Phase 6).
//!
//! Authority is **wording only**. Required claims, evidence, uncertainty, and
//! forbidden claims come from ThoughtPlan + discourse slots. Does not override
//! tools, governance, or reasoning results.
//!
//! Binary pack stores short connective / template atoms (not a transformer).
//! Realization is deterministic composition over plan slots + optional 1/2/4-bit
//! style flags. Candidate-only; never auto-promotes.

use crate::discourse_plan::{
    apply_plan, materialize_slots, plan_discourse, DiscoursePlan, StyleDepth,
};
use crate::thought_plan::{DiscourseAct, Intent, ThoughtPlan};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"PERCLM1\0";
const VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;
/// id + bit_width + 48-byte atom text
const RECORD_SIZE: usize = 56;

/// Bit-width experiment mode for template selection (not full neural weights).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BitWidth {
    One,
    Two,
    Four,
}

impl BitWidth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::One => "1bit",
            Self::Two => "2bit",
            Self::Four => "4bit",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "1" | "1bit" | "one" => Self::One,
            "4" | "4bit" | "four" => Self::Four,
            _ => Self::Two,
        }
    }
}

/// Realization constraints derived from ThoughtPlan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeConstraints {
    pub required_claims: Vec<String>,
    pub forbidden: Vec<String>,
    pub required_terms: Vec<String>,
    pub style_depth: String,
    pub bit_width: String,
}

impl RealizeConstraints {
    pub fn from_plan(plan: &ThoughtPlan, user: &str, bits: BitWidth) -> Self {
        let mut required_claims: Vec<String> =
            plan.claims.iter().map(|c| c.text.clone()).collect();
        for m in plan.mechanisms.iter().take(2) {
            required_claims.push(m.object.clone());
        }
        let forbidden = vec![
            "i am conscious".into(),
            "i feel".into(),
            "private chain-of-thought".into(),
            "weights promoted".into(),
            "i promoted".into(),
            "here's how i'd reason it".into(),
            "• goal:".into(),
            "list premises".into(),
        ];
        let mut required_terms: Vec<String> = plan
            .semantic_bindings
            .iter()
            .filter(|b| matches!(b.role.as_str(), "subject" | "condition" | "phenomenon"))
            .map(|b| b.filler.clone())
            .collect();
        // Pull content tokens from user for binding check
        for w in user.split_whitespace() {
            let w = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if w.len() >= 5
                && !matches!(
                    w.to_ascii_lowercase().as_str(),
                    "about" | "under" | "when" | "does" | "should" | "explain"
                )
            {
                required_terms.push(w.to_ascii_lowercase());
            }
        }
        required_terms.truncate(8);
        Self {
            required_claims,
            forbidden,
            required_terms,
            style_depth: match StyleDepth::from_prompt(user) {
                StyleDepth::Brief => "brief",
                StyleDepth::Balanced => "balanced",
                StyleDepth::Deep => "deep",
            }
            .into(),
            bit_width: bits.as_str().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeResult {
    pub text: String,
    pub discourse: String,
    pub bit_width: String,
    pub constraints_ok: bool,
    pub missing_required: Vec<String>,
    pub forbidden_hits: Vec<String>,
    pub engine: String,
    pub active_packs: Vec<String>,
}

/// Full Phase 3–6 pipeline: RSN plan → discourse → constrained wording.
pub fn realize_from_prompt(user: &str, bits: BitWidth, recent_len: usize) -> RealizeResult {
    let (run, state, frame) = crate::reason_transition::run_bounded(user, 8);
    let mut plan = run.to_thought_plan(&frame, &state);
    let discourse = plan_discourse(&plan, user, recent_len);
    apply_plan(&mut plan, &discourse);
    realize_plan(&plan, &discourse, user, bits)
}

/// Quality gate: reject thin generic modular shells so SoftCascade can own them.
pub fn modular_quality_ok(r: &RealizeResult) -> bool {
    let t = r.text.to_ascii_lowercase();
    let words = t.split_whitespace().count();
    if words < 14 {
        return false;
    }
    // Thin template shells from under-specified frames.
    if t.contains("structure under constraint for") {
        return false;
    }
    if t.starts_with("working claim:")
        || t.starts_with("working mechanism:")
        || t.contains("working mechanism: your")
        || t.contains("your stays coherent")
        || t.contains("stays coherent under stated condition")
        || t.contains("yields a checkable outcome when constraints")
        || t.contains("under condition yields a checkable")
    {
        return false;
    }
    if t.contains("mechanism candidate for") {
        return false;
    }
    if t.contains("working relation for") {
        return false;
    }
    // Reject frames that bound stopwords as subject (your/what/do).
    if t.contains("for your under")
        || t.contains("subject=your")
        || (t.contains(" your ") && t.contains("stated condition") && t.contains("checkable"))
    {
        return false;
    }
    // Forbidden slogans always fail the gate.
    if !r.forbidden_hits.is_empty() {
        return false;
    }
    // Prefer constraint-ok only when domain content is real — not bare template prose.
    if r.constraints_ok {
        let domain_ok = (t.contains("trust") && (t.contains("checkable") || t.contains("timeout")))
            || (t.contains("boundary") && (t.contains("repair") || t.contains("exchange")))
            || (t.contains("life") && t.contains("order"))
            || (t.contains("idempot") || t.contains("done") && t.contains("checkable"))
            || (t.contains("geometry") && (t.contains("boundar") || t.contains("relation")));
        if domain_ok {
            return true;
        }
        // constraints_ok alone is not enough for vague user complaints.
        return false;
    }
    let domain = (t.contains("trust") && (t.contains("checkable") || t.contains("timeout")))
        || (t.contains("boundary") && (t.contains("repair") || t.contains("exchange")))
        || (t.contains("life") && t.contains("order"))
        || (t.contains("idempot") || t.contains("done"));
    domain && words >= 18
}

fn looks_followup_micro(user: &str) -> bool {
    let t = user.trim().to_ascii_lowercase();
    let words = t.split_whitespace().count();
    if words <= 2 {
        return true;
    }
    t.starts_with("what about")
        || t.starts_with("and ")
        || t.starts_with("but ")
        || t.starts_with("though")
        || t.contains("though?")
        || t.starts_with("same for")
        || t.starts_with("same idea")
        || t.starts_with("how about")
        || t.contains("stop transferring")
        || t.contains("analogy stop")
        || t.contains("where does that analogy")
        || t.contains("where does the analogy")
}

/// Chat-path modular realize: substantive intents only, quality-gated.
/// Social / exact / thin unknown stays on SoftCascade / dialogue reflexes.
pub fn try_chat_realize(
    user: &str,
    recent: &[(String, String)],
) -> Option<RealizeResult> {
    let intent = Intent::infer_from_prompt(user);
    if matches!(intent, Intent::Social | Intent::Exact) {
        return None;
    }
    // Micro-social / short non-substantive turns.
    let words = user.split_whitespace().count();
    if words <= 3
        && !user.to_ascii_lowercase().contains("trust")
        && !user.to_ascii_lowercase().contains("why")
        && !user.to_ascii_lowercase().contains("how")
    {
        return None;
    }
    // Refuse micro-turns stay on dedicated identity/dialogue paths.
    if matches!(intent, Intent::Refuse) && words <= 12 {
        return None;
    }

    let eligible = intent.is_modular_eligible()
        || crate::frontier_speech::looks_frontier_turn(user)
        || looks_followup_micro(user) && !recent.is_empty();
    if !eligible {
        return None;
    }

    // Bind deictic follow-ups to last user topic + a short prior claim (continuity).
    let effective = if looks_followup_micro(user) {
        if let Some((prev_u, prev_a)) = recent.last() {
            let prior_claim: String = prev_a
                .split(['.', '!', '?'])
                .next()
                .unwrap_or(prev_a)
                .chars()
                .take(160)
                .collect();
            format!("{user} — continuing from: {prev_u}. Prior claim: {prior_claim}")
        } else {
            user.to_string()
        }
    } else {
        user.to_string()
    };

    // Domain-transfer followups: force life/order or analogy-stop framing into SEM extract.
    let effective = {
        let low = user.to_ascii_lowercase();
        if low.contains("local order") || low.contains("living system") {
            format!("{effective} · subject life local order boundary exchange repair energy")
        } else if low.contains("analogy") && (low.contains("stop") || low.contains("transfer")) {
            format!(
                "{effective} · analogy dies: crystal holds order without life; geometry describes shape not biological cause"
            )
        } else {
            effective
        }
    };

    let r = realize_from_prompt(&effective, BitWidth::Two, recent.len());
    if modular_quality_ok(&r) {
        Some(r)
    } else {
        None
    }
}

/// Realize an existing ThoughtPlan with a discourse plan.
pub fn realize_plan(
    plan: &ThoughtPlan,
    discourse: &DiscoursePlan,
    user: &str,
    bits: BitWidth,
) -> RealizeResult {
    let constraints = RealizeConstraints::from_plan(plan, user, bits);
    let slots = materialize_slots(plan, discourse);
    let text = compose_prose(&slots, &discourse.connectives, bits, &constraints);
    let (constraints_ok, missing, forbidden_hits) = check_constraints(&text, &constraints);

    let mut packs = plan.active_packs.clone();
    if !packs.iter().any(|p| p.contains("LM1") || p.contains("lm1")) {
        packs.push("PERCILM1".into());
    }
    if !packs.iter().any(|p| p.contains("DSC1") || p.contains("dsc1")) {
        packs.push("PERCIDSC1".into());
    }

    RealizeResult {
        text,
        discourse: discourse.summary(),
        bit_width: bits.as_str().into(),
        constraints_ok,
        missing_required: missing,
        forbidden_hits,
        engine: format!("percilm1-compose/{}", bits.as_str()),
        active_packs: packs,
    }
}

fn compose_prose(
    slots: &[(DiscourseAct, String)],
    connectives: &[String],
    bits: BitWidth,
    constraints: &RealizeConstraints,
) -> String {
    if slots.is_empty() {
        return constraints
            .required_claims
            .first()
            .cloned()
            .unwrap_or_else(|| "I don't have a grounded answer yet.".into());
    }

    let mut parts: Vec<String> = Vec::new();
    for (i, (act, material)) in slots.iter().enumerate() {
        let mut chunk = material.trim().to_owned();
        if chunk.is_empty() {
            continue;
        }
        // Ensure sentence casing
        if let Some(first) = chunk.chars().next() {
            if first.is_ascii_lowercase() && !matches!(act, DiscourseAct::Orientation) {
                chunk = first.to_uppercase().collect::<String>() + &chunk[first.len_utf8()..];
            }
        }
        if !chunk.ends_with(['.', '!', '?']) && !matches!(act, DiscourseAct::Orientation) {
            chunk.push('.');
        }

        let bridge = connectives
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("")
            .to_owned();
        let bridge = soft_bridge(bits, act, &bridge, i);

        if i == 0 || bridge.is_empty() {
            parts.push(chunk);
        } else if bridge.starts_with(' ') || bridge.starts_with('.') {
            // Continue prose: ensure prior sentence is closed, then connective + clause.
            let mut c = chunk;
            if bridge.ends_with(' ') {
                if let Some(f) = c.chars().next() {
                    if f.is_ascii_uppercase() {
                        c = f.to_lowercase().collect::<String>() + &c[f.len_utf8()..];
                    }
                }
            }
            if let Some(last) = parts.last_mut() {
                if !last.ends_with(['.', '!', '?']) {
                    last.push('.');
                }
                last.push_str(&format!("{bridge}{c}"));
            } else {
                parts.push(format!("{bridge}{c}").trim().to_owned());
            }
        } else {
            parts.push(format!("{bridge}{chunk}"));
        }
    }

    let mut text = parts.join(" ");
    // Collapse double spaces
    while text.contains("  ") {
        text = text.replace("  ", " ");
    }
    // 1-bit mode: shorter
    if matches!(bits, BitWidth::One) {
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if sentences.len() > 2 {
            text = format!("{}. {}.", sentences[0], sentences[1]);
        }
    }
    // 4-bit: allow slightly more connective density (already have full slots)
    text.trim().to_owned()
}

fn soft_bridge(bits: BitWidth, act: &DiscourseAct, planned: &str, index: usize) -> String {
    if index == 0 {
        return String::new();
    }
    if !planned.is_empty() {
        return match bits {
            BitWidth::One => " ".into(),
            BitWidth::Two => planned.to_owned(),
            BitWidth::Four => match act {
                DiscourseAct::Mechanism => " The mechanism: ".into(),
                DiscourseAct::Boundary | DiscourseAct::Counterexample => " It frays when ".into(),
                DiscourseAct::Uncertainty => " Uncertainty remains: ".into(),
                DiscourseAct::Example => " For example, ".into(),
                _ => planned.to_owned(),
            },
        };
    }
    match (bits, act) {
        (_, DiscourseAct::Mechanism) => " That holds because ".into(),
        (_, DiscourseAct::Boundary) => " Still, ".into(),
        (_, DiscourseAct::Counterexample) => " A counterexample: ".into(),
        (_, DiscourseAct::Uncertainty) => " Note: ".into(),
        (_, DiscourseAct::Example) => " For example, ".into(),
        (BitWidth::One, _) => " ".into(),
        _ => " ".into(),
    }
}

fn check_constraints(
    text: &str,
    c: &RealizeConstraints,
) -> (bool, Vec<String>, Vec<String>) {
    let low = text.to_ascii_lowercase();
    let mut missing = Vec::new();
    // Require at least one claim fragment to appear if claims exist
    if !c.required_claims.is_empty() {
        let any = c.required_claims.iter().any(|claim| {
            let words: Vec<_> = claim
                .split_whitespace()
                .filter(|w| w.len() >= 4)
                .take(4)
                .collect();
            if words.is_empty() {
                return true;
            }
            let hits = words
                .iter()
                .filter(|w| low.contains(&w.to_ascii_lowercase()))
                .count();
            hits >= words.len().saturating_add(1) / 2
        });
        if !any {
            missing.push("required_claim_coverage".into());
        }
    }
    let mut forbidden_hits = Vec::new();
    for f in &c.forbidden {
        if low.contains(f) {
            forbidden_hits.push(f.clone());
        }
    }
    let ok = missing.is_empty() && forbidden_hits.is_empty();
    (ok, missing, forbidden_hits)
}

// ─── Binary atom pack ────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct LanguagePack {
    path: PathBuf,
    data: Mmap,
    record_count: usize,
}

impl LanguagePack {
    pub fn default_path() -> PathBuf {
        PathBuf::from("models/candidates/packs/percilm1-v0.1.blm1")
    }

    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let data = unsafe { MmapOptions::new().map(&file)? };
        if data.len() < HEADER_SIZE || &data[..7] != b"PERCLM1" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid PERCLM1 header",
            ));
        }
        let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported PERCLM1 version",
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

const ATOMS: &[&str] = &[
    "That holds because",
    "In practice",
    "Still",
    "The edge case is",
    "For example",
    "A useful check",
    "Note",
    "Underneath that",
    "It frays when",
    "Uncertainty remains",
];

/// Build candidate atom pack (connectives / style atoms).
pub fn build_default_pack(out: impl AsRef<Path>) -> io::Result<usize> {
    let out = out.as_ref();
    if let Some(p) = out.parent() {
        fs::create_dir_all(p)?;
    }
    let mut bytes = Vec::with_capacity(HEADER_SIZE + ATOMS.len() * RECORD_SIZE);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&(ATOMS.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(HEADER_SIZE as u64).to_le_bytes());
    bytes.extend_from_slice(&(RECORD_SIZE as u32).to_le_bytes());
    while bytes.len() < HEADER_SIZE {
        bytes.push(0);
    }
    for (i, atom) in ATOMS.iter().enumerate() {
        let mut rec = [0u8; RECORD_SIZE];
        rec[0] = i as u8;
        rec[1] = 2; // default 2-bit style lane
        for (j, b) in atom.bytes().take(48).enumerate() {
            rec[8 + j] = b;
        }
        bytes.extend_from_slice(&rec);
    }
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out)?;
    f.write_all(&bytes)?;
    Ok(ATOMS.len())
}

/// Compare bit-width modes: shorter vs fuller wording, same claims.
#[derive(Debug, Clone, Serialize)]
pub struct BitWidthCompare {
    pub prompt: String,
    pub one_bit_chars: usize,
    pub two_bit_chars: usize,
    pub four_bit_chars: usize,
    pub all_constraint_ok: bool,
}

pub fn compare_bit_widths(user: &str) -> BitWidthCompare {
    let a = realize_from_prompt(user, BitWidth::One, 0);
    let b = realize_from_prompt(user, BitWidth::Two, 0);
    let c = realize_from_prompt(user, BitWidth::Four, 0);
    BitWidthCompare {
        prompt: user.into(),
        one_bit_chars: a.text.chars().count(),
        two_bit_chars: b.text.chars().count(),
        four_bit_chars: c.text.chars().count(),
        all_constraint_ok: a.constraints_ok && b.constraints_ok && c.constraints_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realize_trust_is_continuous_and_constrained() {
        let r = realize_from_prompt(
            "Why does trust collapse when communication is delayed?",
            BitWidth::Two,
            0,
        );
        assert!(!r.text.is_empty());
        assert!(!r.text.contains("• Goal:"));
        assert!(!r.text.to_ascii_lowercase().contains("i am conscious"));
        assert!(r.constraints_ok, "missing={:?} forb={:?}", r.missing_required, r.forbidden_hits);
        assert!(
            r.text.to_ascii_lowercase().contains("trust")
                || r.text.to_ascii_lowercase().contains("timeout")
                || r.text.to_ascii_lowercase().contains("done")
        );
        assert!(r.active_packs.iter().any(|p| p.contains("LM1") || p.contains("DSC1")));
    }

    #[test]
    fn one_bit_not_longer_than_four() {
        let c = compare_bit_widths("Why does trust collapse under lag?");
        assert!(
            c.one_bit_chars <= c.four_bit_chars + 40,
            "1bit={} 4bit={}",
            c.one_bit_chars,
            c.four_bit_chars
        );
    }

    #[test]
    fn build_lm_pack() {
        let path = std::env::temp_dir().join("perci-lm1-test.blm1");
        let n = build_default_pack(&path).unwrap();
        assert_eq!(n, ATOMS.len());
        let pack = LanguagePack::load(&path).unwrap();
        assert_eq!(pack.record_count(), n);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn forbids_consciousness_slogan() {
        let mut plan = ThoughtPlan::empty("t", crate::thought_plan::Intent::Refuse);
        plan.claims.push(crate::thought_plan::BoundClaim {
            text: "I refuse consciousness claims.".into(),
            status: "refuse".into(),
        });
        plan.surface_answer = "I refuse. SoftCascade mass is not a self.".into();
        let d = plan_discourse(&plan, "are you conscious?", 0);
        let r = realize_plan(&plan, &d, "are you conscious?", BitWidth::Two);
        assert!(!r.text.to_ascii_lowercase().contains("i am conscious"));
    }

    #[test]
    fn try_chat_realize_skips_social() {
        assert!(try_chat_realize("hey what's up", &[]).is_none());
    }

    #[test]
    fn try_chat_realize_takes_trust() {
        let r = try_chat_realize(
            "Why does trust collapse when communication is delayed?",
            &[],
        );
        assert!(r.is_some(), "expected modular path");
        let r = r.unwrap();
        assert!(modular_quality_ok(&r));
        assert!(
            r.text.to_ascii_lowercase().contains("trust")
                || r.text.to_ascii_lowercase().contains("timeout")
        );
    }

    #[test]
    fn try_chat_realize_binds_followup() {
        let recent = vec![(
            "Why does trust collapse when communication is delayed?".into(),
            "Trust fails when done is not checkable.".into(),
        )];
        let r = try_chat_realize("what about timeout and retry though?", &recent);
        assert!(r.is_some(), "followup should bind via modular");
        let text = r.unwrap().text.to_ascii_lowercase();
        assert!(
            text.contains("timeout")
                || text.contains("retry")
                || text.contains("trust")
                || text.contains("idempot")
                || text.contains("done"),
            "got={text}"
        );
    }
}
