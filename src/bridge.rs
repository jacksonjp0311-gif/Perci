//! SoftCascade bridge — LLM-like multi-hypothesis answers without transformer latency.
//!
//! # Math (transformer jobs without matmuls)
//!
//! | Job | SoftCascade |
//! |-----|-------------|
//! | Multi-head attention | soft-α mixture + residual hops from Bitwork |
//! | Value projection | Willshaw concept insights + semantic frame lattice |
//! | Residual stream | hop-1/2 ANDNOT supports already on CognitiveMatch |
//! | Soft binding | VSA composition frame |
//! | Decode | structured compose (not token sampling) |
//!
//! All paths are integer/string work on already-classified evidence — target
//! warm path remains interactive (ms-scale after pack load).

use crate::cognitive::CognitiveMatch;
use crate::deliberation;

/// Evidence packet assembled for one soft-cascade reply.
#[derive(Debug, Clone)]
pub struct BridgePacket {
    /// Primary insight / lead claim.
    pub lead: Option<String>,
    /// Supporting facets ordered by attention (mixture then residual).
    pub supports: Vec<String>,
    /// Semantic-frame clauses activated from the open lattice.
    pub frames: Vec<String>,
    /// Mechanism lines from activated frames (when distinct).
    pub mechanisms: Vec<String>,
    /// Whether evidence is rich enough to replace stock domain_body prose.
    pub rich: bool,
    /// Contested geometry (low margin) → force multi-facet voice.
    pub contested: bool,
    /// Telemetry.
    pub attention_primary_pm: u16,
    pub mixture_n: usize,
    pub residual_n: usize,
    pub frame_n: usize,
}

/// Build the soft-cascade packet from a Bitwork match + user text.
pub fn assemble(matched: &CognitiveMatch, user: &str) -> BridgePacket {
    let mut supports: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    let push = |out: &mut Vec<String>, seen: &mut Vec<String>, s: &str| {
        let t = s.trim();
        if t.chars().count() < 16 || t.chars().count() > 180 {
            return;
        }
        let low = t.to_ascii_lowercase();
        if seen.iter().any(|e| e == &low || e.contains(&low[..low.len().min(36)])) {
            return;
        }
        // Suppress stock method cards.
        if low.contains("list premises")
            || low.contains("compare on capability")
            || low.contains("fake certainty")
            || low.contains("objective, constraints")
        {
            return;
        }
        seen.push(low);
        out.push(t.to_owned());
    };

    let lead = matched.insight.as_ref().and_then(|i| {
        let t = i.trim();
        if t.chars().count() >= 16 && t.chars().count() <= 200 {
            Some(t.to_owned())
        } else {
            None
        }
    });
    if let Some(ref l) = lead {
        seen.push(l.to_ascii_lowercase());
    }

    let multi = looks_multi_domain_user(user);
    let user_tokens = content_tokens_bridge(user);

    // Attention-ordered non-residual mixture first — filter cross-domain contamination.
    let mut mix: Vec<_> = matched
        .mixture
        .iter()
        .filter(|m| !m.residual)
        .filter(|m| support_is_relevant(user, matched.label.as_str(), m, multi, &user_tokens))
        .collect();
    mix.sort_by_key(|m| std::cmp::Reverse(m.attention_pm));
    for m in mix {
        if let Some(ref i) = m.insight {
            push(&mut supports, &mut seen, i);
        }
        if supports.len() >= 3 {
            break;
        }
    }

    // Residual stream (hop order) — same relevance gate.
    let mut res: Vec<_> = matched
        .mixture
        .iter()
        .filter(|m| m.residual)
        .filter(|m| support_is_relevant(user, matched.label.as_str(), m, multi, &user_tokens))
        .collect();
    res.sort_by_key(|m| (m.hop, std::cmp::Reverse(m.attention_pm)));
    for m in res {
        if let Some(ref i) = m.insight {
            push(&mut supports, &mut seen, i);
        }
        if supports.len() >= 4 {
            break;
        }
    }

    // Semantic frame lattice — only frames that touch user tokens (or multi-domain).
    let activated = deliberation::activate_semantic_frames(user, 3);
    let mut frames = Vec::new();
    let mut mechanisms = Vec::new();
    for f in activated {
        if !multi && !frame_touches_user(&f.clause, &user_tokens) && f.score < 40 {
            continue;
        }
        push(&mut frames, &mut seen, &f.clause);
        if f.mechanism.chars().count() >= 20 {
            let mlow = f.mechanism.to_ascii_lowercase();
            if !seen.iter().any(|e| e.contains(&mlow[..mlow.len().min(30)])) {
                mechanisms.push(f.mechanism);
            }
        }
    }

    let residual_n = matched.mixture.iter().filter(|m| m.residual).count();
    let frame_n = frames.len();
    let contested = matched.margin < 16 || residual_n > 0 || frame_n >= 2;
    let rich = lead.is_some()
        || !supports.is_empty()
        || frame_n >= 2
        || (matched.primary_attention_pm >= 400 && !matched.mixture.is_empty());

    BridgePacket {
        lead,
        supports,
        frames,
        mechanisms,
        rich,
        contested,
        attention_primary_pm: matched.primary_attention_pm,
        mixture_n: matched.mixture.iter().filter(|m| !m.residual).count(),
        residual_n,
        frame_n,
    }
}

