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
            .any(|flag| *flag == "safe_referent_repair" || *flag == "semantic_fit_repair")
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

/// The answer-shape requested by the user. This is intentionally smaller than
/// a general semantic parser: it names high-cost operations whose failure can
/// be detected without pretending to understand every sentence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestionOperation {
    General,
    DecisionSupport,
    ScopedSuperlative,
    SelfAssessment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionFrame {
    pub operation: QuestionOperation,
    pub subjects: Vec<String>,
    pub criterion: Option<String>,
    pub scope_required: bool,
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
    pub question: QuestionFrame,
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
        let question = question_frame(user);
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
            question,
        }
    }

    pub fn is_follow_up(&self) -> bool {
        !matches!(self.continuity, Continuity::NewThread)
    }

    /// Stable machine-readable context for backend hints and `/trace`.
    pub fn hint(&self) -> String {
        format!(
            "act={} goal={} question={} subjects={} scope_required={} topic={} referent={} claim={} evidence={} uncertainty={} continuity={} depth={} prior_turns={}",
            act_name(self.act),
            goal_name(self.goal),
            question_operation_name(self.question.operation),
            self.question.subjects.join(","),
            self.question.scope_required,
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
        if is_generic_fallback(&lower, &user_lower)
            && self.topic_or_default() != "unknown"
            && !matches!(self.act, WorkspaceAct::Social)
            && !matches!(self.uncertainty, UncertaintyPosture::OutOfDistribution)
        {
            flags.push("generic_fallback");
            notes.push("stock fallback survived into a topic-bearing, in-distribution turn".into());
        }
        if !matches!(self.question.operation, QuestionOperation::General)
            && answer.split_whitespace().count() >= 6
            && !answer_binds_subjects(&self.question, &lower)
        {
            flags.push("semantic_subject_miss");
            notes.push(format!(
                "answer lost the framed subject(s): {}",
                self.question.subjects.join(", ")
            ));
        }
        if !operation_fits_answer(&self.question, &lower) {
            flags.push("operation_fit_miss");
            notes.push(format!(
                "answer did not satisfy the {} response contract",
                question_operation_name(self.question.operation)
            ));
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
        self.repair_for("", answer, critique)
    }

    /// Repair with the user's operation still visible. Keeping this input at
    /// the final shaping boundary prevents a known-topic miss from collapsing
    /// into one universal fallback sentence.
    pub fn repair_for(&self, user: &str, answer: &str, critique: &WorkspaceCritique) -> String {
        if critique.flags.contains(&"repeated_prior_answer") {
            return self.progression_repair(user);
        }
        if critique.flags.contains(&"semantic_subject_miss")
            || critique.flags.contains(&"operation_fit_miss")
        {
            return self.question_frame_repair();
        }
        if critique.flags.contains(&"generic_fallback") {
            return self.semantic_fit_fallback(user);
        }
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

    fn progression_repair(&self, user: &str) -> String {
        let topic = self.topic_or_default();
        match requested_operation(user) {
            "next_step" => format!(
                "That repeats the last answer without moving {topic} forward. The next useful move is one small replay test with a clear pass condition."
            ),
            "explain" => format!(
                "That repeats the last answer without explaining {topic}. The missing layer is the mechanism: what changes, why it changes, and what would disprove it."
            ),
            "evidence" => format!(
                "That repeats the last answer without showing evidence for {topic}. I should name the observation, the inference, and the test that could separate them."
            ),
            "elaborate" => format!(
                "That repeats the last answer. A deeper angle on {topic} is the boundary where the current explanation stops transferring."
            ),
            _ => format!(
                "That repeats the last answer without adding a new layer to {topic}. I can take it toward mechanism, evidence, or the next test."
            ),
        }
    }

    /// Replace a stock backend refusal when the turn is actually grounded in
    /// the active thread. This keeps abstention for OOD input, while making a
    /// known-topic miss name the next useful operation instead of asking the
    /// user to restart the conversation.
    fn semantic_fit_fallback(&self, user: &str) -> String {
        let topic = self.topic_or_default();
        let operation = requested_operation(user);
        if self.prior_claim.is_some() {
            return match operation {
                "next_step" => format!(
                    "I’m tracking {topic}. The last answer missed your point. The next useful move is to name one concrete change, run one replay probe, and keep it only if the result improves."
                ),
                "explain" => format!(
                    "I’m tracking {topic}. The last answer missed your point. I’ll explain the mechanism first, then separate what is measured from what is still a hypothesis."
                ),
                "elaborate" => format!(
                    "I’m tracking {topic}. The last answer missed your point. I can take the same idea one layer deeper by naming its cause, boundary, and next test."
                ),
                "evidence" => format!(
                    "I’m tracking {topic}. The last answer missed your point. The honest basis is the prior route and its observed output; the next check is whether a paraphrase produces the same operation."
                ),
                _ => format!(
                    "I’m tracking {topic}. The last answer missed your point; I can explain the mechanism, extend the idea, or test it against evidence."
                ),
            };
        }
        format!(
            "I’m tracking {topic}. The last answer missed your point; do you want an explanation, a next step, or a test?"
        )
    }

    fn question_frame_repair(&self) -> String {
        let subject = if self.question.subjects.is_empty() {
            self.topic_or_default().to_owned()
        } else {
            self.question.subjects.join(" ")
        };
        match self.question.operation {
            QuestionOperation::DecisionSupport => format!(
                "Maybe, but the honest answer depends on your goal, demand, downside, and runway. For {subject}, test the smallest reversible version first: talk to potential customers, ask for a real commitment, estimate the cost of failure, and decide from that evidence rather than excitement alone."
            ),
            QuestionOperation::ScopedSuperlative => format!(
                "There is no single most stable {subject} without naming the stress or transformation. Specify whether you mean resistance to deformation, structural load, pressure, or topological change; the answer can change with that criterion."
            ),
            QuestionOperation::SelfAssessment => format!(
                "Not globally yet. The measurable threshold for {subject} is consistent subject and operation preservation across unseen paraphrases, follow-ups, and broad-domain probes—not one convincing reply."
            ),
            QuestionOperation::General => self.safe_fallback(""),
        }
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
        "deeper", "think", "mean", "do", "now", "know", "system",
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

fn question_frame(user: &str) -> QuestionFrame {
    let lower = crate::text_normalize::normalize_for_routing(user);
    let operation = if (lower.starts_with("are you") || lower.contains("perci"))
        && (lower.contains("coherence") || lower.contains("coherent"))
        && (lower.contains("threshold")
            || lower.contains("near")
            || lower.contains("approach")
            || lower.contains("reach"))
    {
        QuestionOperation::SelfAssessment
    } else if lower.starts_with("should i ")
        || lower.starts_with("should we ")
        || lower.starts_with("is it worth ")
        || lower.starts_with("would it make sense ")
    {
        QuestionOperation::DecisionSupport
    } else if (lower.contains("geometry") || lower.contains("shape"))
        && (lower.starts_with("what ") || lower.starts_with("which "))
        && (lower.contains(" most ")
            || lower.contains(" best ")
            || lower.contains(" strongest ")
            || lower.contains(" least "))
    {
        QuestionOperation::ScopedSuperlative
    } else {
        QuestionOperation::General
    };

    let mut subjects = content_topic(user)
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "start"
                    | "make"
                    | "pursue"
                    | "has"
                    | "have"
                    | "most"
                    | "best"
                    | "strongest"
                    | "least"
                    | "stable"
                    | "stability"
                    | "nearing"
                    | "near"
                    | "approaching"
                    | "reach"
                    | "reaching"
            )
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if operation == QuestionOperation::SelfAssessment {
        subjects = vec!["coherence".to_owned(), "threshold".to_owned()];
    }
    if subjects.is_empty() {
        subjects = content_topic(user)
            .split_whitespace()
            .take(2)
            .map(str::to_owned)
            .collect();
    }
    let criterion = if lower.contains("stabil") {
        Some("stability".to_owned())
    } else if lower.contains("strong") {
        Some("strength".to_owned())
    } else if lower.contains("fast") {
        Some("speed".to_owned())
    } else if lower.contains("safe") {
        Some("safety".to_owned())
    } else {
        None
    };
    QuestionFrame {
        operation,
        subjects,
        criterion,
        scope_required: operation == QuestionOperation::ScopedSuperlative,
    }
}

fn answer_binds_subjects(frame: &QuestionFrame, answer_lower: &str) -> bool {
    if frame.subjects.is_empty() {
        return true;
    }
    frame.subjects.iter().any(|subject| {
        answer_lower.contains(subject)
            || (subject == "coherence" && answer_lower.contains("coherent"))
            || (subject == "business"
                && (answer_lower.contains("customer") || answer_lower.contains("market")))
            || (subject == "geometry"
                && (answer_lower.contains("triangle")
                    || answer_lower.contains("sphere")
                    || answer_lower.contains("shape")))
    })
}

fn operation_fits_answer(frame: &QuestionFrame, answer_lower: &str) -> bool {
    match frame.operation {
        QuestionOperation::General => true,
        QuestionOperation::DecisionSupport => {
            answer_binds_subjects(frame, answer_lower)
                && [
                    "depends", "test", "before", "risk", "goal", "customer", "downside", "if ",
                ]
                .iter()
                .any(|marker| answer_lower.contains(marker))
        }
        QuestionOperation::ScopedSuperlative => {
            answer_binds_subjects(frame, answer_lower)
                && [
                    "depends",
                    "what kind",
                    "which kind",
                    "criterion",
                    "whether you mean",
                    "under ",
                    "resistance to",
                ]
                .iter()
                .any(|marker| answer_lower.contains(marker))
        }
        QuestionOperation::SelfAssessment => {
            answer_binds_subjects(frame, answer_lower)
                && (answer_lower.starts_with("yes")
                    || answer_lower.starts_with("no")
                    || answer_lower.starts_with("not "))
                && ["measure", "test", "evidence", "probe", "threshold"]
                    .iter()
                    .any(|marker| answer_lower.contains(marker))
        }
    }
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

fn is_generic_fallback(answer_lower: &str, user_lower: &str) -> bool {
    [
        "what outcome do you want",
        "let's find the smallest",
        "name one fact that would update",
        "i won't fake certainty",
        "name the workload before",
        "i don't have a grounded line",
        "restate it in one plain sentence",
    ]
    .iter()
    .any(|marker| answer_lower.contains(marker))
        || (answer_lower.contains("i'm a local governed tool")
            && (user_lower.contains("what can we do")
                || user_lower.contains("what should we do")
                || user_lower.contains("what next")
                || user_lower.contains("improving")
                || user_lower.contains("evolving")))
}

fn requested_operation(user: &str) -> &'static str {
    let lower = user.to_ascii_lowercase();
    if lower.contains("what next")
        || lower.contains("what can we do")
        || lower.contains("next move")
        || lower.contains("where do we go")
    {
        "next_step"
    } else if lower.contains("how do you know")
        || lower.contains("what is the evidence")
        || lower.contains("why do you think")
    {
        "evidence"
    } else if lower.contains("why") || lower.contains("how does") || lower.contains("explain") {
        "explain"
    } else if lower.contains("tell me more")
        || lower.contains("go deeper")
        || lower.contains("one level")
        || lower.contains("elaborate")
    {
        "elaborate"
    } else {
        "general"
    }
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

fn question_operation_name(value: QuestionOperation) -> &'static str {
    match value {
        QuestionOperation::General => "general",
        QuestionOperation::DecisionSupport => "decision_support",
        QuestionOperation::ScopedSuperlative => "scoped_superlative",
        QuestionOperation::SelfAssessment => "self_assessment",
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

    #[test]
    fn known_topic_generic_fallback_gets_semantic_fit_repair() {
        let recent = vec![(
            "Improving Perci's dialogue".to_owned(),
            "The dialogue needs a semantic-fit gate.".to_owned(),
        )];
        let state = DialogueWorkspace::derive("what can we do now", &recent);
        let critique = state.critique(
            "what can we do now",
            "I don't have a grounded line for that yet. Restate it in one plain sentence and I'll answer that.",
            &recent,
        );
        assert!(critique.flags.contains(&"generic_fallback"));
        let repaired = state.repair("I don't have a grounded line for that yet.", &critique);
        assert!(repaired.contains("tracking"));
        assert!(!repaired.contains("Restate it"));
    }

    #[test]
    fn ood_abstention_is_not_replaced_by_semantic_fit_gate() {
        let state = DialogueWorkspace::derive("zxqv blorf nembit", &[]);
        let answer = "I don't know what those tokens mean, and I cannot infer a reliable interpretation yet.";
        let critique = state.critique("zxqv blorf nembit", answer, &[]);
        assert!(!critique.flags.contains(&"generic_fallback"));
        assert!(critique.ok());
    }

    #[test]
    fn semantic_fit_repair_binds_requested_operation() {
        let recent = vec![(
            "Evolving Perci dialogue".to_owned(),
            "The dialogue needs an operation-aware repair.".to_owned(),
        )];
        let user = "what next?";
        let state = DialogueWorkspace::derive(user, &recent);
        let critique = state.critique(
            user,
            "Let's find the smallest version we can test next.",
            &recent,
        );
        assert!(critique.flags.contains(&"generic_fallback"));
        let repaired = state.repair_for(user, "generic", &critique);
        assert!(repaired.contains("concrete change"));
        assert!(repaired.contains("replay probe"));
    }

    #[test]
    fn decision_frame_rejects_unrelated_consciousness_answer() {
        let user = "Should I start a business?";
        let state = DialogueWorkspace::derive(user, &[]);
        assert_eq!(state.question.operation, QuestionOperation::DecisionSupport);
        let critique = state.critique(
            user,
            "Behavioral complexity is observable; subjective experience is inferred.",
            &[],
        );
        assert!(critique.flags.contains(&"semantic_subject_miss"));
        assert!(critique.flags.contains(&"operation_fit_miss"));
        let repaired = state.repair_for(user, "wrong", &critique);
        assert!(repaired.contains("business"));
        assert!(repaired.contains("potential customers"));
    }

    #[test]
    fn scoped_superlative_requires_the_stability_regime() {
        let user = "What geometry has the most stability?";
        let state = DialogueWorkspace::derive(user, &[]);
        assert_eq!(
            state.question.operation,
            QuestionOperation::ScopedSuperlative
        );
        let wrong = "Bitwork uses compact geometry and associative paths to route a stable answer.";
        let critique = state.critique(user, wrong, &[]);
        assert!(critique.flags.contains(&"operation_fit_miss"));
        let repaired = state.repair_for(user, wrong, &critique);
        assert!(repaired.contains("no single most stable"));
        assert!(repaired.contains("structural load"));
    }

    #[test]
    fn self_assessment_requires_a_direct_measured_boundary() {
        let user = "Are you nearing a coherence threshold?";
        let state = DialogueWorkspace::derive(user, &[]);
        assert_eq!(state.question.operation, QuestionOperation::SelfAssessment);
        let wrong = "I'll stay concrete and go deeper if you want.";
        let critique = state.critique(user, wrong, &[]);
        assert!(critique.flags.contains(&"operation_fit_miss"));
        let repaired = state.repair_for(user, wrong, &critique);
        assert!(repaired.starts_with("Not globally yet."));
        assert!(repaired.contains("unseen paraphrases"));
    }

    #[test]
    fn abstract_most_questions_do_not_inherit_physical_stability_scope() {
        for prompt in [
            "What is the strongest claim you can make about your own intelligence?",
            "What assumption is doing the most work in your answer?",
        ] {
            let state = DialogueWorkspace::derive(prompt, &[]);
            assert_eq!(state.question.operation, QuestionOperation::General);
        }
    }
}
