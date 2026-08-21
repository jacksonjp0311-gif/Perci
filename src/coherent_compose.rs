//! PERCICOMP1 — bounded compositional response planning.
//!
//! This layer joins pieces that previously operated independently:
//! dialogue state, question operation, Bitwork concept mixtures, discourse
//! roles, and the observer metric. It does not invent facts and it never
//! outranks exact tools or named operators. It proposes and ranks wording for
//! the open associative lane; the existing dialogue workspace remains the
//! final critic.

use crate::cognitive::CognitiveMatch;
use crate::dialogue_workspace::{DialogueWorkspace, QuestionOperation, WorkspaceAct};
use crate::thought_plan::Intent;
use std::collections::BTreeSet;

const MIN_CARD_WORDS: usize = 5;
const MAX_CARD_WORDS: usize = 34;

#[derive(Clone, Debug)]
pub struct CompositionCandidate {
    pub text: String,
    pub source: &'static str,
    pub score: f64,
    pub cards_used: usize,
    pub relation_terms: Vec<String>,
    pub relation_resonance: f64,
    pub critique_flags: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CardRole {
    Claim,
    Mechanism,
    Boundary,
    Action,
}

#[derive(Clone, Debug)]
struct SemanticCard {
    text: String,
    tokens: BTreeSet<String>,
    relevance: usize,
    role: CardRole,
}

/// Build a seed-bound answer from compatible Bitwork cards.
///
/// Returning `None` is important: weak or geometrically disconnected cards
/// must not be polished into an answer-shaped hallucination.
pub fn compose_from_match(
    user: &str,
    recent: &[(String, String)],
    matched: &CognitiveMatch,
) -> Option<CompositionCandidate> {
    let workspace = DialogueWorkspace::derive(user, recent);
    if !eligible(user, &workspace) {
        return None;
    }

    let user_tokens = content_tokens(user);
    if user_tokens.is_empty() {
        return None;
    }

    let mut raw = matched.concept_skeleton(5);
    raw.extend(matched.residual_skeleton(1));
    let label_tokens = content_tokens(&matched.label);
    let label_fit = !label_tokens.is_disjoint(&user_tokens);
    let mut cards = raw
        .into_iter()
        .filter_map(|text| SemanticCard::new(text, &user_tokens, label_fit))
        .collect::<Vec<_>>();
    cards.sort_by(|a, b| {
        b.relevance
            .cmp(&a.relevance)
            .then_with(|| role_rank(a.role).cmp(&role_rank(b.role)))
    });

    let primary = cards.first()?.clone();
    if primary.relevance == 0 {
        return None;
    }

    let explicit_synthesis = matches!(Intent::infer_from_prompt(user), Intent::Synthesis)
        || matches!(workspace.act, WorkspaceAct::Synthesize)
        || asks_for_relation(user);
    let secondary = cards
        .iter()
        .skip(1)
        .find(|card| compatible(&primary, card, &user_tokens, explicit_synthesis))
        .cloned();
    let boundary = cards
        .iter()
        .skip(1)
        .find(|card| {
            card.role == CardRole::Boundary
                && card.text != primary.text
                && secondary
                    .as_ref()
                    .map(|value| value.text != card.text)
                    .unwrap_or(true)
        })
        .cloned();

    // This is a compositional lane, not a renamed single-card retrieval lane.
    let secondary = secondary?;
    let relation_terms = shared_or_bridged_terms(&primary, &secondary, &user_tokens);
    if relation_terms.is_empty() {
        return None;
    }

    let text = realize(
        user,
        &workspace,
        &primary,
        &secondary,
        boundary.as_ref(),
        &relation_terms,
    );
    if text.split_whitespace().count() < 14 {
        return None;
    }

    Some(score_candidate(
        user,
        recent,
        text,
        "bitwork-composition",
        2 + usize::from(boundary.is_some()),
        relation_terms,
    ))
}

/// Rank alternate surface forms with the same observer contract used by the
/// dialogue workspace. This makes realization a small beam instead of a
/// first-acceptable-sentence decision.
pub fn select_best(
    user: &str,
    recent: &[(String, String)],
    candidates: impl IntoIterator<Item = (&'static str, String)>,
) -> Option<CompositionCandidate> {
    let relation_program = crate::relation_field::parse_program(user);
    let relation_active = !relation_program.edges.is_empty()
        && (relation_program.asks_counterfactual || relation_program.asks_invariant);
    candidates
        .into_iter()
        .filter(|(_, text)| !text.trim().is_empty())
        .map(|(source, text)| score_candidate(user, recent, text, source, 1, Vec::new()))
        .filter(|candidate| {
            !candidate.critique_flags.iter().any(|flag| {
                matches!(
                    *flag,
                    "semantic_subject_miss"
                        | "operation_fit_miss"
                        | "ood_without_abstention"
                        | "empty_turn"
                )
            })
        })
        .filter(|candidate| !relation_active || candidate.relation_resonance >= 0.45)
        .max_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

fn score_candidate(
    user: &str,
    recent: &[(String, String)],
    text: String,
    source: &'static str,
    cards_used: usize,
    relation_terms: Vec<String>,
) -> CompositionCandidate {
    let workspace = DialogueWorkspace::derive(user, recent);
    let context = crate::context_card::ContextCard::derive(user, recent);
    let critique = workspace.critique(user, &text, recent);
    let metrics = context.observe_answer(&text, recent);
    let user_tokens = content_tokens(user);
    let answer_tokens = content_tokens(&text);
    let subject_fit = overlap_ratio(&user_tokens, &answer_tokens);
    let relation_bonus = if cards_used >= 2 && !relation_terms.is_empty() {
        0.08
    } else {
        0.0
    };
    let relation_program = crate::relation_field::parse_program(user);
    let relation_active = !relation_program.edges.is_empty()
        && (relation_program.asks_counterfactual || relation_program.asks_invariant);
    let relation_resonance = if relation_active {
        crate::relation_field::score_response(user, &text).score
    } else {
        1.0
    };
    let repair_penalty = critique.flags.len() as f64 * 0.07;
    let score = (metrics.observer_score
        + 0.12 * subject_fit
        + relation_bonus
        + if relation_active {
            0.10 * relation_resonance
        } else {
            0.0
        }
        + if source == "bitwork-composition" {
            0.025
        } else {
            0.0
        }
        - repair_penalty)
        .clamp(0.0, 1.0);

    CompositionCandidate {
        text,
        source,
        score,
        cards_used,
        relation_terms,
        relation_resonance,
        critique_flags: critique.flags,
    }
}

impl SemanticCard {
    fn new(text: String, user_tokens: &BTreeSet<String>, label_fit: bool) -> Option<Self> {
        let text = clean_sentence(&text);
        let word_count = text.split_whitespace().count();
        if !(MIN_CARD_WORDS..=MAX_CARD_WORDS).contains(&word_count) || is_stock_shell(&text) {
            return None;
        }
        let tokens = content_tokens(&text);
        let lexical = tokens.intersection(user_tokens).count();
        let relevance = lexical + usize::from(label_fit && lexical > 0);
        Some(Self {
            role: card_role(&text),
            text,
            tokens,
            relevance,
        })
    }
}

fn realize(
    user: &str,
    workspace: &DialogueWorkspace,
    primary: &SemanticCard,
    secondary: &SemanticCard,
    boundary: Option<&SemanticCard>,
    relation_terms: &[String],
) -> String {
    let seed = stable_seed(user, workspace.prior_turns);
    let mut parts = vec![ensure_sentence(&primary.text)];
    let secondary_text = lower_first(&secondary.text);
    let relation = relation_terms
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" and ");

    let bridge = match (Intent::infer_from_prompt(user), seed % 3) {
        (Intent::Synthesis | Intent::Comparison, 0) => {
            format!("The connection runs through {relation}: {secondary_text}")
        }
        (Intent::Synthesis | Intent::Comparison, 1) => {
            format!("Seen together, {secondary_text}")
        }
        (Intent::Synthesis | Intent::Comparison, _) => {
            format!("What links them is {relation}; {secondary_text}")
        }
        (Intent::Plan, _) | (_, _) if secondary.role == CardRole::Action => {
            format!("The practical consequence is that {secondary_text}")
        }
        (Intent::CausalExplanation, 0) => {
            format!("The mechanism becomes clearer when {secondary_text}")
        }
        (Intent::CausalExplanation, _) => {
            format!("That holds alongside a second constraint: {secondary_text}")
        }
        _ => format!("A second part of the same picture is that {secondary_text}"),
    };
    parts.push(ensure_sentence(&bridge));

    if let Some(boundary) = boundary {
        if boundary.text != primary.text && boundary.text != secondary.text {
            parts.push(ensure_sentence(&format!(
                "The boundary is {}",
                lower_first(&boundary.text)
            )));
        }
    } else if matches!(
        Intent::infer_from_prompt(user),
        Intent::Synthesis | Intent::Comparison
    ) || matches!(
        workspace.question.operation,
        QuestionOperation::ScopedSuperlative
    ) {
        parts.push(
            "That is a structural connection; it does not make the underlying mechanisms identical."
                .to_owned(),
        );
    }

    match workspace.response_budget {
        crate::dialogue_workspace::ResponseBudget::Brief => {
            parts.truncate(2);
        }
        crate::dialogue_workspace::ResponseBudget::Balanced => {
            parts.truncate(3);
        }
        crate::dialogue_workspace::ResponseBudget::Deep => {}
    }
    parts.join(" ")
}

fn eligible(user: &str, workspace: &DialogueWorkspace) -> bool {
    let lower = crate::text_normalize::normalize_for_routing(user);
    let words = lower.split_whitespace().count();
    if words < 4 || matches!(workspace.act, WorkspaceAct::Social | WorkspaceAct::Learn) {
        return false;
    }
    if matches!(
        Intent::infer_from_prompt(user),
        Intent::Exact | Intent::Refuse
    ) {
        return false;
    }
    if lower.contains("password")
        || lower.contains("secret")
        || lower.contains("delete ")
        || lower.contains("push to github")
        || lower.contains("publish ")
    {
        return false;
    }
    lower.contains('?')
        || lower.starts_with("why ")
        || lower.starts_with("how ")
        || lower.starts_with("what ")
        || lower.starts_with("connect ")
        || lower.starts_with("compare ")
        || lower.starts_with("explain ")
        || lower.starts_with("analyze ")
}

fn compatible(
    first: &SemanticCard,
    second: &SemanticCard,
    user_tokens: &BTreeSet<String>,
    explicit_synthesis: bool,
) -> bool {
    if first.text == second.text || second.relevance == 0 {
        return false;
    }
    let shared = first.tokens.intersection(&second.tokens).count();
    if shared > 0 {
        return true;
    }
    if !explicit_synthesis {
        return false;
    }
    let first_user = first.tokens.intersection(user_tokens).count();
    let second_user = second.tokens.intersection(user_tokens).count();
    first_user > 0 && second_user > 0
}

fn shared_or_bridged_terms(
    first: &SemanticCard,
    second: &SemanticCard,
    user_tokens: &BTreeSet<String>,
) -> Vec<String> {
    let mut shared = first
        .tokens
        .intersection(&second.tokens)
        .cloned()
        .collect::<Vec<_>>();
    if shared.is_empty() {
        shared.extend(first.tokens.intersection(user_tokens).take(1).cloned());
        shared.extend(second.tokens.intersection(user_tokens).take(1).cloned());
    }
    shared.sort();
    shared.dedup();
    shared
}

fn card_role(text: &str) -> CardRole {
    let lower = text.to_ascii_lowercase();
    if [
        "cannot",
        "does not",
        "not ",
        "limit",
        "boundary",
        "without",
        "distinct",
        "only when",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        CardRole::Boundary
    } else if [
        "test",
        "measure",
        "check",
        "start",
        "build",
        "compare",
        "reproduce",
        "observe",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        CardRole::Action
    } else if [
        "because", "through", "process", "maintain", "causes", "enables", "changes", "depends",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        CardRole::Mechanism
    } else {
        CardRole::Claim
    }
}

fn role_rank(role: CardRole) -> usize {
    match role {
        CardRole::Claim => 0,
        CardRole::Mechanism => 1,
        CardRole::Action => 2,
        CardRole::Boundary => 3,
    }
}

fn asks_for_relation(user: &str) -> bool {
    let lower = user.to_ascii_lowercase();
    lower.contains("connect ")
        || lower.contains("relationship")
        || lower.contains("between ")
        || lower.contains("across ")
        || lower.contains("shared structure")
}

fn content_tokens(text: &str) -> BTreeSet<String> {
    const STOP: &[&str] = &[
        "about", "after", "again", "also", "and", "answer", "because", "before", "between",
        "could", "does", "from", "give", "have", "into", "just", "make", "more", "most", "only",
        "other", "over", "same", "should", "system", "that", "their", "there", "these", "they",
        "this", "through", "under", "very", "want", "what", "when", "where", "which", "while",
        "with", "without", "would", "your",
    ];
    crate::text_normalize::normalize_for_routing(text)
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .filter(|word| word.len() >= 4 && !STOP.contains(word))
        .map(stem)
        .collect()
}

fn stem(word: &str) -> String {
    for suffix in ["ing", "tion", "ions", "ment", "ness", "ed", "es", "s"] {
        if word.len() > suffix.len() + 4 && word.ends_with(suffix) {
            return word[..word.len() - suffix.len()].to_owned();
        }
    }
    word.to_owned()
}

fn overlap_ratio(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    a.intersection(b).count() as f64 / a.len() as f64
}

fn clean_sentence(text: &str) -> String {
    text.trim()
        .trim_start_matches(|c: char| matches!(c, '-' | '*' | '•'))
        .trim()
        .replace("  ", " ")
}

fn ensure_sentence(text: &str) -> String {
    let text = text.trim();
    if text.ends_with(['.', '!', '?']) {
        text.to_owned()
    } else {
        format!("{text}.")
    }
}

fn lower_first(text: &str) -> String {
    let text = text.trim().trim_end_matches('.');
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn is_stock_shell(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "what outcome do you want",
        "name the workload",
        "smallest next test",
        "i won't fake certainty",
        "list evidence already",
        "terms mean what they usually mean",
        "name one fact that would update",
    ]
    .iter()
    .any(|shell| lower.contains(shell))
}

fn stable_seed(user: &str, turn: usize) -> usize {
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ turn as u64;
    for byte in user.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive::{CognitiveMatch, MixtureSupport};

    fn fixture() -> CognitiveMatch {
        CognitiveMatch {
            label: "geometry".into(),
            variant: 0,
            concept_id: 0,
            insight: Some(
                "Geometry preserves relations under transformation, not merely isolated shapes."
                    .into(),
            ),
            score: 80,
            overlap: 40,
            runner_up_score: 70,
            margin: 10,
            query_popcount: 100,
            prototype_popcount: 100,
            positive_overlap: 40,
            negative_overlap: 0,
            hamming: 120,
            jaccard: 0.4,
            overlap_z: 2.0,
            mixture: vec![
                MixtureSupport {
                    label: "learning".into(),
                    score: 75,
                    overlap: 36,
                    concept_id: 1,
                    insight: Some(
                        "Learning preserves useful structure while changing its response to new evidence."
                            .into(),
                    ),
                    residual: false,
                    hop: 0,
                    attention_pm: 300,
                },
                MixtureSupport {
                    label: "logic".into(),
                    score: 68,
                    overlap: 31,
                    concept_id: 2,
                    insight: Some(
                        "The analogy is limited because geometric invariance and biological adaptation use different mechanisms."
                            .into(),
                    ),
                    residual: false,
                    hop: 0,
                    attention_pm: 180,
                },
            ],
            composition: vec![],
            primary_attention_pm: 520,
        }
    }

    #[test]
    fn composes_connected_cards_into_continuous_prose() {
        let out = compose_from_match(
            "Connect geometry and learning through preserved structure.",
            &[],
            &fixture(),
        )
        .expect("composition");
        let lower = out.text.to_ascii_lowercase();
        assert!(lower.contains("geometry"));
        assert!(lower.contains("learning"));
        assert!(lower.contains("structure"));
        assert!(out.cards_used >= 2);
        assert!(!out.text.contains("\n-"));
    }

    #[test]
    fn abstains_when_cards_do_not_share_or_bridge_user_subjects() {
        let mut matched = fixture();
        matched.insight = Some("Triangles have three sides and three vertices.".into());
        matched.mixture[0].insight =
            Some("A checksum detects accidental changes in a byte sequence.".into());
        assert!(
            compose_from_match("Why do promises matter in childhood?", &[], &matched).is_none()
        );
    }

    #[test]
    fn beam_rejects_operation_miss() {
        let candidates = [
            ("bad", "Triangles can be structurally rigid.".to_owned()),
            (
                "good",
                "Maybe, but starting a business depends on demand, downside, and runway. Test a reversible offer with real customers before making the full commitment.".to_owned(),
            ),
        ];
        let out = select_best("Should I start a business?", &[], candidates).expect("candidate");
        assert_eq!(out.source, "good");
    }
}