/// Compose a multi-hypothesis answer as **continuous reasoning prose**.
///
/// Avoids labeled presets ("Lattice:", "Mixture read:", "Bound as…"). Facets are
/// woven with causal/contrast transitions so it reads like thinking, not a card dump.
pub fn compose_soft_cascade(
    user: &str,
    matched: &CognitiveMatch,
    domain_body: &str,
    variant: usize,
) -> String {
    let packet = assemble(matched, user);
    let tokens = content_tokens_bridge(user);
    let topic = if tokens.is_empty() {
        matched.label.clone()
    } else {
        tokens.iter().take(4).cloned().collect::<Vec<_>>().join(" ")
    };
    let ask = ask_shape(user);

    if !packet.rich {
        return domain_body.to_owned();
    }

    // Collect claim sentences (strip trailing periods for rejoin).
    let mut claims: Vec<String> = Vec::new();
    let push_claim = |claims: &mut Vec<String>, s: &str| {
        let t = s.trim().trim_end_matches('.').trim();
        if t.len() < 12 {
            return;
        }
        let low = t.to_ascii_lowercase();
        if claims
            .iter()
            .any(|c| c.to_ascii_lowercase().contains(&low[..low.len().min(40)]))
        {
            return;
        }
        claims.push(t.to_owned());
    };

    if let Some(ref lead) = packet.lead {
        push_claim(&mut claims, lead);
    } else if domain_body.split_whitespace().count() >= 6 {
        push_claim(&mut claims, domain_body);
    }
    for s in &packet.supports {
        push_claim(&mut claims, s);
        if claims.len() >= 4 {
            break;
        }
    }
    for f in &packet.frames {
        push_claim(&mut claims, f);
        if claims.len() >= 5 {
            break;
        }
    }
    if packet.contested {
        if let Some(m) = packet.mechanisms.first() {
            push_claim(&mut claims, m);
        }
    }

    if claims.is_empty() {
        return domain_body.to_owned();
    }

    // Opening: answer the ask shape without "On topic:" cardboard.
    let mut out = String::new();
    let c0 = decapitalize_if_mid(&claims[0]);
    match (ask, variant % 4) {
        (AskShape::Why, 0) => {
            out.push_str(&format!("Because {c0}."));
        }
        (AskShape::Why, 1) => {
            out.push_str(&format!("It comes down to this: {}.", claims[0]));
        }
        (AskShape::Why, _) => {
            out.push_str(&format!(
                "The structural reason {topic} fails or frays is that {}.",
                c0
            ));
        }
        (AskShape::How, 0) => {
            out.push_str(&format!("Practically, {}.", c0));
        }
        (AskShape::How, 1) => {
            out.push_str(&format!("It happens when {}.", c0));
        }
        (AskShape::How, _) => {
            out.push_str(&format!("Step through it: {}.", claims[0]));
        }
        (AskShape::What, 0) => {
            out.push_str(&format!("{} is best read as {}.", topic, c0));
        }
        (AskShape::What, _) => {
            out.push_str(&format!("{}.", claims[0]));
        }
        (AskShape::Connect, _) => {
            out.push_str(&format!(
                "A workable bridge for {topic} starts here: {}.",
                claims[0]
            ));
        }
        (AskShape::Open, 0) => {
            out.push_str(&format!("{}.", claims[0]));
        }
        (AskShape::Open, _) => {
            out.push_str(&format!("On {topic}, {}.", c0));
        }
    }

    // Weave remaining claims with natural reasoning transitions (not section labels).
    let transitions = match variant % 5 {
        0 => ["That connects to ", "Which means ", "And under stress, "],
        1 => ["From another angle, ", "So in practice, ", "Put differently, "],
        2 => ["Alongside that, ", "The mechanism is that ", "If you push it, "],
        3 => ["Equally important: ", "This only holds when ", "Otherwise, "],
        _ => ["Zooming out, ", "The quiet constraint is that ", "You can check it by noting "],
    };

    for (i, claim) in claims.iter().skip(1).enumerate() {
        if i >= 3 {
            break;
        }
        let body = decapitalize_if_mid(claim);
        out.push(' ');
        out.push_str(transitions[i % transitions.len()]);
        out.push_str(&body);
        if !out.ends_with('.') {
            out.push('.');
        }
    }

    // Light epistemic honesty only when contested — plain language, no jargon dump.
    if packet.contested && claims.len() >= 3 && variant % 2 == 0 {
        out.push_str(" I'm holding more than one working frame here; the pieces above are the ones that still cohere.");
    }

    // Bind user topic if diluted.
    let ol = out.to_ascii_lowercase();
    let hit = tokens.iter().filter(|t| ol.contains(t.as_str())).count();
    if tokens.len() >= 2 && hit == 0 {
        out.push(' ');
        out.push_str(&format!(
            "All of that still answers {}.",
            tokens.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
        ));
    }
    out
}

