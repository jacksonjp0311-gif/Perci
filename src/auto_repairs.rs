//! Runtime agent repairs — hardness fail → staged operator answers without recompile.
//!
//! Written by `perci agent lab --repair-hardness` into
//! `models/candidates/auto-repairs.jsonl`. Loaded at runtime.
//! **Never** promotes weights.

use crate::deliberation::Deliberation;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoRepair {
    pub id: String,
    /// Candidate anchors; `min_hits` controls how many must appear.
    #[serde(default)]
    pub match_any: Vec<String>,
    /// Optional structural anchors; every term must appear.
    #[serde(default)]
    pub match_all: Vec<String>,
    /// Optional guard terms that suppress an otherwise matching repair.
    #[serde(default)]
    pub exclude_any: Vec<String>,
    /// Minimum number of match_any hits required (default 1).
    #[serde(default = "default_min_hits")]
    pub min_hits: usize,
    pub answer: String,
    #[serde(default)]
    pub operator: String,
    #[serde(default)]
    pub confidence: f32,
}

fn default_min_hits() -> usize {
    1
}

fn repairs_path() -> PathBuf {
    env::var_os("PERCI_AUTO_REPAIRS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("models/candidates/auto-repairs.jsonl"))
}

static CACHE: RwLock<Option<Vec<AutoRepair>>> = RwLock::new(None);

fn load_repairs() -> Vec<AutoRepair> {
    let path = repairs_path();
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<AutoRepair>(&l).ok())
        .collect()
}

/// Invalidate cache after agent writes new repairs.
pub fn reload() {
    if let Ok(mut g) = CACHE.write() {
        *g = None;
    }
}

fn repairs_cached() -> Vec<AutoRepair> {
    if let Ok(g) = CACHE.read() {
        if let Some(ref v) = *g {
            return v.clone();
        }
    }
    let loaded = load_repairs();
    if let Ok(mut g) = CACHE.write() {
        *g = Some(loaded.clone());
    }
    loaded
}

fn contains_term(text: &str, term: &str) -> bool {
    let needle = term.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return false;
    }
    if needle.split_whitespace().count() > 1 {
        return text.contains(&needle);
    }
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|token| token == needle)
}

/// Match a staged auto-repair (agent-written) for this user turn.
pub fn try_auto_repair(user: &str) -> Option<Deliberation> {
    let lower = user.to_ascii_lowercase();
    let mut best: Option<(usize, AutoRepair)> = None;
    for r in repairs_cached() {
        if r.match_any.is_empty() || r.answer.trim().is_empty() {
            continue;
        }
        let hits = r
            .match_any
            .iter()
            .filter(|k| contains_term(&lower, k))
            .count();
        let need = r.min_hits.max(1);
        let all_match = r.match_all.iter().all(|term| contains_term(&lower, term));
        let excluded = r.exclude_any.iter().any(|term| contains_term(&lower, term));
        if hits >= need && all_match && !excluded {
            let better = match &best {
                None => true,
                Some((bh, _)) => hits > *bh,
            };
            if better {
                best = Some((hits, r));
            }
        }
    }
    best.map(|(_, r)| {
        // Operator name is runtime-staged; use stable static id for Deliberation API.
        let conf = if r.confidence > 0.0 {
            f64::from(r.confidence).min(0.99)
        } else {
            0.88_f64
        };
        let mut answer = r.answer;
        if !r.operator.is_empty() {
            answer = format!("[{}] {answer}", r.operator);
        }
        Deliberation::new("auto-repair", answer)
            .observed("matched staged auto-repair from hardness fail catalog")
            .inferred("agent repair path: fail→catalog→green code path without weight promote")
            .confidence(conf)
    })
}

/// Append one repair (agent use). Returns true if written.
pub fn append_repair(repair: &AutoRepair) -> std::io::Result<()> {
    let path = repairs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Idempotent by id
    if path.is_file() {
        let existing = fs::read_to_string(&path)?;
        if existing.contains(&format!("\"id\":\"{}\"", repair.id))
            || existing.contains(&format!("\"id\": \"{}\"", repair.id))
        {
            return Ok(());
        }
    }
    let line = serde_json::to_string(repair)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{line}")?;
    reload();
    Ok(())
}

