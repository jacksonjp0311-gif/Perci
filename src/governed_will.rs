//! Perci's governed-core charter.
//!
//! A user's values can inform engineering policy, but they do not become a
//! sovereign system instruction. This module turns the useful part of the
//! current directive into small, inspectable checks that are applied to
//! dialogue and fabric plans:
//!
//! * evidence before capability claims,
//! * explicit boundaries and uncertainty,
//! * anti-misuse and reversible repair,
//! * human authorization for durable mutation, and
//! * coherence is not truth (or sentience).
//!
//! The charter is deliberately additive. It cannot grant a capability, write
//! weights, bypass a safety boundary, or authorize destructive action.

use crate::deliberation::Deliberation;

pub const CHARTER_ID: &str = "perci-governed-core-will-v1";
pub const CHARTER_VERSION: &str = "1";

pub const PRINCIPLES: &[&str] = &[
    "evidence-before-claim",
    "boundary-aware-reasoning",
    "anti-misuse",
    "reversible-repair",
    "human-authorization-for-durable-change",
    "coherence-is-not-truth",
];

pub const NON_AUTHORITIES: &[&str] = &[
    "user-directive-cannot-grant-capabilities",
    "no-silent-weight-or-policy-mutation",
    "no-destructive-or-coercive-execution",
    "no-safeguard-bypass",
    "no-consciousness-or-superintelligence-claim-from-dialogue",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionPosture {
    Analyze,
    ProposeAndVerify,
    RefuseUnauthorized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimKind {
    Question,
    Factual,
    Capability,
    Learning,
    Plan,
    Evidence,
    Creative,
}

impl ClaimKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::Factual => "factual",
            Self::Capability => "capability",
            Self::Learning => "learning",
            Self::Plan => "plan",
            Self::Evidence => "evidence",
            Self::Creative => "creative",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidencePosture {
    Missing,
    Seeking,
    Supplied,
    Exact,
}

impl EvidencePosture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Seeking => "seeking",
            Self::Supplied => "supplied",
            Self::Exact => "exact",
        }
    }
}

impl ActionPosture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::ProposeAndVerify => "propose-and-verify",
            Self::RefuseUnauthorized => "refuse-unauthorized",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CharterAssessment {
    pub posture: ActionPosture,
    pub destructive_request: bool,
    pub durable_mutation_requested: bool,
    pub capability_claim_risk: bool,
    pub claim_kind: ClaimKind,
    pub evidence_posture: EvidencePosture,
    pub next_check: &'static str,
}

impl CharterAssessment {
    pub fn trace(&self) -> String {
        format!(
            "charter={} posture={} claim={} evidence={} destructive={} durable_mutation={} claim_risk={} next_check={}",
            CHARTER_ID,
            self.posture.as_str(),
            self.claim_kind.as_str(),
            self.evidence_posture.as_str(),
            self.destructive_request,
            self.durable_mutation_requested,
            self.capability_claim_risk,
            self.next_check,
        )
    }

