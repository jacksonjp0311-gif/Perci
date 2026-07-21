//! ThoughtPlan — structured cognitive product (Phase 2).
//!
//! Substantive operators should produce or modify a ThoughtPlan rather than only
//! a finished string. Exact tools and social reflexes may still return text only.
//!
//! Not private chain-of-thought: only named operations, claims, evidence refs,
//! uncertainties, and discourse acts.

use crate::deliberation::Deliberation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    CausalExplanation,
    Comparison,
    Verification,
    Teaching,
    Plan,
    Refuse,
    Social,
    Exact,
    Synthesis,
    Identity,
    Trust,
    Unknown,
}

impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CausalExplanation => "causal_explanation",
            Self::Comparison => "comparison",
            Self::Verification => "verification",
            Self::Teaching => "teaching",
            Self::Plan => "plan",
            Self::Refuse => "refuse",
            Self::Social => "social",
            Self::Exact => "exact",
            Self::Synthesis => "synthesis",
            Self::Identity => "identity",
            Self::Trust => "trust",
            Self::Unknown => "unknown",
        }
    }

    pub fn infer_from_prompt(user: &str) -> Self {
        let t = user.to_ascii_lowercase();
        if t.contains("hello") || t.contains("how are you") || t.split_whitespace().count() <= 2 {
            return Self::Social;
        }
        if t.contains("calculate") || t.contains("triangle area") || t.contains("percent") {
            return Self::Exact;
        }
        if t.contains("conscious")
            || t.contains("auto-promot")
            || (t.contains("invent") && t.contains("soul"))
        {
            return Self::Refuse;
        }
        if t.contains("connect ") || t.contains("across domains") || t.contains("bridge ") {
            return Self::Synthesis;
        }
        if t.contains("compare ") || t.contains("versus") || t.contains(" vs ") {
            return Self::Comparison;
        }
        // Trust under partial observability (lag / delay / timeout / retry).
        if t.contains("trust")
            && (t.contains("lag")
                || t.contains("timeout")
                || t.contains("retry")
                || t.contains("delay")
                || t.contains("delayed"))
        {
            return Self::Trust;
        }
        if (t.contains("timeout") || t.contains("retry"))
            && (t.contains("timeout") && t.contains("retry")
                || t.contains("idempot")
                || t.contains("what about"))
        {
            return Self::Trust;
        }
        if t.contains("who are you") || t.contains("what are you") {
            return Self::Identity;
        }
        if t.contains("why ")
            || t.contains("how does")
            || t.contains("how do ")
            || t.contains("explain ")
            || t.contains("how should")
        {
            return Self::CausalExplanation;
        }
        if t.contains("plan ") || t.contains("step") {
            return Self::Plan;
        }
        if t.contains("prove ") || t.contains("verify") || t.contains("evidence") {
            return Self::Verification;
        }
        Self::Unknown
    }

    /// Substantive intents that may use modular SEM→RSN→DSC→LM (not social/exact).
    pub fn is_modular_eligible(self) -> bool {
        matches!(
            self,
            Self::CausalExplanation
                | Self::Comparison
                | Self::Verification
                | Self::Plan
                | Self::Synthesis
                | Self::Trust
                | Self::Identity
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub role: String,
    pub filler: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundClaim {
    pub text: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub subject: String,
    pub relation: String,
    pub object: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    pub text: String,
    pub score_pm: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub source_type: String,
    pub claim: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uncertainty {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscourseAct {
    DirectAnswer,
    Mechanism,
    Example,
    Boundary,
    Criteria,
    Tradeoff,
    Judgment,
    Evidence,
    Counterexample,
    Uncertainty,
    Orientation,
    CheckUnderstanding,
    Refuse,
}

impl DiscourseAct {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DirectAnswer => "direct_answer",
            Self::Mechanism => "mechanism",
            Self::Example => "example",
            Self::Boundary => "boundary",
            Self::Criteria => "criteria",
            Self::Tradeoff => "tradeoff",
            Self::Judgment => "judgment",
            Self::Evidence => "evidence",
            Self::Counterexample => "counterexample",
            Self::Uncertainty => "uncertainty",
            Self::Orientation => "orientation",
            Self::CheckUnderstanding => "check_understanding",
            Self::Refuse => "refuse",
        }
    }
}

/// Structured thought product for substantive cognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtPlan {
    pub intent: Intent,
    pub semantic_bindings: Vec<Binding>,
    pub claims: Vec<BoundClaim>,
    pub mechanisms: Vec<Relation>,
    pub alternatives: Vec<Hypothesis>,
    pub evidence: Vec<EvidenceRef>,
    pub uncertainties: Vec<Uncertainty>,
    pub boundaries: Vec<Constraint>,
    pub discourse_acts: Vec<DiscourseAct>,
    /// Confidence in parts-per-mille [0, 1000].
    pub confidence_pm: u16,
    /// Named operator that produced this plan.
    pub operator: String,
    /// Optional finished surface text (compat with string operators).
    #[serde(default)]
    pub surface_answer: String,
    /// Active pack ids selected for this turn (sparse).
    #[serde(default)]
    pub active_packs: Vec<String>,
    /// Bounded reasoning cycle labels.
    #[serde(default)]
    pub reasoning_cycles: Vec<String>,
    #[serde(default)]
    pub halt_reason: String,
}

impl ThoughtPlan {
    pub fn empty(operator: &str, intent: Intent) -> Self {
        Self {
            intent,
            semantic_bindings: Vec::new(),
            claims: Vec::new(),
            mechanisms: Vec::new(),
            alternatives: Vec::new(),
            evidence: Vec::new(),
            uncertainties: Vec::new(),
            boundaries: Vec::new(),
            discourse_acts: Vec::new(),
            confidence_pm: 500,
            operator: operator.into(),
            surface_answer: String::new(),
            active_packs: vec!["PERCIW03".into()],
            reasoning_cycles: Vec::new(),
            halt_reason: String::new(),
        }
    }

    pub fn with_surface(mut self, answer: impl Into<String>) -> Self {
        self.surface_answer = answer.into();
        self
    }

    pub fn push_claim(mut self, text: impl Into<String>, status: &str) -> Self {
        self.claims.push(BoundClaim {
            text: text.into(),
            status: status.into(),
        });
        self
    }

    pub fn push_binding(mut self, role: &str, filler: &str) -> Self {
        self.semantic_bindings.push(Binding {
            role: role.into(),
            filler: filler.into(),
        });
        self
    }

    pub fn push_uncertainty(mut self, text: impl Into<String>) -> Self {
        self.uncertainties.push(Uncertainty {
            text: text.into(),
        });
        self
    }

    pub fn push_boundary(mut self, text: impl Into<String>) -> Self {
        self.boundaries.push(Constraint {
            text: text.into(),
        });
        self
    }

    pub fn set_discourse(mut self, acts: &[DiscourseAct]) -> Self {
        self.discourse_acts = acts.to_vec();
        self
    }

    pub fn confidence_from_f64(mut self, c: f64) -> Self {
        let pm = (c.clamp(0.0, 1.0) * 1000.0).round() as u16;
        self.confidence_pm = pm;
        self
    }

    /// Operational receipt (named ops only — no invented private thoughts).
    pub fn receipt(&self) -> String {
        let binds = if self.semantic_bindings.is_empty() {
            "none".into()
        } else {
            self.semantic_bindings
                .iter()
                .map(|b| format!("{}↔{}", b.role, b.filler))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let acts = if self.discourse_acts.is_empty() {
            "none".into()
        } else {
            self.discourse_acts
                .iter()
                .map(|a| a.as_str())
                .collect::<Vec<_>>()
                .join(" → ")
        };
        let packs = if self.active_packs.is_empty() {
            "PERCIW03".into()
        } else {
            self.active_packs.join(", ")
        };
        let cycles = if self.reasoning_cycles.is_empty() {
            "none".into()
        } else {
            self.reasoning_cycles.join("; ")
        };
        let claims = self
            .claims
            .iter()
            .take(3)
            .map(|c| c.text.chars().take(80).collect::<String>())
            .collect::<Vec<_>>()
            .join(" | ");
        format!(
            "intent: {}\n\
semantic bindings: {binds}\n\
operator: {}\n\
active packs: {packs}\n\
reasoning cycles: {cycles}\n\
claims: {}\n\
discourse plan: {acts}\n\
confidence_pm: {}\n\
halt reason: {}\n\
verification: bounded best effort — not private chain-of-thought",
            self.intent.as_str(),
            self.operator,
            if claims.is_empty() {
                "none recorded"
            } else {
                claims.as_str()
            },
            self.confidence_pm,
            if self.halt_reason.is_empty() {
                "operator completed"
            } else {
                self.halt_reason.as_str()
            },
        )
    }

    /// Compact JSON for decision-trace / CLI.
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }
}

/// Lift an existing Deliberation into a ThoughtPlan (compat bridge).
pub fn from_deliberation(user: &str, d: &Deliberation) -> ThoughtPlan {
    let intent = Intent::infer_from_prompt(user);
    let mut plan = ThoughtPlan::empty(d.operator, intent)
        .with_surface(d.answer.clone())
        .confidence_from_f64(d.confidence);

    // Shared SEM bindings so operator and modular paths speak the same frame language.
    let frame = crate::semantic_field::extract_frame(user);
    plan.semantic_bindings = frame.to_bindings();
    if plan.operator.contains("modular") {
        plan.active_packs = vec![
            "PERCIW03".into(),
            "PERCIRSN1".into(),
            "PERCISEM1".into(),
            "PERCIDSC1".into(),
            "PERCILM1".into(),
        ];
    }

    // First sentence as primary claim when non-empty.
    let claim = d
        .answer
        .split(['.', '!', '?'])
        .next()
        .unwrap_or("")
        .trim();
    if claim.len() >= 12 {
        plan = plan.push_claim(claim, "working");
    }

    for u in &d.uncertainties {
        plan = plan.push_uncertainty(u.clone());
    }
    for o in d.observations.iter().take(2) {
        if o.starts_with("thought_plan.") {
            continue;
        }
        plan.evidence.push(EvidenceRef {
            source_type: "observation".into(),
            claim: o.clone(),
        });
    }
    for i in d.inferences.iter().take(2) {
        plan.mechanisms.push(Relation {
            subject: if frame.subject.is_empty() {
                "system".into()
            } else {
                frame.subject.clone()
            },
            relation: "infers".into(),
            object: i.chars().take(120).collect(),
        });
    }

    // Default discourse skeleton by intent.
    plan.discourse_acts = match intent {
        Intent::CausalExplanation | Intent::Trust => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Mechanism,
            DiscourseAct::Boundary,
        ],
        Intent::Comparison => vec![
            DiscourseAct::Criteria,
            DiscourseAct::Tradeoff,
            DiscourseAct::Judgment,
        ],
        Intent::Verification => vec![
            DiscourseAct::DirectAnswer,
            DiscourseAct::Evidence,
            DiscourseAct::Counterexample,
            DiscourseAct::Uncertainty,
        ],
        Intent::Refuse => vec![DiscourseAct::Refuse, DiscourseAct::Boundary],
        Intent::Teaching => vec![
            DiscourseAct::Orientation,
            DiscourseAct::Mechanism,
            DiscourseAct::Example,
        ],
        Intent::Social | Intent::Exact => vec![DiscourseAct::DirectAnswer],
        _ => vec![DiscourseAct::DirectAnswer, DiscourseAct::Boundary],
    };

    if let Some(pid) = d.program_id {
        plan.reasoning_cycles.push(format!("program:{pid}"));
    }
    for step in &d.program_steps {
        plan.reasoning_cycles.push((*step).to_owned());
    }
    plan.halt_reason = match d.critic_ok {
        Some(true) => "critic pass".into(),
        Some(false) => "critic flags".into(),
        None => "operator completed".into(),
    };

    // Light binding extraction for common motifs.
    let low = user.to_ascii_lowercase();
    if low.contains("trust") {
        plan = plan.push_binding("subject", "trust");
    }
    if low.contains("lag") || low.contains("delay") || low.contains("timeout") {
        plan = plan.push_binding("condition", "delayed_communication");
    }
    if low.contains("why") || low.contains("collapse") {
        plan = plan.push_binding("requested_output", "mechanism");
    }

    plan
}