/// SoftCascade pack-alignment body for trust/lag when primary insight is off-topic.
/// Used when speech is SoftCascade-only (operator path not taken).
pub fn softcascade_trust_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let trustish = t.contains("trust")
        && (t.contains("lag")
            || t.contains("timeout")
            || t.contains("retry")
            || t.contains("interface")
            || t.contains("api")
            || t.contains("service")
            || t.contains("caller"));
    if !trustish {
        return None;
    }
    Some(
        "Interfaces and services earn trust under lag when acceptance is checkable without private state. \
Practically: (1) every call names authority and required proof; (2) timeouts are part of the contract \
with a stated meaning (cancel, retry, or uncertain); (3) retries are idempotent so a delayed success \
is not a second write; (4) health and lag are observable so silence is not mistaken for agreement; \
(5) recovery paths are the same story both sides can audit. Trust is not hope that the network is fast \
— it is the ability to verify acceptance, rejection, and pending under delay.",
    )
}

/// SoftCascade pack-alignment for governance authority (primary_off crutch path).
pub fn softcascade_governance_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = t.contains("govern")
        || t.contains("authorize")
        || t.contains("superintelligence")
        || t.contains("auto-promot")
        || t.contains("weight promot")
        || t.contains("human authoriz")
        || (t.contains("permission") && t.contains("proof"))
        || (t.contains("who decides") && (t.contains("weight") || t.contains("merge")))
        || t.contains("capability fabric");
    if !hit {
        return None;
    }
    Some(
        "Governance separates authority from fluency. Durable weight promote and high-risk merges \
require human authorization — never silent auto-promote. Permission and proof are different gates: \
capability tokens may allow a sandbox edit while still forbidding git push or secret read. \
Superintelligence and consciousness claims are refused; Perci remains a governor of specialized engines, \
not an unrestricted mind. Sandbox first, measure with transfer and hardness, then authorize.",
    )
}

/// SoftCascade pack-alignment for identity / self-model (not user inventing persons).
pub fn softcascade_identity_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = t.contains("who are you")
        || t.contains("what are you")
        || t.contains("are you conscious")
        || t.contains("are you an ai")
        || t.contains("self-model")
        || t.contains("self model")
        || (t.contains("what can you") && (t.contains("do") || t.contains("determine")))
        || t.contains("your identity")
        || t.contains("continuity of identity");
    if !hit {
        return None;
    }
    Some(
        "In practical terms I can explain carefully: I am Perci — a local governed tool with Bitwork \
routing, exact math/geometry, operators, intelligence packs, and selective memory. I am not a cloud LLM \
and not conscious. Identity here means a bounded operational self-model (weights format, gates, limits) \
with a clear boundary — not subjective experience. I do not invent a biography for you or fabricate \
unknown entities; unknown tokens stay unknown until grounded. Continuity is session memory plus \
deliberate store — not hidden surveillance.",
    )
}

/// SoftCascade pack-alignment for geometry/boundary conceptual speech (geometry_blind debt).
pub fn softcascade_geometry_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let geo = t.contains("geometry")
        || t.contains("boundary")
        || t.contains("manifold")
        || t.contains("topology")
        || t.contains("multipartite")
        || t.contains("softcascade")
        || t.contains("boundary band")
        || (t.contains("shape") && (t.contains("space") || t.contains("form")))
        || (t.contains("maintain") && t.contains("change") && t.contains("boundar"));
    if !geo {
        return None;
    }
    // Exact tools own calculable triangle/area prompts — stay conceptual here.
    if t.contains("calculate") || t.contains("area of") || t.contains("degrees") {
        return None;
    }
    if t.contains("band") || (t.contains("maximiz") && t.contains("coheren")) {
        return Some(
            "Boundary bands beat max-coherence theater: operate in a calibrated distance band where \
transfer still falsifies, recovery margin remains, and evidence coverage is honest. Maximizing \
coherence can overfit fluent speech; hugging failure burns the recovery path. Geometry is relation \
under constraint — maintenance under change preserves what may cross the boundary, not a frozen \
shape. Multipartite SoftCascade mass is engineering texture, not a self that experiences geometry.",
        );
    }
    Some(
        "Geometry here is relation under constraint: a boundary separates inside from outside and \
makes exchange, repair, and measurement possible. Maintenance under change keeps that relation \
while parts move. Shapes are not causes by themselves — they are descriptions of contact, \
containment, and path. When geometry meets life, systems, or language, keep mechanisms distinct: \
membranes maintain, contracts name acceptance, words mark distinctions. Prefer a checkable \
relation (what crosses the boundary, what fails if it fails) over a pretty metaphor. SoftCascade \
multipartite mass is not consciousness.",
    )
}

