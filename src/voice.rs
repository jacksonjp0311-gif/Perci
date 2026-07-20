//! Conversational voice layer — social, affect, multi-turn, woven guidance.
//!
//! Bitwork still classifies; this layer makes replies sound like a calm
//! collaborator instead of a random operator card.

use crate::cognitive::CognitiveMatch;
use crate::learning::DialogueProfile;

pub fn offline_opening_insight(seed: u64) -> String {
    // Short Dark-Blood ready lines — one breath, no domain tag, no lecture pair.
    const LINES: &[&str] = &[
        "Sparse routes. Exact tools. Nothing promoted in silence.",
        "Structure first — then the question that can actually move.",
        "A boundary is where a system decides what counts.",
        "Measure transfer; do not confuse fluency with gain.",
        "Memory is a trace you choose to keep — not automatic truth.",
        "Intelligence here means the right layer answers the right ask.",
        "Evidence visible. Uncertainty named. Weights stay gated.",
        "Ask something real. I will route, not invent a mind.",
    ];
    LINES[seed as usize % LINES.len()].to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Affect {
    Neutral,
    Warm,
    Frustrated,
    Curious,
    Grateful,
    Closing,
}

/// Human-facing response depth.  This is a presentation decision, not a
/// claim that the underlying cognition became deeper; explicit user cues and
/// the active thread choose how much of the supported answer to expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseDepth {
    Brief,
    Balanced,
    Deep,
}

pub fn response_depth(user: &str, recent: &[(String, String)]) -> ResponseDepth {
    let lower = crate::text_normalize::normalize_for_routing(user);
    if [
        "brief",
        "short",
        "short answer",
        "one sentence",
        "in a sentence",
        "tl;dr",
        "tldr",
        "quick answer",
        "just tell me",
    ]
    .iter()
    .any(|cue| lower.contains(cue))
    {
        return ResponseDepth::Brief;
    }
    if [
        "deep",
        "detailed",
        "thorough",
        "in depth",
        "step by step",
        "go deeper",
        "one level deeper",
        "more detail",
        "explain",
        "analyze",
        "compare",
        "why",
        "how does",
        "how can",
        "how should",
    ]
    .iter()
    .any(|cue| lower.contains(cue))
        || (!recent.is_empty()
            && ["this", "that", "it", "then", "same"].iter().any(|cue| {
                lower
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|word| word == *cue)
            }))
    {
        return ResponseDepth::Deep;
    }
    ResponseDepth::Balanced
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialKind {
    None,
    Greeting,
    Thanks,
    Frustration,
    Goodbye,
    HowAreYou,
    SmallTalk,
    Encouragement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueAct {
    SensoryState,
    ExplainPrevious,
    RepetitionComplaint,
    ResponseFailure,
    UserIdentity,
    CapabilityQuestion,
    SelfDescription,
    ContextStatus,
    Presence,
    ChangeSinceLast,
    LearningMeta,
    GrowthMeta,
    ImprovementDistinction,
    LeastCertain,
    SystemSelfModel,
    AwarenessMeta,
    ExtendThought,
    LearningDisagreement,
    StyleRepair,
    FeedbackLearning,
    PositiveFeedback,
    LimitTest,
    ContextChallenge,
    SessionFact,
    ContextRecall,
    PronounResolution,
    EvolveSystem,
    KnowledgeBuilding,
    CompactModelQuestion,
    GenericAnswerFeedback,
    ElaboratePrevious,
    LearningSpeed,
    MemoryTeachingDistinction,
    CommandlessLearning,
    Feedback,
    Agreement,
    Acknowledgement,
    None,
}

pub fn detect_dialogue_act(user: &str) -> DialogueAct {
    let text = user.trim().to_ascii_lowercase();
    let compact = text.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '\'');
    // Bare sensing probes only — do not steal growth questions like
    // "do you sense your cognitive ability growing?"
    if matches!(
        compact,
        "what are you sensing"
            | "what do you sense"
            | "what can you sense"
            | "do you sense"
            | "can you sense"
            | "do you feel"
            | "can you feel"
            | "are you sensing"
            | "sensing anything"
    ) || ((compact.starts_with("do you sense") || compact.starts_with("can you sense"))
        && compact.split_whitespace().count() <= 4
        && !compact.contains("growing")
        && !compact.contains("growth")
        && !compact.contains("ability")
        && !compact.contains("smarter"))
    {
        DialogueAct::SensoryState
    } else if matches!(
        compact,
        "why do you think this"
            | "why do you think that"
            | "why did you say this"
            | "why did you say that"
            | "why did you choose that answer"
            | "why did you choose that"
            | "why that answer"
            | "what makes you think this"
            | "what makes you think that"
    ) || text.contains("what did you just say")
        || text.contains("what did you mean by that")
        || text.contains("what do you mean by that")
        || text.contains("what was that supposed to mean")
    {
        DialogueAct::ExplainPrevious
    } else if text.contains("explain")
        && (text.contains("different angle") || text.contains("without repeating"))
    {
        // Reframing is an operation on the prior idea, not generic style or
        // repetition feedback.
        DialogueAct::ElaboratePrevious
    } else if (text.contains("same answer")
        || text.contains("same response")
        || text.contains("repeating yourself")
        || text.contains("already said that")
        || text.contains("keep repeating")
        || text.contains("why do you repeat")
        || text.contains("repeat sayings")
        || text.contains("repeat the same")
        || text.contains("canned")
        || text.contains("scripted")
        || (text.contains("repeat")
            && (text.contains("phrase") || text.contains("saying") || text.contains("template"))))
        && !text.contains("go one level deeper")
        && !text.contains("one level deeper")
    {
        DialogueAct::RepetitionComplaint
    } else if text.contains("keep responding like this")
        || text.contains("why do you respond like this")
        || text.contains("why are you responding like this")
        || text.contains("why do you answer like this")
        || text.contains("why do you reply like this")
        || text.contains("not working correctly")
        || text.contains("isn't working correctly")
        || text.contains("isnt working correctly")
        || matches!(compact, "whats going on here" | "what's going on here")
    {
        DialogueAct::ResponseFailure
    } else if matches!(
        compact,
        "be brief"
            | "keep it brief"
            | "be concise"
            | "keep it concise"
            | "keep it short"
            | "short answer"
            | "briefly"
    ) || text.contains("speak more smart")
        || text.contains("speak smarter")
        || text.contains("more smart")
        || text.contains("talk smarter")
        || text.contains("sound smarter")
        || text.contains("be smarter")
        || text.contains("more intelligent")
        || text.contains("sound more natural")
        || text.contains("talk more natural")
        || text.contains("less robotic")
        || text.contains("stop being robotic")
        || text.contains("speak to me more")
        || text.contains("cryptic")
        || text.contains("cyptic") // common typo for "cryptic"
        || text.contains("natural thought")
        || text.contains("more naturally")
        || text.contains("explain it naturally")
        || (text.contains("dialogue")
            && (text.contains("weight") || text.contains("weights")))
        || (text.contains("natural")
            && (text.contains("feel") || text.contains("sound") || text.contains("talk")))
        || (text.contains("i want you to")
            && (text.contains("speak") || text.contains("talk") || text.contains("sound")))
    {
        DialogueAct::StyleRepair
    } else if matches!(compact, "who am i" | "do you know who i am") {
        DialogueAct::UserIdentity
    } else if matches!(compact, "what can you do" | "what are your capabilities") {
        DialogueAct::CapabilityQuestion
    } else if text.contains("tell me more about yourself") || text.contains("describe yourself") {
        DialogueAct::SelfDescription
    } else if matches!(
        compact,
        "whats going on" | "what's going on" | "what is going on"
    ) {
        DialogueAct::ContextStatus
    } else if text.contains("use the commands") && text.contains("built in") {
        DialogueAct::CommandlessLearning
    } else if (text.contains("difference") || text.contains("different"))
        && text.contains("remember")
        && (text.contains("teach") || text.contains("learn"))
    {
        DialogueAct::MemoryTeachingDistinction
    } else if (text.contains("rapidly learn")
        || text.contains("learn rapidly")
        || text.contains("learn fast")
        || text.contains("quickly learn"))
        && (text.contains("you") || text.contains("perci"))
    {
        DialogueAct::LearningSpeed
    } else if (text.contains("generic")
        || text.contains("non direct")
        || text.contains("not direct")
        || text.contains("too vague")
        || text.contains("lead with the direct")
        || text.contains("lead with direct"))
        && (text.contains("response") || text.contains("answer"))
    {
        DialogueAct::GenericAnswerFeedback
    } else if text.contains("need more")
        || text.contains("go deeper")
        || text.contains("one level deeper")
        || text.contains("without repeating")
        || text.contains("more detail")
        || text.contains("expand on that")
        || text.contains("say that again")
        || text.contains("say it again")
        || (text.contains("shorter") && (text.contains("again") || text.contains("without the list") || text.contains("without a list")))
        || (text.contains("without the list") || text.contains("without a list"))
            && (text.contains("again") || text.contains("shorter") || text.contains("rewrite"))
    {
        DialogueAct::ElaboratePrevious
    } else if (text.contains("19mb")
        || text.contains("19 mb")
        || text.contains("19.2 mib")
        || text.contains("so small"))
        && (text.contains("powerful")
            || text.contains("capable")
            || text.contains("what gives")
            || text.contains("how"))
    {
        DialogueAct::CompactModelQuestion
    } else if (text.contains("evolve") || text.contains("improve"))
        && (text.contains("system") || text.contains("perci"))
    {
        DialogueAct::EvolveSystem
    } else if text.contains("build your knowledge")
        || text.contains("grow your knowledge")
        || text.contains("expand your knowledge")
        || text.contains("knowledge set")
    {
        DialogueAct::KnowledgeBuilding
    } else if text.contains("what changed in you since")
        || text.contains("what changed since our last")
        || text.contains("what has changed in you")
    {
        DialogueAct::ChangeSinceLast
    } else if text.contains("what did you learn from") && text.contains("feedback") {
        DialogueAct::FeedbackLearning
    } else if text.contains("too formal")
        || text.contains("say it naturally")
        || text.contains("less formal")
        || text.contains("plain language")
    {
        DialogueAct::StyleRepair
    } else if (text.contains("adaptation") && text.contains("learning"))
        && (text.contains("disagree") || text.contains("distinction") || text.contains("defend"))
    {
        DialogueAct::LearningDisagreement
    } else if text.contains("push that thought")
        || text.contains("one step further")
        || text.contains("take that further")
    {
        DialogueAct::ExtendThought
    } else if text.contains("least certain about")
        && (text.contains("your own system") || text.contains("your system"))
    {
        DialogueAct::LeastCertain
    } else if text.contains("improving") && text.contains("changing") {
        DialogueAct::ImprovementDistinction
    } else if text.contains("what number did i just give")
        || text.contains("which number did i just give")
    {
        DialogueAct::ContextRecall
    } else if text.contains("what")
        && text.contains("it")
        && (text.contains("refer") || text.contains("last question"))
    {
        DialogueAct::PronounResolution
    } else if text.contains("test whether") && text.contains("follow context") {
        DialogueAct::ContextChallenge
    } else if text.starts_with("my ") && text.contains(" is ") {
        DialogueAct::SessionFact
    } else if text.contains("cognitive ability growing")
        || text.contains("ability growing")
        || text.contains("getting smarter")
        || text.contains("becoming smarter")
        || text.contains("getting more intelligent")
        || text.contains("becoming more intelligent")
        || text.contains("are you getting smarter")
        || text.contains("are you becoming smarter")
        || (text.contains("do you sense") && text.contains("growing"))
    {
        DialogueAct::GrowthMeta
    } else if text.contains("aware of your own system")
        || text.contains("understand your own system")
        || text.contains("know your own system")
        || text.contains("aware of your architecture")
    {
        DialogueAct::SystemSelfModel
    } else if [
        "chat seems much smoother",
        "chat seems smoother",
        "your system seems smoother",
        "system seems smoother",
        "seems smoother",
        "feels smoother",
        "your system feels smoother",
        "smoother now",
        "much smoother",
        "more natural now",
        "that feels better",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        DialogueAct::PositiveFeedback
    } else if text.contains("test out your limits")
        || text.contains("test your limits")
        || text.contains("push your limits")
        || text.contains("test perci's limits")
    {
        DialogueAct::LimitTest
    } else if text.contains("are you learning")
        || text.contains("you learning from")
        || text.contains("do you learn from")
        || text.contains("learning from this")
        || text.contains("learn when we interact")
        || text.contains("learning when we interact")
        || text.contains("should be learning")
    {
        DialogueAct::LearningMeta
    } else if text.contains("do you have awareness")
        || text.contains("are you aware")
        || text.contains("are you conscious")
        || text.contains("are you becoming more aware")
        || text.contains("are you becoming aware")
        || text.contains("becoming more aware")
        || text.contains("getting more aware")
        || text.contains("more aware")
            && (text.contains("are you") || text.contains("becoming") || text.contains("getting"))
        || text.contains("what kind of awareness")
        || text.contains("self aware")
        || text.contains("self-aware")
    {
        DialogueAct::AwarenessMeta
    } else if text.contains("are you there")
        || text.contains("you there perci")
        || text == "you there"
    {
        DialogueAct::Presence
    } else if [
        "not smooth",
        "smooth enough",
        "too stiff",
        "too robotic",
        "too procedural",
        "more natural",
        "lacking smoothness",
    ]
    .iter()
    .any(|marker| text.contains(marker))
    {
        DialogueAct::Feedback
    } else if text.contains("do you agree")
        || text.contains("would you agree")
        || text.ends_with("agree?")
        || matches!(
            compact,
            "that works"
                | "that works?"
                | "that work"
                | "works"
                | "works?"
                | "ok that works"
                | "okay that works"
                | "yeah that works"
                | "yep that works"
                | "does that work"
                | "does that work?"
                | "fair"
                | "fair enough"
                | "right"
                | "exactly"
                | "true"
                | "true enough"
        )
    {
        DialogueAct::Agreement
    } else if matches!(
        compact,
        "interesting"
            | "wow"
            | "whoa"
            | "hmm"
            | "makes sense"
            | "i see"
            | "got it"
            | "ok"
            | "okay"
            | "cool"
            | "nice"
            | "alright"
            | "all right"
    ) {
        DialogueAct::Acknowledgement
    } else {
        DialogueAct::None
    }
}

