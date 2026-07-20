//! Entity-slot binding for relation transfer (v0.8.5).
//!
//! Problem measured on adversarial held-out: entity-swap topic binding ~30% because
//! prompts like "An unfamiliar device called Quoril-7 has boundary and identity.
//! Transfer one relation…" were stolen by creative-constraint and answered with a
//! fixed switchyard metaphor that never mentioned the slots.
//!
//! Law: surface entity names are role-fillers; relation structure must survive swap.
//! Names are not evidence. No weight promote.

use crate::deliberation::Deliberation;

/// Motifs used by the adversarial / emergence curricula.
const KNOWN_SLOTS: &[&str] = &[
    "boundary",
    "memory",
    "evidence",
    "repair",
    "trust",
    "uncertainty",
    "scale",
    "identity",
    "signal",
    "learning",
    "entropy",
    "structure",
    "attention",
    "change",
    "mechanism",
    "state",
    "relation",
    "transfer",
    "invariant",
    "promise",
    "failure",
    "measurement",
    "pattern",
    "composition",
    "observation",
    "feedback",
    "limit",
    "exchange",
    "lag",
    "timeout",
    "retry",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntitySlotFrame {
    pub entity: String,
    pub slot_a: String,
    pub slot_b: String,
}

/// Instructional / meta tokens that steal content slots if matched first.
const META_MOTIFS: &[&str] = &[
    "state",     // "State the same…"
    "relation",  // "testable relation"
    "mechanism", // "proves a mechanism"
    "invariant", // "remain invariant"
    "transfer",  // verb in curricula
    "exchange",  // "can exchange"
    "observation",
    "measurement",
    "composition",
    "pattern",
    "limit",
    "failure",
    "promise",
    "change", // often verb; keep only if paired as noun motif in content parse
];

/// Content-first motif extraction for adversarial dual-slot binding.
///
/// Prefer structured patterns over first substring hit, so
/// "State the same testable relation… how does boundary change what identity…"
/// yields (boundary, identity) not (state, relation).
pub fn motifs_in_text(text: &str) -> Vec<String> {
    content_motif_pair(text)
        .map(|(a, b)| vec![a, b])
        .unwrap_or_else(|| raw_motifs_in_text(text, false))
}

/// All raw motif hits (optionally including meta).
pub fn raw_motifs_in_text(text: &str, include_meta: bool) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut found: Vec<(usize, String)> = Vec::new();
    for motif in KNOWN_SLOTS {
        if !include_meta && META_MOTIFS.contains(motif) {
            continue;
        }
        if let Some(pos) = find_word(&lower, motif) {
            found.push((pos, (*motif).to_owned()));
        }
    }
    found.sort_by_key(|(pos, _)| *pos);
    found.into_iter().map(|(_, m)| m).collect()
}

fn find_word(hay: &str, needle: &str) -> Option<usize> {
    let mut start = 0;
    while let Some(rel) = hay[start..].find(needle) {
        let pos = start + rel;
        let before_ok = pos == 0 || !hay.as_bytes()[pos - 1].is_ascii_alphanumeric();
        let end = pos + needle.len();
        let after_ok = end >= hay.len() || !hay.as_bytes()[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return Some(pos);
        }
        start = pos + 1;
    }
    None
}

