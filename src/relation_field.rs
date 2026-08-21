//! PERCIREL2 — executable relational field and counterfactual propagation.
//!
//! The field is deliberately small and inspectable. It extracts explicit
//! subject–relation–object edges from a turn, applies a named intervention,
//! and reports which conclusions lose support and which supplied relations
//! remain invariant. It is not an open-world fact database and it does not
//! infer hidden causes.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    IsA,
    DependsOn,
    Requires,
    Supports,
    Enables,
    Causes,
    Prevents,
    Preserves,
    Constrains,
    Changes,
    Implies,
}

impl RelationKind {
    pub fn phrase(self) -> &'static str {
        match self {
            Self::IsA => "is",
            Self::DependsOn => "depends on",
            Self::Requires => "requires",
            Self::Supports => "supports",
            Self::Enables => "enables",
            Self::Causes => "causes",
            Self::Prevents => "prevents",
            Self::Preserves => "preserves",
            Self::Constrains => "constrains",
            Self::Changes => "changes",
            Self::Implies => "implies",
        }
    }

    fn loses_support_when_subject_changes(self) -> bool {
        matches!(
            self,
            Self::Supports
                | Self::Enables
                | Self::Causes
                | Self::Prevents
                | Self::Preserves
                | Self::Implies
        )
    }

    fn loses_support_when_object_changes(self) -> bool {
        matches!(self, Self::DependsOn | Self::Requires)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationEdge {
    pub subject: String,
    pub relation: RelationKind,
    pub object: String,
    pub negated: bool,
}

impl RelationEdge {
    pub fn proposition(&self) -> String {
        if self.negated {
            match self.relation {
                RelationKind::IsA => format!("{} is not {}", self.subject, self.object),
                _ => format!(
                    "{} {} {}",
                    self.subject,
                    negative_phrase(self.relation),
                    self.object
                ),
            }
        } else {
            format!(
                "{} {} {}",
                self.subject,
                self.relation.phrase(),
                self.object
            )
        }
    }

    fn touches(&self, target: &str) -> bool {
        phrase_overlap(&self.subject, target) || phrase_overlap(&self.object, target)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Intervention {
    pub target: String,
    pub change: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RelationProgram {
    pub edges: Vec<RelationEdge>,
    pub intervention: Option<Intervention>,
    pub asks_counterfactual: bool,
    pub asks_invariant: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationExecution {
    pub text: String,
    pub impacted: Vec<String>,
    pub invariant: Vec<String>,
    pub contradictions: Vec<String>,
    pub resonance: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RelationResonance {
    pub node_coverage: f64,
    pub edge_coverage: f64,
    pub operation_fit: f64,
    pub contradiction_tension: f64,
    pub score: f64,
}

/// Parse an explicit relational program. Ordinary descriptive prose returns an
/// empty or non-counterfactual program and does not gain execution authority.
pub fn parse_program(text: &str) -> RelationProgram {
    // Relation parsing must preserve punctuation: sentence boundaries separate
    // independent premises. Routing normalization intentionally removes that
    // punctuation and would fuse unrelated edges into one proposition.
    let lower = crate::text_normalize::repair_typos(text).to_ascii_lowercase();
    let asks_counterfactual = [
        "what changes",
        "what follows",
        "what happens",
        "would happen",
        "what survives",
        "what breaks",
        "what is affected",
        "which claims fail",
        "what can we conclude",
        "what no longer follows",
        "still follow",
        "does it follow",
        "counterfactual",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    let asks_invariant = lower.contains("invariant")
        || lower.contains("what remains")
        || lower.contains("what stays")
        || lower.contains("what survives");
    let intervention = parse_intervention(&lower);
    let edges = parse_edges(&lower);
    RelationProgram {
        edges,
        intervention,
        asks_counterfactual,
        asks_invariant,
    }
}

/// Execute only explicit counterfactual programs with at least one relation
/// and a named intervention. This is a closed-world calculation over supplied
/// premises, not a claim about the external world.
pub fn execute(text: &str) -> Option<RelationExecution> {
    execute_program(parse_program(text), text)
}

/// Continue a prior relational program when the new turn names the same
/// operation (what remains / what failed) or a fresh intervention without
/// restating the premises. Ordinary new questions still return `None`.
pub fn execute_followup(user: &str, prior_user: &str) -> Option<RelationExecution> {
    if !looks_relation_continuation(user) {
        return None;
    }
    let prior = parse_program(prior_user);
    if prior.edges.is_empty() || !prior.asks_counterfactual {
        return None;
    }
    let current = parse_program(user);
    if current.edges.is_empty() {
        if let Some(intervention) = current.intervention.or_else(|| parse_intervention(&user.to_ascii_lowercase())) {
            return execute_program(
                RelationProgram {
                    edges: prior.edges,
                    intervention: Some(intervention),
                    asks_counterfactual: true,
                    asks_invariant: current.asks_invariant || prior.asks_invariant,
                },
                user,
            );
        }
        return execute_program(prior, prior_user);
    }
    None
}

pub fn looks_relation_continuation(user: &str) -> bool {
    let lower = user.to_ascii_lowercase();
    if lower.split_whitespace().count() > 14
        || lower.contains("how do ")
        || lower.contains("relationship between")
        || lower.contains("interact")
    {
        return false;
    }
    [
        "what still holds",
        "what remains",
        "what remained",
        "what failed",
        "what stays",
        "what survived",
        "and if ",
        "if that fails",
        "what was invariant",
        "which claims still",
        "what still follows",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn execute_program(program: RelationProgram, text: &str) -> Option<RelationExecution> {
    if !program.asks_counterfactual || program.edges.is_empty() {
        return None;
    }
    let intervention = program.intervention.as_ref()?;
    let contradictions = contradictions(&program.edges);
    let affected = propagate_affected(&program.edges, &intervention.target);
    let mut impacted = Vec::new();
    let mut invariant = Vec::new();

    for edge in &program.edges {
        if edge_touches_any(edge, &affected) {
            impacted.push(edge.proposition());
        } else {
            invariant.push(edge.proposition());
        }
    }
    if impacted.is_empty() {
        let answer = format!(
            "The intervention on {} touches no supplied relation, so no change follows from the stated premises. The supplied relation{} remain{} invariant: {}. A causal bridge would have to be stated or evidenced before propagation is justified.",
            intervention.target,
            if program.edges.len() == 1 { "" } else { "s" },
            if program.edges.len() == 1 { "s" } else { "" },
            program
                .edges
                .iter()
                .map(RelationEdge::proposition)
                .collect::<Vec<_>>()
                .join("; ")
        );
        let resonance = score_response(text, &answer).score;
        return Some(RelationExecution {
            text: answer,
            impacted,
            invariant,
            contradictions,
            resonance,
        });
    }

    let primary = program
        .edges
        .iter()
        .find(|edge| edge.touches(&intervention.target))?;
    let target_is_subject = phrase_overlap(&primary.subject, &intervention.target);
    let consequence = if target_is_subject && primary.relation.loses_support_when_subject_changes()
    {
        format!(
            "the claim that {} {} {} no longer follows from that pathway",
            primary.subject,
            primary.relation.phrase(),
            primary.object
        )
    } else if !target_is_subject && primary.relation.loses_support_when_object_changes() {
        format!(
            "{} loses a stated dependency, so its prior conclusion is no longer established",
            primary.subject
        )
    } else {
        format!(
            "the supplied relation “{}” must be re-evaluated",
            primary.proposition()
        )
    };

    let mut parts = vec![format!(
        "Changing {} by {} removes support only along relations that touch it; unrelated premises remain intact.",
        intervention.target, intervention.change
    )];
    parts.push(format!("Given “{},” {consequence}.", primary.proposition()));
    let propagated = affected
        .iter()
        .filter(|node| !phrase_overlap(node, &intervention.target))
        .cloned()
        .collect::<Vec<_>>();
    if !propagated.is_empty() {
        parts.push(format!(
            "The effect propagates through the declared relation field to {}; it does not jump to unconnected nodes.",
            propagated.join(", ")
        ));
    }
    if program.asks_invariant {
        if let Some(edge) = program
            .edges
            .iter()
            .find(|edge| !edge_touches_any(edge, &affected))
        {
            parts.push(format!(
                "What remains invariant from the supplied premises is “{}.”",
                edge.proposition()
            ));
        } else {
            parts.push(
                "No independent invariant was supplied; anything beyond the affected relation would be an assumption."
                    .to_owned(),
            );
        }
    }
    if !contradictions.is_empty() {
        parts.push(format!(
            "There is also unresolved tension: {}.",
            contradictions[0]
        ));
    }
    parts.push(
        "This is a local counterfactual over the stated relations, not evidence that the same causal structure holds outside the prompt."
            .to_owned(),
    );
    let answer = parts.join(" ");
    let resonance = score_response(text, &answer).score;
    Some(RelationExecution {
        text: answer,
        impacted,
        invariant,
        contradictions,
        resonance,
    })
}

/// Measure whether an answer carries the prompt's named nodes, typed edges,
/// and requested counterfactual operation. "Resonance" here is an engineering
/// consistency score, not a biological frequency or mental state.
pub fn score_response(user: &str, answer: &str) -> RelationResonance {
    let program = parse_program(user);
    if program.edges.is_empty() {
        return RelationResonance {
            node_coverage: 1.0,
            edge_coverage: 1.0,
            operation_fit: 1.0,
            contradiction_tension: 0.0,
            score: 1.0,
        };
    }
    let lower = answer.to_ascii_lowercase();
    let nodes = program
        .edges
        .iter()
        .flat_map(|edge| [edge.subject.as_str(), edge.object.as_str()])
        .collect::<BTreeSet<_>>();
    let node_hits = nodes
        .iter()
        .filter(|node| phrase_overlap(&lower, node))
        .count();
    let node_coverage = ratio(node_hits, nodes.len());
    let edge_hits = program
        .edges
        .iter()
        .filter(|edge| lower.contains(edge.relation.phrase()))
        .count();
    let edge_coverage = ratio(edge_hits, program.edges.len());
    let operation_fit = if program.asks_counterfactual {
        if [
            "change",
            "no longer",
            "still",
            "remain",
            "invariant",
            "follow",
            "support",
            "re-evaluat",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            1.0
        } else {
            0.2
        }
    } else {
        1.0
    };
    let answer_edges = parse_edges(answer);
    let contradiction_count = contradictions(&answer_edges).len();
    let contradiction_tension = (contradiction_count as f64 * 0.25).min(1.0);
    let score = harmonic_mean(&[node_coverage, edge_coverage, operation_fit])
        * (-contradiction_tension).exp();
    RelationResonance {
        node_coverage,
        edge_coverage,
        operation_fit,
        contradiction_tension,
        score,
    }
}

fn parse_edges(text: &str) -> Vec<RelationEdge> {
    let mut edges = Vec::new();
    let normalized = text.replace(';', ".").replace(" but ", ". ");
    for clause in normalized.split(['.', '?', '!']) {
        let clause = clause.trim();
        if clause.is_empty() {
            continue;
        }
        if let Some(edge) = parse_without_clause(clause) {
            push_unique_edge(&mut edges, edge);
            continue;
        }
        if let Some(edge) = parse_enablement_clause(clause) {
            push_unique_edge(&mut edges, edge);
            continue;
        }
        for cue in relation_cues() {
            let Some(index) = clause.find(cue.phrase) else {
                continue;
            };
            // "what changes?" names the requested operation, not a factual
            // Changes edge. Treating it as a premise creates a self-referential
            // graph edge from the intervention clause.
            if cue.relation == RelationKind::Changes
                && clause[..index]
                    .split_whitespace()
                    .any(|word| word == "what" || word == "which")
            {
                continue;
            }
            let left = clean_side(&clause[..index], true);
            let right = head_noun(&clean_side(&clause[index + cue.phrase.len()..], false));
            if cue.relation == RelationKind::IsA && is_status_word(&right) {
                continue;
            }
            if valid_side(&left) && valid_side(&right) && left != right {
                let (subject, object) = if cue.flip {
                    (right, left)
                } else {
                    (left, right)
                };
                let edge = RelationEdge {
                    subject,
                    relation: cue.relation,
                    object,
                    negated: cue.negated,
                };
                push_unique_edge(&mut edges, edge);
                break;
            }
        }
    }
    edges
}

struct RelationCue {
    phrase: &'static str,
    relation: RelationKind,
    negated: bool,
    flip: bool,
}

fn relation_cues() -> Vec<RelationCue> {
    vec![
        cue(" does not rely on ", RelationKind::DependsOn, true, false),
        cue(" does not need ", RelationKind::Requires, true, false),
        cue(" does not allow ", RelationKind::Enables, true, false),
        cue(" does not lead to ", RelationKind::Causes, true, false),
        cue(" does not block ", RelationKind::Prevents, true, false),
        cue(" does not protect ", RelationKind::Preserves, true, false),
        cue(" does not imply ", RelationKind::Implies, true, false),
        cue(" does not cause ", RelationKind::Causes, true, false),
        cue(" doesn't rely on ", RelationKind::DependsOn, true, false),
        cue(" doesn't need ", RelationKind::Requires, true, false),
        cue(" doesn't allow ", RelationKind::Enables, true, false),
        cue(" doesn't lead to ", RelationKind::Causes, true, false),
        cue(" doesn't block ", RelationKind::Prevents, true, false),
        cue(" doesn't protect ", RelationKind::Preserves, true, false),
        cue(" doesn't imply ", RelationKind::Implies, true, false),
        cue(" doesn't cause ", RelationKind::Causes, true, false),
        cue(" is not ", RelationKind::IsA, true, false),
        cue(" is required for ", RelationKind::Requires, false, true),
        cue(" is needed for ", RelationKind::Requires, false, true),
        cue(" is preserved by ", RelationKind::Preserves, false, true),
        cue(" is kept by ", RelationKind::Preserves, false, true),
        cue(" is grounded in ", RelationKind::DependsOn, false, false),
        cue(" is based on ", RelationKind::DependsOn, false, false),
        cue(" hinges on ", RelationKind::DependsOn, false, false),
        cue(" rests on ", RelationKind::DependsOn, false, false),
        cue(" depends on ", RelationKind::DependsOn, false, false),
        cue(" relies on ", RelationKind::DependsOn, false, false),
        cue(" requires ", RelationKind::Requires, false, false),
        cue(" needs ", RelationKind::Requires, false, false),
        cue(" preserves ", RelationKind::Preserves, false, false),
        cue(" protects ", RelationKind::Preserves, false, false),
        cue(" keeps ", RelationKind::Preserves, false, false),
        cue(" holds ", RelationKind::Preserves, false, false),
        cue(" constrains ", RelationKind::Constrains, false, false),
        cue(" limits ", RelationKind::Constrains, false, false),
        cue(" supports ", RelationKind::Supports, false, false),
        cue(" enables ", RelationKind::Enables, false, false),
        cue(" allows ", RelationKind::Enables, false, false),
        cue(" prevents ", RelationKind::Prevents, false, false),
        cue(" blocks ", RelationKind::Prevents, false, false),
        cue(" causes ", RelationKind::Causes, false, false),
        cue(" leads to ", RelationKind::Causes, false, false),
        cue(" implies ", RelationKind::Implies, false, false),
        cue(" changes ", RelationKind::Changes, false, false),
        cue(" is ", RelationKind::IsA, false, false),
    ]
}

fn cue(phrase: &'static str, relation: RelationKind, negated: bool, flip: bool) -> RelationCue {
    RelationCue {
        phrase,
        relation,
        negated,
        flip,
    }
}

fn push_unique_edge(edges: &mut Vec<RelationEdge>, edge: RelationEdge) {
    if !edges.contains(&edge) {
        edges.push(edge);
    }
}

fn parse_without_clause(clause: &str) -> Option<RelationEdge> {
    let rest = clause.trim().strip_prefix("without ")?;
    let (object_raw, subject_raw) = rest.split_once(',')?;
    let object = clean_side(object_raw, true);
    let subject = head_noun(&clean_side(subject_raw, false));
    if valid_side(&subject) && valid_side(&object) && subject != object {
        Some(RelationEdge {
            subject,
            relation: RelationKind::Requires,
            object,
            negated: false,
        })
    } else {
        None
    }
}

fn parse_enablement_clause(clause: &str) -> Option<RelationEdge> {
    let rest = clause.trim().strip_prefix("if ")?;
    let (cond, tail) = rest
        .split_once(',')
        .or_else(|| rest.split_once(" then "))?;
    if interventionish(cond) {
        return None;
    }
    let subject = noun_before_status(cond)?;
    let object = head_noun(&clean_side(tail, false));
    if valid_side(&subject) && valid_side(&object) && subject != object {
        Some(RelationEdge {
            subject,
            relation: RelationKind::Enables,
            object,
            negated: false,
        })
    } else {
        None
    }
}

fn noun_before_status(cond: &str) -> Option<String> {
    for marker in [" is present", " is available", " exists", " is true"] {
        if let Some(index) = cond.find(marker) {
            let noun = clean_side(&cond[..index], true);
            if valid_side(&noun) {
                return Some(noun);
            }
        }
    }
    None
}

fn interventionish(text: &str) -> bool {
    [
        "fail",
        "fails",
        "failing",
        "drop",
        "drops",
        "gone",
        "missing",
        "absent",
        "unreliable",
        "disabled",
        "removed",
        "lost",
        "break",
        "breaks",
        "change",
        "changes",
        "stop",
        "stops",
    ]
    .iter()
    .any(|marker| {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == *marker)
    })
}

fn is_status_word(value: &str) -> bool {
    matches!(
        value,
        "present"
            | "available"
            | "absent"
            | "missing"
            | "gone"
            | "true"
            | "false"
            | "unreliable"
            | "intact"
            | "disabled"
    )
}

fn head_noun(value: &str) -> String {
    const STOP: &[&str] = &[
        "can", "may", "will", "cannot", "can't", "should", "must", "fails", "fail", "run",
        "stand", "drops", "still", "no", "longer", "follow", "follows", "change", "changes",
    ];
    let clipped = value
        .split_whitespace()
        .take_while(|word| !STOP.contains(word))
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    if clipped.is_empty() {
        value.to_owned()
    } else {
        clipped
    }
}

fn parse_intervention(text: &str) -> Option<Intervention> {
    let markers = [
        (" becomes unreliable", "becoming unreliable"),
        (" becomes unavailable", "becoming unavailable"),
        (" is removed", "being removed"),
        (" is disabled", "being disabled"),
        (" is unavailable", "being unavailable"),
        (" is absent", "being absent"),
        (" is missing", "being missing"),
        (" is gone", "being gone"),
        (" is lost", "being lost"),
        (" disappears", "disappearing"),
        (" is false", "being false"),
        (" degrades", "degrading"),
        (" breaks", "breaking"),
        (" fails", "failing"),
        (" drops", "dropping"),
        (" increases", "increasing"),
        (" decreases", "decreasing"),
        (" changes", "changing"),
        (" stops", "stopping"),
    ];
    for (marker, change) in markers {
        let Some(index) = text.find(marker) else {
            continue;
        };
        let prefix = &text[..index];
        let raw = prefix
            .rsplit_once([',', '.', ';'])
            .map(|(_, tail)| tail)
            .unwrap_or(prefix)
            .trim();
        let target = clean_side(raw, true);
        if valid_side(&target) {
            return Some(Intervention {
                target,
                change: change.to_owned(),
            });
        }
    }
    None
}

fn clean_side(raw: &str, left: bool) -> String {
    let mut value = raw.trim().to_ascii_lowercase();
    for prefix in [
        "if ",
        "suppose ",
        "assume ",
        "given ",
        "when ",
        "and ",
        "but ",
        "then ",
        "what follows if ",
        "what changes if ",
    ] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            value = stripped.trim().to_owned();
        }
    }
    if !left {
        for marker in [
            ", what ", " what ", ", which ", " which ", ", if ", " if ", ", while ", " while ",
            ", and ",
        ] {
            if let Some((head, _)) = value.split_once(marker) {
                value = head.trim().to_owned();
            }
        }
    }
    value = value
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && !c.is_whitespace())
        .split_whitespace()
        .filter(|word| {
            !matches!(
                *word,
                "a" | "an" | "the" | "every" | "all" | "some" | "that"
            )
        })
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    value
}

fn valid_side(value: &str) -> bool {
    let words = value.split_whitespace().count();
    (1..=8).contains(&words)
        && value
            .chars()
            .any(|character| character.is_ascii_alphabetic())
        && !matches!(value, "what" | "which" | "then" | "something" | "anything")
}

fn contradictions(edges: &[RelationEdge]) -> Vec<String> {
    let mut out = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        for other in edges.iter().skip(index + 1) {
            if edge.subject == other.subject
                && edge.object == other.object
                && edge.relation == other.relation
                && edge.negated != other.negated
            {
                out.push(format!(
                    "“{}” conflicts with “{}”",
                    edge.proposition(),
                    other.proposition()
                ));
            }
        }
    }
    out
}

fn propagate_affected(edges: &[RelationEdge], target: &str) -> Vec<String> {
    let mut affected = vec![target.to_owned()];
    for _ in 0..edges.len().saturating_add(1) {
        let mut changed = false;
        for edge in edges.iter().filter(|edge| !edge.negated) {
            let subject_hit = affected
                .iter()
                .any(|node| phrase_overlap(&edge.subject, node));
            let object_hit = affected
                .iter()
                .any(|node| phrase_overlap(&edge.object, node));
            if object_hit && edge.relation.loses_support_when_object_changes() {
                if !affected
                    .iter()
                    .any(|node| phrase_overlap(node, &edge.subject))
                {
                    affected.push(edge.subject.clone());
                    changed = true;
                }
            }
            if subject_hit && edge.relation.loses_support_when_subject_changes() {
                if !affected
                    .iter()
                    .any(|node| phrase_overlap(node, &edge.object))
                {
                    affected.push(edge.object.clone());
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    affected
}

fn edge_touches_any(edge: &RelationEdge, affected: &[String]) -> bool {
    affected.iter().any(|node| edge.touches(node))
}

fn negative_phrase(relation: RelationKind) -> &'static str {
    match relation {
        RelationKind::IsA => "is not",
        RelationKind::DependsOn => "does not depend on",
        RelationKind::Requires => "does not require",
        RelationKind::Supports => "does not support",
        RelationKind::Enables => "does not enable",
        RelationKind::Causes => "does not cause",
        RelationKind::Prevents => "does not prevent",
        RelationKind::Preserves => "does not preserve",
        RelationKind::Constrains => "does not constrain",
        RelationKind::Changes => "does not change",
        RelationKind::Implies => "does not imply",
    }
}

fn phrase_overlap(left: &str, right: &str) -> bool {
    let left_tokens = tokens(left);
    let right_tokens = tokens(right);
    !left_tokens.is_disjoint(&right_tokens)
}

fn tokens(text: &str) -> BTreeSet<String> {
    const STOP: &[&str] = &[
        "and", "are", "does", "every", "from", "have", "into", "some", "that", "the", "then",
        "this", "what", "when", "which", "with",
    ];
    text.to_ascii_lowercase()
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
        .filter(|word| word.len() >= 3 && !STOP.contains(word))
        .map(str::to_owned)
        .collect()
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn harmonic_mean(values: &[f64]) -> f64 {
    if values.is_empty() || values.iter().any(|value| *value <= 0.0) {
        return 0.0;
    }
    values.len() as f64 / values.iter().map(|value| 1.0 / value).sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executes_dependency_failure_without_global_collapse() {
        let execution = execute(
            "Trust depends on verification. Memory preserves identity. If verification fails, what changes and what remains invariant?",
        )
        .expect("execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("trust"));
        assert!(lower.contains("verification"));
        assert!(lower.contains("memory preserves identity"));
        assert!(lower.contains("no longer established"));
        assert_eq!(
            execution.impacted.len(),
            1,
            "impacted={:?}; invariant={:?}",
            execution.impacted,
            execution.invariant
        );
        assert_eq!(execution.invariant.len(), 1);
    }

    #[test]
    fn parses_and_reports_explicit_contradiction() {
        let program = parse_program(
            "Signal causes change. Signal does not cause change. If signal fails, what follows?",
        );
        assert_eq!(program.edges.len(), 2);
        assert_eq!(contradictions(&program.edges).len(), 1);
    }

    #[test]
    fn resonance_rewards_relation_preservation() {
        let user =
            "Trust depends on verification. If verification fails, what changes and what remains?";
        let good = "Trust depends on verification, so when verification fails, trust is no longer established; the stated dependency must be re-evaluated.";
        let bad = "A triangle is stable under structural load.";
        assert!(score_response(user, good).score > score_response(user, bad).score);
    }

    #[test]
    fn ordinary_question_does_not_claim_execution_authority() {
        assert!(execute("What is the relationship between memory and identity?").is_none());
    }

    #[test]
    fn dependency_failure_propagates_across_two_hops() {
        let execution = execute(
            "Trust depends on verification. Verification depends on evidence. If evidence fails, what changes?",
        )
        .expect("execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("verification"));
        assert!(lower.contains("trust"));
        assert_eq!(execution.impacted.len(), 2);
    }

    #[test]
    fn disconnected_intervention_does_not_invent_a_bridge() {
        let execution = execute(
            "Trust depends on evidence. If latency changes, what changes and what remains invariant?",
        )
        .expect("bounded no-path answer");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("touches no supplied relation"));
        assert!(lower.contains("no change follows"));
        assert!(lower.contains("trust depends on evidence"));
        assert!(execution.impacted.is_empty());
    }

    #[test]
    fn hinges_on_unseen_entity_and_signal_drop() {
        let execution = execute(
            "Quoril-7 hinges on a boundary. The boundary needs a signal. If the signal drops, which claims fail and what stays invariant?",
        )
        .expect("execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("quoril-7"));
        assert!(lower.contains("boundary"));
        assert!(lower.contains("signal"));
        assert!(
            lower.contains("no longer established")
                || lower.contains("loses a stated dependency")
                || lower.contains("no longer follows")
        );
        assert!(!lower.contains("touches no supplied relation"));
        assert!(
            execution
                .impacted
                .iter()
                .any(|claim| claim.contains("quoril-7") || claim.contains("boundary"))
        );
    }

    #[test]
    fn without_clause_requires_the_missing_support() {
        let program = parse_program(
            "Without verification, trust cannot stand. Memory still keeps identity. If verification is gone, what follows and what remains?",
        );
        assert!(
            program.edges.iter().any(|edge| {
                edge.subject.contains("trust")
                    && edge.object.contains("verification")
                    && edge.relation == RelationKind::Requires
            }),
            "edges={:?}",
            program.edges
        );
        assert!(program.edges.iter().any(|edge| {
            edge.subject.contains("memory") && edge.relation == RelationKind::Preserves
        }));
        let execution = execute(
            "Without verification, trust cannot stand. Memory still keeps identity. If verification is gone, what follows and what remains?",
        )
        .expect("execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("trust"));
        assert!(lower.contains("verification"));
        assert!(lower.contains("memory"));
    }

    #[test]
    fn required_for_flips_subject_and_object() {
        let program = parse_program(
            "A signal is required for the boundary. Identity is preserved by memory. If the signal is missing, what changes and what remains invariant?",
        );
        assert!(
            program.edges.iter().any(|edge| {
                edge.subject.contains("boundary")
                    && edge.object.contains("signal")
                    && edge.relation == RelationKind::Requires
            }),
            "edges={:?}",
            program.edges
        );
        assert!(program.edges.iter().any(|edge| {
            edge.subject.contains("memory")
                && edge.object.contains("identity")
                && edge.relation == RelationKind::Preserves
        }));
        assert!(execute(
            "A signal is required for the boundary. Identity is preserved by memory. If the signal is missing, what changes and what remains invariant?",
        )
        .is_some());
    }

    #[test]
    fn present_enablement_is_not_an_isa_status_edge() {
        let program = parse_program(
            "If evidence is present, verification can run. Trust rests on verification. If evidence is absent, what no longer follows?",
        );
        assert!(
            program.edges.iter().any(|edge| {
                edge.subject.contains("evidence")
                    && edge.object.contains("verification")
                    && edge.relation == RelationKind::Enables
            }),
            "edges={:?}",
            program.edges
        );
        assert!(program
            .edges
            .iter()
            .any(|edge| edge.subject.contains("trust") && edge.object.contains("verification")));
        assert!(!program
            .edges
            .iter()
            .any(|edge| edge.relation == RelationKind::IsA));
        let execution = execute(
            "If evidence is present, verification can run. Trust rests on verification. If evidence is absent, what no longer follows?",
        )
        .expect("execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("trust"));
        assert!(lower.contains("verification"));
        assert!(lower.contains("evidence"));
    }

    #[test]
    fn disconnected_novel_noun_does_not_invent_a_bridge() {
        let execution = execute(
            "Kestrel-9 hinges on a lattice. If latency changes, what changes and what remains invariant?",
        )
        .expect("bounded no-path answer");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("touches no supplied relation"));
        assert!(lower.contains("kestrel-9"));
        assert!(execution.impacted.is_empty());
    }

    #[test]
    fn ordinary_relationship_question_still_does_not_execute() {
        assert!(execute("What is the relationship between harbor-4 and a witness?").is_none());
        assert!(execute("What is the relationship between trust and verification?").is_none());
        assert!(execute("How do memory and attention interact under load?").is_none());
    }

    #[test]
    fn followup_reuses_prior_graph_without_restating_premises() {
        let prior = "Trust rests on verification. Verification is grounded in evidence. Memory keeps identity. If evidence fails, what changes and what remains invariant?";
        let follow = execute_followup("What still holds?", prior).expect("follow-up");
        let lower = follow.text.to_ascii_lowercase();
        assert!(lower.contains("memory"));
        assert!(lower.contains("invariant") || lower.contains("preserves") || lower.contains("keeps"));
    }

    #[test]
    fn followup_can_apply_a_new_intervention() {
        let prior = "Harbor-4 relies on a checksum. The checksum needs a witness. If the witness breaks, which claims fail?";
        let follow = execute_followup("And if the checksum fails too?", prior).expect("follow-up");
        let lower = follow.text.to_ascii_lowercase();
        assert!(lower.contains("harbor-4") || lower.contains("checksum"));
        assert!(!lower.contains("touches no supplied relation"));
    }

    #[test]
    fn multi_hop_affected_edge_is_not_reported_as_invariant() {
        let execution = execute(
            "Trust depends on verification. Verification depends on evidence. Memory preserves identity. If evidence fails, what changes and what remains invariant?",
        )
        .expect("execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("memory preserves identity"));
        assert!(!lower.contains(
            "what remains invariant from the supplied premises is “trust depends on verification"
        ));
        assert_eq!(execution.impacted.len(), 2);
        assert_eq!(execution.invariant, vec!["memory preserves identity"]);
    }

    #[test]
    fn synonym_relations_and_interventions_transfer() {
        let execution = execute(
            "Trust relies on verification. Verification needs evidence. If evidence breaks, which claims fail?",
        )
        .expect("synonym execution");
        let lower = execution.text.to_ascii_lowercase();
        assert!(lower.contains("verification"));
        assert!(lower.contains("trust"));
        assert!(lower.contains("evidence"));
        assert_eq!(execution.impacted.len(), 2);

        let causal =
            execute("Signal leads to change. If signal is disabled, what no longer follows?")
                .expect("causal synonym execution");
        assert!(causal.text.to_ascii_lowercase().contains("change"));
    }
}