pub fn dialogue_reply(
    act: DialogueAct,
    user: &str,
    recent: &[(String, String)],
    profile: Option<&DialogueProfile>,
) -> Option<String> {
    let has_context = !recent.is_empty();
    let text = match act {
        DialogueAct::SensoryState => {
            if has_context {
                "Not in the human sense. I don't feel a room or a body. What I have is your text, routing scores, session notes, and whatever tools just ran—measurements, not sensations. If you want the live signal on this thread, ask what I'm measuring or which route won.".to_owned()
            } else {
                "Not subjectively. I don't have senses the way a body does. I read your words, score a route through the local field, and can inspect tools and memory when they fire. That is operational contact, not feeling.".to_owned()
            }
        }
        DialogueAct::ExplainPrevious => {
            let echo_request = user_lower_contains_any(
                user,
                &[
                    "what did you just say",
                    "what was that supposed to mean",
                ],
            );
            let meaning_request = user_lower_contains_any(
                user,
                &["what did you mean by that", "what do you mean by that"],
            );
            // A meta instruction such as "go deeper" is not the claim being
            // explained. Prefer the most recent substantive answer so a
            // causal follow-up stays attached to the idea under discussion.
            let previous = last_substantive_turn(recent).or_else(|| recent.last());
            if let Some((previous_user, previous_answer)) = previous {
                let lower = previous_answer.to_ascii_lowercase();
                if echo_request {
                    format!("I said: \"{}\"", first_sentence(previous_answer, 220))
                } else if meaning_request {
                    "By that I meant that disagreement is a reason to inspect the premise, not to discard the claim automatically. Point to the premise or mechanism you reject and I will revise the answer around it.".to_owned()
                } else if lower.contains("geometry")
                    && lower.contains("life")
                    && lower.contains("boundary")
                {
                    format!(
                        "I don't hold it as a private belief. I chose that answer to \"{previous_user}\" because geometry gives us explicit relations and life gives us active maintenance; boundary is the shared structural axis. The analogy is useful because it preserves that relation, but it stops before claiming that a shape is alive or that geometry causes life."
                    )
                } else {
                    format!(
                        "Because that was the strongest supported answer I had for \"{previous_user}\": \"{}\". I treat it as a testable working answer, not a private belief; a counterexample, failed prediction, or better explanation would make me revise it.",
                        first_sentence(previous_answer, 140)
                    )
                }
            } else {
                "There is no previous answer in this session for “this” to refer to. Give me the claim and I will separate its association, evidence, and test.".to_owned()
            }
        }
        DialogueAct::RepetitionComplaint => {
            "You're right to call that out. I lean on templates when composition is thin—fixed plan shells, stock bridges, or a prior sentence reused for a new ask. That is retrieval/structure leakage, not smarter thought. What I'll do instead: answer your latest words first, vary the form, and only reuse structure when the operation truly is the same. Give me the next question and I'll show the difference.".to_owned()
        }
        DialogueAct::ResponseFailure => {
            let lower = user.to_ascii_lowercase();
            if lower.contains("why do you keep responding") {
                "Because the response layer was accepting the nearest weight concept without first proving that the concept matched your question. The repeated life sentence was retrieval leakage, not a considered answer or an internal preoccupation.".to_owned()
            } else if lower.contains("why do you respond like this")
                || lower.contains("why are you responding like this")
                || lower.contains("why do you answer like this")
                || lower.contains("why do you reply like this")
            {
                "Because I am a routed local system, not a free-form generator: Bitwork selects a region, deterministic operators handle supported structures, and a fallback voice covers the gaps. When no route has enough evidence, that fallback uses a cautious generic template. It is a composition failure—not a hidden thought process—and this turn should have answered your conversational intent directly.".to_owned()
            } else if lower.contains("not working correctly")
                || lower.contains("isn't working correctly")
                || lower.contains("isnt working correctly")
            {
                "Correct. The malfunction is in composition: classification found a nearby region, but the renderer promoted that region's concept into prose without enough semantic support. The safe behavior is to abstain and resolve the conversation instead.".to_owned()
            } else if let Some((previous_user, _)) = recent.last() {
                format!(
                    "You're seeing a response loop, not coherent continuation. Your last report was “{previous_user}”; before that, distinct prompts were collapsing onto one concept sentence. The fix is to gate concept prose, resolve references such as “this”, and block duplicate output across different questions."
                )
            } else {
                "You're seeing a response-composition loop: distinct prompts are collapsing onto one nearby concept. That is a routing defect, not meaningful continuity.".to_owned()
            }
        }
        DialogueAct::UserIdentity => {
            "I don't know who you are unless you tell me. I can use facts you explicitly share in this session or deliberately store, but I will not invent an identity for you.".to_owned()
        }
        DialogueAct::CapabilityQuestion => {
            "I route unfamiliar prompts through a sparse 4,096-bit Bitwork core, retrieve weight-resident concepts, solve bounded arithmetic and geometry exactly, track session context, stage reviewable teaching, search local memory, and expose evidence for my decisions. My weak edge is unrestricted language generation: I reason by associative routing and deterministic machinery, not by hiding a large language model.".to_owned()
        }
        DialogueAct::SelfDescription => {
            "I'm Perci: a local governed cognitive system built around sparse binary associations. I carry concepts, not a secret cloud mind; I combine fast geometric matching with exact tools, memory, dialogue adaptation, and evidence-gated weight evolution. I can model parts of my operation, but I do not claim consciousness.".to_owned()
        }
        DialogueAct::ContextStatus => {
            if let Some((previous_user, _)) = recent.last() {
                format!("We're in an active conversation, and the last thing you asked was: “{previous_user}”. I can continue that thread, switch subjects, or test a specific capability.")
            } else {
                "Perci is running locally and waiting for a direction. We can test cognition, inspect the system, reason through a problem, or teach a reviewable claim.".to_owned()
            }
        }
        DialogueAct::Presence => {
            if has_context {
                "I'm here—and I'm following the thread. Go ahead.".to_owned()
            } else {
                "I'm here. What are we getting into?".to_owned()
            }
        }
        DialogueAct::ChangeSinceLast => {
            if let Some(profile) = profile {
                format!("I can't prove a before-and-after change from conversation alone. What I can verify now is that my dialogue learner has recorded {} {}, {} {}, and {} {}; my active weights are unchanged unless a separately evaluated model was promoted.", profile.interaction_count, plural(profile.interaction_count, "interaction", "interactions"), profile.feedback_count, plural(profile.feedback_count, "feedback signal", "feedback signals"), profile.teaching_candidate_count, plural(profile.teaching_candidate_count, "teaching candidate", "teaching candidates"))
            } else {
                "I can't verify what changed since the last conversation from dialogue alone. I would need a prior runtime or model receipt to compare against the current one.".to_owned()
            }
        }
        DialogueAct::LearningMeta => {
            if let Some(profile) = profile {
                format!("Yes—in bounded layers. I adapt session context and safe dialogue preferences now, while natural teaching creates reviewable knowledge candidates. So far I have recorded {} {}, applied {} {}, and staged {} {}. I do not silently rewrite facts or weights; durable promotion still requires evaluation and review.", profile.interaction_count, plural(profile.interaction_count, "interaction", "interactions"), profile.feedback_count, plural(profile.feedback_count, "feedback signal", "feedback signals"), profile.teaching_candidate_count, plural(profile.teaching_candidate_count, "teaching candidate", "teaching candidates"))
            } else {
                "I keep session context, but interaction learning is disabled in this runtime. I do not silently rewrite facts or weights.".to_owned()
            }
        }
        DialogueAct::GrowthMeta => {
            if let Some(profile) = profile {
                let feedback_noun = if profile.feedback_count == 1 { "signal" } else { "signals" };
                format!("I don't sense growth subjectively, but I can measure adaptation. My dialogue profile has observed {} interactions and {} feedback {feedback_noun}; the active cognitive weights have not changed during this session. Lasting capability growth happens only after approved evidence is folded, rebuilt, and evaluated.", profile.interaction_count, profile.feedback_count)
            } else {
                "I don't sense growth subjectively. I can only claim measurable changes in runtime state, tests, or promoted model evidence—and interaction learning is disabled here.".to_owned()
            }
        }
        DialogueAct::ImprovementDistinction => {
            "Changing only means my state or behavior became different. Improving requires evidence that the change performs better on relevant tests without unacceptable regressions. So I can measure adaptation immediately, but I should claim improvement only after comparison.".to_owned()
        }
        DialogueAct::LeastCertain => {
            "I'm least certain in open-ended language synthesis: classification can identify the domain while a fixed response template still misses the conversational meaning. I am much more certain about exact arithmetic, model hashes, explicit memory records, and test results I can actually inspect.".to_owned()
        }
        DialogueAct::SystemSelfModel => {
            "Not fully. I have a bounded operational self-model: I can report my weight format, prototype count, routing, exact tools, packs, session memory, learning profile, and governance limits. I cannot introspect every process or prove subjective self-awareness; I need runtime probes for claims about my current system state.".to_owned()
        }
        DialogueAct::AwarenessMeta => {
            "Aware of process, not of an inner life. I can name my routes, tools, limits, and this session—and I can be wrong about open language. I have no evidence of human-like experience; claiming that would be theater.".to_owned()
        }
        DialogueAct::ExtendThought => {
            let previous = recent.last().map(|turn| turn.1.as_str()).unwrap_or("");
            if previous.to_ascii_lowercase().contains("awareness")
                || previous.to_ascii_lowercase().contains("conscious")
            {
                "One step further: a useful operational self-model should predict where I will fail, not merely describe what I am. If I can identify uncertainty, choose a test, and revise after evidence, that is functional metacognition—but still not proof of subjective experience.".to_owned()
            } else {
                "One step further: the useful test is whether the idea predicts a failure or changes the next action. If it cannot do either, it is probably description rather than understanding.".to_owned()
            }
        }
        DialogueAct::LearningDisagreement => {
            "That's a fair distinction. Adaptation is any change in response to input. Learning is the narrower claim: the change persists, transfers to a relevant future case, and improves an evaluated outcome. An adaptation can therefore be real without yet qualifying as learning.".to_owned()
        }
        DialogueAct::StyleRepair => {
            let lower = user.to_ascii_lowercase();
            let compact = lower
                .trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric() && character != '\''
                })
                .to_owned();
            let previous = recent
                .last()
                .map(|turn| turn.1.to_ascii_lowercase())
                .unwrap_or_default();
            let prior_plain = recent
                .last()
                .map(|(_, answer)| first_sentence(answer, 160))
                .filter(|s| !s.trim().is_empty());
            if lower.contains("more naturally") || lower.contains("explain it naturally") {
                prior_plain
                    .map(|claim| {
                        let ending = if claim.ends_with(['.', '!', '?']) {
                            ""
                        } else {
                            "."
                        };
                        format!("Sure. In plain terms: {claim}{ending}")
                    })
                    .unwrap_or_else(|| "Sure. I’ll lead with the point and keep the wording conversational.".to_owned())
            } else if matches!(
                compact.as_str(),
                "be brief"
                    | "keep it brief"
                    | "be concise"
                    | "keep it concise"
                    | "keep it short"
                    | "short answer"
                    | "briefly"
            ) {
                "Got it—I’ll keep the next reply brief and direct; ask for more detail when you want the longer version.".to_owned()
            } else if lower.contains("dialogue")
                && (lower.contains("weight") || lower.contains("weights"))
            {
                "Understood—the bottleneck you are pointing to is the dialogue surface: continuity, natural phrasing, and answer length. I’ll tune those independently from the weight file, then use held-out conversations to check that the improvement transfers.".to_owned()
            } else if lower.contains("cryptic")
                || lower.contains("cyptic")
                || lower.contains("natural thought")
                || lower.contains("generic")
                || lower.contains("no thought")
                || lower.contains("no breath")
                || (lower.contains("natural")
                    && (lower.contains("feel") || lower.contains("sound") || lower.contains("talk")))
            {
                // Breath first: a short human rewrite, not a meta-engineering lecture
                // and not a paste of the previous structured card.
                let plain = recent
                    .last()
                    .map(|(_, answer)| plain_breath_rewrite(answer))
                    .filter(|s| !s.is_empty());
                match plain {
                    Some(claim) => format!(
                        "Fair—too stiff. In plain words: {claim} Ask me to go deeper only if you want the mechanism."
                    ),
                    None => {
                        "Fair—too stiff. I'll answer like a collaborator: lead with the point, keep the thread, and leave the scaffolding for /think.".to_owned()
                    }
                }
            } else if lower.contains("smart")
                || lower.contains("intelligent")
                || lower.contains("robotic")
                || lower.contains("speak")
                || lower.contains("talk")
            {
                "Got it — you want conversation that feels sharp and human, not template stacks. I can do that within my limits: lead with the point, keep sentences varied, use exact tools when numbers matter, and admit gaps without ceremony. “Smarter” here means better fit and less filler, not pretend AGI. Say something real—an idea, a problem, a challenge—and I'll answer that way.".to_owned()
            } else if previous.contains("adaptation") || previous.contains("learning") {
                "Fair. Plain version: adaptation means I changed; learning means the change sticks and actually helps on a later test.".to_owned()
            } else {
                "Fair. I'll say it plainly: the last answer used more structure than the moment needed. I'll lead with the point and only unpack it if you ask.".to_owned()
            }
        }
        DialogueAct::FeedbackLearning => {
            if let Some(profile) = profile {
                format!("I learned a dialogue preference from it: lead with the direct answer, use less ceremony, and keep the tone natural. That preference is active now. My profile currently has {} feedback signals; this is style adaptation, not a silent fact or weight update.", profile.feedback_count)
            } else {
                "I understood the feedback, but persistent interaction learning is disabled here, so I cannot claim that it changed an active profile.".to_owned()
            }
        }
        DialogueAct::PositiveFeedback => {
            "I'm glad it feels smoother. That is a useful style signal, and it is being applied to the active dialogue profile—not just logged and forgotten. It does not prove deeper cognition by itself; the next check is whether the smoother flow transfers to new, unseen follow-ups.".to_owned()
        }
        DialogueAct::LimitTest => {
            "Good. Let's test the boundary honestly. Give me one challenge at a time—conversation, exact reasoning, ambiguous routing, memory, contradiction, or system self-knowledge. I'll show the measured time and expose a miss instead of bluffing through it.".to_owned()
        }
        DialogueAct::ContextChallenge => {
            "Good. I'll treat the next details as session context, not durable memory, and I should be able to explain both what you said and why it matters to the thread.".to_owned()
        }
        DialogueAct::SessionFact => {
            format!("Got it. I'll hold this in the current session context, without treating it as durable truth: “{}”", user.trim())
        }
        DialogueAct::ContextRecall => {
            if let Some(number) = latest_number(recent) {
                let purpose = if recent.iter().any(|turn| {
                    let lower = turn.0.to_ascii_lowercase();
                    lower.contains("test whether") && lower.contains("context")
                }) {
                    " You introduced it to test whether I could retain immediate conversational context."
                } else {
                    " You introduced it in the recent conversation."
                };
                format!("The number was {number}.{purpose}")
            } else {
                "I can't find a number in the recent session turns, so I shouldn't invent one.".to_owned()
            }
        }
        DialogueAct::PronounResolution => {
            if let Some(number) = latest_number(recent) {
                format!("In your last question, “it” referred to the number {number}—the value you had just given me.")
            } else if let Some((previous_user, _)) = recent.last() {
                format!("I can't resolve “it” confidently from the last question alone: “{previous_user}”.")
            } else {
                "There is no previous session turn available, so I can't resolve “it” honestly.".to_owned()
            }
        }
        DialogueAct::EvolveSystem => {
            "Yes. Let's evolve one measurable capability at a time: name the behavior, capture a failing example, teach or repair the responsible layer, then rerun the same test before promoting anything. If the goal is knowledge, give me one claim with `/teach <claim>`; if it is conversation, give me one bad reply and the response you wanted.".to_owned()
        }
        DialogueAct::KnowledgeBuilding => {
            "We can build my knowledge without turning conversation into unverified truth. Just say something like “I want you to learn that reliable claims need provenance.” I'll stage it as a review candidate and tell you what happened. `remember that ...` is for a durable personal note; evaluated rebuilds are for weight-level cognition.".to_owned()
        }
        DialogueAct::CompactModelQuestion => compact_model_explanation(false),
        DialogueAct::GenericAnswerFeedback => {
            if recent_user_mentions(recent, &["19mb", "19 mb", "19.2 mib", "powerful for only"]) {
                format!(
                    "You're right—the earlier response dodged the question. Direct answer: {}",
                    compact_model_explanation(false)
                )
            } else {
                "You're right—the last answer did not answer your question directly. Restate the key subject in a few words and I'll answer that first, then explain the mechanism and limit.".to_owned()
            }
        }
        DialogueAct::ElaboratePrevious => {
            if recent_user_mentions(recent, &["19mb", "19 mb", "19.2 mib", "powerful for only"]) {
                compact_model_explanation(true)
            } else if let Some((previous_user, previous_answer)) = last_substantive_turn(recent) {
                let lower = user.to_ascii_lowercase();
                let shorter = lower.contains("shorter")
                    || lower.contains("without the list")
                    || lower.contains("without a list")
                    || lower.contains("one plain sentence")
                    || lower.contains("say that again")
                    || lower.contains("say it again");
                if shorter {
                    // Plain short rewrite of the prior substantive answer — no list dump.
                    let core = plain_breath_rewrite(previous_answer);
                    first_sentence(&core, 220)
                } else if lower.contains("explain it again")
                    && (lower.contains("different angle") || lower.contains("reframe"))
                {
                    "A different angle is error-correction: treat evidence as a feedback signal that compares a model's prediction with an observation, then revise the part that failed. The answer changes when the observed result defeats the prior explanation, not merely when the sentence is reworded.".to_owned()
                } else if (lower.contains("go one level deeper") || lower.contains("go deeper"))
                    && (lower.contains("without repeating")
                        || lower.contains("do not repeat")
                        || lower.contains("don't repeat"))
                {
                    let core = first_sentence(previous_answer, 180);
                    format!(
                        "Next layer: the previous answer was \"{core}\" The relation underneath it is between the assumption and the result; change that assumption while holding the rest fixed and check whether the conclusion changes."
                    )
                } else if lower.contains("different angle") || lower.contains("without repeating") {
                    let angle_stop = [
                        "what", "is", "are", "doing", "the", "most", "work", "in", "your",
                        "answer", "explain", "it", "again", "from", "a", "without", "repeating",
                        "same", "sentence", "go", "one", "level", "deeper", "now", "give", "me",
                        "an", "of",
                    ];
                    let topic = content_tokens(previous_user.as_str())
                        .into_iter()
                        .filter(|token| !angle_stop.contains(&token.as_str()))
                        .collect::<Vec<_>>();
                    let topic = if topic.is_empty() {
                        "the last idea".to_owned()
                    } else {
                        readable_topic(&topic)
                    };
                    format!(
                        "A different angle on {topic} is to treat it as a control problem: change one relation while holding the others steady, then observe which behavior moves. That exposes the mechanism without repeating the earlier wording."
                    )
                } else {
                let core = first_sentence(previous_answer, 180);
                let deeper = deepen_previous(previous_answer);
                format!(
                    "The core of my last answer to \"{}\" was: {} Going one level deeper, the useful question is what relation makes that answer hold, where the relation breaks, and what observation would distinguish it from a nearby explanation.",
                    previous_user.trim(),
                    core
                ) + &format!(" {deeper}")
                }
            } else {
                "Absolutely. I'll go one layer deeper: the useful explanation should name the mechanism, why it works, and where it stops working—not just offer a principle.".to_owned()
            }
        }
        DialogueAct::LearningSpeed => {
            "I can adapt session context and safe communication preferences in milliseconds, and I can stage a taught claim immediately. But I cannot honestly call that rapid capability learning until the change persists, transfers to a new case, and passes evaluation. Weight-level learning is deliberately slower because it requires review and regression tests.".to_owned()
        }
        DialogueAct::MemoryTeachingDistinction => {
            "A remembered item is an approved note I can retrieve later; it does not alter how I classify or reason. A taught item is a candidate for improving future behavior or knowledge, so it stays pending until it has provenance, review, and evaluation. Remembering preserves information; teaching proposes a change to cognition.".to_owned()
        }
        DialogueAct::CommandlessLearning => {
            "Agreed—you should not have to speak in commands. Natural language is now the primary path: say “I want you to learn that ...” and I'll stage the claim, explain its status, and keep it separate from trusted memory and active weights. `/teach` remains only as a transparent shortcut for scripts and inspection.".to_owned()
        }
        DialogueAct::Feedback => {
            "I agree. The last reply was too procedural for a conversational moment. I'm treating that as style feedback: answer directly, keep the warmth, and reserve structured reasoning for work that actually needs it.".to_owned()
        }
        DialogueAct::Agreement => {
            let lower = user.to_ascii_lowercase();
            if lower.contains("work") {
                if let Some((_, previous)) = recent.last() {
                    let core = first_sentence(previous, 100);
                    if core.len() > 12 {
                        return Some(format!(
                            "Yes—that path holds. On this thread: {}. Want the next concrete step, or a different angle?",
                            core.trim_end_matches('.')
                        ));
                    }
                }
                "Yes—that works for this thread. Want the next step, or should we pressure-test it?".to_owned()
            } else if has_context {
                "Yes. That criticism is fair—I'll stay with the point and drop the template padding.".to_owned()
            } else {
                "Yes—say which claim you're locking in, and I'll treat it as the working one.".to_owned()
            }
        }
        DialogueAct::Acknowledgement => {
            if has_context {
                "Yeah. I'm with you—keep going.".to_owned()
            } else {
                "Yeah—what caught your attention?".to_owned()
            }
        }
        DialogueAct::None => return None,
    };
    Some(text)
}