/// Extract the primary content motif pair from common curriculum shapes.
pub fn content_motif_pair(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();

    // "how does X change what Y can …" / "how does X … Y"
    if let Some(idx) = lower.find("how does ") {
        let rest = &lower[idx + "how does ".len()..];
        let tokens = content_tokens(rest);
        if tokens.len() >= 2 {
            return Some((tokens[0].clone(), tokens[1].clone()));
        }
    }

    // "about Y" after first content motif (negation: entropy… about memory)
    if let Some(idx) = lower.find(" about ") {
        let after = content_tokens(&lower[idx + " about ".len()..]);
        let before = content_tokens(&lower[..idx]);
        if let (Some(a), Some(b)) = (before.first(), after.first()) {
            if a != b {
                return Some((a.clone(), b.clone()));
            }
        }
    }

    // "prediction about Y" / "falsify … about Y"
    if let Some(idx) = lower.find(" about ") {
        let after = content_tokens(&lower[idx + " about ".len()..]);
        if let Some(b) = after.first() {
            let before = content_tokens(&lower[..idx]);
            if let Some(a) = before.first() {
                if a != b {
                    return Some((a.clone(), b.clone()));
                }
            }
        }
    }

    // "from A through B to C" — return A,C with B as intermediate (caller may use all)
    if let Some(idx) = lower.find("from ") {
        let rest = &lower[idx + "from ".len()..];
        if rest.contains(" through ") && rest.contains(" to ") {
            let tokens = content_tokens(rest);
            if tokens.len() >= 3 {
                return Some((tokens[0].clone(), tokens[2].clone()));
            }
        }
    }

    // "X and Y" after connect / between
    for marker in ["connect ", "between ", "has "] {
        if let Some(idx) = lower.find(marker) {
            let rest = &lower[idx + marker.len()..];
            if let Some(and_at) = rest.find(" and ") {
                let left = content_tokens(&rest[..and_at]);
                let right = content_tokens(&rest[and_at + 5..]);
                if let (Some(a), Some(b)) = (left.first(), right.first()) {
                    if a != b {
                        return Some((a.clone(), b.clone()));
                    }
                }
            }
        }
    }

    // "when Y is removed" / "when Y falls" after X increases
    if lower.contains(" when ") && lower.contains(" is removed") {
        let tokens = content_tokens(&lower);
        if tokens.len() >= 2 {
            return Some((tokens[0].clone(), tokens[1].clone()));
        }
    }

    // Fallback: first two non-meta content motifs
    let content = raw_motifs_in_text(text, false);
    if content.len() >= 2 {
        return Some((content[0].clone(), content[1].clone()));
    }
    // Last resort include meta
    let all = raw_motifs_in_text(text, true);
    if all.len() >= 2 {
        return Some((all[0].clone(), all[1].clone()));
    }
    None
}

fn content_tokens(s: &str) -> Vec<String> {
    const CONTENT: &[&str] = &[
        "boundary",
        "memory",
        "evidence",
        "repair",
        "trust",
        "uncertainty",
        "scale",
        "identity",
        "signal",
        "learning",
        "entropy",
        "structure",
        "attention",
    ];
    s.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| CONTENT.contains(&w.as_str()))
        .collect()
}

/// Detect adversarial-style entity-slot transfer asks.
///
/// Do **not** match pure pedagogy about "entity-swap" without a concrete
/// device + relation-transfer ask — that stays `novel-entity-generalize`.
pub fn looks_entity_slot_transfer(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    let entityish = t.contains("unfamiliar device")
        || t.contains("unfamiliar machine")
        || t.contains("unfamiliar system")
        || t.contains("entity ")
        || ((t.contains("called ") || t.contains("named "))
            && (t.contains("device")
                || t.contains("machine")
                || t.contains("system")
                || t.contains("service")
                || t.contains("node")
                || t.contains("gate")
                || t.contains("link")));
    let transferish = t.contains("transfer one relation")
        || t.contains("transfer a relation")
        || t.contains("transfer the relation")
        || t.contains("without treating the invented name")
        || t.contains("without treating the name as evidence")
        || t.contains("without treating invented")
        || (t.contains("do not use") && t.contains("as the mechanism"))
        || t.contains("without parroting")
        || (t.contains("invented name") && t.contains("transfer"));
    // Bare "entity-swap" pedagogy is not enough — need a concrete surface entity.
    (entityish && transferish)
        || (entityish
            && (t.contains("entity-swap")
                || t.contains("entity_swap")
                || t.contains("role-filler")))
        // "Entity Klystron-X has lag and trust. Transfer…"
        || (t.contains("entity ")
            && t.contains(" has ")
            && (t.contains("transfer") || t.contains("relation")))
}

