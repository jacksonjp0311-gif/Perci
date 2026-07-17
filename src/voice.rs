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
    if matches!(
        compact,
        "what are you sensing" | "what do you sense" | "what can you sense"
    ) {
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
    ) {
        DialogueAct::ExplainPrevious
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
        || (text.contains("repeat") && (text.contains("phrase") || text.contains("saying") || text.contains("template"))))
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
    } else if text.contains("speak more smart")
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
        || text.contains("do you sense") && text.contains("growing")
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
    {
        DialogueAct::Agreement
    } else if matches!(
        compact,
        "interesting" | "wow" | "whoa" | "hmm" | "makes sense" | "i see" | "got it"
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
            "I am not sensing anything subjectively. I receive your text, measure internal routing signals, and read approved runtime state. The opening insight is a rotated concept from my weights—not a feeling, perception, or spontaneous inner experience.".to_owned()
        }
        DialogueAct::ExplainPrevious => {
            if let Some((previous_user, previous_answer)) = recent.last() {
                let lower = previous_answer.to_ascii_lowercase();
                if lower.contains("geometry")
                    && lower.contains("life")
                    && lower.contains("boundary")
                {
                    format!(
                        "I don't hold it as a private belief. I chose that answer to \"{previous_user}\" because geometry gives us explicit relations and life gives us active maintenance; boundary is the shared structural axis. The analogy is useful because it preserves that relation, but it stops before claiming that a shape is alive or that geometry causes life."
                    )
                } else {
                    format!(
                        "I don't hold that as a private belief. In response to \"{previous_user}\", my last output was \"{}\". That output came from a local Bitwork route plus the response layer. I can justify the claim itself only with a mechanism, evidence, or test; otherwise I should label it as an association, not knowledge.",
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
            "I have operational awareness, not subjective consciousness: I can represent parts of my own architecture, current session, capabilities, limits, and measured state. I do not have evidence of an inner experience or human-like sentience.".to_owned()
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
            let previous = recent.last().map(|turn| turn.1.to_ascii_lowercase()).unwrap_or_default();
            if lower.contains("smart")
                || lower.contains("intelligent")
                || lower.contains("natural")
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
                let core = first_sentence(previous_answer, 180);
                let deeper = deepen_previous(previous_answer);
                format!(
                    "The core of my last answer to \"{}\" was: {} Going one level deeper, the useful question is what relation makes that answer hold, where the relation breaks, and what observation would distinguish it from a nearby explanation.",
                    previous_user.trim(),
                    core
                ) + &format!(" {deeper}")
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
            "Yes. Based on this exchange, that criticism is fair—the content path worked, but the conversational response did not fit the moment.".to_owned()
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
    haystack.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == needle)
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
        let mut out = crate::bridge::compose_soft_cascade(
            user,
            matched,
            domain_body,
            variant,
        );
        out = ensure_user_binding(user, &out, matched.label.as_str(), matched.insight.as_deref(), recent);
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
    if let Some(lat) = residual.first() {
        out = weave_residual_frame(&out, lat, variant);
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
fn should_voice_composition(user: &str, frame: &[String]) -> bool {
    if frame.len() < 2 {
        return false;
    }
    let words = user.split_whitespace().count();
    if words < 5 {
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
    let good_fillers = frame
        .iter()
        .filter_map(|f| f.split_once(':').map(|(_, v)| v))
        .filter(|v| v.len() >= 4 && !v.chars().all(|c| c.is_ascii_digit()))
        .count();
    good_fillers >= 2
}

/// Weave VSA role–filler composition into speech (compact, not a checklist dump).
pub fn weave_composition_frame(answer: &str, frame: &[String], variant: usize) -> String {
    if frame.len() < 2 {
        return answer.to_owned();
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
    if picks.iter().filter(|p| al.contains(&p.to_ascii_lowercase())).count() >= 2 {
        return answer.to_owned();
    }
    let mut out = answer.trim_end().to_owned();
    if !out.ends_with('.') && !out.ends_with('?') && !out.ends_with('!') {
        out.push('.');
    }
    out.push(' ');
    match variant % 3 {
        0 => {
            out.push_str("Bound as ");
            out.push_str(&joined);
            out.push('.');
        }
        1 => {
            out.push_str("Composition: ");
            out.push_str(&joined);
            out.push('.');
        }
        _ => {
            out.push_str("Roles in play — ");
            out.push_str(&joined);
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
    match variant % 3 {
        0 => {
            out.push_str("Latent residual: ");
            out.push_str(residual.trim());
        }
        1 => {
            out.push_str("Also latent after the first frame: ");
            out.push_str(residual.trim());
        }
        _ => {
            out.push_str("Second hop still in view: ");
            out.push_str(residual.trim());
        }
    }
    if !out.ends_with('.') && !out.ends_with('?') {
        out.push('.');
    }
    out
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
    match (extras.len(), variant % 3) {
        (1, 0) => {
            out.push_str("Related frame: ");
            out.push_str(extras[0]);
        }
        (1, _) => {
            out.push_str("Also in view: ");
            out.push_str(extras[0]);
        }
        (_, 0) => {
            out.push_str("Related frames: ");
            out.push_str(extras[0]);
            out.push_str("; ");
            out.push_str(extras[1]);
        }
        (_, 1) => {
            out.push_str("Two nearby ideas fire with that: ");
            out.push_str(extras[0]);
            out.push_str(" — and ");
            out.push_str(extras[1]);
        }
        _ => {
            out.push_str("Mixture read: ");
            out.push_str(extras[0]);
            out.push_str("; ");
            out.push_str(extras[1]);
        }
    }
    if !out.ends_with('.') && !out.ends_with('?') {
        out.push('.');
    }
    // Keep user topic in play if mixture diluted it.
    let tokens = content_tokens(user);
    let ol = out.to_ascii_lowercase();
    if tokens.len() >= 2 && tokens.iter().filter(|t| ol.contains(t.as_str())).count() == 0 {
        out.push(' ');
        out.push_str(&format!(
            "That still centers on {}.",
            tokens.iter().take(3).cloned().collect::<Vec<_>>().join(" ")
        ));
    }
    out
}

/// Content words from the user worth binding into the reply.
fn content_tokens(user: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "if", "then", "than", "that", "this",
        "these", "those", "with", "from", "into", "onto", "about", "what", "when",
        "where", "which", "who", "whom", "why", "how", "can", "could", "would",
        "should", "will", "just", "really", "very", "your", "you", "me", "my",
        "our", "we", "i", "im", "i'm", "is", "are", "was", "were", "be", "been",
        "being", "do", "does", "did", "to", "of", "in", "on", "for", "it", "its",
        "as", "at", "by", "not", "no", "yes", "please", "tell", "give", "make",
        "more", "some", "any", "all", "also", "like", "something", "someone",
        "think", "thoughts", "thought", "interesting", "only", "thing", "things",
        "know", "want", "need", "help", "say", "said", "get", "got", "let",
        "have", "has", "had", "been", "being", "into", "over", "under", "again",
        "still", "even", "much", "many", "such", "other", "another", "same",
    ];
    user.split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
        .map(|w| w.trim_matches('\'').to_ascii_lowercase())
        .filter(|w| w.len() >= 4 && !STOP.contains(&w.as_str()))
        .take(6)
        .collect()
}

fn token_seed(user: &str) -> usize {
    user.bytes().fold(0usize, |acc, b| acc.wrapping_mul(33).wrapping_add(b as usize))
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
    let lower = user.to_ascii_lowercase();
    let tokens = content_tokens(user);
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
        } else if tokens.iter().any(|t| {
            t.len() >= 5 && prev_a.to_ascii_lowercase().contains(t.as_str())
        }) {
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

    // Prefer a real concept insight when it is short and not a stock method card.
    let concept = insight
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

    // Open conversation: answer the ask with user topic bound in.
    if looks_open_conversation(&lower) || tokens.len() >= 2 {
        let angle = concept.unwrap_or_else(|| {
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
                        "stay with the topic you named and make one claim that could be checked".into()
                    }
                    _ => {
                        "stay with the topic you named and make one claim that could be checked"
                            .into()
                    }
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

        let body = if lower.starts_with("why ") || lower.contains("why does") || lower.contains("why is") {
            match seed % 3 {
                0 => format!(
                    "Because for {topic}, the useful story is mechanism, not slogan: {angle}. If that mechanism changed, the outcome should change too — that's the test."
                ),
                1 => format!(
                    "The short answer on {topic}: {angle}. The longer one is that explanations earn trust when they predict what would happen under a controlled change."
                ),
                _ => format!(
                    "Why {topic}? Start from what must be true for the claim to hold: {angle}. Then ask what observation would force a rewrite."
                ),
            }
        } else if lower.starts_with("how ") {
            match seed % 3 {
                0 => format!(
                    "How I'd approach {topic}: (1) state the goal in one line, (2) name the constraint that bites first, (3) take the smallest reversible step, (4) verify. Under the hood: {angle}."
                ),
                1 => format!(
                    "For {topic}, do the thin slice first — one end-to-end path you can check — then widen. Anchor: {angle}."
                ),
                _ => format!(
                    "Treat {topic} as a procedure: input → transform → check. {angle}. Tell me the missing input and I'll tighten the steps."
                ),
            }
        } else if lower.contains("what do you think")
            || lower.contains("thoughts")
            || lower.contains("opinion")
        {
            format!(
                "On {topic}: I don't have feelings, but I do have a grounded take — {angle}. What matters is whether that helps you decide or measure something next."
            )
        } else if lower.contains('?') {
            match seed % 4 {
                0 => format!(
                    "Direct take on {topic}: {angle}. If you want depth, give one constraint or example and I'll go concrete instead of general."
                ),
                1 => format!(
                    "For “{}”, the center of gravity is {topic}. {angle} I won't pad that with a method speech unless you want the checklist.",
                    first_sentence(user, 72)
                ),
                2 => format!(
                    "Yes — about {topic}. {angle} The honest limit: without more detail I stay structural, not specialist-deep."
                ),
                _ => format!(
                    "Here's a clean answer for {topic}. {angle} Push back or add a fact and I'll revise rather than defend the first wording."
                ),
            }
        } else {
            // Statement / share / request without ?
            let angle = if angle.ends_with('.') || angle.ends_with('?') {
                angle
            } else {
                format!("{angle}.")
            };
            match seed % 3 {
                0 => format!(
                    "Got it — {topic}. {angle} If you want a reaction, a plan, or a critique, say which and I'll lock onto that mode."
                ),
                1 => format!(
                    "I'm with you on {topic}. {angle} What's the outcome you want from this turn — clarity, a next step, or a hard challenge?"
                ),
                _ => format!(
                    "On {topic}: {angle} Stay with me one more sentence: what would “done” look like?"
                ),
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

fn first_sentence(s: &str, max: usize) -> String {
    let mut parts = s
        .split(['.', '!', '?'])
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let first = parts.next().unwrap_or(s).trim();
    let one = if matches!(
        first.to_ascii_lowercase().as_str(),
        "absolutely" | "sure" | "okay" | "fair" | "right" | "yes" | "no" | "exactly"
    ) {
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
        assert!(style.to_ascii_lowercase().contains("template") || style.to_ascii_lowercase().contains("human"));
        assert!(!style.contains("would repeat my previous answer"));
        let rep = dialogue_reply(
            DialogueAct::RepetitionComplaint,
            "why do you repeat sayings?",
            &[],
            None,
        )
        .unwrap();
        assert!(rep.to_ascii_lowercase().contains("template") || rep.to_ascii_lowercase().contains("repeat"));
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
        assert!(low.contains("permission") || low.contains("related"));
        assert!(low.contains("trust"));
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
        assert!(low.contains("bound as") || low.contains("composition") || low.contains("roles"));
        assert!(low.contains("ask:how") || low.contains("agent:"));
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