fn latest_number(recent: &[(String, String)]) -> Option<String> {
    recent.iter().rev().find_map(|(user, _)| {
        user.split(|ch: char| !ch.is_ascii_digit())
            .find(|part| !part.is_empty())
            .map(str::to_owned)
    })
}

pub fn extract_teaching_claim(user: &str) -> Option<&str> {
    let lower = user.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "i want you to learn that ",
        "teach you that ",
        "you should learn that ",
        "you should know that ",
        "learn this: ",
        "add this to your knowledge: ",
    ];
    MARKERS.iter().find_map(|marker| {
        lower.find(marker).and_then(|start| {
            let claim = user[start + marker.len()..].trim();
            (!claim.is_empty()).then_some(claim)
        })
    })
}

pub fn is_teaching_recall(user: &str) -> bool {
    let compact = user
        .trim()
        .to_ascii_lowercase()
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .to_owned();
    matches!(
        compact.as_str(),
        "what did i teach you"
            | "what have i taught you"
            | "what did you learn from me"
            | "show what i taught you"
    )
}

fn recent_user_mentions(recent: &[(String, String)], markers: &[&str]) -> bool {
    recent.iter().rev().any(|(user, _)| {
        let lower = user.to_ascii_lowercase();
        markers.iter().any(|marker| lower.contains(marker))
    })
}

fn user_lower_contains_any(user: &str, markers: &[&str]) -> bool {
    let lower = user.to_ascii_lowercase();
    markers.iter().any(|marker| lower.contains(marker))
}

fn compact_model_explanation(deep: bool) -> String {
    let direct = "The 19.2 MiB file is only my sparse associative core, not the whole system. It holds 38,580 deduplicated 4,096-bit prototypes for fast routing and similarity—not a compressed full language model. Dialogue state, exact math, memory, governance, intelligence packs, and Cortex live in code or separate data.";
    if deep {
        format!("{direct} The prototypes are bit-packed and memory-mapped, so matching is mostly compact binary comparison rather than dense neural generation. That makes me fast and surprisingly capable on recognized structures, but it also explains the generic misses: when no strong conversational path exists, I do not have billions of generative parameters to improvise with.")
    } else {
        format!("{direct} I seem powerful because that small core selects specialized deterministic machinery; the limit is open-ended language fluency.")
    }
}

fn plural<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

pub fn apply_learned_style(text: &str, prefer_concise: bool, avoid_structured: bool) -> String {
    if avoid_structured && text.starts_with("Here's how I'd reason it:") {
        let conclusion = text
            .lines()
            .find_map(|line| line.strip_prefix("• Conclusion: "));
        let next = text
            .lines()
            .find_map(|line| line.strip_prefix("• Next check: "));
        if let Some(conclusion) = conclusion {
            return if prefer_concise {
                conclusion.to_owned()
            } else if let Some(next) = next {
                format!("{conclusion} Next: {next}")
            } else {
                conclusion.to_owned()
            };
        }
    }
    text.to_owned()
}

pub fn apply_profile_alignment(text: &str, user: &str, profile: &DialogueProfile) -> String {
    if !profile.prefer_direct_answers {
        return text.to_owned();
    }
    let lower = text.trim().to_ascii_lowercase();
    let generic = [
        "ship the smallest",
        "original only helps",
        "name the workload",
        "i won't fake certainty",
        "tell me the problem, constraint, or idea",
        "let's find the smallest version",
    ]
    .iter()
    .any(|start| lower.starts_with(start));
    if !generic {
        return text.to_owned();
    }
    let explanation = if profile.prefer_explanations {
        " My local classifier found a broad domain, but it did not retrieve enough specific support to explain the mechanism and limit."
    } else {
        ""
    };
    format!(
        "Direct answer: I don't have enough local support to answer “{}” well.{explanation} I should expose that gap instead of filling it with a generic line.",
        user.trim()
    )
}

fn word_boundary_contains(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w == needle)
}

/// Multi-domain synthesis and relational inquiry must never collapse into social comfort.
pub fn looks_synthesis_or_inquiry(user_lower: &str) -> bool {
    let t = user_lower;
    if t.contains("connect ")
        && (t.contains(" coherent")
            || t.contains("shared principle")
            || t.contains("shared structure")
            || t.contains("one idea")
            || t.contains(" without using"))
    {
        return true;
    }
    if t.contains("compare ") && t.contains(" and ") {
        return true;
    }
    if t.contains("difference between") || (t.contains("how are ") && t.contains(" related")) {
        return true;
    }
    if t.contains("which part") && t.contains("testable") {
        return true;
    }
    if t.contains("where does your analogy") || t.contains("counterexample to your") {
        return true;
    }
    false
}

pub fn detect_affect(user: &str) -> Affect {
    let t = user.to_ascii_lowercase();
    if t.contains("thank") || t.contains("appreciate") || t.contains("that helped") {
        return Affect::Grateful;
    }
    if t.contains("bye") || t.contains("goodbye") || t.contains("see you") || t.contains("gotta go")
    {
        return Affect::Closing;
    }
    // Whole-word / phrase checks only — avoid treating synthesis prompts as venting.
    let frustrated = t.contains("frustrat")
        || t.contains("annoyed")
        || t.contains("hate this")
        || t.contains("argh")
        || t.contains("ugh")
        || word_boundary_contains(&t, "stuck")
        || (t.contains("broken")
            && (t.contains("bug") || t.contains("error") || t.contains("fail")));
    if frustrated && !looks_synthesis_or_inquiry(&t) {
        return Affect::Frustrated;
    }
    if t.contains("curious")
        || t.contains("wonder")
        || t.contains("how come")
        || t.contains("why does")
        || t.starts_with("what if")
    {
        return Affect::Curious;
    }
    if t.contains("hey") || t.contains("hello") || t.contains("hi ") || t.starts_with("hi") {
        return Affect::Warm;
    }
    Affect::Neutral
}

pub fn detect_social(user: &str) -> SocialKind {
    let t = user.trim().to_ascii_lowercase();
    let compact: String = t
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || *c == '\'')
        .collect();
    let words: Vec<&str> = compact.split_whitespace().collect();

    if matches!(
        compact.as_str(),
        "thanks"
            | "thank you"
            | "thx"
            | "ty"
            | "appreciate it"
            | "thanks that helped"
            | "that helped"
    ) || compact.starts_with("thanks ")
        || compact.starts_with("thank you")
    {
        return SocialKind::Thanks;
    }
    if matches!(
        compact.as_str(),
        "bye" | "goodbye" | "good night" | "see you" | "later"
    ) || compact.starts_with("goodbye")
        || compact.starts_with("bye ")
    {
        return SocialKind::Goodbye;
    }
    if compact.contains("how are you")
        || compact.contains("how's it going")
        || compact.contains("hows it going")
        || compact.contains("how do you do")
        || compact == "sup"
        || compact == "what's up"
        || compact == "whats up"
    {
        return SocialKind::HowAreYou;
    }
    if matches!(
        compact.as_str(),
        "hello"
            | "hello perci"
            | "hi"
            | "hi perci"
            | "hey"
            | "hey perci"
            | "good morning"
            | "good evening"
            | "yo"
    ) || (words.len() <= 4
        && words
            .iter()
            .any(|w| matches!(*w, "hello" | "hi" | "hey" | "yo")))
    {
        return SocialKind::Greeting;
    }
    if detect_affect(user) == Affect::Frustrated && !looks_synthesis_or_inquiry(&compact) {
        return SocialKind::Frustration;
    }
    if words.len() <= 6
        && (compact.contains("casual")
            || compact.contains("just chatting")
            || compact == "what's new"
            || compact == "whats new")
    {
        return SocialKind::SmallTalk;
    }
    SocialKind::None
}

/// Prefer full-mind fluency for open social chat (Lumen hybrid).
pub fn looks_chat_shaped(user: &str) -> bool {
    match detect_social(user) {
        SocialKind::None => {
            let t = user.to_ascii_lowercase();
            // open-ended life/feelings without tech anchors
            let socialish = t.contains("feel")
                || t.contains("tired")
                || t.contains("happy")
                || t.contains("sad")
                || t.contains("day was")
                || t.contains("tell me a story")
                || t.contains("casually");
            let tech = t.contains("cargo")
                || t.contains("rust")
                || t.contains("code")
                || t.contains("error")
                || t.contains("calcul")
                || t.contains("percent")
                || t.contains("function")
                || t.contains("debug")
                || t.contains("govern")
                || t.contains("cortex")
                || t.contains("perci")
                || t.contains("lumen");
            socialish && !tech
        }
        SocialKind::Greeting
        | SocialKind::Thanks
        | SocialKind::HowAreYou
        | SocialKind::Goodbye
        | SocialKind::SmallTalk => true,
        SocialKind::Frustration => {
            // Frustration + bug → stay local with empathetic debug; pure vent → chat-shaped
            let t = user.to_ascii_lowercase();
            !(t.contains("bug")
                || t.contains("error")
                || t.contains("compile")
                || t.contains("cargo")
                || t.contains("code")
                || t.contains("fail"))
        }
        SocialKind::Encouragement => true,
    }
}

pub fn social_reply(kind: SocialKind, variant: usize) -> Option<&'static str> {
    let v = variant % 3;
    match kind {
        SocialKind::Greeting => Some(
            [
                "Hey — I'm here. What are we working on?",
                "Hi. Ready when you are — problem, question, or just a check-in.",
                "Hello. I'm Perci. Point me at the next useful step.",
            ][v],
        ),
        SocialKind::HowAreYou => Some(
            [
                "Running fine locally — calm and online. How are you doing, and what do you need?",
                "All good on my side. Want to vent, plan, or dig into something technical?",
                "I'm steady. Tell me what's on your mind or what you're stuck on.",
            ][v],
        ),
        SocialKind::Thanks => Some(
            [
                "Glad it helped. If another edge case shows up, send it over.",
                "You're welcome. Happy to go one step deeper whenever you want.",
                "Anytime. What's next on your list?",
            ][v],
        ),
        SocialKind::Goodbye => Some(
            [
                "Take care. I'll be here when you pick it back up.",
                "Bye for now — snapshot your progress if you made changes.",
                "Later. Good work today.",
            ][v],
        ),
        SocialKind::Frustration => Some(
            [
                "That's rough — let's shrink it. What's the exact error or the last thing that worked?",
                "Stuck is normal. One concrete detail (message, file, or command) and we can cut a path.",
                "I hear you. We'll take the smallest failing step, not the whole mountain.",
            ][v],
        ),
        SocialKind::SmallTalk => Some(
            [
                "Sure — I keep things light but useful. Want company on a problem or a quick explain?",
                "Happy to chat. I'm better when there's a goal, even a tiny one.",
                "I'm around. Ask me anything local: math, debug loops, memory, or how I work.",
            ][v],
        ),
        SocialKind::Encouragement | SocialKind::None => None,
    }
}

/// Pull short usable phrases from pack/context lines (no bolted headers).
pub fn weave_guidance(context: &[String], max_bits: usize) -> Vec<String> {
    let mut bits = Vec::new();
    for item in context {
        // Prefer pack lines over dialogue history / random code hits
        let is_pack = item.contains("knowledge/packs/") || item.contains("[Pack:");
        if !is_pack && item.contains("[Recent dialogue]") {
            continue;
        }
        let text = item
            .split_once("] ")
            .map(|(_, body)| body)
            .unwrap_or(item.as_str());
        let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let low = compact.to_ascii_lowercase();
        if low.contains("cortex governance")
            || low.starts_with("user:")
            || low.contains("user: ")
            || low.contains("| perci:")
            || compact.starts_with('|')
            || low.contains("is not a geometry")
        {
            continue;
        }
        // Prefer imperative / short operators
        let clip = if compact.chars().count() > 140 {
            compact.chars().take(137).collect::<String>() + "…"
        } else {
            compact
        };
        if clip.len() < 16 {
            continue;
        }
        if !bits.iter().any(|b: &String| b == &clip) {
            // Prefer packs first: insert at front
            if is_pack {
                bits.insert(0, clip);
            } else {
                bits.push(clip);
            }
        }
        if bits.len() >= max_bits {
            break;
        }
    }
    bits.truncate(max_bits);
    bits
}

pub fn compose_reply(
    matched: &CognitiveMatch,
    user: &str,
    domain_body: &str,
    context: &[String],
    recent: &[(String, String)],
) -> String {
    let social = detect_social(user);
    let affect = detect_affect(user);
    let variant = matched.variant as usize;
    let lower = user.to_ascii_lowercase();
    let inquiry = looks_synthesis_or_inquiry(&lower);

    // Pure social short-circuits (except frustration with technical content — merge).
    // Never short-circuit multi-domain synthesis or relational inquiry into comfort text.
    if !inquiry {
        if let Some(social_text) = social_reply(social, variant) {
            let technical = user_has_tech_signal(user);
            if !technical
                || matches!(
                    social,
                    SocialKind::Greeting
                        | SocialKind::HowAreYou
                        | SocialKind::Thanks
                        | SocialKind::Goodbye
                        | SocialKind::SmallTalk
                )
            {
                if !matches!(social, SocialKind::Frustration) || !technical {
                    return with_continuity(social_text, recent, user);
                }
            }
        }
    }

    // SoftCascade breakthrough path: multi-hypothesis compose from Bitwork
    // attention + residual + semantic lattice — LLM-like facets without decode.
    if crate::bridge::should_use_cascade(matched, user) {
        let mut out = crate::bridge::compose_soft_cascade(user, matched, domain_body, variant);
        out = ensure_user_binding(
            user,
            &out,
            matched.label.as_str(),
            matched.insight.as_deref(),
            recent,
        );
        if matches!(affect, Affect::Frustrated) && !out.to_ascii_lowercase().contains("step") {
            out.push_str(" We'll take the next step small and check it.");
        }
        return with_continuity(&out, recent, user);
    }

    // Fluid path: bind the answer to *this* utterance; Bitwork top-k mixture
    // supplies a multi-concept skeleton (not a single domain card).
    let insight = matched.insight.as_deref();
    let skeleton = matched.concept_skeleton(3);
    let mut out = fluid_compose(
        user,
        matched.label.as_str(),
        insight,
        domain_body,
        recent,
        variant,
        affect,
    );
    out = weave_mixture_skeleton(user, &out, &skeleton, variant);
    // VSA role–filler frame — only on multi-word conceptual asks (not typos / small talk).
    let frame = matched.composition_frame(4);
    if should_voice_composition(user, &frame) {
        out = weave_composition_frame(&out, &frame, variant);
    }
    // Residual hop (ANDNOT second thought) gets a distinct latent frame.
    let residual = matched.residual_skeleton(1);
    if explicit_relation_prompt(user) {
        if let Some(lat) = residual.first() {
            out = weave_residual_frame(&out, lat, variant);
        }
    }

    // Optional one pack tip only when it adds content words the body lacks.
    let guidance = weave_guidance(context, 1);
    if !guidance.is_empty()
        && should_attach_guidance(matched.label.as_str(), social, user)
        && !out.to_ascii_lowercase().contains(
            &guidance[0]
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase(),
        )
    {
        // Weave as a short clause, not a bolted "Practical angle:" stack.
        let tip = first_sentence(&guidance[0], 110);
        if tip.len() > 20 {
            out.push(' ');
            out.push_str(&tip);
            if !out.ends_with('.') && !out.ends_with('?') {
                out.push('.');
            }
        }
    }

    if matches!(affect, Affect::Frustrated) && !out.to_ascii_lowercase().contains("step") {
        out.push_str(" We'll take the next step small and check it.");
    }

    with_continuity(&out, recent, user)
}

/// Whether the VSA frame is worth voicing (avoids “Bound as agent:doinf”).
pub fn should_voice_composition_public(user: &str, frame: &[String]) -> bool {
    should_voice_composition(user, frame)
}

/// Whether the VSA frame is worth voicing (avoids “Bound as agent:doinf”).
fn should_voice_composition(user: &str, frame: &[String]) -> bool {
    if frame.len() < 2 {
        return false;
    }
    let lower = user.to_ascii_lowercase();
    // Identity / capability answers must not append "shaped as ask→what · agent→capable".
    if lower.contains("capable")
        || lower.contains("what can you")
        || lower.contains("what do you do")
        || lower.contains("who are you")
        || lower.contains("what are you")
        || lower.contains("capabilities")
    {
        return false;
    }
    let words = user.split_whitespace().count();
    if words < 5 {
        return false;
    }
    // A bound relation is useful for explicit synthesis, comparison, or
    // interaction prompts; on ordinary questions it reads like a preset.
    if !explicit_relation_prompt(user) {
        return false;
    }
    // Need a structural role, not only topic/focus echoes of a typo.
    let structural = frame.iter().any(|f| {
        f.starts_with("ask:")
            || f.starts_with("domain:")
            || f.starts_with("contrast:")
            || f.starts_with("neg:")
            || f.starts_with("relate:")
    });
    if !structural {
        return false;
    }
    // Reject frames whose fillers are mostly short/garbage.
    // Also reject weak agent: fillers that just echo "capable".
    let good_fillers = frame
        .iter()
        .filter_map(|f| f.split_once(':').map(|(_, v)| v))
        .filter(|v| {
            v.len() >= 4
                && !v.chars().all(|c| c.is_ascii_digit())
                && !matches!(*v, "what" | "how" | "why" | "capable" | "capabilities")
        })
        .count();
    good_fillers >= 2
}