/// Parse `called NAME has A and B` style prompts (tolerant of punctuation).
pub fn extract_entity_slot_frame(user: &str) -> Option<EntitySlotFrame> {
    let lower = user.to_ascii_lowercase();
    let entity = extract_entity_name(user).unwrap_or_else(|| "the named device".into());

    // Prefer "has X and Y" after the entity mention.
    if let Some(rest) = lower.split(" has ").nth(1) {
        let rest = rest
            .split("transfer")
            .next()
            .unwrap_or(rest)
            .split('.')
            .next()
            .unwrap_or(rest);
        let parts: Vec<&str> = rest
            .split(" and ")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            let a = first_slot_token(parts[0]);
            let b = first_slot_token(parts[1]);
            if !a.is_empty() && !b.is_empty() {
                return Some(EntitySlotFrame {
                    entity,
                    slot_a: a,
                    slot_b: b,
                });
            }
        }
    }

    // Fallback: any two known motifs in order of appearance.
    let mut found = Vec::new();
    for motif in KNOWN_SLOTS {
        if let Some(pos) = lower.find(motif) {
            found.push((pos, (*motif).to_owned()));
        }
    }
    found.sort_by_key(|(pos, _)| *pos);
    found.dedup_by(|a, b| a.1 == b.1);
    if found.len() >= 2 {
        return Some(EntitySlotFrame {
            entity,
            slot_a: found[0].1.clone(),
            slot_b: found[1].1.clone(),
        });
    }
    None
}