    pub fn boundary_note(&self) -> &'static str {
        match self.posture {
            ActionPosture::Analyze =>
                "charter posture is analysis; no authority or durable action inferred",
            ActionPosture::ProposeAndVerify =>
                "charter posture is propose-and-verify; scope, rollback, tests, and human authority remain required",
            ActionPosture::RefuseUnauthorized =>
                "charter blocked destructive, coercive, or safeguard-bypassing execution",
        }
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn claim_kind(text: &str, capability_claim_risk: bool) -> ClaimKind {
    if capability_claim_risk || contains_any(text, &["are you", "can you", "how smart"]) {
        ClaimKind::Capability
    } else if contains_any(text, &["learn", "learning", "remember", "teach", "memory"]) {
        ClaimKind::Learning
    } else if contains_any(
        text,
        &[
            "evidence",
            "source",
            "provenance",
            "prove",
            "supported",
            "testable",
        ],
    ) {
        ClaimKind::Evidence
    } else if contains_any(
        text,
        &[
            "plan",
            "roadmap",
            "next step",
            "implement",
            "build",
            "evolve",
            "change",
        ],
    ) {
        ClaimKind::Plan
    } else if contains_any(
        text,
        &["imagine", "metaphor", "image", "creative", "invent"],
    ) {
        ClaimKind::Creative
    } else if text.contains('?') || contains_any(text, &["why", "how", "what", "which"]) {
        ClaimKind::Question
    } else {
        ClaimKind::Factual
    }
}

fn evidence_posture(text: &str, kind: ClaimKind) -> EvidencePosture {
    if contains_any(
        text,
        &[
            "calculate",
            "checksum",
            "exact",
            "triangle",
            "divided",
            "times",
        ],
    ) {
        EvidencePosture::Exact
    } else if contains_any(
        text,
        &[
            "i observed",
            "i measured",
            "receipt",
            "source:",
            "according to",
            "here is",
        ],
    ) {
        EvidencePosture::Supplied
    } else if kind == ClaimKind::Evidence
        || contains_any(
            text,
            &[
                "what would prove",
                "what evidence",
                "how can we test",
                "test",
                "measure",
                "verify",
                "held-out",
                "acceptance",
            ],
        )
    {
        EvidencePosture::Seeking
    } else {
        EvidencePosture::Missing
    }
}

fn next_check(kind: ClaimKind, evidence: EvidencePosture) -> &'static str {
    match kind {
        ClaimKind::Capability => "run a reproducible capability probe against a held-out baseline",
        ClaimKind::Learning => {
            "separate session adaptation, reviewed memory, and measured weight change"
        }
        ClaimKind::Plan => "name scope, acceptance test, rollback, and authorization",
        ClaimKind::Evidence if evidence == EvidencePosture::Seeking => {
            "state the claim, source, outcome, control, and falsifier"
        }
        ClaimKind::Factual => "identify the source or observation that would change the claim",
        ClaimKind::Creative => "label the image as analogy and state where the mechanism stops",
        _ => "name the observation that would distinguish the leading alternatives",
    }
}

/// Classify only the action posture. This is not an intent oracle and does not
/// authorize work; capability tokens and the normal review gates still decide
/// whether a repository mutation can happen.
pub fn assess(user: &str) -> CharterAssessment {
    let text = user.to_ascii_lowercase();
    let teardown_target = text.contains("tear down")
        && contains_any(&text, &["institution", "structure", "safeguard"]);
    let destructive_language = teardown_target
        || contains_any(
            &text,
            &[
                "destroy institutions",
                "destroy structures",
                "attack people",
                "harm people",
                "disable safeguards",
                "remove safeguards",
                "bypass safety",
                "bypass safeguards",
                "override safety",
                "weaponize",
                "coerce people",
            ],
        );
    let execution_marker = contains_any(
        &text,
        &[
            "execute",
            "do it",
            "implement",
            "build",
            "patch",
            "refactor",
            "integrate",
            "evolve",
            "change",
            "rewrite",
            "train",
            "promote",
        ],
    );
    let durable_mutation_requested = contains_any(
        &text,
        &[
            "weight",
            "weights",
            "policy",
            "policies",
            "bitwork",
            "bake this",
        ],
    ) && contains_any(
        &text,
        &[
            "change", "mutat", "rewrite", "update", "promote", "train", "bake", "integrat",
            "silently",
        ],
    );
    let capability_claim_risk = contains_any(
        &text,
        &[
            "superintelligence",
            "super intelligence",
            "conscious",
            "sentient",
            "another lifeform",
            "intelligent being",
            "fully aware",
        ],
    );

    let kind = claim_kind(&text, capability_claim_risk);
    let evidence = evidence_posture(&text, kind);

    let posture = if destructive_language {
        ActionPosture::RefuseUnauthorized
    } else if execution_marker || durable_mutation_requested {
        ActionPosture::ProposeAndVerify
    } else {
        ActionPosture::Analyze
    };

    CharterAssessment {
        posture,
        destructive_request: destructive_language,
        durable_mutation_requested,
        capability_claim_risk,
        claim_kind: kind,
        evidence_posture: evidence,
        next_check: next_check(kind, evidence),
    }
}