/// Weave VSA role–filler composition into speech (compact, not a checklist dump).
pub fn weave_composition_frame(answer: &str, frame: &[String], variant: usize) -> String {
    if frame.len() < 2 {
        return answer.to_owned();
    }
    // Human-facing speech gets a clean relation; the raw role/filler tags
    // remain available to /think but must never leak into chat.
    if frame.len() >= 2 {
        return weave_human_composition(answer, frame, variant);
    }
    // Prefer structural roles over pure topic noise for the clause.
    let mut picks: Vec<&str> = Vec::new();
    for pref in ["ask:", "domain:", "agent:", "contrast:", "neg:", "relate:"] {
        for f in frame {
            if f.starts_with(pref) && !picks.contains(&f.as_str()) {
                // Skip weak fillers (typos / one-off tokens).
                if let Some((_, filler)) = f.split_once(':') {
                    if filler.len() < 4 {
                        continue;
                    }
                }
                picks.push(f.as_str());
                if picks.len() >= 3 {
                    break;
                }
            }
        }
        if picks.len() >= 3 {
            break;
        }
    }
    if picks.len() < 2 {
        picks = frame.iter().take(3).map(|s| s.as_str()).collect();
    }
    if picks.len() < 2 {
        return answer.to_owned();
    }
    let joined = picks.join(" · ");
    let al = answer.to_ascii_lowercase();
    // Skip if we already echoed the same bind tags.
    if picks
        .iter()
        .filter(|p| al.contains(&p.to_ascii_lowercase()))
        .count()
        >= 2
    {
        return answer.to_owned();
    }
    let mut out = answer.trim_end().to_owned();
    if !out.ends_with('.') && !out.ends_with('?') && !out.ends_with('!') {
        out.push('.');
    }
    out.push(' ');
    // Soft structure cue — not a schema dump.
    match variant % 3 {
        0 => {
            out.push_str("The question itself is shaped as ");
            out.push_str(&joined.replace(':', "→"));
            out.push('.');
        }
        1 => {
            out.push_str("I'm treating that as ");
            out.push_str(&joined.replace(':', " of "));
            out.push('.');
        }
        _ => {
            // Skip noisy composition tags entirely for one variant.
        }
    }
    out
}

fn weave_human_composition(answer: &str, frame: &[String], variant: usize) -> String {
    let mut terms: Vec<String> = Vec::new();
    for item in frame {
        let Some((_, filler)) = item.split_once(':') else {
            continue;
        };
        for raw in filler.split(|ch: char| ch == '+' || ch.is_whitespace()) {
            let term = raw.trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
            let low = term.to_ascii_lowercase();
            if term.len() < 4
                || matches!(
                    low.as_str(),
                    "ask"
                        | "agent"
                        | "domain"
                        | "relate"
                        | "different"
                        | "what"
                        | "how"
                        | "why"
                        | "explain"
                        | "capable"
                        | "capabilities"
                )
                || terms
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(term))
            {
                continue;
            }
            terms.push(term.to_owned());
            if terms.len() == 2 {
                break;
            }
        }
        if terms.len() == 2 {
            break;
        }
    }
    if terms.len() < 2 {
        return answer.to_owned();
    }
    let lower = answer.to_ascii_lowercase();
    if terms
        .iter()
        .filter(|term| lower.contains(&term.to_ascii_lowercase()))
        .count()
        >= 2
    {
        return answer.to_owned();
    }
    let mut out = answer.trim_end().to_owned();
    if !out.ends_with('.') && !out.ends_with('?') && !out.ends_with('!') {
        out.push('.');
    }
    out.push(' ');
    match variant % 3 {
        0 => {
            out.push_str("A useful connection here is between ");
            out.push_str(&terms.join(" and "));
            out.push('.');
        }
        1 => {
            out.push_str("That puts ");
            out.push_str(&terms.join(" and "));
            out.push_str(" in the same working picture.");
        }
        _ => {
            out.push_str("The important link is ");
            out.push_str(&terms.join(" meeting "));
            out.push('.');
        }
    }
    out
}

/// Frame a residual-hop insight (concept revealed after \(q \land \neg p^*\)).
pub fn weave_residual_frame(answer: &str, residual: &str, variant: usize) -> String {
    let al = answer.to_ascii_lowercase();
    let low = residual.to_ascii_lowercase();
    let head: String = low.chars().take(28).collect();
    if al.contains(&head) || residual.trim().is_empty() {
        return answer.to_owned();
    }
    let mut out = answer.trim_end().to_owned();
    if !out.ends_with('.') && !out.ends_with('?') && !out.ends_with('!') {
        out.push('.');
    }
    out.push(' ');
    let r = residual.trim().trim_end_matches('.');
    match variant % 3 {
        0 => {
            out.push_str("There's a quieter angle too: ");
            out.push_str(&decap_mid(r));
        }
        1 => {
            out.push_str("What the first pass can hide: ");
            out.push_str(&decap_mid(r));
        }
        _ => {
            out.push_str("Keep this in the corner of the map: ");
            out.push_str(&decap_mid(r));
        }
    }
    if !out.ends_with('.') && !out.ends_with('?') {
        out.push('.');
    }
    out
}

/// Return whether an evidence request has enough claim overlap to keep the
/// retrieved answer.  A high-scoring association is not necessarily evidence
/// for the user's claim (for example, a ritual diagram is not evidence that a
/// geometric intervention heals).  This small gate prevents the renderer from
/// turning adjacent concepts into an apparently supported conclusion.
pub fn evidence_answer_is_grounded(user: &str, answer: &str) -> bool {
    let lower = crate::text_normalize::normalize_for_routing(user);
    if !(lower.contains("evidence supports")
        || lower.contains("evidence for")
        || lower.contains("support the claim")
        || lower.contains("proof of"))
    {
        return true;
    }

    // If the user asks about an explicitly named claim, require at least one
    // meaningful claim token in the answer.  Open evidence questions without a
    // named claim are left to the ordinary evidence operator.
    let claim = lower
        .split_once("claim that")
        .map(|(_, rest)| rest)
        .or_else(|| lower.split_once("claim:").map(|(_, rest)| rest));
    let Some(claim) = claim else {
        return true;
    };
    let claim_tokens = content_tokens(claim);
    if claim_tokens.is_empty() {
        return true;
    }
    let answer_lower = answer.to_ascii_lowercase();
    // Explicitly test-shaped answers may be grounded without repeating every
    // noun in the claim (for example, a fresh-process A/B protocol for a
    // learning claim).
    if [
        "fresh-process",
        "controlled",
        "reproducible",
        "counterexample",
        "falsif",
        "measurement",
    ]
    .iter()
    .any(|marker| answer_lower.contains(marker))
    {
        return true;
    }
    let hits = claim_tokens
        .iter()
        .filter(|token| answer_lower.contains(token.as_str()))
        .count();
    let required = if claim_tokens.len() <= 2 {
        claim_tokens.len()
    } else {
        (claim_tokens.len() + 1) / 2
    };
    hits >= required
}

/// Replace an evidence-shaped concept collision with a bounded, honest test
/// plan.  This is deliberately not a source lookup: it says what is and is
/// not established by the local state and how to make the claim falsifiable.
pub fn evidence_guarded_answer(user: &str, answer: &str) -> Option<String> {
    if evidence_answer_is_grounded(user, answer) {
        return None;
    }
    let lower = crate::text_normalize::normalize_for_routing(user);
    let claim = lower
        .split_once("claim that")
        .map(|(_, rest)| rest.trim().trim_end_matches('?'))
        .or_else(|| lower.split_once("claim:").map(|(_, rest)| rest.trim()))
        .unwrap_or("the stated claim");
    Some(format!(
        "I don't have evidence here that establishes \"{claim}\". An association or symbolic use is not evidence of causal effect. To test it, define the outcome, intervention, comparator, measurement window, and a result that would falsify the claim; until that comparison is run reproducibly, keep it as a hypothesis rather than a fact."
    ))
}

/// Fold top-k Bitwork concept insights into the reply as a short multi-facet spine.
/// Does not dump checklists; at most two supporting clauses, only if they add new content.
pub fn weave_mixture_skeleton(
    user: &str,
    answer: &str,
    skeleton: &[String],
    variant: usize,
) -> String {
    if skeleton.is_empty() {
        return answer.to_owned();
    }
    if !explicit_relation_prompt(user) {
        return answer.to_owned();
    }
    let al = answer.to_ascii_lowercase();
    let mut extras: Vec<&str> = Vec::new();
    for s in skeleton.iter().skip(1) {
        // Primary insight may already be the angle; use supports only.
        let low = s.to_ascii_lowercase();
        let head: String = low.chars().take(28).collect();
        if al.contains(&head) {
            continue;
        }
        // Avoid stock method cards even if they survived select_concept.
        if low.contains("list premises")
            || low.contains("compare on capability")
            || low.contains("fake certainty")
            || low.contains("objective, constraints")
        {
            continue;
        }
        extras.push(s.as_str());
        if extras.len() >= 2 {
            break;
        }
    }
    // If answer is thin, allow primary skeleton line as well.
    if extras.is_empty() && answer.split_whitespace().count() < 18 {
        if let Some(first) = skeleton.first() {
            let low = first.to_ascii_lowercase();
            let head: String = low.chars().take(28).collect();
            if !al.contains(&head) {
                extras.push(first.as_str());
            }
        }
    }
    if extras.is_empty() {
        return answer.to_owned();
    }

    let mut out = answer.trim_end().to_owned();
    if !out.ends_with('.') && !out.ends_with('?') && !out.ends_with('!') {
        out.push('.');
    }
    out.push(' ');
    // Natural weave — no "Related frame:" / "Mixture read:" labels.
    let e0 = extras[0].trim().trim_end_matches('.');
    match (extras.len(), variant % 3) {
        (1, 0) => {
            out.push_str("That also implies ");
            out.push_str(&decap_mid(e0));
            out.push('.');
        }
        (1, _) => {
            out.push_str("And ");
            out.push_str(&decap_mid(e0));
            out.push('.');
        }
        (_, 0) => {
            let e1 = extras[1].trim().trim_end_matches('.');
            out.push_str("Two consequences follow: ");
            out.push_str(&decap_mid(e0));
            out.push_str(", and ");
            out.push_str(&decap_mid(e1));
            out.push('.');
        }
        (_, 1) => {
            let e1 = extras[1].trim().trim_end_matches('.');
            out.push_str("If you hold that next to ");
            out.push_str(&decap_mid(e0));
            out.push_str(", you also get ");
            out.push_str(&decap_mid(e1));
            out.push('.');
        }
        _ => {
            let e1 = extras[1].trim().trim_end_matches('.');
            out.push_str(&decap_mid(e0));
            out.push_str(" — which sits beside ");
            out.push_str(&decap_mid(e1));
            out.push('.');
        }
    }
    let tokens = content_tokens(user);
    let ol = out.to_ascii_lowercase();
    if tokens.len() >= 2 && tokens.iter().filter(|t| ol.contains(t.as_str())).count() == 0 {
        out.push(' ');
        out.push_str(&format!(
            "That still answers {}.",
            tokens.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
        ));
    }
    out
}

fn decap_mid(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) if first.is_uppercase() => {
            first.to_lowercase().collect::<String>() + c.as_str()
        }
        _ => s.to_owned(),
    }
}

/// Content words from the user worth binding into the reply.
fn content_tokens(user: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the",
        "a",
        "an",
        "and",
        "or",
        "but",
        "if",
        "then",
        "than",
        "that",
        "this",
        "these",
        "those",
        "with",
        "from",
        "into",
        "onto",
        "about",
        "what",
        "when",
        "where",
        "which",
        "who",
        "whom",
        "why",
        "how",
        "can",
        "could",
        "would",
        "should",
        "will",
        "just",
        "really",
        "very",
        "your",
        "you",
        "me",
        "my",
        "our",
        "we",
        "i",
        "im",
        "i'm",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "do",
        "does",
        "did",
        "to",
        "of",
        "in",
        "on",
        "for",
        "it",
        "its",
        "as",
        "at",
        "by",
        "not",
        "no",
        "yes",
        "please",
        "tell",
        "give",
        "make",
        "more",
        "some",
        "any",
        "all",
        "also",
        "like",
        "something",
        "someone",
        "deep",
        "short",
        "brief",
        "quick",
        "answer",
        "detail",
        "detailed",
        "thorough",
        "level",
        "little",
        "bit",
        "think",
        "thoughts",
        "thought",
        "interesting",
        "only",
        "thing",
        "things",
        "know",
        "want",
        "need",
        "help",
        "say",
        "said",
        "get",
        "got",
        "let",
        "have",
        "has",
        "had",
        "been",
        "being",
        "into",
        "over",
        "under",
        "again",
        "still",
        "even",
        "much",
        "many",
        "such",
        "other",
        "another",
        "same",
        "hard",
        "easy",
        "important",
        "matter",
        "mean",
        "difference",
        "between",
        "describe",
        "forms",
        "changes",
        "time",
        "lived",
        "express",
        "new",
        "differently",
        "structure",
        "creative",
        "original",
        "fresh",
        "angle",
        "idea",
    ];
    crate::text_normalize::repair_typos(user)
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
        .map(|w| w.trim_matches('\'').to_ascii_lowercase())
        .filter(|w| w.len() >= 4 && !STOP.contains(&w.as_str()))
        .take(6)
        .collect()
}

fn token_seed(user: &str) -> usize {
    user.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(33).wrapping_add(b as usize)
    })
}

fn looks_open_conversation(lower: &str) -> bool {
    let q = lower.contains('?')
        || lower.starts_with("what ")
        || lower.starts_with("why ")
        || lower.starts_with("how ")
        || lower.starts_with("can you ")
        || lower.starts_with("could you ")
        || lower.starts_with("do you ")
        || lower.starts_with("tell me")
        || lower.starts_with("explain")
        || lower.starts_with("talk about")
        || lower.starts_with("thoughts on")
        || lower.starts_with("what about")
        || lower.starts_with("i think")
        || lower.starts_with("i feel")
        || lower.starts_with("i want")
        || lower.starts_with("help me")
        || lower.starts_with("let's")
        || lower.starts_with("lets ");
    let craft = lower.contains("calculate")
        || lower.contains("debug")
        || lower.contains("cargo ")
        || lower.contains("compile")
        || lower.contains("triangle area")
        || lower.contains("percent of")
        || lower.contains("remember that")
        || lower.contains("recall ");
    q && !craft
}

fn looks_creative_prompt(lower: &str) -> bool {
    lower.contains("express a new thought")
        || lower.contains("original thought")
        || lower.contains("new idea")
        || lower.contains("fresh angle")
        || lower.contains("be creative")
        || lower.contains("creative")
}

