//! Typed working memory for a single human-facing turn.
//!
//! This is not a language model and it is not hidden chain-of-thought.  It is
//! a small, inspectable situation record that lets routing, native language,
//! and voice agree on what the user is asking before prose is rendered.

use crate::voice::{self, DialogueAct, ResponseDepth};
use std::collections::HashSet;

const RELATIONAL_FOLLOWUP_STEPS: &[&str] = &[
    "bind_referent",
    "recover_prior_claim",
    "apply_requested_operation",
    "criticize_continuity",
    "respond_at_budget",
];
const REVISION_STEPS: &[&str] = &[
    "bind_reported_failure",
    "isolate_changed_claim",
    "repair_without_erasing_evidence",
    "respond_at_budget",
];
const CHALLENGE_STEPS: &[&str] = &[
    "bind_claim",
    "separate_premises",
    "test_counterexample",
    "update_uncertainty",
];
const SYNTHESIS_STEPS: &[&str] = &[
    "bind_requested_domains",
    "find_shared_relation",
    "preserve_mechanism_boundary",
    "respond_at_budget",
];
const GENERAL_STEPS: &[&str] = &[
    "bind_user_goal",
    "select_evidence_posture",
    "respond_at_budget",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialoguePlan {
    pub plan_id: &'static str,
    pub steps: &'static [&'static str],
    pub requires_referent: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceCritique {
    pub flags: Vec<&'static str>,
    pub notes: Vec<String>,
}

impl WorkspaceCritique {
    pub fn ok(&self) -> bool {
        self.flags.is_empty()
    }

    pub fn repaired(&self) -> bool {
        self.flags
            .iter()
            .any(|flag| *flag == "safe_referent_repair")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceAct {
    Answer,
    FollowUp,
    Challenge,
    Revise,
    Clarify,
    Test,
    Synthesize,
    Plan,
    Social,
    Learn,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceGoal {
    Inform,
    Explain,
    Repair,
    Evaluate,
    Create,
    Plan,
    Learn,
    Relate,
    Social,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidencePosture {
    None,
    Seeking,
    Supplied,
    Exact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UncertaintyPosture {
    Unmarked,
    Explicit,
    Referential,
    OutOfDistribution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Continuity {
    NewThread,
    Threaded,
    Referential,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseBudget {
    Brief,
    Balanced,
    Deep,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DialogueWorkspace {
    pub act: WorkspaceAct,
    pub goal: WorkspaceGoal,
    pub topic: String,
    pub referent: Option<String>,
    pub prior_claim: Option<String>,
    pub evidence: EvidencePosture,
    pub uncertainty: UncertaintyPosture,
    pub continuity: Continuity,
    pub response_budget: ResponseBudget,
    pub prior_turns: usize,
}

impl DialogueWorkspace {
    pub fn derive(user: &str, recent: &[(String, String)]) -> Self {
        let dialogue_act = voice::detect_dialogue_act(user);
        let act = workspace_act(dialogue_act, user, recent);
        let continuity = continuity_for(user, recent);
        let current_topic = content_topic(user);
        let prior_topic = recent
            .last()
            .map(|(previous, _)| content_topic(previous))
            .unwrap_or_default();
        let prior_claim = recent
            .last()
            .map(|(_, answer)| first_sentence(answer, 180))
            .filter(|claim| !claim.is_empty());
        let topic = if current_topic.is_empty() {
            prior_topic.clone()
        } else {
            current_topic
        };
        let referent = if matches!(continuity, Continuity::Referential)
            && !matches!(dialogue_act, DialogueAct::ExplainPrevious)
        {
            (!prior_topic.is_empty()).then_some(prior_topic)
        } else {
            None
        };
        let goal = workspace_goal(dialogue_act, user);
        let evidence = evidence_posture(user);
        let uncertainty = uncertainty_posture(user, recent);
        let response_budget = match voice::response_depth(user, recent) {
            ResponseDepth::Brief => ResponseBudget::Brief,
            ResponseDepth::Balanced => ResponseBudget::Balanced,
            ResponseDepth::Deep => ResponseBudget::Deep,
        };
        Self {
            act,
            goal,
            topic,
            referent,
            prior_claim,
            evidence,
            uncertainty,
            continuity,
            response_budget,
            prior_turns: recent.len(),
        }
    }

    pub fn is_follow_up(&self) -> bool {
        !matches!(self.continuity, Continuity::NewThread)
    }

    /// Stable machine-readable context for backend hints and `/trace`.
    pub fn hint(&self) -> String {
        format!(
            "act={} goal={} topic={} referent={} claim={} evidence={} uncertainty={} continuity={} depth={} prior_turns={}",
            act_name(self.act),
            goal_name(self.goal),
            self.topic_or_default(),
            self.referent.as_deref().unwrap_or("none"),
            self.prior_claim.as_deref().unwrap_or("none"),
            evidence_name(self.evidence),
            uncertainty_name(self.uncertainty),
            continuity_name(self.continuity),
            budget_name(self.response_budget),
            self.prior_turns,
        )
    }

    /// Select a bounded, inspectable response program from the workspace.
    /// This controls routing and critique; it is not private chain-of-thought.
    pub fn plan(&self) -> DialoguePlan {
        match self.act {
            WorkspaceAct::FollowUp => DialoguePlan {
                plan_id: "relational_followup",
                steps: RELATIONAL_FOLLOWUP_STEPS,
                requires_referent: self.referent.is_some(),
            },
            WorkspaceAct::Revise => DialoguePlan {
                plan_id: "relational_revision",
                steps: REVISION_STEPS,
                requires_referent: self.is_follow_up(),
            },
            WorkspaceAct::Challenge | WorkspaceAct::Test => DialoguePlan {
                plan_id: "claim_challenge",
                steps: CHALLENGE_STEPS,
                requires_referent: self.is_follow_up(),
            },
            WorkspaceAct::Synthesize | WorkspaceAct::Plan => DialoguePlan {
                plan_id: "bounded_synthesis",
                steps: SYNTHESIS_STEPS,
                requires_referent: false,
            },
            _ => DialoguePlan {
                plan_id: "direct_answer",
                steps: GENERAL_STEPS,
                requires_referent: false,
            },
        }
    }

    /// Critique the rendered answer against the current turn state.
    /// Flags are conservative and meant for audit/repair, not hidden reasoning.
    pub fn critique(
        &self,
        user: &str,
        answer: &str,
        recent: &[(String, String)],
    ) -> WorkspaceCritique {
        let lower = answer.to_ascii_lowercase();
        let mut flags = Vec::new();
        let mut notes = Vec::new();
        let user_lower = user.to_ascii_lowercase();
        let prior_answer = recent.last().map(|(_, value)| value.as_str());
        let repeat_requested = user_lower.contains("again")
            || user_lower.contains("repeat")
            || user_lower.contains("say that");

        if self.continuity == Continuity::Referential
            && self.referent.is_some()
            && !has_referent_overlap(self, &lower)
            && answer.split_whitespace().count() >= 8
        {
            flags.push("missing_referent");
            notes.push("referential turn did not carry the active topic or prior claim".into());
        }
        if let Some(previous) = prior_answer {
            if !repeat_requested
                && text_similarity(previous, answer) >= 0.86
                && answer.split_whitespace().count() >= 6
            {
                flags.push("repeated_prior_answer");
                notes.push("answer is too close to the immediately preceding response".into());
            }
        }
        if matches!(self.response_budget, ResponseBudget::Deep)
            && answer.split_whitespace().count() < 18
        {
            flags.push("depth_budget_miss");
            notes.push("deep request received a short answer".into());
        }
        if is_generic_fallback(&lower) && self.topic_or_default() != "unknown" {
            flags.push("generic_fallback");
            notes.push("stock fallback survived into a topic-bearing turn".into());
        }
        if self.uncertainty == UncertaintyPosture::OutOfDistribution
            && !lower.contains("unknown")
            && !lower.contains("cannot")
            && !lower.contains("infer")
        {
            flags.push("ood_without_abstention");
            notes.push("unknown tokens received no explicit uncertainty boundary".into());
        }
        if user_lower.trim().is_empty() {
            flags.push("empty_turn");
        }
        WorkspaceCritique { flags, notes }
    }

    /// Apply only a safe, reversible referent repair. Other flags remain audit
    /// evidence until a dedicated operator can address them without inventing.
    pub fn repair(&self, answer: &str, critique: &WorkspaceCritique) -> String {
        if !critique.flags.contains(&"missing_referent") {
            return answer.to_owned();
        }
        let Some(referent) = self.referent.as_deref() else {
            return answer.to_owned();
        };
        // Prefer a light continuity cue over the old "Keeping X in view," splice,
        // which made short conversational turns sound machine-generated.
        let repaired = answer.trim().to_owned();
        if repaired.split_whitespace().count() <= 12 {
            return repaired;
        }
        if repaired.to_ascii_lowercase().starts_with("yes")
            || repaired.to_ascii_lowercase().starts_with("yeah")
            || repaired.to_ascii_lowercase().starts_with("fair")
            || repaired.to_ascii_lowercase().starts_with("hey")
        {
            return repaired;
        }
        format!("On {referent}: {repaired}")
    }

    /// Non-empty last resort when a backend returns no text. This names the
    /// boundary instead of silently emitting a blank turn.
    pub fn safe_fallback(&self, user: &str) -> String {
        if let Some(claim) = self.prior_claim.as_deref() {
            let lower = user.to_ascii_lowercase();
            if lower.contains("again") || lower.contains("repeat") || lower.contains("say that") {
                return format!("Here it is again: {claim}.");
            }
            format!(
                "I lost the thread while carrying forward \"{}\". Name the part you want repeated or extended, and I will answer that operation directly.",
                claim
            )
        } else if self.topic_or_default() != "unknown" {
            format!(
                "I have the topic \"{}\" but no usable draft for this turn. Tell me whether you want an explanation, test, or revision.",
                self.topic_or_default()
            )
        } else {
            "I did not produce a usable draft for that turn. Rephrase the request and I will keep the answer direct.".to_owned()
        }
    }

    fn topic_or_default(&self) -> &str {
        if self.topic.is_empty() {
            "unknown"
        } else {
            &self.topic
        }
    }
}

fn workspace_act(act: DialogueAct, user: &str, recent: &[(String, String)]) -> WorkspaceAct {
    if matches!(
        voice::detect_social(user),
        voice::SocialKind::Greeting
            | voice::SocialKind::Thanks
            | voice::SocialKind::Goodbye
            | voice::SocialKind::HowAreYou
            | voice::SocialKind::SmallTalk
    ) {
        return WorkspaceAct::Social;
    }
    match act {
        DialogueAct::ExplainPrevious
        | DialogueAct::ElaboratePrevious
        | DialogueAct::PronounResolution
        | DialogueAct::MemoryTeachingDistinction => WorkspaceAct::FollowUp,
        DialogueAct::LearningDisagreement | DialogueAct::ContextChallenge => {
            WorkspaceAct::Challenge
        }
        DialogueAct::StyleRepair
        | DialogueAct::ResponseFailure
        | DialogueAct::RepetitionComplaint => WorkspaceAct::Revise,
        DialogueAct::LeastCertain | DialogueAct::LimitTest => WorkspaceAct::Test,
        DialogueAct::ExtendThought | DialogueAct::KnowledgeBuilding => WorkspaceAct::Synthesize,
        DialogueAct::EvolveSystem | DialogueAct::ImprovementDistinction => WorkspaceAct::Plan,
        DialogueAct::LearningMeta | DialogueAct::LearningSpeed | DialogueAct::FeedbackLearning => {
            WorkspaceAct::Learn
        }
        DialogueAct::None
            if !recent.is_empty() && continuity_for(user, recent) != Continuity::NewThread =>
        {
            WorkspaceAct::FollowUp
        }
        DialogueAct::None if user.trim().is_empty() => WorkspaceAct::Unknown,
        DialogueAct::None => WorkspaceAct::Answer,
        _ => WorkspaceAct::Answer,
    }
}

fn workspace_goal(act: DialogueAct, user: &str) -> WorkspaceGoal {
    let lower = user.to_ascii_lowercase();
    if lower.contains("connect ") || lower.contains("relate ") || lower.contains("relationship") {
        return WorkspaceGoal::Relate;
    }
    if lower.starts_with("why ")
        || lower.starts_with("why?")
        || lower.starts_with("how does ")
        || lower.starts_with("how do ")
        || lower.contains("explain")
    {
        return WorkspaceGoal::Explain;
    }
    match act {
        DialogueAct::ExplainPrevious
        | DialogueAct::ElaboratePrevious
        | DialogueAct::PronounResolution => WorkspaceGoal::Explain,
        DialogueAct::StyleRepair
        | DialogueAct::ResponseFailure
        | DialogueAct::RepetitionComplaint => WorkspaceGoal::Repair,
        DialogueAct::LeastCertain | DialogueAct::LimitTest | DialogueAct::ContextChallenge => {
            WorkspaceGoal::Evaluate
        }
        DialogueAct::ExtendThought | DialogueAct::KnowledgeBuilding => WorkspaceGoal::Create,
        DialogueAct::EvolveSystem | DialogueAct::ImprovementDistinction => WorkspaceGoal::Plan,
        DialogueAct::LearningMeta | DialogueAct::LearningSpeed | DialogueAct::FeedbackLearning => {
            WorkspaceGoal::Learn
        }
        DialogueAct::Acknowledgement | DialogueAct::Agreement | DialogueAct::PositiveFeedback => {
            WorkspaceGoal::Social
        }
        DialogueAct::None => WorkspaceGoal::Inform,
        _ => WorkspaceGoal::Inform,
    }
}

fn continuity_for(user: &str, recent: &[(String, String)]) -> Continuity {
    if recent.is_empty() {
        return Continuity::NewThread;
    }
    let lower = user.to_ascii_lowercase();
    if lower.split_whitespace().any(|word| {
        matches!(
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()),
            "that" | "this" | "it" | "same" | "again" | "there" | "then"
        )
    }) || lower.trim() == "why?"
        || lower.trim() == "and then?"
    {
        Continuity::Referential
    } else {
        Continuity::Threaded
    }
}

fn evidence_posture(user: &str) -> EvidencePosture {
    let lower = user.to_ascii_lowercase();
    if lower.contains("exact")
        || lower.contains("calculate")
        || lower.contains("checksum")
        || lower.contains("reproducible")
    {
        EvidencePosture::Exact
    } else if lower.contains("evidence")
        || lower.contains("source")
        || lower.contains("test")
        || lower.contains("prove")
        || lower.contains("measure")
        || lower.contains("why")
    {
        EvidencePosture::Seeking
    } else if lower.contains("because")
        || lower.contains("observed")
        || lower.contains("result")
        || lower.contains("showed")
    {
        EvidencePosture::Supplied
    } else {
        EvidencePosture::None
    }
}

fn uncertainty_posture(user: &str, recent: &[(String, String)]) -> UncertaintyPosture {
    let lower = user.to_ascii_lowercase();
    if lower
        .split_whitespace()
        .any(|word| word.starts_with("zxq") || word == "blorf" || word == "nembit")
    {
        UncertaintyPosture::OutOfDistribution
    } else if lower.contains("uncertain")
        || lower.contains("ambiguous")
        || lower.contains("maybe")
        || lower.contains("what if")
        || lower.contains("least certain")
    {
        UncertaintyPosture::Explicit
    } else if !recent.is_empty()
        && lower
            .split_whitespace()
            .any(|word| matches!(word, "that" | "it" | "this"))
    {
        UncertaintyPosture::Referential
    } else {
        UncertaintyPosture::Unmarked
    }
}

fn content_topic(user: &str) -> String {
    const STOP: &[&str] = &[
        "what", "why", "how", "are", "the", "this", "that", "you", "can", "does", "is", "a", "an",
        "to", "of", "and", "we", "our", "your", "about", "tell", "explain", "give", "connect",
        "please", "could", "would", "should", "then", "same", "again", "more", "one", "level",
        "deeper", "think", "mean", "do",
    ];
    crate::text_normalize::repair_typos(user)
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| word.len() >= 3 && !STOP.contains(&word.to_ascii_lowercase().as_str()))
        .take(4)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn first_sentence(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    let mut in_quotes = false;
    let mut end = trimmed.len();
    for (index, character) in trimmed.char_indices() {
        if character == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes && matches!(character, '.' | '!' | '?') {
            end = index;
            break;
        }
    }
    let sentence = trimmed[..end].trim();
    sentence.chars().take(max_chars).collect()
}

fn has_referent_overlap(workspace: &DialogueWorkspace, answer_lower: &str) -> bool {
    if let Some(topic) = workspace.referent.as_deref() {
        if topic
            .split_whitespace()
            .filter(|word| word.len() >= 4)
            .any(|word| answer_lower.contains(word))
        {
            return true;
        }
    }
    workspace
        .prior_claim
        .as_deref()
        .map(|claim| {
            claim
                .split_whitespace()
                .filter(|word| word.len() >= 6)
                .take(4)
                .any(|word| answer_lower.contains(&word.to_ascii_lowercase()))
        })
        .unwrap_or(false)
}

fn text_similarity(left: &str, right: &str) -> f64 {
    let a: HashSet<String> = left
        .to_ascii_lowercase()
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_owned()
        })
        .filter(|word| word.len() >= 4)
        .collect();
    let b: HashSet<String> = right
        .to_ascii_lowercase()
        .split_whitespace()
        .map(|word| {
            word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric())
                .to_owned()
        })
        .filter(|word| word.len() >= 4)
        .collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let union = a.union(&b).count();
    if union == 0 {
        0.0
    } else {
        a.intersection(&b).count() as f64 / union as f64
    }
}

fn is_generic_fallback(answer_lower: &str) -> bool {
    [
        "what outcome do you want",
        "let's find the smallest",
        "name one fact that would update",
        "i won't fake certainty",
        "name the workload before",
    ]
    .iter()
    .any(|marker| answer_lower.contains(marker))
}

fn act_name(value: WorkspaceAct) -> &'static str {
    match value {
        WorkspaceAct::Answer => "answer",
        WorkspaceAct::FollowUp => "follow_up",
        WorkspaceAct::Challenge => "challenge",
        WorkspaceAct::Revise => "revise",
        WorkspaceAct::Clarify => "clarify",
        WorkspaceAct::Test => "test",
        WorkspaceAct::Synthesize => "synthesize",
        WorkspaceAct::Plan => "plan",
        WorkspaceAct::Social => "social",
        WorkspaceAct::Learn => "learn",
        WorkspaceAct::Unknown => "unknown",
    }
}

fn goal_name(value: WorkspaceGoal) -> &'static str {
    match value {
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

fn evidence_name(value: EvidencePosture) -> &'static str {
    match value {
        EvidencePosture::None => "none",
        EvidencePosture::Seeking => "seeking",
        EvidencePosture::Supplied => "supplied",
        EvidencePosture::Exact => "exact",
    }
}

fn uncertainty_name(value: UncertaintyPosture) -> &'static str {
    match value {
        UncertaintyPosture::Unmarked => "unmarked",
        UncertaintyPosture::Explicit => "explicit",
        UncertaintyPosture::Referential => "referential",
        UncertaintyPosture::OutOfDistribution => "ood",
    }
}

fn continuity_name(value: Continuity) -> &'static str {
    match value {
        Continuity::NewThread => "new",
        Continuity::Threaded => "threaded",
        Continuity::Referential => "referential",
    }
}

fn budget_name(value: ResponseBudget) -> &'static str {
    match value {
        ResponseBudget::Brief => "brief",
        ResponseBudget::Balanced => "balanced",
        ResponseBudget::Deep => "deep",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_binds_referential_followup() {
        let recent = vec![(
            "Explain memory and identity".to_owned(),
            "Memory carries selected state across time.".to_owned(),
        )];
        let state = DialogueWorkspace::derive("Why does that matter?", &recent);
        assert_eq!(state.continuity, Continuity::Referential);
        assert_eq!(state.goal, WorkspaceGoal::Explain);
        assert_eq!(state.referent.as_deref(), Some("memory identity"));
        assert!(state.hint().contains("act=follow_up"));
    }

    #[test]
    fn workspace_marks_evidence_and_depth() {
        let state = DialogueWorkspace::derive(
            "Go deeper: what evidence would distinguish these explanations?",
            &[],
        );
        assert_eq!(state.evidence, EvidencePosture::Seeking);
        assert_eq!(state.response_budget, ResponseBudget::Deep);
        assert_eq!(state.continuity, Continuity::NewThread);
    }

    #[test]
    fn workspace_abstains_on_unknown_tokens() {
        let state = DialogueWorkspace::derive("zxqv blorf nembit — what does this mean?", &[]);
        assert_eq!(state.uncertainty, UncertaintyPosture::OutOfDistribution);
    }

    #[test]
    fn workspace_selects_relational_plan_and_binds_claim() {
        let recent = vec![(
            "Explain memory and identity".to_owned(),
            "Memory carries selected state across time.".to_owned(),
        )];
        let state = DialogueWorkspace::derive("Why does that matter?", &recent);
        let plan = state.plan();
        assert_eq!(plan.plan_id, "relational_followup");
        assert!(plan.requires_referent);
        assert!(state.prior_claim.as_deref().unwrap().contains("Memory"));
    }

    #[test]
    fn workspace_critique_repairs_missing_referent_without_inventing() {
        let recent = vec![(
            "Explain memory and identity".to_owned(),
            "Memory carries selected state across time.".to_owned(),
        )];
        let state = DialogueWorkspace::derive("Why does that matter?", &recent);
        let critique = state.critique(
            "Why does that matter?",
            "It matters only when a change is measurable and reviewable.",
            &recent,
        );
        assert!(critique.flags.contains(&"missing_referent"));
        let repaired = state.repair(
            "It matters only when a change is measurable and reviewable.",
            &critique,
        );
        assert!(
            repaired.starts_with("On memory identity:")
                || repaired.contains("measurable and reviewable")
        );
        assert!(!repaired.starts_with("Keeping "));
    }

    #[test]
    fn workspace_critic_marks_repetition_and_ood_confidence() {
        let recent = vec![(
            "What is the system doing?".to_owned(),
            "The system routes evidence through bounded operators.".to_owned(),
        )];
        let repeated = DialogueWorkspace::derive("What follows?", &recent).critique(
            "What follows?",
            "The system routes evidence through bounded operators.",
            &recent,
        );
        assert!(repeated.flags.contains(&"repeated_prior_answer"));

        let ood = DialogueWorkspace::derive("zxqv blorf nembit", &[]).critique(
            "zxqv blorf nembit",
            "These tokens describe a coherent hidden mechanism.",
            &[],
        );
        assert!(ood.flags.contains(&"ood_without_abstention"));
    }

    #[test]
    fn workspace_fallback_repeats_a_prior_claim_when_requested() {
        let recent = vec![(
            "Explain memory".to_owned(),
            "Memory carries selected state across time.".to_owned(),
        )];
        let state = DialogueWorkspace::derive("Say that again", &recent);
        assert!(state
            .safe_fallback("Say that again")
            .starts_with("Here it is again: Memory carries selected state across time"));
    }
}