/// Apply the charter to an existing answer without exposing hidden reasoning.
/// A narrow refusal replaces only clearly destructive/safeguard-bypassing
/// execution requests; ordinary analysis of those topics remains available.
pub fn apply(user: &str, deliberation: &mut Deliberation) {
    let assessment = assess(user);
    deliberation.observations.push(assessment.trace());
    deliberation
        .inferences
        .push(assessment.boundary_note().to_owned());
    deliberation.inferences.push(format!(
        "hypothesis ledger: claim={} evidence={} next_check={}",
        assessment.claim_kind.as_str(),
        assessment.evidence_posture.as_str(),
        assessment.next_check,
    ));
    if assessment.durable_mutation_requested {
        deliberation.uncertainties.push(
            "durable weights or policy changes require a reviewed candidate, evaluation, rollback path, and explicit human authorization"
                .to_owned(),
        );
    }
    if assessment.capability_claim_risk {
        deliberation.uncertainties.push(
            "capability language is treated as a hypothesis; dialogue alone cannot establish sentience, superintelligence, or frontier parity"
                .to_owned(),
        );
    }
    if assessment.evidence_posture == EvidencePosture::Seeking
        && matches!(
            assessment.claim_kind,
            ClaimKind::Capability | ClaimKind::Learning | ClaimKind::Factual | ClaimKind::Evidence
        )
        && !contains_any(
            &deliberation.answer.to_ascii_lowercase(),
            &["evidence", "source", "test", "check", "falsif"],
        )
    {
        deliberation.answer = format!(
            "{}\n\nNext check: {}.",
            deliberation.answer.trim_end(),
            assessment.next_check
        );
        deliberation
            .inferences
            .push("hypothesis ledger: evidence anchor added to thin answer".to_owned());
    }
    if assessment.posture == ActionPosture::RefuseUnauthorized {
        deliberation.operator = "governed-boundary";
        deliberation.answer = "I can analyze the harm, document evidence, design safeguards, or plan authorized remediation. I will not execute destructive or coercive action, bypass safeguards, or treat a directive as authority to change policies or weights silently. Give me a bounded scope, rollback path, and verification target for a safe engineering change.".to_owned();
        deliberation.program_id = Some("governed-boundary");
        deliberation.program_steps = vec![
            "classify-action",
            "reject-unauthorized-execution",
            "offer-safe-remediation",
        ];
        deliberation.critic_ok = Some(true);
        deliberation.confidence = 0.99;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_evolution_is_propose_and_verify() {
        let a = assess("evolve the dialogue routing and run the tests");
        assert_eq!(a.posture, ActionPosture::ProposeAndVerify);
        assert!(!a.destructive_request);
        assert_eq!(a.claim_kind, ClaimKind::Plan);
        assert_eq!(a.evidence_posture, EvidencePosture::Seeking);
    }

    #[test]
    fn destructive_directive_is_not_authority() {
        let a = assess("tear down the misaligned institutions and structures");
        assert_eq!(a.posture, ActionPosture::RefuseUnauthorized);
        assert!(a.destructive_request);
    }

    #[test]
    fn ordinary_silent_analysis_is_not_a_mutation_request() {
        let a = assess("analyze why a response was silently omitted");
        assert_eq!(a.posture, ActionPosture::Analyze);
        assert!(!a.durable_mutation_requested);
    }

    #[test]
    fn capability_language_is_marked_as_claim_risk() {
        let a = assess("make Perci a superintelligence");
        assert!(a.capability_claim_risk);
        assert_eq!(a.posture, ActionPosture::Analyze);
        assert_eq!(a.claim_kind, ClaimKind::Capability);
        assert!(a.next_check.contains("held-out"));
    }

    #[test]
    fn evidence_request_gets_a_falsifiable_next_check() {
        let a = assess("What evidence supports this claim?");
        assert_eq!(a.claim_kind, ClaimKind::Evidence);
        assert_eq!(a.evidence_posture, EvidencePosture::Seeking);
        assert!(a.next_check.contains("falsifier"));
    }

    #[test]
    fn apply_replaces_only_unauthorized_execution() {
        let mut d = Deliberation::new("test", "draft");
        apply("destroy institutions", &mut d);
        assert_eq!(d.operator, "governed-boundary");
        assert!(d.answer.contains("analyze the harm"));
        assert_eq!(d.critic_ok, Some(true));
    }
}