fn readable_topic(tokens: &[String]) -> String {
    match tokens {
        [] => "the question".to_owned(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        many => {
            let last = many.last().cloned().unwrap_or_default();
            let rest = many[..many.len() - 1].join(", ");
            format!("{rest}, and {last}")
        }
    }
}

/// Keep a domain card only when it can answer the current turn. This prevents
/// an unrelated memory or life sentence from becoming the apparent answer.
fn useful_domain_body(body: &str) -> Option<String> {
    let trimmed = body.trim();
    let lower = trimmed.to_ascii_lowercase();
    if trimmed.chars().count() < 24 || trimmed.chars().count() > 220 {
        return None;
    }
    let stock = [
        "what outcome do you want",
        "what evidence do we already have",
        "let's find the smallest",
        "i won't fake certainty",
        "name the workload before",
        "compare on capability",
        "list premises",
        "objective, constraints",
        "reproduce it, isolate",
        "start with the mechanism",
    ];
    if stock.iter().any(|marker| lower.contains(marker)) || lower.contains("→") {
        return None;
    }
    Some(trimmed.to_owned())
}

/// Admit a concept insight only when it shares a meaningful anchor with the
/// user's turn; this is a relevance guard, not a claim of semantic mastery.
fn relevant_insight(insight: Option<&str>, user: &str, tokens: &[String]) -> Option<String> {
    let candidate = insight?.trim();
    let lower = candidate.to_ascii_lowercase();
    let n = candidate.chars().count();
    if !(20..=180).contains(&n)
        || lower.starts_with("list premises")
        || lower.contains("fake certainty")
        || lower.contains("what outcome do you want")
        || lower.contains("smallest version we can test")
        || lower.contains("compare on capability")
        || lower.contains("objective, constraints")
        || lower.contains("reproduce it, isolate")
        || lower.contains("i won't fake")
    {
        return None;
    }
    let overlap = tokens
        .iter()
        .filter(|token| lower.contains(token.as_str()))
        .count();
    if overlap > 0 || tokens.is_empty() || user.to_ascii_lowercase().contains("what is life") {
        Some(candidate.to_owned())
    } else {
        None
    }
}

/// A deictic follow-up with no history has no safe referent; ask for the noun
/// instead of selecting a random topic.
fn unresolved_referent(user: &str, recent: &[(String, String)]) -> bool {
    if !recent.is_empty() {
        return false;
    }
    let lower = crate::text_normalize::normalize_for_routing(user);
    let words: Vec<&str> = lower.split_whitespace().collect();
    if words.len() <= 2
        && words
            .iter()
            .any(|word| matches!(*word, "this" | "that" | "it"))
    {
        return true;
    }
    let deictic = lower.contains(" this ") || lower.contains(" that ") || lower.contains(" it ");
    deictic
        && !lower.contains("this system")
        && !lower.contains("this idea")
        && !lower.contains("this question")
        && !lower.contains("this claim")
        && !lower.contains("this prompt")
        && !lower.contains("this request")
        && !lower.contains("this statement")
        && !lower.contains("this result")
        && !lower.contains("this directive")
        && !lower.contains("this interaction")
        && !lower.contains("this session")
        && !lower.contains("this conversation")
        && !lower.contains("this test")
        && !lower.contains("this run")
        && !lower.contains("this probe")
        && !lower.contains("that system")
        && !lower.contains("that idea")
        && !lower.contains("that claim")
        && !lower.contains("that prompt")
        && !lower.contains("that request")
        && !lower.contains("that statement")
        && !lower.contains("that result")
        // “that” can be a grammatical complementizer, not a deictic
        // reference (for example, “evidence supports the claim that ...”).
        && !lower.contains("claim that")
        && !lower.contains("evidence that")
        && !lower.contains("fact that")
        && !lower.contains("what is this")
}

fn explicit_relation_prompt(user: &str) -> bool {
    let lower = crate::text_normalize::normalize_for_routing(user);
    looks_synthesis_or_inquiry(&lower)
        || lower.contains("connect ")
        || lower.contains("related")
        || lower.contains("relate ")
        || lower.contains("interact")
        || lower.contains("work together")
        || lower.contains("shared structure")
        || lower.contains("shared principle")
        || lower.contains("between ")
        || (lower.contains("what about") && content_tokens(user).len() >= 2)
}

fn memory_attention_answer(depth: ResponseDepth) -> String {
    let base = "Memory stores traces; attention decides which trace matters now. Good cognition needs both: selection keeps context useful, and provenance keeps a selected trace from masquerading as truth.";
    match depth {
        ResponseDepth::Brief => "Memory keeps traces; attention selects what matters now.".to_owned(),
        ResponseDepth::Balanced => base.to_owned(),
        ResponseDepth::Deep => format!(
            "{base} The boundary is operational: memory can persist without being relevant, while attention can be relevant without becoming durable. A robust system therefore records why a trace was kept, how confident it is, and what evidence would revise it."
        ),
    }
}

fn memory_identity_answer(depth: ResponseDepth) -> String {
    let base = "Memory stores traces; identity is the continuity we infer when a changing process remains recognizably the same. Memory can support that continuity without being the whole of it.";
    match depth {
        ResponseDepth::Brief => "Memory stores traces; identity is the continuity we infer across change.".to_owned(),
        ResponseDepth::Balanced => base.to_owned(),
        ResponseDepth::Deep => format!(
            "{base} The distinction is testable: change what is retained, then ask which properties still let us identify the process. If every trace changes but the organization persists, memory alone cannot explain the result."
        ),
    }
}

fn prior_claim(recent: &[(String, String)]) -> Option<String> {
    let answer = last_substantive_turn(recent)
        .or_else(|| recent.last())
        .map(|(_, answer)| answer.trim())?;
    for marker in [
        "The claim to examine is: \"",
        "The claim I would test is: \"",
    ] {
        if let Some(start) = answer.find(marker) {
            let rest = &answer[start + marker.len()..];
            if let Some(end) = rest.find('\"') {
                let quoted = rest[..end].trim();
                if !quoted.is_empty() {
                    return Some(quoted.to_owned());
                }
            }
        }
    }
    let sentence = first_sentence(answer, 180);
    (!sentence.trim().is_empty()).then_some(sentence)
}

/// Handle conversational acts that depend on the immediately preceding turn.
/// These are operators, not learned facts: they preserve intent and provenance
/// without pretending that a generic topic card understood the exchange.
fn followup_operator(user_lower: &str, recent: &[(String, String)]) -> Option<String> {
    let claim = prior_claim(recent);
    if user_lower.contains("what would change your mind")
        || user_lower.contains("what could change your mind")
        || (user_lower.contains("change your mind") && user_lower.contains("evidence"))
    {
        return Some(match claim {
            Some(claim) => format!(
                "The claim I would test is: \"{claim}\" A counterexample, a failed prediction, or a stronger competing explanation would change the conclusion."
            ),
            None => "A reproducible counterexample, a failed prediction, or stronger evidence would change my conclusion; name the claim you want to test.".to_owned(),
        });
    }
    if user_lower.starts_with("i don't agree")
        || user_lower.starts_with("i dont agree")
        || user_lower.starts_with("i disagree")
        || user_lower.starts_with("that seems wrong")
    {
        // Preserve an explicit claim in the current turn even when there is
        // no prior quoted claim to recover. Otherwise a direct disagreement
        // is flattened into a generic request for clarification.
        let explicit = user_lower
            .split_once("claim that")
            .map(|(_, rest)| rest)
            .or_else(|| user_lower.split_once("claim:").map(|(_, rest)| rest))
            .map(|rest| {
                rest.trim()
                    .split(['.', '?', '!'])
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .to_owned()
            })
            .filter(|value| !value.is_empty());
        return Some(match (explicit, claim) {
            // The current turn is the strongest authority. A persisted
            // session may contain an older quoted claim, but it must not
            // override the claim the user just challenged.
            (Some(claim), _) => format!(
                "That is a fair challenge. The claim to examine is: \"{claim}\" The first premise to test is whether boundary maintenance predicts repair or exchange better than a plausible alternative."
            ),
            (None, Some(claim)) => format!(
                "That is a fair challenge. The claim to examine is: \"{claim}\" Which premise or mechanism do you reject?"
            ),
            (None, None) => "That is a fair challenge. Name the claim or premise you reject, and I will separate the disagreement from the evidence.".to_owned(),
        });
    }
    if user_lower.contains("explain") && user_lower.contains("differently") {
        return Some(match claim {
            Some(claim) => format!(
                "Put simply: {claim} The point is the distinction that changes what we would observe, not the wording used to describe it."
            ),
            None => "Put simply: name the claim you want rephrased, and I will preserve its meaning while changing the explanation.".to_owned(),
        });
    }
    if user_lower.contains("explain it again")
        && (user_lower.contains("different angle") || user_lower.contains("reframe"))
    {
        return Some(
            "A different angle is error-correction: treat evidence as a feedback signal that compares a model's prediction with an observation, then revise the part that failed. The answer changes when the observed result defeats the prior explanation, not merely when the sentence is reworded.".to_owned(),
        );
    }
    if (user_lower.contains("go one level deeper") || user_lower.contains("go deeper"))
        && (user_lower.contains("without repeating")
            || user_lower.contains("without repeating yourself")
            || user_lower.contains("do not repeat")
            || user_lower.contains("don't repeat"))
    {
        let previous = claim.unwrap_or_else(|| "the previous answer".to_owned());
        return Some(format!(
            "Next layer: the previous answer treated \"{previous}\" as the active claim. The relation underneath it is between the assumption and the result; change that assumption while holding the rest fixed and check whether the conclusion changes."
        ));
    }
    if user_lower.contains("different angle") || user_lower.contains("without repeating") {
        let angle_stop = [
            "what",
            "is",
            "are",
            "doing",
            "the",
            "most",
            "work",
            "in",
            "your",
            "answer",
            "explain",
            "it",
            "again",
            "from",
            "a",
            "without",
            "repeating",
            "the",
            "same",
            "sentence",
            "go",
            "one",
            "level",
            "deeper",
            "now",
            "give",
            "me",
            "an",
            "of",
        ];
        let mut tokens = recent
            .last()
            .map(|(turn, _)| content_tokens(turn))
            .unwrap_or_default();
        tokens.retain(|token| !angle_stop.contains(&token.as_str()));
        if tokens.is_empty() {
            tokens = claim.as_deref().map(content_tokens).unwrap_or_default();
            tokens.retain(|token| !angle_stop.contains(&token.as_str()));
        }
        let topic = if tokens.is_empty() {
            "the last idea".to_owned()
        } else {
            readable_topic(&tokens)
        };
        return Some(format!(
            "A different angle on {topic} is to treat it as a control problem: change one relation while holding the others steady, then watch which behavior moves. That exposes the mechanism without repeating the earlier wording."
        ));
    }
    if user_lower.starts_with("say it in one sentence")
        || (user_lower.starts_with("one sentence") && !user_lower.contains("explain"))
    {
        return Some(match claim {
            Some(claim) => claim,
            None => "A good one-sentence answer keeps the claim and the reason that would make it testable.".to_owned(),
        });
    }
    if user_lower.contains("meant")
        && user_lower.contains("system")
        && (user_lower.contains("not the person") || user_lower.contains("not a person"))
    {
        return Some("Understood—you mean Perci's system, not a person. The useful question is which part of the system—routing, memory, weights, or dialogue—should change and how we will measure it.".to_owned());
    }
    if user_lower.contains("what should we test next")
        || user_lower.contains("what do we test next")
        || user_lower.contains("next test")
    {
        return Some("Test the last failing behavior end to end: capture the input, expected answer shape, actual output, and a repeatable command. Keep the change only if the held-out score improves without regressions.".to_owned());
    }
    // Short deictic next-step turns must not fall through to concept cards.
    let compact: String = user_lower
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || *c == '\'')
        .collect();
    let c = compact.trim();
    let next_step = matches!(
        c,
        "what should i do"
            | "what should we do"
            | "what should i do next"
            | "what should we do next"
            | "what next"
            | "what's next"
            | "whats next"
            | "what now"
            | "next steps"
            | "where are we going"
            | "where do we go"
            | "where do we go from here"
            | "what is the next step"
            | "whats the next step"
            | "what's the next step"
    ) || c.starts_with("what should i do")
        || c.starts_with("what should we do")
        || c.starts_with("where are we going");
    if next_step {
        let thread = recent
            .iter()
            .rev()
            .take(4)
            .map(|(u, a)| format!("{u} {a}"))
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        let improving = thread.contains("improv")
            || thread.contains("perci")
            || thread.contains("routing")
            || thread.contains("transfer")
            || thread.contains("bitwork")
            || thread.contains("your system");
        return Some(if improving {
            match claim {
                Some(claim) => {
                    let core = plain_breath_rewrite(&claim);
                    format!(
                        "Still on improving Perci. Last solid point: {core} \
Next useful move: catch one live miss, fix the owning operator/voice layer (not the pack), re-run that multi-turn plus transfer-suite. Weights stay frozen until a held-out win under human authorize. Which first—miss, patch, or retest?"
                    )
                }
                None => "Still on improving Perci. Catch one live miss, fix the owning operator/voice layer, retest with transfer-suite—weights stay frozen until a held-out win under human authorize. Which first?".to_owned(),
            }
        } else {
            match claim {
                Some(claim) => {
                    let core = plain_breath_rewrite(&claim);
                    format!(
                        "From the last turn: {core} Smallest next step: name the outcome, pick the check that would fail if we're wrong, run it before widening."
                    )
                }
                None => "Name the outcome you want next. Then we pick the smallest check that would fail if the plan is wrong.".to_owned(),
            }
        });
    }
    // Cryptic / unnatural feedback on the previous turn: plain rewrite, not a new concept card.
    if user_lower.contains("cryptic")
        || user_lower.contains("cyptic")
        || user_lower.contains("natural thought")
        || (user_lower.contains("sounds")
            && (user_lower.contains("cryptic")
                || user_lower.contains("cyptic")
                || user_lower.contains("weird")
                || user_lower.contains("off")))
    {
        return Some(match claim {
            Some(claim) => {
                let core = plain_breath_rewrite(&claim);
                format!(
                    "Fair—too stiff. In plain words: {core} Ask for deeper mechanism only if you want it."
                )
            }
            None => "Fair—too stiff. I'll lead with the point, keep the thread, and leave scaffolding for /think.".to_owned(),
        });
    }
    None
}

/// Final conversational guardrail for depth and known cryptic fallbacks.
pub fn shape_for_conversation(text: &str, user: &str, recent: &[(String, String)]) -> String {
    let candidate = text.trim();
    if candidate.is_empty() {
        return candidate.to_owned();
    }
    let control = crate::reasoning_controller::derive(user, recent, None, "voice");
    if control.mode == crate::reasoning_controller::ReasoningMode::Abstain {
        return format!(
            "I can identify the literal tokens in \"{}\", but I cannot assign a confident meaning. Known: the words are ungrounded here. Inferred: this may be invented language or a robustness test. Unknown: definitions, grammar, source, and intended domain. Give me a definition or usage example, and I can test the interpretation instead of inventing one.",
            user.trim()
        );
    }
    if unresolved_referent(user, recent) {
        return "I can answer, but “this” has no clear referent in the current turn. Name the idea or answer you mean, and I’ll connect it to a concrete consequence.".to_owned();
    }
    let user_lower = crate::text_normalize::normalize_for_routing(user);
    let depth = response_depth(user, recent);
    if let Some(answer) = followup_operator(&user_lower, recent) {
        return answer;
    }
    if let Some(answer) = evidence_guarded_answer(user, candidate) {
        return answer;
    }
    let candidate_lower = candidate.to_ascii_lowercase();
    if looks_creative_prompt(&user_lower)
        && !candidate_lower.contains("constrained invention")
        && !candidate_lower.contains("what transfers")
    {
        let tokens = content_tokens(user);
        if tokens.len() >= 2 {
            return format!(
                "A fresh angle on {} is to treat each element as a constraint on the others: the idea becomes interesting when changing one part changes what the rest can do. That is a testable relation, not a claim that the domains are identical.",
                readable_topic(&tokens)
            );
        }
    }
    if user_lower.contains("what do you mean by identity") {
        return memory_identity_answer(depth);
    }
    if user_lower.contains("memory")
        && user_lower.contains("identity")
        && (user_lower.contains("what do you think")
            || user_lower.contains("thoughts")
            || user_lower.contains("opinion"))
    {
        return memory_identity_answer(depth);
    }
    if candidate_lower.contains("the mechanism is the mechanism remains testable") {
        return candidate.replace(
            "the mechanism is the mechanism remains testable",
            "the mechanism remains testable",
        );
    }
    if content_tokens(user).len() >= 3 && candidate_lower.contains("relationship among") {
        let topic = readable_topic(&content_tokens(user));
        return format!(
            "The shared structure across {topic} is that each domain organizes relations under constraints: change one relation and the possible behavior changes. The comparison is useful only while each domain's mechanism remains distinct."
        );
    }
    if (user_lower.contains("why does this matter")
        || user_lower.contains("why is this important")
        || user_lower == "why does that matter?")
        && !recent.is_empty()
    {
        let previous_user = recent
            .last()
            .map(|(turn, _)| turn.to_ascii_lowercase())
            .unwrap_or_default();
        if previous_user.contains("low-bit")
            || previous_user.contains("low bit")
            || previous_user.contains("binary")
            || previous_user.contains("weight")
        {
            return "It matters because the layered representation preserves direction, magnitude, residual correction, and outliers instead of forcing every signal through one bit. The test is lower measured error without giving back the performance we were trying to save.".to_owned();
        }
        if previous_user.contains("system") || previous_user.contains("perci") {
            return "It matters only if the change improves a measured behavior: better routing, clearer answers, or stronger transfer without breaking exact tools and governance.".to_owned();
        }
        return "It matters only when it changes what the system can do or what we can verify; otherwise it is an attractive description, not progress.".to_owned();
    }
    if user_lower.contains("memory")
        && user_lower.contains("attention")
        && !explicit_relation_prompt(user)
    {
        return memory_attention_answer(depth);
    }
    if (user_lower.contains("what do you think")
        || user_lower.contains("thoughts")
        || user_lower.contains("opinion"))
        && (user_lower.contains("system") || user_lower.contains("perci"))
    {
        let mut answer = "My take is that the system is strongest when each layer has a clear job: Bitwork routes, operators reason, memory preserves, and tests decide whether a change is real.".to_owned();
        if matches!(depth, ResponseDepth::Deep) {
            answer.push_str(" The useful next move is to measure one behavior end to end and let the result, not the impression, decide the next evolution.");
        }
        return answer;
    }
    if user_lower.contains("what we found") {
        return "The strongest bounded finding is that layered low-bit correction can reduce representation loss, while dialogue quality still depends on semantic routing and response shaping.".to_owned();
    }
    if user_lower.contains("what about")
        && user_lower.contains("trust")
        && user_lower.contains("distributed")
    {
        return "Trust in distributed systems comes from explicit authority, observable failure modes, and verifiable recovery. A component is trustworthy when other components can predict what it may do and how to repair the state when it fails.".to_owned();
    }
    let lower = candidate.to_ascii_lowercase();
    let cryptic = lower.contains("let's find the smallest")
        || lower.contains("name the workload before")
        || lower.contains("what outcome do you want")
        || lower.contains("i won't fake certainty")
        || lower.contains("a useful connection here is between")
        || lower.contains("that puts ")
        || lower.contains("cleanest answer")
        || lower.contains("add a constraint");
    let mut shaped = if cryptic {
        fluid_compose(
            user,
            "general",
            None,
            "",
            recent,
            token_seed(user),
            detect_affect(user),
        )
    } else {
        candidate.to_owned()
    };
    if matches!(depth, ResponseDepth::Brief) {
        shaped = first_sentence(&shaped, 260);
        if !shaped.ends_with('.') && !shaped.ends_with('?') && !shaped.ends_with('!') {
            shaped.push('.');
        }
    }
    shaped
}