#[derive(Clone, Copy)]
enum AskShape {
    Why,
    How,
    What,
    Connect,
    Open,
}

fn ask_shape(user: &str) -> AskShape {
    let l = user.to_ascii_lowercase();
    if l.contains("why ") || l.starts_with("why") || l.contains("reason for") {
        AskShape::Why
    } else if l.contains("how ") || l.starts_with("how") || l.contains("in what way") {
        AskShape::How
    } else if l.contains("connect ") || l.contains("relate ") || l.contains("relationship") {
        AskShape::Connect
    } else if l.contains("what is") || l.contains("what are") || l.contains("explain ") {
        AskShape::What
    } else {
        AskShape::Open
    }
}

fn decapitalize_if_mid(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) if first.is_uppercase() => {
            // Keep acronyms / short all-caps.
            if s.chars().take(3).all(|ch| ch.is_uppercase() || !ch.is_alphabetic()) {
                s.to_owned()
            } else {
                first.to_lowercase().collect::<String>() + c.as_str()
            }
        }
        _ => s.to_owned(),
    }
}

/// Prefer SoftCascade when Bitwork evidence is rich enough.
pub fn should_use_cascade(matched: &CognitiveMatch, user: &str) -> bool {
    let words = user.split_whitespace().count();
    if words < 3 {
        return false;
    }
    let social = user.to_ascii_lowercase();
    if matches!(
        social.trim(),
        "hi" | "hello" | "hey" | "thanks" | "thank you" | "bye" | "goodbye"
    ) {
        return false;
    }
    let packet = assemble(matched, user);
    packet.rich
        || matched.margin < 20
        || matched.mixture.len() >= 2
        || matched.composition.len() >= 3
}

fn content_tokens_bridge(user: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "if", "then", "than", "that", "this", "what",
        "when", "where", "which", "who", "why", "how", "can", "could", "would", "should",
        "will", "just", "really", "very", "your", "you", "me", "my", "our", "we", "i", "is",
        "are", "was", "were", "be", "been", "do", "does", "did", "to", "of", "in", "on", "for",
        "it", "its", "as", "at", "by", "not", "no", "please", "tell", "about", "with", "from",
    ];
    user.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|w| w.len() >= 4 && !STOP.contains(&w.as_str()))
        .take(8)
        .collect()
}

fn looks_multi_domain_user(user: &str) -> bool {
    let l = user.to_ascii_lowercase();
    l.contains("connect ")
        || l.contains(" vs ")
        || l.contains(" versus ")
        || l.contains("difference between")
        || l.contains(" and ")
            && l.split_whitespace().count() >= 6
}

fn looks_capability_user(user: &str) -> bool {
    let l = user.to_ascii_lowercase();
    l.contains("capable")
        || l.contains("what can you")
        || l.contains("what do you do")
        || l.contains("abilities")
        || l.contains("capabilities")
        || (l.contains("what are you") && l.contains("do"))
}