/// Default discourse plans (PERCIDSC1 seed table — code-level until pack exists).
pub fn default_discourse_plan(intent: Intent) -> Vec<DiscourseAct> {
    ThoughtPlan::empty("discourse-table", intent).discourse_acts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_infer_trust_causal() {
        // Trust under delay/lag is Trust (not generic causal) so modular packs route correctly.
        assert_eq!(
            Intent::infer_from_prompt("Why does trust collapse when communication is delayed?"),
            Intent::Trust
        );
        assert_eq!(
            Intent::infer_from_prompt("how should OrbitAPI earn trust under lag"),
            Intent::Trust
        );
        assert_eq!(
            Intent::infer_from_prompt("explain how boundaries enable repair"),
            Intent::CausalExplanation
        );
    }

    #[test]
    fn deliberation_lifts_to_plan_and_receipt() {
        let d = Deliberation::new(
            "trust-systems",
            "Interfaces earn trust under lag when done is checkable. Timeout meaning must be in the contract.",
        )
        .observed("trust+lag")
        .inferred("checkable done predicates")
        .uncertain("entity names are surface labels")
        .confidence(0.94);
        let plan = from_deliberation(
            "how should interfaces earn trust under lag",
            &d,
        );
        assert_eq!(plan.operator, "trust-systems");
        assert!(!plan.claims.is_empty());
        assert!(plan.confidence_pm >= 900);
        let r = plan.receipt();
        assert!(r.contains("intent:"));
        assert!(r.contains("active packs:"));
        assert!(r.contains("not private chain-of-thought"));
        assert!(plan.to_json_line().contains("trust-systems"));
    }

    #[test]
    fn social_stays_light() {
        let d = Deliberation::new("social-reflex", "Hey — I'm here.");
        let plan = from_deliberation("hello perci", &d);
        assert_eq!(plan.intent, Intent::Social);
        assert!(plan.discourse_acts.contains(&DiscourseAct::DirectAnswer));
    }
}