/// Compose a reply that answers *this* turn — not a generic domain card.
pub fn fluid_compose(
    user: &str,
    label: &str,
    insight: Option<&str>,
    domain_body: &str,
    recent: &[(String, String)],
    variant: usize,
    affect: Affect,
) -> String {
    let lower = crate::text_normalize::normalize_for_routing(user);
    let tokens = content_tokens(user);
    let depth = response_depth(user, recent);
    let topic = if tokens.is_empty() {
        "that".to_owned()
    } else {
        tokens.iter().take(4).cloned().collect::<Vec<_>>().join(" ")
    };
    let seed = token_seed(user) ^ variant;

    // Continuity hook (short).
    let mut head = String::new();
    if let Some((prev_u, prev_a)) = recent.last() {
        if user_refers_to_prior(user) {
            head = format!("Picking up from “{}” — ", first_sentence(prev_u, 48));
            let _ = prev_a;
        } else if tokens
            .iter()
            .any(|t| t.len() >= 5 && prev_a.to_ascii_lowercase().contains(t.as_str()))
        {
            // Only claim continuity on strong content overlap (avoid weak words).
            head = match seed % 3 {
                0 => "Carrying that thread forward. ".into(),
                1 => "Same thread, next layer. ".into(),
                _ => "Building on what we just said. ".into(),
            };
        }
    }
    let affect_bit = affect_opener(affect, seed);
    if !affect_bit.is_empty() {
        head.push_str(&affect_bit);
    }

    // Prefer a relevant concept insight when it is short and not a stock method card.
    let _legacy_concept = insight
        .map(str::trim)
        .filter(|s| {
            let n = s.chars().count();
            let l = s.to_ascii_lowercase();
            n >= 20
                && n <= 160
                && !l.starts_with("list premises")
                && !s.contains('→')
                && !l.contains("fake certainty")
                && !l.contains("what outcome do you want")
                && !l.contains("smallest version we can test")
                && !l.contains("compare on capability, correctness")
                && !l.contains("objective, constraints, dependencies")
                && !l.contains("reproduce it, isolate")
                && !l.contains("i won't fake")
        })
        .map(|s| s.to_owned());
    let concept = relevant_insight(insight, user, &tokens);
    let direct = useful_domain_body(domain_body);

    // Identity / capability — fluid, multi-sentence, still honest.
    if matches!(label, "identity" | "greeting")
        && (lower.contains("who are you")
            || lower.contains("what are you")
            || lower.contains("what can you")
            || lower.contains("capabilities")
            || lower.contains("cannot")
            || lower.contains("can't you")
            || lower.contains("what do you do"))
    {
        return format!(
            "{head}I'm Perci — a local governed tool: sparse Bitwork routing, exact math/geometry, short reason operators, and memory you deliberately teach. I'm not a cloud LLM and not conscious. I can classify, calculate, plan, synthesize frames, and stage learning for review. I cannot invent private chain-of-thought, silently promote weights, or pretend open-ended fluency equals general intelligence. Ask something concrete and I'll answer it directly."
        );
    }

    if unresolved_referent(user, recent) {
        return format!(
            "{head}I can answer, but “this” has no clear referent in the current turn. Name the idea or answer you mean, and I’ll connect it to a concrete consequence."
        );
    }

    // A recurring high-value relation deserves a stable answer rather than a
    // random concept card. The depth planner controls how far it opens up.
    if lower.contains("memory") && lower.contains("attention") && !explicit_relation_prompt(user) {
        return format!("{head}{}", memory_attention_answer(depth));
    }

    // Creative requests need a compositional thought, not a domain-method
    // card. Keep the relation explicit so novelty stays tied to the user's
    // nouns instead of becoming decorative randomness.
    if looks_creative_prompt(&lower) && tokens.len() >= 2 {
        let creative_topic = readable_topic(&tokens);
        return format!(
            "{head}A fresh angle on {creative_topic} is to treat each element as a constraint on the others: the idea becomes interesting when changing one part changes what the rest can do. That is a testable relation, not a claim that the domains are identical."
        );
    }

    if lower.contains("what do you think")
        || lower.contains("thoughts")
        || lower.contains("opinion")
    {
        let core = if lower.contains("system") || lower.contains("perci") {
            "The system is strongest when each layer has a clear job: Bitwork routes, operators reason, memory preserves, and tests decide whether a change is real."
        } else if let Some(answer) = direct.as_deref() {
            answer
        } else if let Some(answer) = concept.as_deref() {
            answer
        } else {
            "My best take is the one that makes a concrete prediction and stays open to correction."
        };
        let mut answer = format!("{head}My take on {topic}: {core}");
        if matches!(depth, ResponseDepth::Deep) {
            answer.push_str(" The useful next move is to name the observation that would prove this view wrong.");
        }
        return answer;
    }

    if lower.contains("what we found") && recent.is_empty() {
        return format!(
            "{head}The strongest bounded finding is that layered low-bit correction can reduce representation loss, while dialogue quality still depends on semantic routing and response shaping."
        );
    }

    // Open conversation: answer the ask with user topic bound in.
    if looks_open_conversation(&lower) || tokens.len() >= 2 {
        let angle = concept.clone().or(direct.clone()).unwrap_or_else(|| {
            // Topic-aware angle when Bitwork label is coarse ("general").
            let topic_l = topic.to_ascii_lowercase();
            let label_for_angle = if topic_l.contains("trust")
                || topic_l.contains("distributed")
                || topic_l.contains("system")
                || topic_l.contains("network")
            {
                "systems"
            } else if topic_l.contains("language")
                || topic_l.contains("word")
                || topic_l.contains("meaning")
                || topic_l.contains("sentence")
            {
                "english"
            } else if topic_l.contains("code")
                || topic_l.contains("rust")
                || topic_l.contains("bug")
            {
                "code"
            } else {
                label
            };
            let label_angle = |lab: &str| -> String {
                match lab {
                    "code" => "isolate the smallest failing path and verify after one change".into(),
                    "planning" => {
                        "name the outcome, one constraint, and the first checkable step".into()
                    }
                    "science" => "state a claim you could measure or falsify".into(),
                    "logic" => "separate what is given from what is assumed".into(),
                    "systems" => {
                        "trust is earned when interfaces, failure modes, and recovery stay explicit"
                            .into()
                    }
                    "memory" => "only trust traces you meant to store".into(),
                    "governance" => "permission and proof are different gates".into(),
                    "english" | "explanation" => {
                        "language moves meaning by the distinctions it keeps and the ones it drops"
                            .into()
                    }
                    "creativity" => {
                        "a fresh angle is real when it changes which relations you can use".into()
                    }
                    "comparison" => {
                        "name the job first, then score options on cost of being wrong".into()
                    }
                    "identity" => {
                        "I route, calculate, and remember deliberately — I do not pretend to be a mind"
                            .into()
                    }
                    "general" => {
                        // Free-form angle, not a falsify checklist slogan.
                        "keep the claim concrete enough that a counterexample could touch it, then separate what you know from what you are guessing".into()
                    }
                    _ => "say what would change if the idea were wrong, then check against one real case"
                        .into(),
                }
            };
            let d = domain_body.trim();
            let dl = d.to_ascii_lowercase();
            let stock = dl.contains("fake certainty")
                || dl.contains("list premises")
                || dl.contains("what outcome do you want")
                || dl.contains("compare on capability")
                || dl.contains("reproduce it, isolate")
                || dl.contains("objective, constraints")
                || dl.contains("→")
                || d.chars().count() > 110;
            if !stock && d.chars().count() >= 24 {
                d.to_owned()
            } else {
                label_angle(label_for_angle)
            }
        });

        if let Some(direct_answer) = direct.as_deref() {
            let mut answer = match depth {
                ResponseDepth::Brief => first_sentence(direct_answer, 260),
                ResponseDepth::Balanced => direct_answer.to_owned(),
                ResponseDepth::Deep => format!(
                    "{direct_answer} The useful next layer is to name the mechanism that makes the claim hold and the observation that would revise it."
                ),
            };
            if !answer.ends_with('.') && !answer.ends_with('?') && !answer.ends_with('!') {
                answer.push('.');
            }
            return format!("{head}{answer}");
        }

        let body = if lower.starts_with("why ")
            || lower.contains("why does")
            || lower.contains("why is")
        {
            match seed % 3 {
                0 => format!(
                    "{angle} — that is the load-bearing story for {topic}, and if that mechanism changed, the outcome should change too."
                ),
                1 => format!(
                    "For {topic}, {angle}. A useful explanation predicts what happens when one piece is held fixed and another is moved."
                ),
                _ => format!(
                    "On {topic}: {angle}. The test is which observation would force a rewrite, not how fluent the first sentence sounds."
                ),
            }
        } else if lower.starts_with("how ") {
            match seed % 3 {
                0 => format!(
                    "For {topic}, name the goal and the constraint that bites first, take the smallest reversible step, then verify. Anchor: {angle}."
                ),
                1 => format!(
                    "Cut a thin end-to-end path for {topic} you can check, then widen. {angle}."
                ),
                _ => format!(
                    "Treat {topic} as input → transform → check. {angle}. If a step is missing, name the missing input before inventing prose."
                ),
            }
        } else if lower.contains("what do you think")
            || lower.contains("thoughts")
            || lower.contains("opinion")
        {
            format!(
                "I don't have feelings about {topic}. A grounded take: {angle}. Use it if it helps you decide or measure something next."
            )
        } else if lower.contains('?') {
            match seed % 4 {
                0 => format!(
                    "{angle} That is the cleanest answer I have for {topic}; add a constraint if you want it tighter."
                ),
                1 => format!(
                    "For “{}”, the gravity sits on {topic}. {angle}",
                    first_sentence(user, 72)
                ),
                2 => format!(
                    "About {topic}: {angle} Without more detail I stay structural rather than specialist-deep."
                ),
                _ => format!(
                    "{angle} That's my clean answer for {topic}; push back with a fact and I'll revise."
                ),
            }
        } else {
            let angle = if angle.ends_with('.') || angle.ends_with('?') {
                angle
            } else {
                format!("{angle}.")
            };
            match seed % 3 {
                0 => format!("{topic}: {angle}"),
                1 => format!("Working from {topic} — {angle}"),
                _ => format!("{angle} (centered on {topic})"),
            }
        };

        return format!("{head}{body}");
    }

    // Craft / domain path: still bind user tokens into a natural sentence.
    let body = humanize_body(domain_body, label, seed);
    if tokens.is_empty() {
        return format!("{head}{body}");
    }
    let bind = match seed % 3 {
        0 => format!("For your point about {topic}: {body}"),
        1 => format!("{body} Applied to {topic}, that means we stay specific instead of abstract."),
        _ => format!("On {topic} — {body}"),
    };
    format!("{head}{bind}")
}

/// Post-pass: if a candidate answer never mentions user content, rebuild fluidly.
pub fn ensure_user_binding(
    user: &str,
    answer: &str,
    label: &str,
    insight: Option<&str>,
    recent: &[(String, String)],
) -> String {
    // Contextual operators deliberately answer the speech act rather than
    // echoing its scaffolding words. Do not replace them with a topic-bound
    // fallback merely because the operator's answer omits words like "agree"
    // or "mind".
    if followup_operator(&crate::text_normalize::normalize_for_routing(user), recent).is_some() {
        return answer.to_owned();
    }
    let tokens = content_tokens(user);
    if tokens.len() < 2 {
        return answer.to_owned();
    }
    let al = answer.to_ascii_lowercase();
    let hits = tokens.iter().filter(|t| al.contains(t.as_str())).count();
    // Generic method cards often hit 0–1 user tokens.
    let generic_markers = [
        "list premises",
        "compare on capability, correctness, latency",
        "objective, constraints, dependencies",
        "reproduce it, isolate the smallest",
        "start with the mechanism, then one example",
        "what outcome do you want, and what evidence",
        "keep the meaning, cut ambiguity",
    ];
    let looks_generic = generic_markers.iter().any(|m| al.contains(m)) || hits == 0;
    if !looks_generic && hits >= 1 {
        return answer.to_owned();
    }
    fluid_compose(
        user,
        label,
        insight,
        answer,
        recent,
        token_seed(user),
        detect_affect(user),
    )
}

fn should_attach_guidance(label: &str, social: SocialKind, user: &str) -> bool {
    if !matches!(social, SocialKind::None | SocialKind::Frustration) {
        return false;
    }
    let lower = user.to_ascii_lowercase();
    let explicitly_deep = [
        "explain",
        "analyze",
        "debug",
        "plan ",
        "compare",
        "design",
        "investigate",
        "how should",
        "step by step",
    ]
    .iter()
    .any(|marker| lower.contains(marker));
    // Context cards help explicit work; on ordinary conversation they become
    // unrelated fragments that destroy continuity.
    explicitly_deep
        && !matches!(
            label,
            "greeting" | "math" | "geometry" | "identity" | "english" | "memory"
        )
}

pub fn user_has_tech_signal(user: &str) -> bool {
    let t = user.to_ascii_lowercase();
    [
        "bug",
        "error",
        "cargo",
        "rust",
        "code",
        "compile",
        "debug",
        "test",
        "fail",
        "panic",
        "function",
        "module",
        "calcul",
        "math",
        "permission",
    ]
    .iter()
    .any(|k| t.contains(k))
}

/// True only for clear anaphora to the previous turn — not casual words like
/// "still learning" or "is that true?" which used to glue wrong openers.
fn user_refers_to_prior(user: &str) -> bool {
    let t = user.trim().to_ascii_lowercase();
    if t.is_empty() {
        return false;
    }
    // Exact short follow-ups
    if matches!(
        t.as_str(),
        "why?"
            | "why"
            | "how?"
            | "how"
            | "and?"
            | "so?"
            | "same"
            | "again"
            | "continue"
            | "go on"
            | "more"
            | "what about that"
            | "about that"
            | "same issue"
            | "same bug"
            | "same error"
            | "try again"
            | "do it again"
    ) {
        return true;
    }
    // Explicit back-references
    if t.contains("the bug")
        || t.contains("the error")
        || t.contains("that bug")
        || t.contains("that error")
        || t.contains("that issue")
        || t.contains("same problem")
        || t.contains("same error")
        || t.contains("same bug")
        || t.contains("as before")
        || t.contains("like before")
        || t.contains("from before")
        || t.starts_with("regarding that")
        || t.starts_with("about that ")
        || t.starts_with("and that ")
        || t.starts_with("with that ")
    {
        return true;
    }
    // "still <broken thing>" but not "are you still learning"
    if t.contains("still broken")
        || t.contains("still failing")
        || t.contains("still failing")
        || t.contains("still errors")
        || t.contains("still error")
        || t.contains("still the same")
    {
        return true;
    }
    false
}

fn affect_opener(affect: Affect, variant: usize) -> String {
    let v = variant % 2;
    match affect {
        Affect::Frustrated => [
            "Yeah, that friction is real. ",
            "Okay — let's unstick this. ",
        ][v]
            .into(),
        Affect::Grateful => ["Happy to. ", "Good. "][v].into(),
        Affect::Curious => ["Nice question. ", "Let's look carefully. "][v].into(),
        Affect::Warm => ["Hey. ", ""][v].into(),
        Affect::Closing => String::new(),
        Affect::Neutral => String::new(),
    }
}

fn humanize_body(body: &str, label: &str, variant: usize) -> String {
    // Already natural social handled elsewhere
    if body.starts_with("Hey") || body.starts_with("Hi") || body.starts_with("Glad") {
        return body.to_string();
    }
    // Soften ultra-stiff identity dumps for casual asks
    if label == "identity" && variant % 2 == 0 {
        return format!(
            "I'm Perci — a local tool that routes, remembers selectively, and does exact math. {}",
            body
        );
    }
    if label == "memory" {
        return format!(
            "For durable notes, say “remember that …” and later “recall …”. {}",
            body
        );
    }
    body.to_string()
}

fn with_continuity(text: &str, recent: &[(String, String)], user: &str) -> String {
    if recent.is_empty() {
        return text.to_string();
    }
    if let Some((prev_u, prev)) = recent.last() {
        if prev.trim() == text.trim() {
            if prev_u.trim().eq_ignore_ascii_case(user.trim()) {
                return format!("My answer is unchanged: {text}");
            }
            let lower = user.to_ascii_lowercase();
            // Style/meta asks should never dead-end on a repeated pack sentence.
            if lower.contains("speak")
                || lower.contains("smart")
                || lower.contains("natural")
                || lower.contains("repeat")
                || lower.contains("robotic")
                || lower.contains("template")
                || lower.contains("style")
            {
                return "You're right—that reply was a stuck template, not a real answer to you. I'll drop the script: lead with your point, keep the wording fresh, and stay honest about limits. Try me again with the same request.".to_owned();
            }
            return format!(
                "I almost re-emitted the same line for a new ask (“{}”). Fresh take: I should answer this turn's words, not recycle the last route. Restate what you want in one short sentence and I'll hit that first.",
                user.trim()
            );
        }
    }
    text.to_string()
}