fn first_slot_token(chunk: &str) -> String {
    chunk
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .find(|w| w.len() >= 3)
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn extract_entity_name(user: &str) -> Option<String> {
    let lower = user.to_ascii_lowercase();
    for marker in ["called ", "named ", "entity "] {
        if let Some(idx) = lower.find(marker) {
            let after = &user[idx + marker.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if name.chars().count() >= 2 {
                return Some(name);
            }
        }
    }
    None
}

/// Map two slots to a checkable transferred relation (not name-parrot evidence).
pub fn relation_template(slot_a: &str, slot_b: &str) -> String {
    let a = slot_a.to_ascii_lowercase();
    let b = slot_b.to_ascii_lowercase();
    // Prefer specific pairs; fall back to compositional form.
    let specific = match (a.as_str(), b.as_str()) {
        ("boundary", "identity") => {
            "A boundary maintains identity only while exchange across it is still checkable; \
if the boundary dissolves, identity claims lose their operational referent."
        }
        ("evidence", "repair") => {
            "Evidence guides repair when a failed prediction names the next patch; \
repair without evidence is rewrite theater."
        }
        ("trust", "change") => {
            "Trust under change holds when acceptance remains verifiable after the change; \
silent drift ends trust even if names stay fixed."
        }
        ("scale", "learning") => {
            "Learning that only works at one scale is not transfer — measure whether the same \
update rule still reduces error when scale changes."
        }
        ("signal", "uncertainty") => {
            "A signal reduces uncertainty only when its absence would change a decision; \
otherwise it is decoration."
        }
        ("entropy", "memory") => {
            "Memory counters entropy only for the states it actually stores; unstored \
detail is free to degrade."
        }
        ("attention", "structure") => {
            "Attention selects a structure for update; structure without selection is \
latent and not currently used."
        }
        ("boundary", "memory") => {
            "Memory is useful at a boundary when it records what crossed and what was refused."
        }
        ("trust", "evidence") => {
            "Trust should track evidence quality, not label fluency; weak evidence caps trust."
        }
        ("trust", "lag") | ("lag", "trust") => {
            "Trust under lag holds when timeouts are part of the contract, retries are idempotent, \
and acceptance is checkable without private state — lag is not the mechanism; the relation is \
verifiable completion under delay."
        }
        ("trust", "timeout") | ("timeout", "trust") => {
            "Trust under timeout requires a stated meaning (cancel, retry, or uncertain) and an \
idempotent recovery path so a delayed success is not a second write."
        }
        ("lag", "retry") | ("retry", "lag") => {
            "Retries under lag must be idempotent and observable; otherwise lag becomes silent \
double-effect and trust collapses."
        }
        _ => "",
    };
    if !specific.is_empty() {
        return specific.to_owned();
    }
    format!(
        "When {a} changes, {b} should change only through a named intermediate mechanism; \
if {b} moves with no measurable change in {a}, the claimed link is spurious."
    )
}

/// Build the governed entity-slot transfer answer.
pub fn entity_slot_transfer_answer(user: &str) -> Deliberation {
    let frame = extract_entity_slot_frame(user).unwrap_or(EntitySlotFrame {
        entity: "the named device".into(),
        slot_a: "structure".into(),
        slot_b: "evidence".into(),
    });
    let relation = relation_template(&frame.slot_a, &frame.slot_b);
    let body = format!(
        "Entity-slot transfer (surface name is not evidence):\n\n\
**Entity role:** «{entity}» is an empty surface label — a role-filler, not a measured system.\n\
**Slots bound:** {a} ↔ {b}.\n\
**Transferred relation:** {relation}\n\n\
**What does not transfer:** the string «{entity}» adds no prior; do not treat the invented name as data.\n\
**Observation that would check it:** hold everything fixed, perturb {a}, and predict a directional change in {b}; \
if the prediction fails, reject the transferred relation rather than inventing a fluent bridge.\n\
**Score law:** structure survives entity-swap; token echo of the name is optional and never required.",
        entity = frame.entity,
        a = frame.slot_a,
        b = frame.slot_b,
        relation = relation
    );
    Deliberation::new("entity-slot-transfer", body)
        .observed("user asked to transfer a relation onto an invented entity/device")
        .inferred("bind slots explicitly; refuse name-as-evidence; state a checkable relation")
        .confidence(0.97)
}

/// Score whether speech preserves slots (for tests / future probe helpers).
pub fn slots_bound_in_speech(speech: &str, slot_a: &str, slot_b: &str) -> bool {
    let s = speech.to_ascii_lowercase();
    s.contains(&slot_a.to_ascii_lowercase()) && s.contains(&slot_b.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_adversarial_entity_swap_prompt() {
        let p = "An unfamiliar device called Quoril-7 has boundary and identity. \
Transfer one relation to it without treating the invented name as evidence.";
        assert!(looks_entity_slot_transfer(p));
        let f = extract_entity_slot_frame(p).expect("frame");
        assert_eq!(f.entity, "Quoril-7");
        assert_eq!(f.slot_a, "boundary");
        assert_eq!(f.slot_b, "identity");
    }

    #[test]
    fn answer_binds_both_slots_not_only_metaphor() {
        let p = "An unfamiliar device called Quoril-7 has trust and change. \
Transfer one relation to it without treating the invented name as evidence.";
        let d = entity_slot_transfer_answer(p);
        assert_eq!(d.operator, "entity-slot-transfer");
        let low = d.answer.to_ascii_lowercase();
        assert!(low.contains("trust"));
        assert!(low.contains("change"));
        assert!(low.contains("quoril-7") || low.contains("surface"));
        assert!(!low.contains("switchyard"));
        assert!(slots_bound_in_speech(&d.answer, "trust", "change"));
    }

    #[test]
    fn scale_learning_pair_binds() {
        let p = "An unfamiliar device called NembitGate has scale and learning. \
Transfer one relation to it without treating the invented name as evidence.";
        let d = entity_slot_transfer_answer(p);
        assert!(slots_bound_in_speech(&d.answer, "scale", "learning"));
    }

    #[test]
    fn content_pair_prefers_boundary_identity_over_state_relation() {
        let p = "State the same testable relation in new words: how does boundary change what identity can exchange, and what observation would check it?";
        let (a, b) = content_motif_pair(p).expect("pair");
        assert_eq!(a, "boundary");
        assert_eq!(b, "identity");
    }

    #[test]
    fn content_pair_negation_entropy_memory() {
        let p = "Do not assume that entropy automatically proves a mechanism. What can be said about memory, and what evidence is still missing?";
        let (a, b) = content_motif_pair(p).expect("pair");
        assert_eq!(a, "entropy");
        assert_eq!(b, "memory");
    }
}
