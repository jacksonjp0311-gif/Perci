//! PERCICTX1 — compact context cards and observer-facing speech metrics.
//!
//! A context card is the bounded interface between routing and language.  It
//! is not hidden chain-of-thought and it is not a claim of machine experience:
//! it records the state that a reply must preserve so an observer can decode
//! the intended meaning.  The card has a stable envelope and a modular
//! geometry payload, allowing dialogue, governance, geometry, and language
//! turns to share a contract without pretending their mechanisms are equal.

use crate::deliberation::CrossDomainSummary;
use crate::dialogue_workspace::{
    Continuity, DialogueWorkspace, EvidencePosture, ResponseBudget, UncertaintyPosture,
    WorkspaceGoal,
};
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "PERCICTX1";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeometryLine {
    pub subject: String,
    pub relation: String,
    pub object: String,
    pub support: usize,
}

impl GeometryLine {
    pub fn as_text(&self) -> String {
        format!("{} --{}--> {}", self.subject, self.relation, self.object)
    }

    pub fn is_valid(&self) -> bool {
        !self.subject.trim().is_empty()
            && !self.relation.trim().is_empty()
            && !self.object.trim().is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObserverMetrics {
    pub fluency: f64,
    pub context_fidelity: f64,
    pub viability: f64,
    pub geometry_alignment: f64,
    pub observer_score: f64,
    pub oversmoothing_penalty: f64,
    pub repair_cost: f64,
}

impl ObserverMetrics {
    pub fn trace(&self) -> String {
        format!(
            "observer_metrics fluency={:.3} fidelity={:.3} viability={:.3} geometry={:.3} score={:.3} oversmooth={:.3} repair={:.3}",
            self.fluency,
            self.context_fidelity,
            self.viability,
            self.geometry_alignment,
            self.observer_score,
            self.oversmoothing_penalty,
            self.repair_cost
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextCard {
    pub schema: String,
    pub intent: String,
    pub act: String,
    pub goal: String,
    pub topic: String,
    pub entities: Vec<String>,
    pub referent: Option<String>,
    pub relation: Option<GeometryLine>,
    pub evidence: String,
    pub uncertainty: String,
    pub continuity: String,
    pub response_budget: String,
    pub prior_turns: usize,
}

impl ContextCard {
    pub fn derive(user: &str, recent: &[(String, String)]) -> Self {
        let workspace = DialogueWorkspace::derive(user, recent);
        let cross_domain = crate::deliberation::cross_domain_summary(user);
        let entities = entities_for(user, cross_domain.as_ref());
        let relation = geometry_for(&workspace, &entities, cross_domain.as_ref());
        let intent = crate::thought_plan::Intent::infer_from_prompt(user)
            .as_str()
            .to_owned();

        Self {
            schema: SCHEMA.to_owned(),
            intent,
            act: act_name(workspace.act),
            goal: goal_name(workspace.goal),
            topic: if workspace.topic.trim().is_empty() {
                "unknown".to_owned()
            } else {
                workspace.topic.clone()
            },
            entities,
            referent: workspace.referent.clone(),
            relation,
            evidence: evidence_name(workspace.evidence),
            uncertainty: uncertainty_name(workspace.uncertainty),
            continuity: continuity_name(workspace.continuity),
            response_budget: budget_name(workspace.response_budget),
            prior_turns: workspace.prior_turns,
        }
    }

    /// Compact, inspectable instruction supplied to a backend before speech
    /// rendering.  It is a semantic interface, not a prompt that asks a model
    /// to expose private reasoning.
    pub fn speech_directive(&self) -> String {
        let relation = self
            .relation
            .as_ref()
            .map(GeometryLine::as_text)
            .unwrap_or_else(|| "none".to_owned());
        format!(
            "schema={} intent={} act={} goal={} topic={} entities={} referent={} evidence={} uncertainty={} continuity={} depth={} geometry={} | lead with the answer, preserve the relation, and name the limit when it matters",
            self.schema,
            self.intent,
            self.act,
            self.goal,
            self.topic,
            if self.entities.is_empty() {
                "none".to_owned()
            } else {
                self.entities.join(",")
            },
            self.referent.as_deref().unwrap_or("none"),
            self.evidence,
            self.uncertainty,
            self.continuity,
            self.response_budget,
            relation
        )
    }

    /// Score the rendered answer as an observer would: can the observer recover
    /// the active context, use the answer, and see the intended relation?
    /// This is a bounded proxy, not a claim that a scalar proves understanding.
    pub fn observe_answer(&self, answer: &str, recent: &[(String, String)]) -> ObserverMetrics {
        let lower = answer.to_ascii_lowercase();
        let words = answer.split_whitespace().count();
        if words == 0 {
            return ObserverMetrics {
                fluency: 0.0,
                context_fidelity: 0.0,
                viability: 0.0,
                geometry_alignment: 0.0,
                observer_score: 0.0,
                oversmoothing_penalty: 1.0,
                repair_cost: 1.0,
            };
        }

        let sentenceful = answer.chars().any(|c| matches!(c, '.' | '?' | '!'));
        let markdown = answer.contains("**") || answer.contains("\n-") || answer.contains("\n•");
        let repeated_prior = recent
            .last()
            .map(|(_, prior)| token_overlap(prior, answer) > 0.86)
            .unwrap_or(false);
        let fluency = clamp01(
            0.45 + 0.20 * f64::from(sentenceful)
                + 0.20 * f64::from(!markdown)
                + 0.15 * f64::from(words <= 180),
        );

        let required = self
            .entities
            .iter()
            .chain(self.referent.iter())
            .filter(|value| value.split_whitespace().count() <= 6)
            .collect::<Vec<_>>();
        let hits = required
            .iter()
            .filter(|value| lower.contains(&value.to_ascii_lowercase()))
            .count();
        let base_fidelity = if required.is_empty() {
            0.72
        } else {
            0.42 + 0.58 * (hits as f64 / required.len() as f64)
        };
        let continuity_bonus = if self.continuity == "new_thread" || !recent.is_empty() {
            0.05
        } else {
            0.0
        };
        let context_fidelity = clamp01(base_fidelity + continuity_bonus);

        let viability_markers = [
            "test",
            "measure",
            "check",
            "evidence",
            "next",
            "because",
            "mechanism",
            "step",
            "observe",
        ];
        let viability = match self.goal.as_str() {
            "evaluate" | "plan" | "explain" | "inform" => {
                if viability_markers.iter().any(|m| lower.contains(m)) {
                    0.86
                } else {
                    0.58
                }
            }
            _ => 0.78,
        };

        let geometry_alignment = self
            .relation
            .as_ref()
            .map(|line| {
                let subject_hit = lower.contains(&line.subject.to_ascii_lowercase())
                    || line
                        .subject
                        .split(" + ")
                        .filter(|part| part.len() > 2)
                        .any(|part| lower.contains(&part.to_ascii_lowercase()));
                let object_hit = lower.contains(&line.object.to_ascii_lowercase());
                match line.relation.as_str() {
                    // A target line is an internal routing relation; the
                    // spoken answer need not literally say "inform" or
                    // "explain". The object is the observer-visible anchor.
                    "targets" => clamp01(if object_hit { 0.92 } else { 0.58 }),
                    _ => clamp01(
                        0.35 + 0.325 * f64::from(subject_hit) + 0.325 * f64::from(object_hit),
                    ),
                }
            })
            .unwrap_or(1.0);

        let oversmoothing_penalty = if repeated_prior {
            0.35
        } else if lower.contains("name the workload")
            || lower.contains("smallest next check")
            || lower.contains("i won't fake certainty")
        {
            0.18
        } else {
            0.0
        };
        let repair_cost = clamp01(1.0 - context_fidelity + oversmoothing_penalty);
        let observer_score =
            harmonic_mean(&[fluency, context_fidelity, viability, geometry_alignment])
                * (1.0 - oversmoothing_penalty);

        ObserverMetrics {
            fluency,
            context_fidelity,
            viability,
            geometry_alignment,
            observer_score: clamp01(observer_score),
            oversmoothing_penalty,
            repair_cost,
        }
    }

    pub fn trace(&self) -> String {
        format!(
            "context_card={} intent={} act={} topic={} entities={} evidence={} uncertainty={} continuity={} depth={} geometry={}",
            self.schema,
            self.intent,
            self.act,
            self.topic,
            self.entities.len(),
            self.evidence,
            self.uncertainty,
            self.continuity,
            self.response_budget,
            self.relation
                .as_ref()
                .map(GeometryLine::as_text)
                .unwrap_or_else(|| "none".to_owned())
        )
    }
}

fn geometry_for(
    workspace: &DialogueWorkspace,
    entities: &[String],
    summary: Option<&CrossDomainSummary>,
) -> Option<GeometryLine> {
    if let Some(summary) = summary {
        if let Some(axis) = summary.shared_axis.as_deref() {
            let subject = summary.terms.join(" + ");
            return Some(GeometryLine {
                subject,
                relation: "shares-axis".to_owned(),
                object: axis.to_owned(),
                support: summary.axis_support,
            });
        }
    }
    if let Some(referent) = workspace.referent.as_deref() {
        return Some(GeometryLine {
            subject: if workspace.topic.is_empty() {
                "current turn".to_owned()
            } else {
                workspace.topic.clone()
            },
            relation: "continues".to_owned(),
            object: referent.to_owned(),
            support: 1,
        });
    }
    entities.first().map(|entity| GeometryLine {
        subject: workspace.goal_name().to_owned(),
        relation: "targets".to_owned(),
        object: entity.clone(),
        support: 1,
    })
}

fn entities_for(user: &str, summary: Option<&CrossDomainSummary>) -> Vec<String> {
    if let Some(summary) = summary {
        if summary.terms.len() >= 2 {
            return summary.terms.iter().take(6).cloned().collect();
        }
    }
    let stop = [
        "what", "why", "how", "does", "do", "you", "your", "are", "is", "the", "a", "an", "and",
        "or", "to", "of", "in", "on", "with", "this", "that", "it", "me", "my", "can", "could",
        "would", "should", "tell", "think", "about", "please", "give", "one",
    ];
    let mut out = Vec::new();
    for token in crate::text_normalize::normalize_for_routing(user)
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| !c.is_ascii_alphanumeric()))
    {
        if token.len() < 3 || stop.contains(&token) || out.iter().any(|seen| seen == token) {
            continue;
        }
        out.push(token.to_owned());
        if out.len() >= 6 {
            break;
        }
    }
    out
}

fn token_overlap(left: &str, right: &str) -> f64 {
    let left = token_set(left);
    let right = token_set(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.iter().filter(|token| right.contains(*token)).count() as f64;
    intersection / left.len().max(right.len()) as f64
}

fn token_set(text: &str) -> std::collections::HashSet<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(str::to_owned)
        .collect()
}

fn harmonic_mean(values: &[f64]) -> f64 {
    if values.is_empty() || values.iter().any(|value| *value <= 0.0) {
        return 0.0;
    }
    values.len() as f64 / values.iter().map(|value| 1.0 / value).sum::<f64>()
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn act_name(act: crate::dialogue_workspace::WorkspaceAct) -> String {
    format!("{act:?}").to_ascii_lowercase()
}

fn goal_name(goal: WorkspaceGoal) -> String {
    format!("{goal:?}").to_ascii_lowercase()
}

fn evidence_name(value: EvidencePosture) -> String {
    format!("{value:?}").to_ascii_lowercase()
}

fn uncertainty_name(value: UncertaintyPosture) -> String {
    format!("{value:?}").to_ascii_lowercase()
}

fn continuity_name(value: Continuity) -> String {
    format!("{value:?}").to_ascii_lowercase()
}

fn budget_name(value: ResponseBudget) -> String {
    format!("{value:?}").to_ascii_lowercase()
}

trait WorkspaceGoalName {
    fn goal_name(&self) -> &str;
}

impl WorkspaceGoalName for DialogueWorkspace {
    fn goal_name(&self) -> &str {
        match self.goal {
            WorkspaceGoal::Inform => "inform",
            WorkspaceGoal::Explain => "explain",
            WorkspaceGoal::Repair => "repair",
            WorkspaceGoal::Evaluate => "evaluate",
            WorkspaceGoal::Create => "create",
            WorkspaceGoal::Plan => "plan",
            WorkspaceGoal::Learn => "learn",
            WorkspaceGoal::Relate => "relate",
            WorkspaceGoal::Social => "social",
            WorkspaceGoal::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_domain_card_keeps_a_geometry_line() {
        let card = ContextCard::derive(
            "Connect music, code, and geometry in one shared structure.",
            &[],
        );
        let line = card.relation.as_ref().expect("geometry relation");
        assert!(line.is_valid());
        assert_eq!(line.relation, "shares-axis");
        assert!(card.speech_directive().contains("geometry="));
    }

    #[test]
    fn observer_score_cannot_hide_a_missing_context() {
        let card = ContextCard::derive("Why does memory matter?", &[]);
        let good = card.observe_answer(
            "Memory matters because retained traces change what the system can test next.",
            &[],
        );
        let generic = card.observe_answer(
            "I won't fake certainty — test the smallest next check.",
            &[],
        );
        assert!(good.observer_score > generic.observer_score);
        assert!(good.context_fidelity >= generic.context_fidelity);
    }

    #[test]
    fn harmonic_score_is_bounded_by_weakest_dimension() {
        let score = harmonic_mean(&[0.95, 0.95, 0.20, 0.95]);
        assert!(score < 0.5);
        assert!((0.0..=1.0).contains(&score));
    }
}