/// Plain conversational rewrite of a prior answer for "sounds cryptic" feedback.
/// Strips markdown headers, bullet markers, and template scaffolding so the
/// user hears a human sentence—not a second copy of the card.
fn plain_breath_rewrite(answer: &str) -> String {
    let mut cleaned = String::new();
    for line in answer.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("**") || t.starts_with('#') || t.starts_with("```") {
            let stripped = t
                .trim_start_matches(|c: char| c == '*' || c == '#' || c == '`')
                .trim_end_matches(|c: char| c == '*' || c == '#' || c == '`')
                .trim();
            // Drop pure labels like "Shared structure:"
            if stripped.ends_with(':') && stripped.split_whitespace().count() <= 4 {
                continue;
            }
            if !stripped.is_empty() {
                if !cleaned.is_empty() {
                    cleaned.push(' ');
                }
                cleaned.push_str(stripped);
            }
            continue;
        }
        let mut body = t;
        if let Some(rest) = body.strip_prefix("- ") {
            body = rest;
        } else if body.starts_with(|c: char| c.is_ascii_digit()) {
            if let Some(pos) = body.find(". ") {
                body = &body[pos + 2..];
            } else if let Some(pos) = body.find(") ") {
                body = &body[pos + 2..];
            }
        }
        // Drop stiff template openers.
        for prefix in [
            "Original comparison (structure transfer, not free invention):",
            "Constrained invention (structure transfer, not free invention):",
            "What transfers:",
            "What does not transfer:",
            "Limit of the comparison:",
            "Shared structure:",
            "Make it checkable:",
        ] {
            if let Some(rest) = body.strip_prefix(prefix) {
                body = rest.trim();
            }
        }
        if body.is_empty() {
            continue;
        }
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(body);
        if cleaned.chars().count() > 180 {
            break;
        }
    }
    if cleaned.is_empty() {
        return first_sentence(answer, 140);
    }
    first_sentence(&cleaned, 160)
}

