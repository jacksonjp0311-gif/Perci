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

    // Attention-ordered non-residual mixture first.
    let mut mix: Vec<_> = matched
        .mixture
        .iter()
        .filter(|m| !m.residual)
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

    // Residual stream (hop order).
    let mut res: Vec<_> = matched.mixture.iter().filter(|m| m.residual).collect();
    res.sort_by_key(|m| (m.hop, std::cmp::Reverse(m.attention_pm)));
    for m in res {
        if let Some(ref i) = m.insight {
            push(&mut supports, &mut seen, i);
        }
        if supports.len() >= 4 {
            break;
        }
    }

    // Semantic frame lattice (operator-side world model, not pack scan).
    let activated = deliberation::activate_semantic_frames(user, 3);
    let mut frames = Vec::new();
    let mut mechanisms = Vec::new();
    for f in activated {
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

/// Compose a multi-hypothesis answer (decoder substitute).
///
/// Keeps latency low: only string joins over already-scored evidence.
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

    // Thin evidence: fall back to domain body seed (caller still fluid-weaves).
    if !packet.rich {
        return domain_body.to_owned();
    }

    let mut parts: Vec<String> = Vec::new();

    // Lead claim.
    if let Some(ref lead) = packet.lead {
        match variant % 3 {
            0 => parts.push(format!("On {topic}: {lead}")),
            1 => parts.push(format!("{lead}")),
            _ => parts.push(format!("The short read on {topic}: {lead}")),
        }
    } else if !domain_body.is_empty() && domain_body.split_whitespace().count() >= 6 {
        parts.push(format!("On {topic}: {domain_body}"));
    }

    // Attention supports (transformer multi-head analog).
    if !packet.supports.is_empty() {
        let s0 = &packet.supports[0];
        if packet.supports.len() == 1 {
            parts.push(match variant % 2 {
                0 => format!("Also in view: {s0}"),
                _ => format!("A second facet: {s0}"),
            });
        } else {
            let s1 = &packet.supports[1];
            parts.push(match variant % 3 {
                0 => format!("Two nearby ideas fire with that: {s0} — and {s1}."),
                1 => format!("Mixture read: {s0}; {s1}."),
                _ => format!("Related frames: {s0}; {s1}."),
            });
            if packet.supports.len() >= 3 && packet.contested {
                parts.push(format!("A further residual angle: {}.", packet.supports[2]));
            }
        }
    }

    // Semantic lattice (world-model clauses not stored as prototypes).
    if !packet.frames.is_empty() {
        let f = packet.frames.join("; ");
        parts.push(format!("Lattice: {f}."));
        if let Some(mech) = packet.mechanisms.first() {
            if packet.contested || packet.frames.len() >= 2 {
                parts.push(format!("Mechanism boundary: {mech}."));
            }
        }
    }

    // Composition roles when structural.
    let comp = matched.composition_frame(3);
    if comp.len() >= 2
        && comp.iter().any(|c| {
            c.starts_with("ask:") || c.starts_with("domain:") || c.starts_with("agent:")
        })
    {
        let joined = comp.join(" · ");
        parts.push(format!("Bound as {joined}."));
    }

    // Honesty footer on contested multi-frame reads.
    if packet.contested && packet.mixture_n + packet.residual_n + packet.frame_n >= 2 {
        parts.push(
            "This is multi-hypothesis readout under partial geometry—not a single decoder claim."
                .to_owned(),
        );
    }

    let mut out = parts.join(" ");
    // Ensure user tokens stay bound.
    let ol = out.to_ascii_lowercase();
    let hit = tokens.iter().filter(|t| ol.contains(t.as_str())).count();
    if tokens.len() >= 2 && hit == 0 {
        out.push(' ');
        out.push_str(&format!(
            "That still centers on {}.",
            tokens.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
        ));
    }
    if !out.ends_with('.') && !out.ends_with('?') && !out.ends_with('!') {
        out.push('.');
    }
    out
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