/// Drop mixture supports that would contaminate a low-margin primary domain.
fn support_is_relevant(
    user: &str,
    primary: &str,
    m: &crate::cognitive::MixtureSupport,
    multi: bool,
    user_tokens: &[String],
) -> bool {
    if multi || m.label == primary {
        return true;
    }
    // Capability asks: identity may legitimately support general.
    if looks_capability_user(user)
        && ((primary == "general" && m.label == "identity")
            || (primary == "identity" && m.label == "general"))
    {
        return true;
    }
    // Share a content token between user and insight.
    if let Some(ref insight) = m.insight {
        let il = insight.to_ascii_lowercase();
        if user_tokens.iter().any(|t| t.len() >= 4 && il.contains(t.as_str())) {
            return true;
        }
    }
    // High attention different-domain support needs stronger evidence.
    if m.attention_pm >= 250 && m.insight.is_some() {
        // Still reject biological/life-death noise on math-y prompts.
        let l = user.to_ascii_lowercase();
        let mathish = l.chars().any(|c| c.is_ascii_digit())
            || l.contains("equal")
            || l.contains("plus")
            || l.contains("calculate");
        if mathish {
            if let Some(ref i) = m.insight {
                let il = i.to_ascii_lowercase();
                if il.contains("death") || il.contains("organism") || il.contains("membrane") {
                    return false;
                }
            }
        }
        return m.score > 0 && m.overlap >= 6;
    }
    false
}

fn frame_touches_user(clause: &str, user_tokens: &[String]) -> bool {
    let cl = clause.to_ascii_lowercase();
    user_tokens
        .iter()
        .any(|t| t.len() >= 4 && cl.contains(t.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::{CognitiveMatch, MixtureSupport};

    fn sample_match() -> CognitiveMatch {
        CognitiveMatch {
            label: "systems".into(),
            variant: 1,
            concept_id: 1,
            insight: Some(
                "trust needs clear interfaces so authority and proof stay checkable".into(),
            ),
            score: 200,
            overlap: 40,
            runner_up_score: 180,
            margin: 8,
            query_popcount: 100,
            prototype_popcount: 120,
            positive_overlap: 30,
            negative_overlap: 5,
            hamming: 140,
            jaccard: 0.2,
            overlap_z: 2.0,
            mixture: vec![
                MixtureSupport {
                    label: "governance".into(),
                    score: 160,
                    overlap: 28,
                    concept_id: 2,
                    insight: Some(
                        "permission and proof are different gates under partial observability"
                            .into(),
                    ),
                    residual: false,
                    hop: 0,
                    attention_pm: 280,
                },
                MixtureSupport {
                    label: "memory".into(),
                    score: 90,
                    overlap: 12,
                    concept_id: 3,
                    insight: Some(
                        "memory reconstructs past state from stored traces under partial cues"
                            .into(),
                    ),
                    residual: true,
                    hop: 1,
                    attention_pm: 120,
                },
            ],
            composition: vec![
                "ask:why".into(),
                "agent:trust".into(),
                "domain:distributed".into(),
            ],
            primary_attention_pm: 400,
        }
    }

    #[test]
    fn assemble_marks_rich_when_insight_and_mixture() {
        let m = sample_match();
        let p = assemble(&m, "why does trust fail in distributed systems?");
        assert!(p.rich);
        assert!(p.lead.is_some());
        assert!(!p.supports.is_empty());
    }

    #[test]
    fn soft_cascade_binds_topic_and_avoids_empty() {
        let m = sample_match();
        let out = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "Give each piece one job.",
            0,
        );
        let low = out.to_ascii_lowercase();
        assert!(low.contains("trust") || low.contains("interface") || low.contains("permission"));
        assert!(out.split_whitespace().count() >= 12);
        assert!(!low.contains("list premises"));
        // No preset section labels.
        assert!(!low.contains("lattice:"));
        assert!(!low.contains("mixture read"));
        assert!(!low.contains("bound as"));
        assert!(!low.contains("multi-hypothesis readout"));
    }

    #[test]
    fn soft_cascade_why_opens_with_reason() {
        let m = sample_match();
        let out = compose_soft_cascade(
            "why does trust fail in distributed systems?",
            &m,
            "placeholder body",
            0,
        );
        let low = out.to_ascii_lowercase();
        assert!(
            low.starts_with("because")
                || low.contains("comes down")
                || low.contains("structural reason")
                || low.contains("trust"),
            "got: {out}"
        );
    }

    #[test]
    fn should_use_cascade_on_conceptual_multi_facet() {
        let m = sample_match();
        assert!(should_use_cascade(
            &m,
            "why does trust fail in distributed systems?"
        ));
        assert!(!should_use_cascade(&m, "hi"));
    }
}