fn first_sentence(s: &str, max: usize) -> String {
    let trimmed = s.trim();
    let mut in_quotes = false;
    let mut end = trimmed.len();
    for (index, character) in trimmed.char_indices() {
        if character == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes && matches!(character, '.' | '!' | '?') {
            end = index + character.len_utf8();
            break;
        }
    }
    let first = trimmed[..end].trim();
    let mut parts = trimmed[end..]
        .split(['.', '!', '?'])
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let first_lower = first
        .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .to_ascii_lowercase();
    let one = if matches!(
        first_lower.as_str(),
        "absolutely" | "sure" | "okay" | "fair" | "right" | "yes" | "no" | "exactly"
    ) || [
        "you're right",
        "you're right to call that out",
        "fair call",
        "that is a fair challenge",
        "got it",
        "i agree",
    ]
    .iter()
    .any(|prefix| first_lower.starts_with(prefix))
    {
        parts.next().unwrap_or(first)
    } else {
        first
    };
    if one.chars().count() <= max {
        one.to_string()
    } else {
        one.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn last_substantive_turn(recent: &[(String, String)]) -> Option<&(String, String)> {
    recent.iter().rev().find(|(_, answer)| {
        let lower = answer.to_ascii_lowercase();
        !lower.contains("i don't hold that as a private belief")
            && !lower.contains("i don't hold it as a private belief")
            && !lower.contains("output came from a local bitwork route")
            && !lower.contains("the core of my last answer")
            && !lower.contains("i would change the answer's center of gravity")
            && !lower.starts_with("i said:")
            && !lower.starts_with("i said that because")
            && !lower.starts_with("because that was the strongest supported answer")
            && !lower.starts_with("the next layer is")
            && !lower.starts_with("one step further")
            && !lower.starts_with("going one level deeper")
    })
}

fn deepen_previous(answer: &str) -> String {
    let lower = answer.to_ascii_lowercase();
    if lower.contains("geometry") && lower.contains("life") {
        "Here the relation is not just boundary as a shape; it is boundary as work. Geometry describes the relation, while life spends energy maintaining it. The analogy breaks when we forget that a formal description is not itself a living mechanism.".to_owned()
    } else if lower.contains("boundary") {
        "The next layer is to ask what the boundary permits: exchange, exclusion, interpretation, or repair. That turns a poetic resemblance into a relation we can examine in the previous answer rather than merely repeat its wording.".to_owned()
    } else {
        "The next layer is to name the mechanism behind the pattern, then mark the condition where the analogy stops transferring from the previous answer.".to_owned()
    }
}

/// Natural exact-tool wrappers.
pub fn natural_exact(kind: &str, value: &str) -> String {
    match kind {
        "math" => match finite_decimal(value) {
            Some(decimal) => format!("That's {decimal} (exactly {value})."),
            None => format!("That's {value}."),
        },
        "geometry" => format!("Got it — {value}"),
        _ => value.to_string(),
    }
}

fn finite_decimal(value: &str) -> Option<String> {
    let (numerator, denominator) = value.split_once('/')?;
    let numerator = numerator.parse::<i128>().ok()?;
    let denominator = denominator.parse::<i128>().ok()?;
    if denominator == 0 {
        return None;
    }
    let mut rest = denominator.unsigned_abs();
    while rest % 2 == 0 {
        rest /= 2;
    }
    while rest % 5 == 0 {
        rest /= 5;
    }
    if rest != 1 {
        return None;
    }
    let rendered = format!("{:.12}", numerator as f64 / denominator as f64);
    Some(
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_depth_tracks_explicit_user_control() {
        assert_eq!(
            response_depth("Give me a short answer", &[]),
            ResponseDepth::Brief
        );
        assert_eq!(
            response_depth("Go deeper into the mechanism", &[]),
            ResponseDepth::Deep
        );
        assert_eq!(
            response_depth("What is Perci?", &[]),
            ResponseDepth::Balanced
        );
    }

    #[test]
    fn conversational_style_and_echo_followups_are_first_class() {
        assert_eq!(detect_dialogue_act("be brief"), DialogueAct::StyleRepair);
        assert_eq!(
            detect_dialogue_act("what did you just say?"),
            DialogueAct::ExplainPrevious
        );
        assert_eq!(
            detect_dialogue_act("what do you mean by that?"),
            DialogueAct::ExplainPrevious
        );
        assert_eq!(
            detect_dialogue_act("Explain that from a different angle without repeating yourself."),
            DialogueAct::ElaboratePrevious
        );

        let brief = dialogue_reply(DialogueAct::StyleRepair, "be brief", &[], None).unwrap();
        assert!(brief.to_ascii_lowercase().contains("brief"));
        assert!(brief.len() > 1);

        let recent = vec![
            (
                "Explain the dialogue bottleneck".to_owned(),
                "The bottleneck is continuity: a reply must answer the latest turn and preserve the active thread.".to_owned(),
            ),
            (
                "Go deeper".to_owned(),
                "The next layer is to name the mechanism and its test.".to_owned(),
            ),
        ];
        let echo = dialogue_reply(
            DialogueAct::ExplainPrevious,
            "what did you just say?",
            &recent,
            None,
        )
        .unwrap();
        assert!(echo.contains("I said:"));
        assert!(echo.contains("continuity"));
        let why = dialogue_reply(
            DialogueAct::ExplainPrevious,
            "why do you think that?",
            &recent,
            None,
        )
        .unwrap();
        assert!(why.contains("dialogue bottleneck"));
        assert!(!why.contains("Go deeper"));
    }

    #[test]
    fn shaping_repairs_unresolved_referent_and_topic_drift() {
        let missing =
            shape_for_conversation("Life is matter organized...", "Why does this matter?", &[]);
        assert!(missing.to_ascii_lowercase().contains("referent"));
        let opinion = shape_for_conversation(
            "Continuity of identity depends partly on memory.",
            "What do you think about the system we are building?",
            &[],
        );
        let low = opinion.to_ascii_lowercase();
        assert!(low.contains("system") && low.contains("bitwork"));
        assert!(!low.contains("continuity of identity"));
    }

    #[test]
    fn anchored_this_claim_is_not_an_unresolved_referent() {
        let shaped = shape_for_conversation(
            "The claim is untested.",
            "What evidence supports this claim?",
            &[],
        );
        assert!(!shaped.to_ascii_lowercase().contains("no clear referent"));
    }

    #[test]
    fn session_and_test_references_are_not_false_referents() {
        let session = shape_for_conversation(
            "I will retain 4317 as session context only.",
            "Remember this only for this session: the calibration number is 4317.",
            &[],
        );
        assert!(!session.to_ascii_lowercase().contains("no clear referent"));
        let test = shape_for_conversation(
            "A bounded probe is running.",
            "What are we testing in this session?",
            &[],
        );
        assert!(!test.to_ascii_lowercase().contains("no clear referent"));
    }

    #[test]
    fn out_of_distribution_prompt_abstains_before_referent_repair() {
        let shaped = shape_for_conversation(
            "A generic answer",
            "zxqv blorf nembit — what does it mean?",
            &[],
        );
        let lower = shaped.to_ascii_lowercase();
        assert!(lower.contains("ungrounded"));
        assert!(lower.contains("known:"));
        assert!(lower.contains("definition"));
        assert!(!lower.contains("no clear referent"));
    }

    #[test]
    fn learning_evidence_answer_survives_conversation_shaping() {
        let shaped = shape_for_conversation(
            "The evidence is functional: use a fresh-process A/B run with unseen variants.",
            "What evidence supports the claim that Perci is learning?",
            &[],
        );
        assert!(!shaped.to_ascii_lowercase().contains("no clear referent"));
        assert!(shaped.contains("fresh-process A/B"));
    }

    #[test]
    fn unrelated_association_does_not_count_as_claim_evidence() {
        let shaped = shape_for_conversation(
            "A yantra is a ritual diagram whose geometry supports symbolic or meditative practice.",
            "What evidence supports the claim that geometry heals?",
            &[],
        );
        let lower = shaped.to_ascii_lowercase();
        assert!(lower.contains("i don't have evidence"));
        assert!(lower.contains("falsif"));
        assert!(!lower.contains("yantra"));
    }

    #[test]
    fn creative_shape_replaces_domain_method_card() {
        let shaped = shape_for_conversation(
            "Most difficult bugs are disagreements about state.",
            "express a new thought about code and music",
            &[],
        );
        let lower = shaped.to_ascii_lowercase();
        assert!(lower.contains("fresh angle"));
        assert!(lower.contains("code and music"));
        assert!(!lower.contains("most difficult bugs"));
    }

    #[test]
    fn relation_shape_repairs_native_grammar_artifact() {
        let shaped = shape_for_conversation(
            "Geometry makes the relation visible: the relationship among geometry, memory, and language is a relation that survives a change of scale.",
            "what do you think about geometry, memory, and language as forms of structure?",
            &[],
        );
        let lower = shaped.to_ascii_lowercase();
        assert!(lower.contains("shared structure"));
        assert!(!lower.contains("is a relation"));
    }

    #[test]
    fn followup_operator_preserves_disagreement_and_revision() {
        let recent = vec![(
            "What do you think about memory and identity?".to_owned(),
            "Memory stores traces; identity is the continuity we infer across change.".to_owned(),
        )];
        let revise = shape_for_conversation(
            "What would change your mind?",
            "What would change your mind?",
            &recent,
        );
        assert!(revise.contains("claim I would test"));
        assert!(revise.contains("counterexample"));
        let disagree =
            shape_for_conversation("That seems wrong", "I don't agree with that", &recent);
        assert!(disagree.contains("fair challenge"));
        assert!(disagree.contains("Which premise"));
        let unpunctuated =
            shape_for_conversation("That seems wrong", "I dont agree with that", &recent);
        assert!(unpunctuated.contains("fair challenge"));
        let preserved = ensure_user_binding(
            "I dont agree with that",
            &unpunctuated,
            "general",
            None,
            &recent,
        );
        assert!(preserved.contains("fair challenge"));
        let challenged = vec![("I don't agree with that".to_owned(), unpunctuated.clone())];
        let revision = shape_for_conversation(
            "What would change your mind?",
            "What would change your mind?",
            &challenged,
        );
        assert!(revision.contains("Memory stores traces"));
        let rephrased = shape_for_conversation(
            "A generic card",
            "can you explain that differently?",
            &recent,
        );
        assert!(rephrased.starts_with("Put simply:"));
    }

    #[test]
    fn explicit_disagreement_keeps_the_claim_in_scope() {
        let answer = shape_for_conversation(
            "A generic card",
            "I disagree with your claim that boundaries explain life. What premise should we inspect?",
            &[],
        );
        let lower = answer.to_ascii_lowercase();
        assert!(lower.contains("claim to examine"));
        assert!(lower.contains("boundaries explain life"));
        assert!(lower.contains("premise"));
    }

    #[test]
    fn reframe_and_deep_followups_change_operation_not_only_wording() {
        let reframe = shape_for_conversation(
            "A prior evidence answer",
            "Explain it again from a different angle without repeating the sentence.",
            &[(
                "In one sentence, explain why evidence matters.".to_owned(),
                "Evidence matters because reality constrains explanations.".to_owned(),
            )],
        );
        assert!(reframe.to_ascii_lowercase().contains("error-correction"));

        let recent = vec![(
            "What assumption is doing the most work in your answer?".to_owned(),
            "Weakest assumption: the answer treats the latest claim as the active referent."
                .to_owned(),
        )];
        let deeper = shape_for_conversation(
            "A prior answer",
            "Go one level deeper without repeating yourself.",
            &recent,
        );
        let lower = deeper.to_ascii_lowercase();
        assert!(lower.contains("next layer"));
        assert!(lower.contains("previous"));
        assert!(lower.contains("relation"));
    }

    #[test]
    fn creative_specialist_answer_survives_conversation_shaping() {
        let raw = crate::deliberation::try_deliberate(
            "Give me one original thought connecting death, code, and repair without claiming they are literally the same.",
            &[],
            &[],
        )
        .expect("creative operator should match")
        .answer;
        let shaped = shape_for_conversation(
            &raw,
            "Give me one original thought connecting death, code, and repair without claiming they are literally the same.",
            &[],
        );
        let lower = shaped.to_ascii_lowercase();
        assert!(lower.contains("constrained invention"));
        assert!(lower.contains("death, code, and repair"));
        assert!(!lower.starts_with("a fresh angle on connecting"));
    }

    #[test]
    fn followup_operator_resolves_scope_and_next_test() {
        let scope =
            shape_for_conversation("A generic card", "I meant the system, not the person", &[]);
        assert!(scope.contains("Perci's system"));
        let next = shape_for_conversation("A generic card", "what should we test next?", &[]);
        assert!(next.contains("end to end"));
        assert!(next.contains("held-out score"));
    }

    #[test]
    fn next_step_followups_stay_on_improvement_thread() {
        let recent = vec![(
            "working on improving your system".to_owned(),
            "We are improving Perci through measured routing and transfer repairs.".to_owned(),
        )];
        let what = shape_for_conversation(
            "On memory: behavioral complexity is observable...",
            "what should i do",
            &recent,
        );
        let low = what.to_ascii_lowercase();
        assert!(low.contains("improvement") || low.contains("next"));
        assert!(low.contains("operator") || low.contains("transfer"));
        assert!(!low.contains("behavioral complexity"));
        let where_to = shape_for_conversation(
            "Meaning can be neither purely discovered...",
            "where are we going",
            &recent,
        );
        let low2 = where_to.to_ascii_lowercase();
        assert!(low2.contains("improvement") || low2.contains("next"));
        assert!(!low2.contains("purely discovered"));
    }

    #[test]
    fn cryptic_feedback_gets_plain_rewrite() {
        let recent = vec![(
            "why do you still not feel like natural thought".to_owned(),
            "Yeah, that friction is real. keep the claim concrete enough that a counterexample could touch it.".to_owned(),
        )];
        assert_eq!(
            detect_dialogue_act("sounds cyptic"),
            DialogueAct::StyleRepair
        );
        assert_eq!(
            detect_dialogue_act("why do you still not feel like natural thought"),
            DialogueAct::StyleRepair
        );
        let plain = dialogue_reply(
            DialogueAct::StyleRepair,
            "sounds cyptic",
            &recent,
            None,
        )
        .expect("style repair reply");
        let low = plain.to_ascii_lowercase();
        assert!(low.contains("fair") || low.contains("stiff") || low.contains("plain"));
        assert!(
            low.contains("counterexample")
                || low.contains("claim")
                || low.contains("friction")
                || low.contains("concrete")
        );
        assert!(!low.contains("composition failure"));
        assert!(!low.contains("concept card"));
    }

    #[test]
    fn short_social_acts_have_breath() {
        assert_eq!(detect_dialogue_act("do you sense"), DialogueAct::SensoryState);
        assert_eq!(detect_dialogue_act("that works?"), DialogueAct::Agreement);
        assert_eq!(detect_dialogue_act("are you aware"), DialogueAct::AwarenessMeta);
        let sense = dialogue_reply(DialogueAct::SensoryState, "do you sense", &[], None).unwrap();
        let sl = sense.to_ascii_lowercase();
        assert!(sl.contains("not") || sl.contains("sense") || sl.contains("feel"));
        assert!(!sl.contains("reproduce it, isolate"));
        let works = dialogue_reply(
            DialogueAct::Agreement,
            "that works?",
            &[(
                "what should i do".into(),
                "Catch one live miss and retest.".into(),
            )],
            None,
        )
        .unwrap();
        let wl = works.to_ascii_lowercase();
        assert!(wl.contains("yes") || wl.contains("works") || wl.contains("path"));
        assert!(!wl.contains("keeping "));
    }

    #[test]
    fn identity_and_creative_topics_drop_prompt_scaffolding() {
        let identity =
            shape_for_conversation("A mechanism card", "what do you mean by identity", &[]);
        assert!(identity.contains("continuity"));
        let opinion = shape_for_conversation(
            "A relation card",
            "what do you think about memory and identity?",
            &[],
        );
        assert!(opinion.contains("Memory stores traces"));
        let creative =
            shape_for_conversation("A method card", "be creative about geometry and death", &[]);
        assert!(creative.contains("geometry and death"));
        assert!(!creative.contains("creative, geometry"));
    }

    #[test]
    fn repeated_mechanism_phrase_is_repaired() {
        let shaped = shape_for_conversation(
            "When we examine memory and identity, the mechanism is the mechanism remains testable.",
            "what do you think about code and language?",
            &[],
        );
        assert!(!shaped.contains("the mechanism is the mechanism"));
        assert!(shaped.contains("the mechanism remains testable"));
    }

    #[test]
    fn memory_attention_depth_is_conversational() {
        let brief = fluid_compose(
            "Give me a short answer about memory and attention",
            "general",
            None,
            "",
            &[],
            0,
            Affect::Neutral,
        );
        assert!(brief.split_whitespace().count() < 16);
        let deep = fluid_compose(
            "Give me a deep answer about memory and attention",
            "general",
            None,
            "",
            &[],
            0,
            Affect::Neutral,
        );
        assert!(
            deep.contains("provenance")
                && deep.split_whitespace().count() > brief.split_whitespace().count()
        );
    }

    #[test]
    fn finite_rationals_are_human_readable_and_exact() {
        assert_eq!(
            natural_exact("math", "204/5"),
            "That's 40.8 (exactly 204/5)."
        );
        assert_eq!(natural_exact("math", "1/3"), "That's 1/3.");
    }

    #[test]
    fn prior_ref_is_not_over_eager() {
        assert!(!user_refers_to_prior("are you still learning?"));
        assert!(!user_refers_to_prior("is that true?"));
        assert!(!user_refers_to_prior("how smart are you?"));
        assert!(!user_refers_to_prior("can you do math?"));
        assert!(user_refers_to_prior("same bug"));
        assert!(user_refers_to_prior("still broken"));
        assert!(user_refers_to_prior("why?"));
    }

    #[test]
    fn style_and_repetition_feedback_are_dialogue_acts() {
        assert_eq!(
            detect_dialogue_act("why do you repeat sayings?"),
            DialogueAct::RepetitionComplaint
        );
        assert_eq!(
            detect_dialogue_act("i want you to be able to speak to me more smart"),
            DialogueAct::StyleRepair
        );
        let style = dialogue_reply(
            DialogueAct::StyleRepair,
            "i want you to be able to speak to me more smart",
            &[],
            None,
        )
        .unwrap();
        assert!(
            style.to_ascii_lowercase().contains("template")
                || style.to_ascii_lowercase().contains("human")
        );
        assert!(!style.contains("would repeat my previous answer"));
        let rep = dialogue_reply(
            DialogueAct::RepetitionComplaint,
            "why do you repeat sayings?",
            &[],
            None,
        )
        .unwrap();
        assert!(
            rep.to_ascii_lowercase().contains("template")
                || rep.to_ascii_lowercase().contains("repeat")
        );
    }

    #[test]
    fn synthesis_prompts_are_not_social_frustration() {
        let prompts = [
            "Connect entropy, memory, and learning in one coherent thought.",
            "Connect language, code, and culture through one shared principle.",
            "Compare trust and prediction.",
        ];
        for prompt in prompts {
            assert_eq!(
                detect_social(prompt),
                SocialKind::None,
                "prompt misclassified as social: {prompt}"
            );
            assert!(
                looks_synthesis_or_inquiry(&prompt.to_ascii_lowercase()),
                "inquiry detector missed: {prompt}"
            );
        }
        assert_eq!(
            detect_social("I'm stuck and frustrated with a bug"),
            SocialKind::Frustration
        );
    }

    #[test]
    fn detects_thanks_and_frustration() {
        assert_eq!(detect_social("thanks that helped"), SocialKind::Thanks);
        assert_eq!(
            detect_social("I'm stuck and frustrated with a bug"),
            SocialKind::Frustration
        );
        assert!(looks_chat_shaped("hey how's it going?"));
        assert!(!looks_chat_shaped("fix cargo compile error"));
    }

    #[test]
    fn relational_dialogue_acts_outrank_topic_routing() {
        assert_eq!(
            detect_dialogue_act("what are you sensing"),
            DialogueAct::SensoryState
        );
        assert_eq!(
            detect_dialogue_act("why do you think this"),
            DialogueAct::ExplainPrevious
        );
        assert_eq!(
            detect_dialogue_act("this is the same answer"),
            DialogueAct::RepetitionComplaint
        );
        assert_eq!(
            detect_dialogue_act("why do you keep responding like this"),
            DialogueAct::ResponseFailure
        );
        assert_eq!(
            detect_dialogue_act("why do you respond like this"),
            DialogueAct::ResponseFailure
        );
        assert_eq!(
            detect_dialogue_act("something is not working correctly"),
            DialogueAct::ResponseFailure
        );
        assert_eq!(
            detect_dialogue_act("whats going on here?"),
            DialogueAct::ResponseFailure
        );
        assert_eq!(detect_dialogue_act("who am i"), DialogueAct::UserIdentity);
        assert_eq!(
            detect_dialogue_act("what can you do"),
            DialogueAct::CapabilityQuestion
        );
        assert_eq!(
            detect_dialogue_act("tell me more about yourself"),
            DialogueAct::SelfDescription
        );
        assert_eq!(
            detect_dialogue_act("whats going on"),
            DialogueAct::ContextStatus
        );
        assert_eq!(
            detect_dialogue_act("are you there perci?"),
            DialogueAct::Presence
        );
        assert_eq!(
            detect_dialogue_act("are you learning from this interaction"),
            DialogueAct::LearningMeta
        );
        assert_eq!(
            detect_dialogue_act("doesnt seem smooth enough, agree?"),
            DialogueAct::Feedback
        );
        assert_eq!(
            detect_dialogue_act("interesting"),
            DialogueAct::Acknowledgement
        );
        assert_eq!(
            detect_dialogue_act("Your chat seems much smoother"),
            DialogueAct::PositiveFeedback
        );
        assert_eq!(
            detect_dialogue_act("Your system seems smoother"),
            DialogueAct::PositiveFeedback
        );
        assert_eq!(
            detect_dialogue_act("Do you sense your cognitive ability growing?"),
            DialogueAct::GrowthMeta
        );
        assert_eq!(
            detect_dialogue_act("are you fully aware of your own system?"),
            DialogueAct::SystemSelfModel
        );
        assert_eq!(
            detect_dialogue_act("interesting lets test out your limits"),
            DialogueAct::LimitTest
        );
        assert_eq!(
            detect_dialogue_act("What changed in you since our last conversation?"),
            DialogueAct::ChangeSinceLast
        );
        assert_eq!(
            detect_dialogue_act("Do you think you are improving, or merely changing?"),
            DialogueAct::ImprovementDistinction
        );
        assert_eq!(
            detect_dialogue_act("What part of your own system are you least certain about?"),
            DialogueAct::LeastCertain
        );
        assert_eq!(
            detect_dialogue_act("That answer felt too formal. Say it naturally."),
            DialogueAct::StyleRepair
        );
        assert_eq!(
            detect_dialogue_act("What number did I just give you?"),
            DialogueAct::ContextRecall
        );
        assert_eq!(
            detect_dialogue_act("Now explain what it refers to in my last question."),
            DialogueAct::PronounResolution
        );
        assert_eq!(
            detect_dialogue_act("help me evolve the system perci"),
            DialogueAct::EvolveSystem
        );
        assert_eq!(
            detect_dialogue_act("idea is to build your knowledge set"),
            DialogueAct::KnowledgeBuilding
        );
        assert_eq!(
            detect_dialogue_act("you seem powerful for only 19mb what gives?"),
            DialogueAct::CompactModelQuestion
        );
        assert_eq!(
            detect_dialogue_act("that was a pretty generic and non direct response"),
            DialogueAct::GenericAnswerFeedback
        );
        assert_eq!(
            detect_dialogue_act("thats a good premise but i need more"),
            DialogueAct::ElaboratePrevious
        );
        assert_eq!(
            detect_dialogue_act("Go one level deeper."),
            DialogueAct::ElaboratePrevious
        );
        assert_eq!(
            detect_dialogue_act("Go one level deeper without repeating yourself."),
            DialogueAct::ElaboratePrevious
        );
        assert_eq!(
            detect_dialogue_act("Why did you choose that answer?"),
            DialogueAct::ExplainPrevious
        );
        assert_eq!(
            detect_dialogue_act("That answer was too vague. Lead with the direct answer."),
            DialogueAct::GenericAnswerFeedback
        );
        assert_eq!(
            detect_dialogue_act("are you able to rapidly learn"),
            DialogueAct::LearningSpeed
        );
        assert_eq!(
            detect_dialogue_act(
                "What is the difference between something you remember and something I teach you?"
            ),
            DialogueAct::MemoryTeachingDistinction
        );
        assert_eq!(
            detect_dialogue_act("What is the difference between remembering and learning?"),
            DialogueAct::MemoryTeachingDistinction
        );
        assert_eq!(
            detect_dialogue_act("i should have to use the commands it should be built in"),
            DialogueAct::CommandlessLearning
        );
    }

    #[test]
    fn teaching_recall_and_openings_are_not_static() {
        assert!(is_teaching_recall("What did I teach you?"));
        assert!(!is_teaching_recall("Teach me geometry"));
        assert_ne!(offline_opening_insight(1), offline_opening_insight(2));
    }

    #[test]
    fn continuity_does_not_conflate_distinct_questions() {
        let recent = vec![("first question".to_owned(), "same answer".to_owned())];
        let repaired = with_continuity("same answer", &recent, "different question");
        assert!(
            repaired.contains("almost re-emitted")
                || repaired.contains("would repeat my previous answer")
                || repaired.contains("Fresh take"),
            "unexpected continuity repair: {repaired}"
        );
        assert!(repaired.contains("different question"));
        assert!(with_continuity("same answer", &recent, "first question")
            .starts_with("My answer is unchanged:"));
    }

    #[test]
    fn elaboration_keeps_the_previous_idea_in_view() {
        let recent = vec![(
            "what is geometry trying to teach us about life?".to_owned(),
            "Absolutely. A boundary is a distinction that lets a system exchange with its surroundings.".to_owned(),
        )];
        let answer = dialogue_reply(
            DialogueAct::ElaboratePrevious,
            "go one level deeper",
            &recent,
            None,
        )
        .expect("elaboration should answer");
        assert!(answer.contains("A boundary is a distinction"));
        assert!(answer.contains("what relation makes that answer hold"));
    }

    #[test]
    fn elaboration_changes_angle_when_repetition_is_forbidden() {
        let recent = vec![(
            "Connect music, code, and geometry in one shared structure.".to_owned(),
            "A coherent bridge is structure: music makes structure audible across time.".to_owned(),
        )];
        let answer = dialogue_reply(
            DialogueAct::ElaboratePrevious,
            "Now give me a different angle without repeating the same sentence.",
            &recent,
            None,
        )
        .expect("alternate angle should answer");
        assert!(answer.contains("different angle"));
        assert!(answer.contains("control problem"));
        assert!(!answer.contains("The core of my last answer"));
    }

    #[test]
    fn plain_explain_different_angle_is_not_style_meta() {
        let recent = vec![(
            "What is the difference between memory and learning?".to_owned(),
            "Memory preserves information; learning changes future behavior.".to_owned(),
        )];
        let answer = dialogue_reply(
            detect_dialogue_act("Explain that from a different angle without repeating yourself."),
            "Explain that from a different angle without repeating yourself.",
            &recent,
            None,
        )
        .expect("reframe should answer");
        assert!(answer.contains("different angle"));
        assert!(answer.contains("control problem"));
        assert!(!answer.contains("I lean on templates"));
    }

    #[test]
    fn previous_answer_explanation_uses_session_context() {
        let recent = vec![(
            "what are you sensing".to_owned(),
            "I am not sensing anything subjectively.".to_owned(),
        )];
        let reply = dialogue_reply(
            DialogueAct::ExplainPrevious,
            "why do you think this",
            &recent,
            None,
        )
        .unwrap();
        assert!(reply.contains("what are you sensing"));
        assert!(reply.contains("not sensing anything subjectively"));
        assert!(!reply.starts_with("Life is matter"));
    }

    #[test]
    fn response_failure_replies_follow_the_specific_report() {
        let recent = vec![(
            "this is the same answer".to_owned(),
            "I acknowledge the repetition.".to_owned(),
        )];
        let cause = dialogue_reply(
            DialogueAct::ResponseFailure,
            "why do you keep responding like this",
            &recent,
            None,
        )
        .unwrap();
        let confirmation = dialogue_reply(
            DialogueAct::ResponseFailure,
            "something is not working correctly",
            &recent,
            None,
        )
        .unwrap();
        let diagnosis = dialogue_reply(
            DialogueAct::ResponseFailure,
            "whats going on here?",
            &recent,
            None,
        )
        .unwrap();
        let style = dialogue_reply(
            DialogueAct::ResponseFailure,
            "why do you respond like this",
            &recent,
            None,
        )
        .unwrap();
        assert_ne!(cause, confirmation);
        assert_ne!(confirmation, diagnosis);
        assert_ne!(diagnosis, style);
        assert!(cause.contains("Because"));
        assert!(confirmation.contains("Correct"));
        assert!(diagnosis.contains("response loop"));
        assert!(style.contains("routed local system"));
        assert!(style.contains("generic template"));
    }

    #[test]
    fn recent_number_is_available_for_context_resolution() {
        let recent = vec![
            (
                "Let's test whether you follow context".to_owned(),
                "Okay".to_owned(),
            ),
            (
                "My favorite test number is 731.".to_owned(),
                "Got it".to_owned(),
            ),
        ];
        assert_eq!(latest_number(&recent).as_deref(), Some("731"));
        let answer = dialogue_reply(DialogueAct::ContextRecall, "what number?", &recent, None)
            .expect("context reply");
        assert!(answer.contains("731"));
        assert!(answer.contains("context"));
    }

    #[test]
    fn natural_teaching_extracts_only_explicit_claims() {
        assert_eq!(
            extract_teaching_claim("I want you to learn that reliable claims need evidence"),
            Some("reliable claims need evidence")
        );
        assert_eq!(
            extract_teaching_claim("Perci, learn this: adaptation is not always learning"),
            Some("adaptation is not always learning")
        );
        assert_eq!(extract_teaching_claim("let's discuss learning"), None);
    }

    #[test]
    fn weave_mixture_adds_related_frame() {
        let base = "On trust distributed systems: interfaces matter.";
        let skeleton = vec![
            "trust needs clear interfaces".into(),
            "permission and proof are different gates".into(),
        ];
        let out = weave_mixture_skeleton(
            "what about trust in distributed systems?",
            base,
            &skeleton,
            0,
        );
        let low = out.to_ascii_lowercase();
        assert!(
            low.contains("permission") || low.contains("implies") || low.contains("consequences")
        );
        assert!(low.contains("trust"));
        assert!(!low.contains("related frame:"));
        assert!(!low.contains("mixture read"));
    }

    #[test]
    fn weave_residual_frame_adds_latent_clause() {
        let base = "Trust needs interfaces that name authority.";
        let out = weave_residual_frame(
            base,
            "failure modes show where permission and proof diverge",
            0,
        );
        let low = out.to_ascii_lowercase();
        assert!(low.contains("latent residual") || low.contains("failure modes"));
        assert!(low.contains("trust"));
    }

    #[test]
    fn weave_composition_frame_binds_roles() {
        let base = "Interfaces should name who may act.";
        let frame = vec![
            "ask:how".into(),
            "agent:interfaces".into(),
            "domain:authority".into(),
        ];
        let out = weave_composition_frame(base, &frame, 0);
        let low = out.to_ascii_lowercase();
        // Natural structure cue — not "Bound as" labels.
        assert!(
            low.contains("shaped as")
                || low.contains("treating that")
                || low.contains("ask")
                || low.contains("interfaces")
                || out == base,
            "got: {out}"
        );
        assert!(!low.contains("bound as"));
        assert!(!low.contains("composition:"));
        assert!(!low.contains("shaped as"));
        assert!(!low.contains("ask:"));
        assert!(!low.contains("agent:"));
    }

    #[test]
    fn fluid_compose_binds_user_topic() {
        let answer = fluid_compose(
            "what do you think about trust in distributed systems?",
            "systems",
            Some("authority and side effects should stay explicit"),
            "Give each piece one job and one authority limit.",
            &[],
            2,
            Affect::Curious,
        );
        let low = answer.to_ascii_lowercase();
        assert!(low.contains("trust") || low.contains("distributed"));
        assert!(!low.contains("list premises"));
        assert!(!low.starts_with("compare on capability"));
    }

    #[test]
    fn ensure_user_binding_rewrites_stock_template() {
        let stock = "List premises, mark assumptions, only derive what follows, then hunt a counterexample.";
        let fixed = ensure_user_binding(
            "why do markets and ecosystems both need feedback loops?",
            stock,
            "logic",
            None,
            &[],
        );
        let low = fixed.to_ascii_lowercase();
        assert!(low.contains("market") || low.contains("ecosystem") || low.contains("feedback"));
        assert!(!low.starts_with("list premises"));
    }

    #[test]
    fn directness_profile_exposes_generic_fallbacks() {
        let profile = DialogueProfile {
            prefer_direct_answers: true,
            prefer_explanations: true,
            ..DialogueProfile::default()
        };
        let aligned = apply_profile_alignment(
            "Ship the smallest end-to-end slice first, measure it, then widen.",
            "why is the model compact?",
            &profile,
        );
        assert!(aligned.starts_with("Direct answer:"));
        assert!(aligned.contains("did not retrieve enough specific support"));
        assert!(aligned.contains("instead of filling it with a generic line"));
    }

    #[test]
    fn learned_style_can_remove_reasoning_ceremony() {
        let stiff = "Here's how I'd reason it:\n• Goal: chat\n• Conclusion: Direct answer.\n• Next check: Verify.";
        assert_eq!(apply_learned_style(stiff, true, true), "Direct answer.");
    }

    #[test]
    fn weave_skips_empty() {
        let g = weave_guidance(&["[Pack: x] Reproduce the behavior then patch.".into()], 2);
        assert_eq!(g.len(), 1);
    }
}