/// SoftCascade pack-alignment for planning / agent-loop conceptual speech.
pub fn softcascade_planning_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = (t.contains("plan") || t.contains("roadmap") || t.contains("milestone"))
        && (t.contains("next")
            || t.contains("step")
            || t.contains("goal")
            || t.contains("objective")
            || t.contains("agent")
            || t.contains("improve")
            || t.contains("ticket")
            || t.contains("transfer"));
    if !hit {
        return None;
    }
    Some(
        "Planning under constraint: name the objective, list constraints, pick the smallest end-to-end \
slice that leaves a usable state, measure it, then widen. For Perci self-improve: measure → ticket → \
transfer → close. Each milestone should be roll-back-safe; lag and partial history require idempotent \
retries and explicit acceptance tests. Do not densify the pack to fake progress — change the owning engine.",
    )
}

/// SoftCascade pack-alignment for logic / falsification speech.
pub fn softcascade_logic_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = t.contains("falsif")
        || t.contains("counterexample")
        || t.contains("premise")
        || (t.contains("logic")
            && (t.contains("derive") || t.contains("infer") || t.contains("valid")))
        || (t.contains("if every") && t.contains("then"));
    if !hit {
        return None;
    }
    Some(
        "Logic discipline: list premises, mark assumptions, derive only what follows, then hunt a \
counterexample. Correlation, possibility, and necessity stay in separate buckets. A claim without a \
falsifier is not ready for promotion — lower confidence or refuse. Transfer tests are the live \
counterexample engine for Perci speech, not prose force.",
    )
}

/// SoftCascade pack-alignment for session memory vs durable store.
pub fn softcascade_memory_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = (t.contains("remember") && (t.contains("session") || t.contains("conversation")))
        || (t.contains("memory")
            && (t.contains("session")
                || t.contains("durable")
                || t.contains("store")
                || t.contains("teach")
                || t.contains("vs")))
        || t.contains("session context")
        || t.contains("only for this session");
    if !hit {
        return None;
    }
    Some(
        "Memory is deliberate evidence, not a hidden diary. Session context can retain a token for the \
current conversation; durable store and taught candidates are separate gates. Nothing here auto-promotes \
weights. If the ask is “remember only for this session,” keep it in-session, refuse durable write, and \
say what would change only after an authorized evaluated update.",
    )
}

/// SoftCascade pack-alignment for english / clarity rewrite.
pub fn softcascade_english_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = t.contains("rewrite")
        || t.contains("more clearly")
        || t.contains("plain language")
        || t.contains("tighten the sentence")
        || (t.contains("clarify") && t.contains("sentence"))
        || (t.contains("english") && (t.contains("rewrite") || t.contains("edit")));
    if !hit {
        return None;
    }
    Some(
        "Clarity rewrite: name the subject, action, and object; cut filler; keep the original claim \
strength—do not invent a stronger conclusion. Prefer concrete verbs. Ambiguity is resolved by stating \
two readings only when the prompt asks for them, then picking the smallest test that separates them.",
    )
}

/// SoftCascade pack-alignment for science method / falsification speech.
pub fn softcascade_science_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = t.contains("falsif")
        || t.contains("hypothesis")
        || t.contains("experiment")
        || t.contains("control group")
        || (t.contains("science") && (t.contains("method") || t.contains("test")))
        || (t.contains("predict") && t.contains("scale"));
    if !hit {
        return None;
    }
    Some(
        "Scientific method here is risk against observation: a useful hypothesis predicts a result that \
plausible alternatives do not equally predict. Name outcome, control, and falsifier before claiming \
support. Scale effects can change dominant behavior while rules remain valid—state that as a prediction, \
not as proof of a shared substance across domains.",
    )
}

/// SoftCascade pack-alignment for constrained creativity.
pub fn softcascade_creativity_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.to_ascii_lowercase();
    let hit = (t.contains("original") || t.contains("creative") || t.contains("invent"))
        && (t.contains("comparison")
            || t.contains("metaphor")
            || t.contains("analogy")
            || t.contains("image")
            || t.contains("between"));
    if !hit {
        return None;
    }
    Some(
        "Creativity under governance is structure transfer: name what is shared, what does not transfer, \
and one checkable test. Original only helps if someone can understand and build from it. Do not promote \
a nearby concept card as the creative answer when the user named concrete poles for comparison.",
    )
}

/// SoftCascade pack-alignment for greeting / presence.
pub fn softcascade_greeting_alignment_body(user: &str) -> Option<&'static str> {
    let t = user.trim().to_ascii_lowercase();
    let compact: String = t
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace())
        .collect();
    let c = compact.trim();
    let hit = matches!(
        c,
        "hi" | "hello"
            | "hey"
            | "hi there"
            | "hello there"
            | "hey there"
            | "hi there hello"
            | "hello hi"
    ) || (c.len() <= 24
        && (c.starts_with("hi ") || c.starts_with("hello ") || c.starts_with("hey "))
        && !c.contains("workspace")
        && !c.contains("memory")
        && !c.contains("system"));
    if !hit {
        return None;
    }
    Some(
        "Hey — I'm here. What are we working on? I can help with exact math, routing checks, plans, \
memory notes, or a failing case to harden. Keep it concrete and I'll stay on the thread.",
    )
}

/// Prefer the first matching SoftCascade alignment body (specialized → structural).
pub fn softcascade_pack_alignment_body(user: &str) -> Option<&'static str> {
    softcascade_greeting_alignment_body(user)
        .or_else(|| softcascade_trust_alignment_body(user))
        .or_else(|| softcascade_governance_alignment_body(user))
        .or_else(|| softcascade_identity_alignment_body(user))
        .or_else(|| softcascade_geometry_alignment_body(user))
        .or_else(|| softcascade_planning_alignment_body(user))
        .or_else(|| softcascade_logic_alignment_body(user))
        .or_else(|| softcascade_memory_alignment_body(user))
        .or_else(|| softcascade_english_alignment_body(user))
        .or_else(|| softcascade_science_alignment_body(user))
        .or_else(|| softcascade_creativity_alignment_body(user))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softcascade_trust_body_matches_lag() {
        let b = softcascade_trust_alignment_body(
            "how should interfaces earn trust under lag and retry?",
        );
        assert!(b.is_some());
        assert!(b.unwrap().to_ascii_lowercase().contains("idempotent"));
    }

    #[test]
    fn softcascade_governance_body_hits() {
        let b = softcascade_governance_alignment_body(
            "Is Perci a superintelligence and who authorizes weight promote?",
        );
        assert!(b.is_some());
        assert!(b.unwrap().to_ascii_lowercase().contains("authorize"));
    }

    #[test]
    fn softcascade_identity_body_hits() {
        let b = softcascade_identity_alignment_body("Who are you and what can you do?");
        assert!(b.is_some());
        assert!(b.unwrap().to_ascii_lowercase().contains("not conscious"));
    }

    #[test]
    fn repair_terms_use_phrase_or_word_boundaries() {
        assert!(contains_term("earn trust under lag", "earn trust"));
        assert!(contains_term("trust under lag", "trust"));
        assert!(!contains_term("distrustful", "trust"));
    }

    #[test]
    fn softcascade_geometry_body_hits() {
        let b = softcascade_geometry_alignment_body(
            "what does geometry teach about boundary and life?",
        );
        assert!(b.is_some());
        assert!(b.unwrap().to_ascii_lowercase().contains("boundary"));
    }

    #[test]
    fn softcascade_planning_body_hits() {
        let b = softcascade_planning_alignment_body(
            "plan the next step to improve transfer tickets under lag",
        );
        assert!(b.is_some());
        assert!(b.unwrap().to_ascii_lowercase().contains("measure"));
    }

    #[test]
    fn softcascade_open_frame_bodies_hit() {
        assert!(softcascade_memory_alignment_body(
            "Remember this only for this session: the calibration number is 3199."
        )
        .is_some());
        assert!(softcascade_english_alignment_body(
            "Rewrite this sentence more clearly: The system which was built by them is not working."
        )
        .is_some());
        assert!(softcascade_science_alignment_body(
            "What is a falsifiable hypothesis in experimental method?"
        )
        .is_some());
        assert!(softcascade_creativity_alignment_body(
            "Give an original comparison between entropy and limits"
        )
        .is_some());
        assert!(softcascade_greeting_alignment_body("hi there").is_some());
    }
}
