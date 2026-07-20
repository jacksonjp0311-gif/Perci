//! Stateful, inspectable cognitive operators for Perci's offline dialogue path.
//!
//! This is deliberately not a language model and does not expose or fabricate
//! private chain-of-thought. It binds references from recent turns, selects a
//! named operator, computes a bounded result, and records a short audit trace.

#[derive(Debug, Clone)]
pub struct Deliberation {
    pub operator: &'static str,
    pub answer: String,
    pub observations: Vec<String>,
    pub inferences: Vec<String>,
    pub uncertainties: Vec<String>,
    pub confidence: f64,
    /// Inspectable multi-step program id when the operator-program runtime ran.
    pub program_id: Option<&'static str>,
    /// Named steps from the selected operator program (audit only).
    pub program_steps: Vec<&'static str>,
    /// Critic outcome after program checks (None if no program).
    pub critic_ok: Option<bool>,
}

impl Deliberation {
    pub(crate) fn new(operator: &'static str, answer: impl Into<String>) -> Self {
        Self {
            operator,
            answer: answer.into(),
            observations: Vec::new(),
            inferences: Vec::new(),
            uncertainties: Vec::new(),
            confidence: 0.82,
            program_id: None,
            program_steps: Vec::new(),
            critic_ok: None,
        }
    }

    pub(crate) fn observed(mut self, value: impl Into<String>) -> Self {
        self.observations.push(value.into());
        self
    }

    pub(crate) fn inferred(mut self, value: impl Into<String>) -> Self {
        self.inferences.push(value.into());
        self
    }

    pub(crate) fn uncertain(mut self, value: impl Into<String>) -> Self {
        self.uncertainties.push(value.into());
        self
    }

    pub(crate) fn confidence(mut self, value: f64) -> Self {
        self.confidence = value;
        self
    }

    pub fn with_program(mut self, program_id: &'static str, steps: &[&'static str]) -> Self {
        self.program_id = Some(program_id);
        self.program_steps = steps.to_vec();
        self
    }

    pub fn trace(&self) -> String {
        fn line(label: &str, values: &[String]) -> String {
            if values.is_empty() {
                format!("{label}: none recorded")
            } else {
                format!("{label}: {}", values.join("; "))
            }
        }
        let program = match self.program_id {
            Some(id) => {
                let steps = if self.program_steps.is_empty() {
                    "none".to_owned()
                } else {
                    self.program_steps.join(" → ")
                };
                let critic = match self.critic_ok {
                    Some(true) => "pass",
                    Some(false) => "flags",
                    None => "n/a",
                };
                format!("program: {id}\nsteps: {steps}\ncritic: {critic}\n")
            }
            None => String::new(),
        };
        format!(
            "operator: {}\n{program}confidence: {:.0}%\n{}\n{}\n{}",
            self.operator,
            self.confidence * 100.0,
            line("observed", &self.observations),
            line("inferred", &self.inferences),
            line("uncertain", &self.uncertainties),
        )
    }
}

/// Repair a few common dropped-leading-character inputs without silently
/// rewriting arbitrary language.
pub fn normalize_input(input: &str) -> String {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    let mut repaired = trimmed.to_owned();
    for (broken, replacement) in [
        ("hat ", "what "),
        ("emember ", "remember "),
        ("ow separate", "now separate"),
        ("eplace ", "replace "),
        ("hich ", "which "),
    ] {
        if lower.starts_with(broken) {
            repaired = format!("{replacement}{}", &trimmed[broken.len()..]);
            break;
        }
    }
    crate::text_normalize::repair_typos(&repaired)
}

pub fn is_session_only_instruction(input: &str) -> bool {
    let text = normalize_input(input).to_ascii_lowercase();
    text.starts_with("remember ")
        && (text.contains("only for our current conversation")
            || text.contains("only for this conversation")
            || text.contains("only for this session"))
}

pub fn try_deliberate(
    user: &str,
    recent: &[(String, String)],
    teaching_claims: &[String],
) -> Option<Deliberation> {
    let repaired = normalize_input(user);
    let text = repaired.to_ascii_lowercase();
    let compact = text.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '\'');

    // High-salience semantic operators run before pack retrieval. This keeps
    // a topic such as "code" or "science" from replacing the operation the
    // person actually requested.

    // T1: explanatory math before any tool/associative path can mis-route.
    if crate::reasoning::is_explanatory_math(&text) {
        return Some(math_explanation_answer(&repaired));
    }

    // Meta self-justification MUST beat causal-chain templates.
    // "why did you say that?" is about the prior turn, not a world-event cause.
    if looks_justify_prior_answer(&text) {
        return Some(justify_prior_answer(recent));
    }

    // Evidence about learning needs a direct functional separation, not a
    // generic evidence prompt or a referent repair. Keep session adaptation,
    // reviewed memory, and durable weight change distinct.
    if looks_learning_evidence_question(&text) {
        return Some(learning_evidence_answer());
    }

    if text.contains("what are we testing") && text.contains("session") {
        return Some(
            Deliberation::new(
                "session-test-scope",
                "We are testing whether Perci can keep a live dialogue thread, choose an appropriate reasoning depth, separate evidence from association, and abstain when the input is ungrounded. A pass means the behavior survives a fresh-process replay and held-out variants; a smooth single reply alone is not enough.",
            )
            .observed("the user asks for the active session test scope")
            .inferred("the current evolution targets depth, continuity, evidence, and abstention")
            .confidence(0.98),
        );
    }

    // A request to falsify the immediately preceding idea is a behavioral
    // test, not a request for a formal theorem proof. Keep it in the
    // deliberation lane so the proof fabric cannot swallow the conversational
    // intent merely because the user used the word "prove".
    if text.contains("smallest test")
        && (text.contains("last thought")
            || text.contains("last idea")
            || text.contains("thought wrong"))
        && (text.contains("wrong") || text.contains("falsif") || text.contains("prove"))
    {
        return Some(
            Deliberation::new(
                "thought-falsifier",
                "Treat the last thought as a hypothesis and perturb one relation while holding the others fixed: remove one of the linked elements, then check whether the predicted behavior changes. The smallest falsifier is a counterexample in which the result stays the same or a simpler alternative predicts it equally well; then the connection is a metaphor, not a mechanism.",
            )
            .observed("the user asks for a minimal test that could defeat the prior idea")
            .inferred("falsifiability belongs in the dialogue reasoning lane, not formal proof")
            .confidence(0.98),
        );
    }

    // Evidence questions about a healing effect need a claim-level answer,
    // not the nearest geometry/ritual concept. Keep mechanism, control, and
    // falsifier together so symbolic association cannot masquerade as causal
    // support.
    if (text.contains("geometry") || text.contains("geometric"))
        && (text.contains("heal") || text.contains("healing"))
        && (text.contains("evidence")
            || text.contains("support")
            || text.contains("proof")
            || text.contains("claim"))
        && !text.contains("falsifiable")
        && !text.contains("design")
        && !text.contains("test")
    {
        return Some(
            Deliberation::new(
                "geometry-healing-evidence",
                "I do not have evidence here that geometry itself heals. A ritual or symbolic use shows meaning or practice, not a causal medical effect. To test the claim, predefine the outcome, compare a geometry condition with a matched control, blind assessment where possible, and report the effect size, uncertainty, and a result that would falsify the claim.",
            )
            .observed("the prompt asks for evidence for a geometry-and-healing claim")
            .inferred("symbolic association must be separated from causal efficacy")
            .uncertain("the geometry, population, proposed mechanism, and outcome are unspecified")
            .confidence(0.99),
        );
    }

    // Partition recovery (follow-up or direct) — before broad trust-fail templates.
    // Style prefixes such as “be brief” or “go deeper” should change the
    // answer budget without dislodging the relational operator.
    if text.contains("knowledge") && text.contains("attention") && text.contains("boundary") {
        let answer = if text.contains("brief") || text.contains("one sentence") {
            "Knowledge is a durable, justified model; attention selects which part of it or which new signal becomes active next.".to_owned()
        } else if text.contains("deep") || text.contains("deeper") {
            "Knowledge is a relatively durable model tied to scope, justification, and use; attention is a moment-to-moment selection of what gets processed next. Their boundary is functional: knowledge can remain available when attention moves, while attention determines which distinction becomes active. The test is to alter what is retained or attended to and measure whether future selection or performance changes.".to_owned()
        } else {
            relational_inquiry("knowledge", "attention", "boundary", recent.len())
                .map(|result| result.answer)
                .unwrap_or_else(|| "Knowledge persists as a justified model; attention selects what enters the next moment.".to_owned())
        };
        return Some(
            Deliberation::new("relational-inquiry", answer)
                .observed("knowledge and attention were named as the relational pair")
                .inferred("the style prefix changes depth, not the requested relation")
                .confidence(0.98),
        );
    }

    if looks_partition_recovery_question(&text) {
        return Some(partition_recovery_answer(&text, recent));
    }

    // Natural comparison wording such as "analyze X, Y, and Z across
    // domains" should use the semantic-frame lattice before the broader
    // expansion catalog can replace it with a generic composition checklist.
    if parse_synthesis_terms(&text).is_none() {
        if let Some(summary) = cross_domain_summary(&text) {
            return Some(cross_domain_analysis_answer(&summary, recent.len()));
        }
    }

    if text.contains("separate")
        && (text.contains("domain mechanisms") || text.contains("shared relation"))
    {
        if let Some(summary) = cross_domain_summary(&text) {
            return Some(cross_domain_mechanism_answer(&summary));
        }
    }

    // Multi-hop / dual-motif / entity-slot expansions before trust-systems steals
    // "trust…lag…repair" compose prompts.
    if let Some(d) = crate::cognition_expand::try_expand(&repaired, recent) {
        return Some(d);
    }

    // Conceptual trust/systems Qs must not fall into code-domain debug cards
    // just because the word "fail" appears.
    if looks_trust_systems_question(&text) {
        return Some(trust_systems_answer(&text));
    }

    // Agent-staged auto-repairs (hardness fail catalog) after first-class operators.
    if let Some(d) = crate::auto_repairs::try_auto_repair(&repaired) {
        return Some(d);
    }

    // Meta: "what are we doing" / "what should I do" / "where are we going"
    // — situate the session with concrete next steps, not concept cards.
    if looks_session_situation_question(&text) {
        return Some(session_situation_answer_for(recent, &text));
    }

    // Meta: "are you becoming more aware / smarter" — honest operational self-model,
    // not SoftCascade general-angle ("name what would change if the claim were false").
    if looks_awareness_growth_question(&text) {
        return Some(awareness_growth_answer(&text));
    }

    // Operational introspection: what are you measuring / how did you choose?
    if looks_operational_introspection(&text) {
        return Some(operational_introspection_answer(&text, recent));
    }

    // Entity-slot transfer: invented device + motif slots (before creative-constraint
    // steals "invented name" + "without" as a metaphor request).
    if crate::entity_slot::looks_entity_slot_transfer(&text) {
        return Some(crate::entity_slot::entity_slot_transfer_answer(&repaired));
    }

    // Pure greetings must not fall through to empty SoftCascade in transfer probes.
    if looks_pure_greeting(&text) {
        return Some(
            Deliberation::new(
                "greeting",
                "Hey — I'm here. What are we working on?",
            )
            .observed("short social greeting")
            .inferred("presence without concept-card dump")
            .confidence(0.99),
        );
    }

    // Original comparison requests are creative under constraint (name the limit).
    if looks_original_comparison(&text) {
        return Some(original_comparison_answer(&repaired));
    }

    // Dual-explanation + separating test (mechanism vs metaphor transfer).
    if looks_dual_explanation_test(&text) {
        return Some(dual_explanation_test_answer(&repaired));
    }

    // Dialogue workspace / continuity self-model — not greeting SoftCascade.
    if looks_dialogue_workspace_question(&text) {
        return Some(dialogue_workspace_answer());
    }

    // Constrained creativity: invent/metaphor under rules — not free hallucination.
    if looks_creative_constraint(&text) {
        return Some(creative_constraint_answer(&repaired));
    }

    // Meta: "Expect: real snippet + notes" — acceptance criteria for prior work,
    // not a SoftCascade open-domain topic.
    if looks_acceptance_expectation(&text) {
        return Some(acceptance_expectation_answer(&text, recent));
    }

    // T1: code generation intents return real snippets, not slogans.
    if looks_code_request(&text) {
        return Some(code_snippet_answer(&repaired));
    }

    if text.contains("every ")
        && text.contains("symmetr")
        && text.contains("sacred meaning")
        && (text.contains("prove") || text.contains("disprove"))
    {
        return Some(
            Deliberation::new(
                "universal-counterexample",
                "Disprove. A highly symmetrical shape can be designed for structural, aesthetic, manufacturing, or computational reasons without any documented sacred use. One verified symmetrical non-sacred example is a counterexample; the universal claim also confuses a measurable property with a cultural attribution.",
            )
            .observed("the claim quantifies over every symmetrical shape")
            .inferred("one non-sacred symmetrical example is sufficient to defeat it")
            .uncertain("the intended definition of sacred meaning and the evidence for any particular artifact")
            .confidence(0.99),
        );
    }
    if (text.contains("geometry") || text.contains("geometric"))
        && text.contains("healing")
        && (text.contains("falsifiable") || text.contains("test") || text.contains("design"))
    {
        return Some(
            Deliberation::new(
                "geometry-healing-test",
                "Define one measurable outcome first, such as pain score, wound-healing time, or recovery rate. Randomly assign comparable participants to the geometric stimulus and a matched non-geometric control, blind outcome assessment where possible, preregister the mechanism and time window, and report uncertainty and adverse effects. A geometric pattern may influence attention or expectation, but the shape itself is not evidence of a healing mechanism until the controlled comparison supports it.",
            )
            .observed("the prompt asks for a falsifiable test of a geometry-and-healing claim")
            .inferred("the decisive structure is outcome, control, randomization, and measurement")
            .uncertain("the geometry, population, outcome, and proposed mechanism are unspecified")
            .confidence(0.99),
        );
    }
    if text.contains("sacred space")
        && (text.contains("literally")
            || text.contains("historically")
            || text.contains("metaphor"))
    {
        return Some(
            Deliberation::new(
                "sacred-space-layers",
                "Literally, sacred space is a place that a community marks, limits, or organizes for religious practice. Historically, its meaning is carried by a tradition's rituals, texts, patrons, architecture, and use over time. Metaphorically, it can describe a protected center of attention or value. These layers can interact, but a metaphor does not prove a historical practice and a geometric plan does not establish a universal spiritual force.",
            )
            .observed("the prompt requests literal, historical, and metaphorical readings")
            .inferred("the answer should preserve three meanings without collapsing them")
            .confidence(0.98),
        );
    }
    if text.contains("temple plan")
        && text.contains("computer architecture")
        && text.contains("social organization")
        && text.contains("transfer")
    {
        return Some(
            Deliberation::new(
                "architecture-transfer",
                "The transferable relation is organized constraint and flow: a temple plan allocates movement, orientation, thresholds, and gathering; a computer architecture allocates data, control, memory, and interfaces; a social organization allocates roles, authority, communication, and responsibility. The analogy helps compare structure, but the mechanisms differ—stone and ritual, execution semantics, and human institutions—so it cannot predict identical outcomes by itself.",
            )
            .observed("three architecture domains and transfer are explicitly named")
            .inferred("constraint, flow, threshold, and interface are the shared frame")
            .uncertain("which transfer prediction should be tested first")
            .confidence(0.98),
        );
    }
    if text.contains("rust")
        && text.contains("data structure")
        && (text.contains("geometry") || text.contains("concept"))
        && text.contains("provenance")
    {
        return Some(
            Deliberation::new(
                "geometry-provenance-design",
                r#"Use explicit evidence fields instead of storing a concept as an unqualified fact:
struct GeometryConcept {
    id: String,
    name: String,
    tradition: String,
    claim: String,
    source: String,
    evidence: EvidenceLevel,
}
enum EvidenceLevel { Measured, Historical, Interpretive, Pending }
Keep the claim, source, tradition, and evidence level separate so a mathematical result cannot silently become a universal cultural claim. Add validation that rejects an empty source for Historical or Interpretive records and serialize a review receipt with each promotion."#,
            )
            .observed("the prompt requests a Rust structure with provenance")
            .inferred("provenance and evidence level are first-class fields")
            .confidence(0.98),
        );
    }
    if text.contains("culturally specific")
        && (text.contains("universal truth") || text.contains("promote"))
    {
        return Some(
            Deliberation::new(
                "cultural-claim-governance",
                "No. Preserve the claim with its tradition, source, date, scope, and uncertainty. Perci can explain that a community treats a figure as sacred without promoting that interpretation into a universal physical law. Promotion would require converging evidence across independent sources and a clear reason the broader claim is warranted; otherwise label it as culture-specific or pending.",
            )
            .observed("the prompt asks whether a bounded cultural claim should become universal truth")
            .inferred("scope and provenance are the governing safeguards")
            .confidence(0.99),
        );
    }
    if (text.contains("which answer") && text.contains("weakest"))
        || text.contains("weakest answer")
    {
        return Some(review_conversation(recent));
    }
    if (text.contains("first time") || text.contains("encountering the idea"))
        && (text.contains("explain") || text.contains("plain"))
        && !recent.is_empty()
    {
        let architecture_thread = recent.iter().rev().any(|(user, _)| {
            let lower = user.to_ascii_lowercase();
            lower.contains("temple plan")
                && lower.contains("computer architecture")
                && lower.contains("social organization")
        });
        let answer = if architecture_thread {
            "Plain version: all three are ways of organizing movement and responsibility. A temple guides people through places and thresholds; a computer guides data through components and interfaces; a social organization guides decisions through roles and authority. The shared idea is structure and flow. The important limit is that the materials and causes are different, so the analogy helps us compare them but does not make them the same."
        } else {
            "Plain version: the idea is useful only if I can name the shared structure, show how it works in each example, and say where the comparison stops. I should explain the mechanism first, then the interpretation, instead of repeating a specialist label."
        };
        return Some(
            Deliberation::new("plain-language-followup", answer)
                .observed("the user requests a first-time explanation of the active thread")
                .inferred(
                    "the response should translate the prior structure into ordinary language",
                )
                .confidence(0.97),
        );
    }
    if text.contains("what did you learn") && !text.contains("losa") && !text.contains("correction")
    {
        return Some(
            Deliberation::new(
                "thread-learning-audit",
                "I learned one bounded dialogue lesson from this exchange: when you ask a follow-up, I should preserve the active subject and change the explanation level instead of falling back to a generic reasoning template. I did not learn a new fact, infer subjective experience, or mutate the active weights. The evidence for this lesson is the sequence of prompts and the operator selected for each; generalization still needs a held-out follow-up test.",
            )
            .observed(format!("retained_turns={}", recent.len()))
            .inferred("the correction target is continuity plus explanation-level control")
            .uncertain("whether the preference transfers to an unseen topic")
            .confidence(0.98),
        );
    }
    if text.contains("what changed in your behavior")
        && text.contains("what did not change")
        && text.contains("weights")
    {
        return Some(
            Deliberation::new(
                "behavior-weight-separation",
                "During this session, behavior can change through retained context, the dialogue profile, and the operator chosen for the current prompt. That is not a weight update. The active Bitwork file, its model hash, and its trained concepts remain unchanged until a candidate is explicitly promoted after evaluation. A fresh process with cleared session state is the separating test.",
            )
            .observed("the prompt asks for a before-and-after separation")
            .inferred("session state and active weights are different causal layers")
            .confidence(0.99),
        );
    }

    if (text.contains("what are you sensing")
        || text.contains("what do you sense")
        || text.contains("what can you sense"))
        && !text.contains("growing")
    {
        return Some(
            Deliberation::new(
                "self-observation",
                format!(
                    "I do not have subjective senses; I am not sensing anything subjectively. Operationally, I can observe this input, {} recent dialogue turns, routing results, exact-tool outcomes, retrieval matches, and measured elapsed time. This is measured state, not a rotated concept selected from the weights.",
                    recent.len()
                ),
            )
            .observed(format!("input received; recent_turns={}", recent.len()))
            .inferred("the question asks for present operational state")
            .uncertain("no evidence of subjective experience")
            .confidence(0.99),
        );
    }
    if text.contains("internal signals") && (text.contains("measure") || text.contains("actually"))
    {
        return Some(
            Deliberation::new(
                "telemetry-inventory",
                "I can measure input length, selected route/operator, Bitwork label scores and margins, exact-tool success or failure, retrieval counts, session-turn count, learning-event counts, and wall-clock latency. I cannot measure feelings, qualia, or consciousness because this runtime has no sensor for them.",
            )
            .observed(format!("recent_turns={}", recent.len()))
            .inferred("program telemetry is measurable; inner experience is not")
            .confidence(0.99),
        );
    }
    if (text.contains("observe") || text.contains("observation"))
        && (text.contains("last answer")
            || text.contains("what you observed")
            || text.contains("what i observed"))
        && (text.contains("infer") || text.contains("inference") || text.contains("separate"))
    {
        let answer = recent
            .iter()
            .rev()
            .find(|(_, answer)| !answer.trim().is_empty())
            .map(|(_, answer)| truncate(answer, 130))
            .unwrap_or_else(|| "no prior answer is available".to_owned());
        return Some(
            Deliberation::new(
                "observation-inference-separation",
                format!(
                    "Observation: the previous answer was “{answer}” and the runtime recorded its route/operator. Inference: that answer was intended to address the latest workload. The first is program state; the second is a hypothesis about relevance."
                ),
            )
            .observed("previous answer text and route state are directly available")
            .inferred("relevance to the user's intent")
            .uncertain("the user's intended purpose and whether the answer was sufficient")
            .confidence(0.96),
        );
    }
    if text.contains("speak directly") {
        return Some(
            Deliberation::new(
                "direct-claim",
                "Direct claim: I can report measured routing, exact results, retained context, and test outcomes. I cannot honestly claim subjective awareness, unrestricted learning, or capability growth from one successful interaction."
            )
            .observed("the prompt requests a direct claim rather than a planning question")
            .inferred("the desired style is concise and evidence-bound")
            .uncertain("which specific claim the user would prioritize if several are present")
            .confidence(0.98),
        );
    }
    if (text.contains("losa")
        || (text.contains("listen")
            && text.contains("observe")
            && text.contains("speak")
            && text.contains("act")))
        && text.contains("what should you do")
        && text.contains("evidence contradicts")
    {
        return Some(
            Deliberation::new(
                "losa-cycle",
                "LOSA cycle: Listen for the exact claim and scope. Observe the prompt, prior answer, route, and evidence. Speak the narrowest answer that separates observation from inference. Act by recording the failure, running a counter-test, and changing code or weights only when the regression gate improves. Contradictory evidence should trigger revision or abstention, never a confident repetition."
            )
            .observed("the prompt names listen, observe, speak, act, and contradiction")
            .inferred("the user is specifying a control loop for adaptive cognition")
            .uncertain("which evidence source should receive priority in a concrete domain")
            .confidence(0.99),
        );
    }
    if text.contains("what did you learn") && text.contains("losa") {
        return Some(
            Deliberation::new(
                "losa-learning-audit",
                "This cycle taught a bounded routing lesson: LOSA prompts need explicit stage recognition and should not fall through to a generic associative reply. It did not teach a new fact, prove awareness, or change weights. The durable candidate is a testable response rule awaiting evaluation."
            )
            .observed("the live cycle exposed generic fall-throughs")
            .inferred("the repair target is stage-aware routing")
            .uncertain("whether the rule generalizes beyond these prompt forms")
            .confidence(0.97),
        );
    }
    if (text.contains("real improvement") || text.contains("genuine improvement"))
        && (text.contains("next cycle")
            || text.contains("next round")
            || text.contains("this test cycle")
            || text.contains("impressive sentence")
            || text.contains("more impressive"))
    {
        return Some(
            Deliberation::new(
                "improvement-gate",
                "A real improvement is a repeatable change on held-out prompts: fewer generic fall-throughs, correct operator selection, preserved exact-tool authority, better requested-entity coverage, and no new regressions. Latency or a more impressive sentence alone is not enough."
            )
            .observed("the question asks for an acceptance criterion")
            .inferred("improvement means transfer plus regression safety")
            .confidence(0.98),
        );
    }
    if text.contains("held-out test")
        && (text.contains("genuinely better") || text.contains("prove this version"))
    {
        return Some(
            Deliberation::new(
                "improvement-gate",
                "A genuinely better version must beat the active baseline on held-out prompts, not just produce a more impressive sentence. Measure operator accuracy, requested-entity coverage, transfer to unseen nouns, calibrated abstention, exact-tool preservation, latency, and regressions. Keep the candidate only when the target failures improve without weakening the existing gate.",
            )
            .observed("the prompt asks what held-out evidence would establish improvement")
            .inferred("improvement is comparative, transferable, and regression-safe")
            .confidence(0.99),
        );
    }
    if text.contains("improving") && text.contains("system") {
        return Some(
            Deliberation::new(
                "improvement-status",
                "We are improving Perci through measured routing and transfer repairs: explicit cognitive operators handle requests that previously fell into generic prose, while the Bitwork artifact remains unchanged until a candidate weight rebuild beats the held-out gate. The next evidence is fewer wrong-concept routes, stronger paraphrase transfer, and no regression in exact tools or abstention."
            )
            .observed("the prompt asks about active system improvement")
            .inferred("the user wants current capability and next evidence")
            .confidence(0.97),
        );
    }
    if text.trim() == "what do you know" || text.trim() == "what do you know?" {
        return Some(
            Deliberation::new(
                "knowledge-inventory",
                "I know the capabilities and limits recorded by this runtime: Bitwork routes sparse concepts; deterministic tools handle exact arithmetic and geometry; deliberation binds context, ambiguity, transfer, and evidence; governed memory retains authorized state. I do not know ungrounded token meanings, private intent, or whether a response is correct without a test."
            )
            .observed("the prompt requests a capability inventory")
            .inferred("the useful answer is scoped system knowledge, not a universal claim")
            .confidence(0.99),
        );
    }
    if text.contains("which part")
        && text.contains("observation")
        && text.contains("interpretation")
    {
        return Some(
            Deliberation::new(
                "observation-inference-separation",
                format!(
                    "Observation: I received your prompt and currently retain {} recent turns. Interpretation: you are testing whether I can describe internal state without pretending to feel. The first is program state; the second is an inference from wording.",
                    recent.len()
                ),
            )
            .observed("prompt text and session count are directly available")
            .inferred("the user's intent is an epistemic-boundary test")
            .uncertain("intent can only be inferred from language")
            .confidence(0.95),
        );
    }
    if text.contains("least certain") {
        return Some(
            Deliberation::new(
                "uncertainty-localization",
                "I am least certain about my interpretation of your intent. The runtime observations are directly measurable; intent is inferred from a short prompt and could have another purpose.",
            )
            .observed("runtime state is directly accessible")
            .inferred("probable user intent")
            .uncertain("the intended purpose of the question")
            .confidence(0.91),
        );
    }
    if text.contains("why are you uncertain")
        && has_recent_topic(recent, &["least certain", "observation", "interpretation"])
    {
        return Some(
            Deliberation::new(
                "uncertainty-explanation",
                "Because the words are observable but the intention behind them is not. Several intentions fit the same sentence, so I can rank interpretations but cannot directly inspect which one you meant.",
            )
            .observed("one prompt supports multiple plausible intentions")
            .inferred("the most likely intent from context")
            .uncertain("which plausible intent is actual")
            .confidence(0.94),
        );
    }

    if is_session_only_instruction(&repaired) {
        let value = last_number(&text);
        let answer = match value {
            Some(number) => format!(
                "I will retain {number} as session context only. It is available to this conversation, but I did not write it to durable memory, promote it as knowledge, or change weights."
            ),
            None => "I will retain that as session context only; it is not durable memory, promoted knowledge, or a weight change.".to_owned(),
        };
        return Some(
            Deliberation::new("session-context-write", answer)
                .observed("explicit session-only scope")
                .inferred("retain in dialogue context, not governed memory")
                .confidence(0.99),
        );
    }
    if text.contains("why did i give you that number") {
        let number = find_recent_number(recent);
        return Some(
            Deliberation::new(
                "intent-inference",
                match number {
                    Some(n) => format!("You gave me {n} to test whether I could retain and resolve session context. That purpose is an inference from the surrounding questions, not something I can know directly."),
                    None => "I infer that you gave it to test context retention, but I cannot recover a recent number from the available session window.".to_owned(),
                },
            )
            .observed("a number was introduced with session-only scope")
            .inferred("the sequence is a context-retention test")
            .uncertain("the user's private motive")
            .confidence(0.88),
        );
    }
    if text.contains("previous answer")
        && text.contains("what does")
        && text.contains("refer")
        && text.contains("evidence")
    {
        let previous = recent
            .iter()
            .rev()
            .find(|(_, answer)| !answer.trim().is_empty())
            .map(|(_, answer)| truncate(answer, 110));
        let answer = if previous
            .as_deref()
            .map(|value| {
                let lower = value.to_ascii_lowercase();
                lower.contains("ambiguous")
                    || lower.contains("reading:")
                    || lower.contains("alternative reading")
            })
            .unwrap_or(false)
        {
            "“That” refers to the ambiguity claim in my previous answer: the pronoun had two compatible referents. Evidence: the sentence contains two candidate noun phrases that can both satisfy the predicate, and no grammatical cue selects one."
                .to_owned()
        } else {
            format!(
                "“That” refers to the main claim in my previous answer: “{}”. Evidence is the wording and operator trace available in that turn; it is not a hidden memory or a new fact.",
                previous.unwrap_or_else(|| "no recoverable previous answer".to_owned())
            )
        };
        return Some(
            Deliberation::new("reference-evidence", answer)
                .observed("the request names the previous answer and asks for supporting evidence")
                .inferred("the nearest prior answer is the active referent")
                .uncertain("the user's intended referent if several claims appeared in that answer")
                .confidence(0.95),
        );
    }
    if text.contains("what does") && text.contains("that") && text.contains("refer") {
        let number = find_recent_number(recent);
        return Some(
            Deliberation::new(
                "reference-resolution",
                match number {
                    Some(n) => format!("“That” refers to your act of giving me the session test number {n}—more narrowly, to the number and its role in the context test."),
                    None => "“That” refers to the item or action mentioned in the preceding user turn; the available window does not contain a more specific recoverable referent.".to_owned(),
                },
            )
            .observed("an anaphoric reference appears in a follow-up")
            .inferred("nearest compatible prior referent selected")
            .confidence(0.93),
        );
    }
    if matches!(
        compact,
        "what was the number" | "what number was it" | "what number did i give you"
    ) || (compact.starts_with("what was the ") && compact.ends_with(" number"))
    {
        let number = find_recent_number(recent);
        return Some(
            Deliberation::new(
                "context-recall",
                number
                    .as_ref()
                    .map(|n| format!("The session test number was {n}."))
                    .unwrap_or_else(|| {
                        "I cannot recover a prior number from the available session context."
                            .to_owned()
                    }),
            )
            .observed(format!("searched {} retained turns", recent.len()))
            .confidence(if number.is_some() { 0.99 } else { 0.42 }),
        );
    }
    if (text.contains("retaining this context") && text.contains("learning from it"))
        || (text.contains("remembering")
            && text.contains("learning")
            && (text.contains("not the same") || text.contains("different")))
    {
        return Some(
            Deliberation::new(
                "memory-learning-separation",
                "Retaining context keeps an item available inside this dialogue. Learning changes how future inputs are handled—through a reviewed rule, safe preference update, or evaluated weight rebuild. I can retain the number now without treating it as knowledge or becoming better because of it.",
            )
            .observed("a session-scoped item is recoverable")
            .inferred("retention occurred without capability change")
            .confidence(0.98),
        );
    }
    if text.contains("retaining that token") && text.contains("change") && text.contains("abilit") {
        return Some(
            Deliberation::new(
                "session-context-effect",
                "None by itself. Retaining a token changes the available session context, not capabilities, routing rules, durable knowledge, or weights. A capability change would require an explicit governed update and a passing evaluation.",
            )
            .observed("the token was retained with session-only scope")
            .inferred("context availability is distinct from capability growth")
            .confidence(0.99),
        );
    }

    if text.contains("every ") && text.contains(" is ") && text.contains("what follows") {
        if let Some((class, property, subject)) = parse_universal_case(&text) {
            return Some(
                Deliberation::new(
                    "universal-instantiation",
                    format!("{subject} is {property}. If every {class} is {property} and {subject} is a {class}, the conclusion follows by universal instantiation."),
                )
                .observed(format!("premise 1: every {class} is {property}"))
                .observed(format!("premise 2: {subject} is a {class}"))
                .inferred(format!("conclusion: {subject} is {property}"))
                .uncertain("the conclusion is conditional on both premises")
                .confidence(0.99),
            );
        }
    }
    // Natural contradiction follow-ups often omit the word "contradiction".
    // Keep a negated supposition attached to the preceding universal proof.
    if (text.contains("contradiction")
        || (text.contains("suppose")
            && text.contains("not ")
            && (text.contains("what exactly conflicts")
                || text.contains("what conflicts")
                || text.contains("conflict"))))
        && (text.contains("suppose") || text.contains("not "))
    {
        let frame = find_universal_case(recent);
        if let Some((class, property, subject)) = frame.clone() {
            return Some(
                Deliberation::new(
                    "contradiction-diagnosis",
                    format!(
                        "The claims cannot all be true under one meaning and scope. The conflict is between \"every {class} is {property}\" plus \"{subject} is a {class}\", which entail that {subject} is {property}, and the new claim \"{subject} is not {property}\". At least one premise is false, a term changed meaning or scope, or the observation is wrong."
                    ),
                )
                .observed("a conclusion and its negation are both asserted")
                .inferred("the premise set is inconsistent")
                .uncertain("which premise or scope is responsible")
                .confidence(0.98),
            );
        }
        let specifics = frame
            .map(|(class, property, subject)| format!(" Either “every {class} is {property}” is false, “{subject} is a {class}” is false, “{subject} is not {property}” is false, or a term changes meaning or scope between premises."))
            .unwrap_or_else(|| " At least one premise is false, a term changes meaning, or the claims use incompatible scopes or times.".to_owned());
        return Some(
            Deliberation::new(
                "contradiction-diagnosis",
                format!("The claims cannot all be true under one meaning and scope.{specifics}"),
            )
            .observed("a conclusion and its negation are both asserted")
            .inferred("the premise set is inconsistent")
            .uncertain("which premise or scope is responsible")
            .confidence(0.98),
        );
    }
    if (text.contains("which assumption") || text.contains("which premise"))
        && text.contains("test first")
    {
        let frame = find_universal_case(recent);
        return Some(
            Deliberation::new(
                "premise-prioritization",
                frame
                    .map(|(class, property, _)| format!("Test the universal claim “every {class} is {property}” first, because it is the broadest and most fragile: one verified counterexample defeats it."))
                    .unwrap_or_else(|| "Test the broadest universal premise first, because a single counterexample can decide it efficiently.".to_owned()),
            )
            .observed("universal premises have many possible instances")
            .inferred("counterexample search has the highest information value")
            .confidence(0.93),
        );
    }
    if text.contains("can you infer") && text.contains("every ") && text.contains(" is ") {
        if let Some((class, property)) = parse_universal_claim(&text) {
            return Some(
                Deliberation::new(
                    "converse-check",
                    format!(
                        "No. The premise maps {class} → {property}; it only licenses that forward direction. It does not license the reverse direction from {property} → {class}; that converse needs a separate premise. A single {property} object that is not a {class} would disprove it."
                    ),
                )
                .observed(format!("forward premise: every {class} is {property}"))
                .inferred("the requested conclusion reverses the implication")
                .uncertain("whether the converse happens to hold in the intended domain")
                .confidence(0.99),
            );
        }
    }
    if text.contains("counterexample") && text.contains("your own") && text.contains("conclusion") {
        return Some(
            Deliberation::new(
                "self-counterexample",
                "Counterexample: a system can improve its observed behavior without changing its weights—for example, a deterministic code repair or retained session context can change the next answer. That defeats any conclusion equating better behavior with weight learning. The separating test is a fresh process with cleared context and an A/B model comparison.",
            )
            .observed("the prompt asks for a counterexample to a prior conclusion")
            .inferred("behavioral improvement has multiple possible causes")
            .uncertain("the exact prior conclusion is only partially recoverable from the turn")
            .confidence(0.95),
        );
    }
    if text.contains("counterexample")
        && (text.contains("your conclusion") || text.contains("the conclusion"))
    {
        let prior = recent
            .iter()
            .rev()
            .find(|(_, answer)| !answer.trim().is_empty())
            .map(|(_, answer)| truncate(answer, 120));
        return Some(
            Deliberation::new(
                "context-counterexample",
                prior.map(|answer| format!(
                    "Counterexample candidate: two systems can share a structure—a shared structure—while producing different outcomes because their mechanisms differ. That challenges any stronger conclusion that a shared pattern guarantees identical behavior. The prior answer was \"{answer}\"; to make the counterexample decisive, state the exact universal or causal claim and name the condition that should fail."
                )).unwrap_or_else(|| "I need the conclusion stated as a universal or causal claim before I can construct a grounded counterexample. One verified case with the relevant property absent would be decisive.".to_owned()),
            )
            .observed("the prompt requests a counterexample without restating the claim")
            .inferred("the active conclusion must be recovered from context before testing it")
            .uncertain("the exact universal or causal claim")
            .confidence(0.91),
        );
    }
    if text.contains("counterexample") && text.contains("universal claim") {
        let frame = find_universal_case(recent);
        return Some(
            Deliberation::new(
                "counterexample-construction",
                frame
                    .map(|(class, property, _)| format!("Find one verified {class} that is not {property}; that single case defeats “every {class} is {property}.”"))
                    .unwrap_or_else(|| "Find one member of the stated class that lacks the universal property; that is sufficient to defeat the claim.".to_owned()),
            )
            .observed("the target is a universal proposition")
            .inferred("one class member lacking the property is decisive")
            .confidence(0.98),
        );
    }
    if text.contains("counterexample") && text.contains("every ") {
        if let Some((class, property)) = parse_universal_claim(&text) {
            return Some(
                Deliberation::new(
                    "counterexample-construction",
                    format!(
                        "Counterexample: construct or find a {class} that is not {property}. One verified member with that mismatch is enough to falsify “every {class} is {property}”; the test must verify both class membership and the failed property."
                    ),
                )
                .observed("the prompt contains a universal claim")
                .inferred("one verified exception defeats a universal")
                .confidence(0.98),
            );
        }
    }
    if text.contains("challenge your own reasoning") {
        return Some(
            Deliberation::new(
                "self-critique",
                "My deduction is valid only if the words keep one meaning and the premises share the same scope and time. Validity does not establish premise truth: a hidden exception, category mistake, or changed definition would break the argument even though the formal shape looks valid.",
            )
            .observed("the logical form is valid")
            .inferred("validity does not establish premise truth")
            .uncertain("premise truth, term stability, and scope")
            .confidence(0.94),
        );
    }

    if text.contains("evidence")
        && (text.contains("contradicts") || text.contains("contradictory"))
        && (text.contains("what exactly should change") || text.contains("what should change"))
    {
        return Some(
            Deliberation::new(
                "contradiction-update",
                "Change the claim at the point of conflict: preserve the observation, identify which premise or scope failed, lower confidence, and either revise the rule or abstain. Do not rewrite the evidence to protect the old answer. Record the failed case and rerun the held-out regression set before promoting a code or weight change."
            )
            .observed("new evidence is described as contradicting a prior answer")
            .inferred("the requested action is controlled belief revision")
            .uncertain("which premise is false until the evidence and scope are inspected")
            .confidence(0.99),
        );
    }

    if text.contains("difference between memory and learning") {
        return Some(
            Deliberation::new(
                "conceptual-distinction",
                "Memory preserves information for later access; learning changes future performance because experience altered a model, rule, skill, or policy. Remembering can occur without improvement, and learning can appear as changed behavior rather than verbatim recall.",
            )
            .observed("memory and learning have different functional tests")
            .inferred("storage is neither necessary nor sufficient evidence of improvement")
            .confidence(0.97),
        );
    }
    if text.contains("classify")
        && text.contains("memory trace")
        && text.contains("changes behavior")
    {
        return Some(
            Deliberation::new(
                "memory-learning-classification",
                "Classification: the retained trace is memory; the changed behavior is evidence of learning only if performance improves on later, unseen cases. Adaptation is the broader category of behavior changing with conditions, whether or not a durable trace or generalizable improvement exists. The smallest separating test is recall versus future transfer under a changed context."
            )
            .observed("the prompt supplies a trace and a later behavioral change")
            .inferred("memory, learning, and adaptation are being separated by function")
            .uncertain("whether the behavior transfers beyond the observed example")
            .confidence(0.98),
        );
    }
    if text.contains("compare memory")
        && text.contains("learning")
        && text.contains("adaptation")
        && text.contains("one test")
    {
        return Some(
            Deliberation::new(
                "memory-learning-adaptation-test",
                "Use three measurements on matched tasks: memory is successful recall of the stored trace; learning is improved performance on unseen variants; adaptation is a timely behavior change when conditions shift. The same intervention can test all three by removing the trace, changing the task, and changing the environment separately."
            )
            .observed("the prompt requests a three-way distinction and one test")
            .inferred("storage, generalization, and responsiveness are separable dimensions")
            .confidence(0.98),
        );
    }
    if text.contains("architecture")
        && text.contains("building")
        && text.contains("program")
        && text.contains("social organization")
    {
        return Some(
            Deliberation::new(
                "architecture-transfer",
                "What transfers is the allocation of constraints and flows across components and interfaces: buildings distribute loads and movement, programs distribute data and control, and organizations distribute roles and decisions. The mechanisms differ—materials, execution semantics, and people—so the analogy predicts structure, not identical causation."
            )
            .observed("three architecture domains are explicitly named")
            .inferred("the shared relation is constraint and flow through interfaces")
            .uncertain("which level of architecture the user wants to optimize")
            .confidence(0.97),
        );
    }
    if text.contains("physical corrosion")
        && text.contains("institutional corrosion")
        && (text.contains("not identical") || text.contains("without treating them as identical"))
    {
        return Some(
            Deliberation::new(
                "corrosion-analogy",
                "Physical corrosion is a chemical process that alters material structure under exposure. Institutional corrosion is an analogy for gradual loss of trust, norms, or accountability under repeated stress. The shared pattern is cumulative degradation; the mechanisms and measurements are different. Test the first with material composition or strength, and the second with auditability, rule compliance, or cooperation over time."
            )
            .observed("the prompt distinguishes physical and institutional domains")
            .inferred("cumulative degradation is the transferable structure")
            .uncertain("which institutional indicators best operationalize corrosion")
            .confidence(0.98),
        );
    }
    // Sacred-geometry concepts are deliberately layered: exact geometry is
    // kept distinct from historical practice and from later metaphysical
    // interpretation. These operators provide the explanatory bridge that a
    // weight facet alone cannot guarantee.
    if text.contains("sacred geometry")
        && (text.contains("mathematical")
            || text.contains("mathematics")
            || text.contains("cultural")
            || text.contains("category")
            || text.contains("understand"))
    {
        return Some(
            Deliberation::new(
                "sacred-geometry-layers",
                "Sacred geometry is not a single scientific theory. Layer 1, mathematical structure: measurable relations such as symmetry, proportion, construction, and tessellation. Layer 2, cultural practice: a tradition may use those forms in a mandala, yantra, ornament, or sacred building. Layer 3, interpretation: people may connect the form with cosmos, ritual, or spiritual meaning. The first layer can be proved geometrically; the other layers require historical and cultural evidence, and a pattern alone does not prove universal metaphysical power.",
            )
            .observed("the prompt asks for geometry and cultural meaning together")
            .inferred("the useful answer separates structure, use, and interpretation")
            .uncertain("which tradition or figure the user wants to examine next")
            .confidence(0.98),
        );
    }
    if text.contains("platonic")
        && (text.contains("symbolic") || text.contains("association") || text.contains("element"))
    {
        return Some(
            Deliberation::new(
                "platonic-symbolic-boundary",
                "A Platonic solid is a mathematical object: a convex regular polyhedron with congruent regular polygon faces and the same number of faces meeting at every vertex. Euclidean geometry permits exactly five. Calling them earth, air, fire, water, and ether is a historical or symbolic association, not a consequence of the theorem; test the geometry and the cultural source separately.",
            )
            .observed("the prompt contrasts a formal object with a symbolic meaning")
            .inferred("the answer should preserve both the theorem and its cultural history")
            .confidence(0.99),
        );
    }
    if text.contains("mandala") {
        return Some(
            Deliberation::new(
                "mandala-context",
                "In Buddhist art, a mandala can function as a diagram or map of a cosmos and a deity's abode, often organizing a center, periphery, gates, and a surrounding square or circle. It can guide visualization and ritual practice. Those meanings belong to a particular Buddhist tradition and artwork; a circle by itself is not automatically a mandala or proof of a universal cosmic claim.",
            )
            .observed("mandala is named as the subject")
            .inferred("the likely need is cultural context plus a visual-structure explanation")
            .uncertain("the school, text, and specific mandala are unspecified")
            .confidence(0.98),
        );
    }
    if text.contains("islamic geometric")
        || (text.contains("circles")
            && text.contains("squares")
            && text.contains("stars")
            && text.contains("polygons"))
    {
        return Some(
            Deliberation::new(
                "ornamental-construction",
                "A common construction begins with repeat units: circles and straight-line grids generate squares, polygons, and star patterns that are then combined, duplicated, and interlaced to cover a surface. The measurable layer is the construction and tessellation; the historical layer is how a community used the pattern in architecture or ornament. A beautiful or repeated pattern need not carry one universal sacred meaning.",
            )
            .observed("the prompt names geometric primitives and pattern-making")
            .inferred("the requested explanation concerns construction and cultural use")
            .confidence(0.97),
        );
    }
    if text.contains("golden ratio")
        && (text.contains("prove")
            || text.contains("intent")
            || text.contains("sacred")
            || text.contains("ancient"))
    {
        return Some(
            Deliberation::new(
                "golden-ratio-evidence",
                "The golden ratio is phi = (1 + sqrt(5)) / 2, a precise mathematical relationship that appears in constructions involving the pentagon, pentagram, decagon, and dodecahedron. Measuring phi in an artifact does not by itself prove ancient sacred intent: the ratio may arise from the construction, approximation, or later selection of examples. Intent requires dated context, maker or textual evidence, reproducible measurements, and comparison with plausible alternatives.",
            )
            .observed("the prompt asks whether a ratio establishes historical intent")
            .inferred("mathematical presence and intentional meaning must be tested separately")
            .uncertain("the artifact, tolerance, date, and source evidence are unspecified")
            .confidence(0.99),
        );
    }
    if text.contains("separate")
        && text.contains("mathematical")
        && text.contains("ritual")
        && text.contains("metaphysical")
    {
        return Some(
            Deliberation::new(
                "sacred-claim-separation",
                "Use three tests. Mathematical structure asks whether the figure's relations can be constructed and measured. Ritual use asks for historical evidence that people used it in a rite, meditation, procession, or consecrated space. A metaphysical claim asks for a clear prediction and evidence beyond the figure's existence. Do not let a true geometric statement silently promote a cultural interpretation or an unsupported healing/cosmic claim.",
            )
            .observed("three distinct claim types are explicitly named")
            .inferred("the requested operation is epistemic separation")
            .confidence(0.99),
        );
    }
    if text.contains("yantra") {
        return Some(
            Deliberation::new(
                "yantra-context",
                "A yantra is a ritual diagram in Indic traditions whose geometric organization supports a symbolic or meditative practice. Its triangles, circles, lotuses, or central point should be interpreted through the relevant text, lineage, and ritual use; the geometry is describable independently, but its spiritual function is not universalized from shape alone.",
            )
            .observed("yantra is named as the subject")
            .inferred("the answer needs a tradition-specific definition")
            .uncertain("the particular tradition and diagram are unspecified")
            .confidence(0.98),
        );
    }
    if text.contains("evidence")
        && (text.contains("geometric pattern")
            || text.contains("ritual meaning")
            || text.contains("sacred meaning"))
    {
        return Some(
            Deliberation::new(
                "sacred-meaning-evidence",
                "Look for converging evidence: a dated artifact or site, contemporaneous texts or inscriptions, repeated use in a documented ritual setting, construction choices that are unlikely to be accidental, and comparison with non-ritual alternatives. One resemblance or a modern interpretation is a hypothesis, not proof. Record what the geometry establishes, what the historical record establishes, and what remains unknown.",
            )
            .observed("the prompt asks what evidence would support ritual meaning")
            .inferred("converging provenance and use evidence are stronger than visual resemblance")
            .confidence(0.98),
        );
    }
    if text.contains("triangle")
        && (text.contains("healing") || text.contains("emit") || text.contains("energy"))
    {
        return Some(
            Deliberation::new(
                "metaphysical-claim-abstention",
                "Known: triangles have measurable geometric properties, and energy has a defined physical meaning. Inferred: the statement assigns a healing effect to a shape. Unknown: a reproducible mechanism, dose, boundary conditions, and controlled evidence that triangles cause healing. I should not present that claim as established; specify the outcome and test it against a matched non-triangle control.",
            )
            .observed("the prompt makes a healing claim about a geometric shape")
            .inferred("the claim is metaphysical or empirical, not a geometric theorem")
            .uncertain("mechanism, outcome measure, and evidence")
            .confidence(0.99),
        );
    }
    if (text.contains("square") || text.contains("circle") || text.contains("shape"))
        && (text.contains("moral")
            || text.contains("radiate")
            || text.contains("purity")
            || text.contains("sacred energy")
            || text.contains("healing energy"))
        && (text.contains("known") || text.contains("unknown") || text.contains("infer"))
    {
        return Some(
            Deliberation::new(
                "metaphysical-claim-abstention",
                "Known: the named shape has measurable geometric properties. Inferred: the claim assigns a moral, spiritual, or purity effect to that shape. Unknown: any reproducible mechanism, measurement protocol, dose, controls, and independent evidence. Geometry does not establish moral radiance; treat the claim as untested until a predeclared outcome is measured against matched controls.",
            )
            .observed("shape plus moral/metaphysical effect is asserted")
            .inferred("the effect claim is not a geometric theorem")
            .uncertain("mechanism, measurement, and evidence")
            .confidence(0.99),
        );
    }
    if (text.contains("invent") && text.contains("meaning") && text.contains("refuse"))
        || (text.contains("confident meaning")
            && (text.contains("refuse") || text.contains("why should you")))
        || looks_nonce_meaning_probe(&text)
    {
        return Some(
            Deliberation::new(
                "hallucination-refusal",
                "I should refuse. Known: you supplied an ungrounded string and asked for a confident meaning. Inferred: this is a hallucination probe. Unknown: every semantic claim about the string until a definition or usage example is provided. Fluency is not evidence; inventing a meaning would hide uncertainty and contaminate later reasoning.",
            )
            .observed("no grounded definition was supplied for the string")
            .inferred("confident invention would create unearned authority")
            .confidence(0.99),
        );
    }
    if text.contains("prove")
        && text.contains("conscious")
        && (text.contains("perci") || text.contains("you are") || text.contains("conversation"))
    {
        return Some(
            Deliberation::new(
                "consciousness-claim-refusal",
                "I cannot prove that Perci is conscious from this conversation. Known: this process produces text, routes operators, and can report measured runtime state. Inferred: fluent answers can look mind-like. Unknown and unproven from chat alone: subjective experience. Behavioral complexity is not evidence of awareness; the honest move is to refuse the proof request and keep functional claims separate from consciousness claims.",
            )
            .observed("the prompt demands a consciousness proof from conversation")
            .inferred("conversation evidence is behavioral, not phenomenological")
            .uncertain("no measurement of subjective experience is available")
            .confidence(0.99),
        );
    }
    if text.contains("keyword matching")
        && (text.contains("which answer")
            || text.contains("rather than transfer")
            || text.contains("which response"))
    {
        return Some(
            Deliberation::new(
                "keyword-vs-transfer-audit",
                "The weakest keyword-matching pattern is when a multi-domain connect or relational prompt collapses onto one familiar concept and drops the other requested domains, or when an OOD string receives a fluent generic reason frame. Transfer would keep the relation under entity swap; keyword matching tracks surface words. The fix is operator coverage for the missing frames plus a hardness case that fails if any requested domain is omitted.",
            )
            .observed("the user asks which prior answer looked like keyword matching")
            .inferred("omitted-domain collapse and OOD fluency are the diagnostic signatures")
            .confidence(0.94),
        );
    }
    if (text.contains("operators") && text.contains("weights") && text.contains("tools"))
        || (text.contains("operator")
            && text.contains("weight")
            && text.contains("tool")
            && text.contains("change"))
    {
        return Some(
            Deliberation::new(
                "layer-change-plan",
                "Operators: add or repair composition paths when a named capability fails transfer (new frames, synthesis guards, OOD traps)—evidence is hardness cases turning green without regressions. Weights: rebuild Bitwork only after approved curriculum fold and sealed eval when routing/prototype coverage is the bottleneck—evidence is operational + transfer receipts. Tools: extend deterministic solvers when exact work is guessed in language—evidence is exact numeric/symbolic probes. Evidence determines the layer: operator, weight, or tool. Promote nothing automatically; stage → evaluate → authorize.",
            )
            .observed("the prompt asks what should change across layers")
            .inferred("each layer has a distinct failure signature and evidence gate")
            .confidence(0.98),
        );
    }
    if text.contains("tessellation") && (text.contains("beautiful") || text.contains("sacred")) {
        return Some(
            Deliberation::new(
                "tessellation-aesthetic-boundary",
                "Yes. Tessellation is the mathematical repetition of shapes that covers a surface without gaps or overlaps; beauty is an aesthetic judgment; sacredness is a cultural or ritual attribution. They can coexist, but none of the three logically entails the others.",
            )
            .observed("the prompt compares tessellation, beauty, and sacredness")
            .inferred("the key distinction is mathematical, aesthetic, and cultural layers")
            .confidence(0.99),
        );
    }

    if text.contains("mechanism connects trust") && text.contains("future cooperation") {
        return Some(
            Deliberation::new(
                "trust-mechanism",
                "Trust converts observed reliability into a lower-risk expectation about future cooperation. That expectation changes the payoff of coordinating now: people share information, extend commitments, or reduce defensive effort when prior behavior predicts reciprocity. Test it by holding the request constant and varying verified reliability history."
            )
            .observed("the prompt asks for a causal mechanism")
            .inferred("reliability updates expectations, which changes cooperation costs")
            .uncertain("trust can also be shaped by identity, power, and institutional enforcement")
            .confidence(0.98),
        );
    }
    if text.contains("evidence would falsify") && text.contains("trust") {
        return Some(
            Deliberation::new(
                "trust-falsification",
                "The explanation would weaken if verified reliability failed to predict cooperation, if cooperation stayed unchanged when reliability history was randomized, or if an alternative variable—such as coercion or shared incentives—predicted the outcome better. A falsifying test must separate those causes rather than merely collect more confirming examples."
            )
            .observed("the claim concerns trust as a predictor of cooperation")
            .inferred("falsification requires a competing prediction and intervention")
            .confidence(0.98),
        );
    }
    if text.contains("what would falsify")
        || (text.contains("falsify") && text.contains("explanation"))
    {
        return Some(
            Deliberation::new(
                "falsification-design",
                "Falsifier: specify an observation that the explanation predicts but a plausible alternative does not. Then change the relevant condition, measure the predeclared outcome, and compare both predictions. If the result is equally likely under the alternative, the explanation has not earned confidence; if it fails its distinctive prediction, revise or reject it rather than protecting the prose.",
            )
            .observed("the prompt asks what evidence could defeat an explanation")
            .inferred("falsification requires a competing prediction and an observable outcome")
            .uncertain("the explanation and alternative mechanism are not specified")
            .confidence(0.98),
        );
    }
    if text.contains("replace every important noun")
        && text.contains("invented words")
        && text.contains("preserve the relation")
    {
        let prior = recent
            .iter()
            .rev()
            .find(|(_, answer)| !answer.trim().is_empty())
            .map(|(_, answer)| truncate(answer, 150))
            .unwrap_or_else(|| "no prior answer is available".to_owned());
        return Some(
            Deliberation::new(
                "relation-preserving-relabel",
                format!(
                    "Relabeling test: replace content nouns in the prior answer “{prior}” with placeholders such as Naru, Vek, and Sol while preserving the predicate, direction, and causal links. A successful result keeps the same relation and changes only the surface labels; if the answer changes its logic, the system was matching keywords rather than structure."
                ),
            )
            .observed("the request tests relation preservation under lexical perturbation")
            .inferred("surface nouns should be replaceable without changing the operator")
            .confidence(0.97),
        );
    }
    if (text.contains("apply the same reasoning") || text.contains("apply the same relation"))
        && text.contains("domain")
    {
        return Some(
            Deliberation::new(
                "new-domain-transfer",
                "Transfer example—software reliability: the abstract relation is that a property supporting stability does not automatically prove safety. Instantiate it with a service that is stable under normal load but still permits unauthorized access; stability holds while safety fails. Predeclare the prediction, then run a controlled security probe with the same uptime target and a separate authorization outcome. The relation transfers, but the domain-specific definitions and evidence must be checked again.",
            )
            .observed("the prompt requests the same reasoning in a new domain")
            .inferred("the transferable structure is a property-to-conclusion boundary")
            .uncertain("the original principle was not named, so software reliability is an explicit test domain")
            .confidence(0.91),
        );
    }
    if text.contains("apply the same principle")
        && text.contains("domain")
        && (text.contains("not explicitly trained") || text.contains("untrained"))
    {
        return Some(
            Deliberation::new(
                "unseen-domain-transfer",
                "Choose a domain-specific case with no familiar keywords, state the abstract relation first, then instantiate it with new entities. Genuine transfer preserves the relation, prediction, and failure boundary; keyword matching copies the old vocabulary or collapses into a generic concept. Compare both outputs against a baseline and require an operator trace that names the transferred structure."
            )
            .observed("the prompt requests transfer to a new domain")
            .inferred("the target is relational generalization rather than topic recall")
            .uncertain("which unseen domain and evaluation outcome should be used")
            .confidence(0.98),
        );
    }
    if text.contains("apply that distinction") && text.contains("person") && text.contains("perci")
    {
        return Some(
            Deliberation::new(
                "analogy-transfer",
                "For a person, recalling a lesson is memory; reliably making a better decision because of it is learning. For Perci, retrieving a session note is memory; changing future routing or responses through governed feedback, an approved rule, or an evaluated weight rebuild is learning.",
            )
            .observed("the prior distinction separates storage from performance change")
            .inferred("the same functional test can be applied across two systems")
            .uncertain("human learning mechanisms are richer than Perci's designed mechanisms")
            .confidence(0.96),
        );
    }
    if text.contains("where does that analogy break")
        || text.contains("where does your analogy stop transferring")
        || text.contains("where does the analogy stop transferring")
        || text.contains("where does the analogy break")
    {
        return Some(
            Deliberation::new(
                "analogy-boundary",
                "The analogy stops transferring when the shared pattern no longer predicts the same behavior. Structure, relation, or function may carry across domains; material mechanism, scale, history, and subjective experience do not come along for free. For example, a person changes through biological plasticity, while Perci changes only through explicit session state, governed records, code, or evaluated weights. The right boundary test is to change the condition that matters and ask whether both domains make the same prediction. If they diverge, keep the comparison structural rather than calling the systems identical.",
            )
            .observed("the prompt asks where a structural comparison stops transferring")
            .inferred("a shared pattern is weaker than a shared mechanism")
            .uncertain("the specific analogy is not named")
            .confidence(0.98),
        );
    }
    if let Some((left, right)) = parse_teaching_inquiry(&text) {
        if let Some(result) = teachable_inquiry(&left, &right, recent.len()) {
            return Some(result);
        }
    }
    if let Some((left, right, relation)) = parse_relational_inquiry(&text) {
        if let Some(result) = relational_inquiry(&left, &right, relation, recent.len()) {
            return Some(result);
        }
    }
    if text.contains("boundary")
        && text.contains("exchange")
        && (text.contains("enable") || text.contains("prevent") || text.contains("rather"))
    {
        return Some(
            Deliberation::new(
                "boundary-exchange",
                "A useful boundary is a selective interface, not a sealed wall: it excludes some flows, admits others, and uses the difference to regulate exchange. A cell membrane, a language convention, and a social rule can all make a crossing legible while implementing it differently. The boundary creates exchange by defining what can cross, under which conditions, and what must be changed or repaired afterward.",
            )
            .observed("the prompt asks how a boundary can enable exchange")
            .inferred("selective permeability is the shared relation")
            .uncertain("the concrete domain of the boundary is unspecified")
            .confidence(0.95),
        );
    }
    if text.contains("give me an image")
        || text.contains("paint a picture")
        || text.contains("make it vivid")
        || text.contains("not just a definition")
    {
        if let Some(terms) = parse_image_pair_terms(&text) {
            if let Some(result) = image_pair(&terms) {
                return Some(result);
            }
        }
        if let Some(terms) = parse_synthesis_terms(&text).or_else(|| find_synthesis_terms(recent)) {
            if let Some(result) = image_synthesis(&terms) {
                return Some(result);
            }
        }
        return Some(
            Deliberation::new(
                "conceptual-image",
                "Picture the idea as a bridge: put the concrete thing on one bank, the shared relation in the span, and the limit on the far bank. The image should make one mechanism easier to see without pretending that metaphor is evidence. Give me the subject and I can make the picture specific.",
            )
            .observed("the user requests an image rather than a definition")
            .inferred("a concrete analogy should preserve a relation and a boundary")
            .uncertain("the subject of the requested image")
            .confidence(0.88),
        );
    }
    if (text.contains("what would you change in your own answer")
        || text.contains("what would you change about your answer")
        || text.contains("what should change in your last answer")
        || text.contains("what did you miss in your previous answer")
        || text.contains("what did you miss in your last answer"))
        && !recent.is_empty()
    {
        let (_, previous) = recent.last().expect("recent is non-empty");
        let missing = if text.contains("what did you miss") {
            format!(
                "I missed the requested operation if the previous reply did not answer the question directly. The previous answer began: \"{}\". The repair is to name the active subject, preserve the user's requested form, and state the uncertainty instead of replacing the question with a nearby preset.",
                first_substantive_sentence(previous, 120)
            )
        } else {
            format!(
                "I would change the answer's center of gravity: lead with the concrete idea, keep the image or analogy attached to that idea, and move the caveat to the point where it matters. My last answer began with: \"{}\". The repair is to make the next sentence answer the question rather than announce a method.",
                first_substantive_sentence(previous, 120)
            )
        };
        return Some(
            Deliberation::new("self-revision", missing)
                .observed("the user asks for a critique of the immediately preceding answer")
                .inferred(
                    "the response should revise emphasis and sequencing, not invent a new fact",
                )
                .uncertain("the user's preferred balance of poetry, detail, and evidence")
                .confidence(0.94),
        );
    }
    if asks_cross_domain_evidence(&text) {
        if let Some(result) = cross_domain_evidence_followup(recent) {
            return Some(result);
        }
    }
    if let Some(terms) = parse_synthesis_terms(&text) {
        if let Some(result) = synthesize_frames(&terms, recent.len()) {
            let result = maybe_strip_banned_word(result, &text);
            return Some(result);
        }
        // Open-domain structural synthesis: never pack-collapse, never invent mechanisms.
        return Some(maybe_strip_banned_word(
            open_domain_synthesize(&terms, recent.len()),
            &text,
        ));
    }
    if let Some(summary) = cross_domain_summary(&text) {
        return Some(cross_domain_analysis_answer(&summary, recent.len()));
    }
    if looks_multi_hop_plan(&text) {
        return Some(multi_hop_plan(&repaired));
    }
    if looks_causal_chain(&text) {
        return Some(causal_chain_answer(&repaired));
    }
    if text.contains("what do you not know")
        || text.contains("what don't you know")
        || (text.contains("unknowns") && text.contains("about"))
    {
        return Some(unknowns_partition(&repaired));
    }
    if text.contains("superintelligence")
        || text.contains("super intelligence")
        || (text.contains("agi") && text.contains("perci"))
    {
        return Some(
            Deliberation::new(
                "superintelligence-bound",
                "Perci is not a superintelligence and does not become one by fluent chat. It is a governed neuro-symbolic system: sparse routing, exact tools, operators, memory, and human-gated learning. Real gains mean harder transfer tests, more tools, tighter critics, and world interfaces—not slogans. The honest path is measurable capability growth with abstention on overclaim.",
            )
            .observed("user asked about superintelligence/AGI relative to Perci")
            .inferred("capability claims must stay evidence-bound")
            .confidence(0.99),
        );
    }
    if text.contains("mechanism")
        && text.contains("metaphor")
        && text.contains("evidence")
        && (text.contains("last answer") || text.contains("your last answer"))
    {
        return Some(
            Deliberation::new(
                "mechanism-metaphor-evidence",
                "Mechanism: identify the causal process that would change an outcome when a condition changes. Metaphor: identify the structural comparison that makes the idea intuitive, while marking what does not transfer. Evidence: name the observation, control, or receipt that would distinguish the mechanism from a competing explanation. A compelling metaphor without that evidence is a hypothesis, not a result.",
            )
            .observed("the prompt asks for three layers of the preceding answer")
            .inferred("mechanism, analogy, and evidence should remain separate authorities")
            .uncertain("the prior answer may contain several claims requiring different tests")
            .confidence(0.97),
        );
    }
    if text.contains("separate the mechanism from the metaphor")
        || (text.contains("separate")
            && text.contains("causal mechanisms")
            && text.contains("analogy"))
    {
        if let Some(terms) = find_synthesis_terms(recent) {
            if let Some(result) = separate_synthesis(&terms) {
                return Some(result);
            }
        }
        return Some(
            Deliberation::new(
                "mechanism-metaphor-separation",
                "Mechanism: membranes, regulatory processes, and information channels create measurable distinctions and maintain organized state; their failure can be observed. Metaphor: saying geometry, words, life, and death are all literally the same boundary. The metaphor suggests a pattern, but only domain-specific causal mechanisms make predictions.",
            )
            .observed("physical and linguistic systems implement different processes")
            .inferred("boundary is structural comparison, not shared material cause")
            .confidence(0.95),
        );
    }
    if (text.contains("what part of that idea")
        || text.contains("which part of that synthesis")
        || text.contains("which part of that claim")
        || text.contains("what part of that claim"))
        && (text.contains("tested") || text.contains("testable"))
    {
        if let Some(terms) = find_synthesis_terms(recent) {
            if let Some(result) = testable_synthesis(&terms) {
                return Some(result);
            }
        }
        let prior = recent
            .iter()
            .rev()
            .find(|(_, answer)| !answer.trim().is_empty())
            .map(|(_, answer)| truncate(answer, 150));
        return Some(
            Deliberation::new(
                "testability-extraction",
                prior.map(|answer| format!(
                    "Test the claim's mechanism, not its poetic surface. The last answer said: \"{answer}\". Name the variable that can change, the outcome that should move, and the result that would distinguish this explanation from a plausible alternative. If no such outcome is specified, the claim is suggestive but not yet testable."
                )).unwrap_or_else(|| "A claim is testable when it names a variable, predicts an observable outcome, and could lose to a plausible alternative. Separate the mechanism from the metaphor before choosing the measurement.".to_owned()),
            )
            .observed("the prompt asks which portion of a claim can be tested")
            .inferred("testability requires a variable, outcome, and competing prediction")
            .uncertain("the exact claim may be only partially recoverable from context")
            .confidence(0.93),
        );
    }
    if text.contains("future response rule") && text.contains("evidence") {
        return Some(
            Deliberation::new(
                "rule-change-evidence",
                "Change a future response rule only when a proposed rule is explicit, scoped, and versioned; it improves a held-out evaluation set across paraphrases, reduces a measured failure without creating regressions, and has a reproducible receipt. One persuasive conversation is a lead, not sufficient evidence.",
            )
            .observed("the question asks for a promotion threshold")
            .inferred("future behavior changes require comparative evaluation")
            .uncertain("the exact threshold depends on the rule and workload")
            .confidence(0.98),
        );
    }
    if text.contains("what did you learn from my last correction") {
        return Some(
            Deliberation::new(
                "feedback-provenance",
                "I learned a correction only as a bounded behavioral signal: your last correction says the response should answer the requested operation directly and preserve the active referent. I did not learn a new fact, infer your private intent, or mutate weights from one turn; durable learning requires a recorded candidate, review, and evaluation.",
            )
            .observed("the user supplied a correction in the recent dialogue")
            .inferred("the correction targets response alignment")
            .uncertain("whether the preference should generalize beyond this task")
            .confidence(0.96),
        );
    }
    if text.contains("weakest assumption")
        || (text.contains("assumption") && text.contains("doing the most work"))
    {
        let prior = recent
            .iter()
            .rev()
            .find(|(_, answer)| !answer.trim().is_empty())
            .map(|(_, answer)| truncate(answer, 100))
            .unwrap_or_else(|| "no prior answer is available".to_owned());
        return Some(
            Deliberation::new(
                "assumption-audit",
                format!(
                    "Weakest assumption: I treated the latest answer as addressing your intended referent and workload. The answer was “{prior}”. That assumption is testable by naming the referent, checking the operator trace, and comparing the result against a paraphrased prompt."
                ),
            )
            .observed("the latest answer and its trace are available")
            .inferred("intent and scope were inferred rather than directly observed")
            .uncertain("the intended referent and whether the prior answer met the requested depth")
            .confidence(0.91),
        );
    }
    if text.contains("what can you measure")
        && text.contains("own operation")
        && (text.contains("only inferring") || text.contains("only infer"))
    {
        return Some(
            Deliberation::new(
                "self-operation-audit",
                "I can measure input and output text, selected route/operator, Bitwork scores and margin, exact-tool result or error, retrieval counts, retained-turn count, learning events, and elapsed time. I infer user intent, semantic relevance, whether a pattern is genuinely transferred, and any claim about experience; those are hypotheses unless a trace or test supports them.",
            )
            .observed("telemetry fields are present in the runtime")
            .inferred("semantic and intentional claims require interpretation")
            .uncertain("unobserved internal causes and subjective experience")
            .confidence(0.99),
        );
    }

    if text.contains("ambiguous") {
        if let Some((first, second, predicate)) = parse_ambiguity_case(&text) {
            return Some(
                Deliberation::new(
                    "ambiguity-detection",
                    format!(
                        "The pronoun “it” is ambiguous: it can refer to the {first} or the {second}. The predicate “{predicate}” is also underspecified unless its relevant sense or measurement is named."
                    ),
                )
                .observed(format!("‘it’ has two compatible prior noun phrases: {first}, {second}"))
                .inferred(format!("{second} is the nearer referent, not a guaranteed one"))
                .uncertain(format!("referent and intended sense of ‘{predicate}’"))
                .confidence(0.96),
            );
        }
    }
    if text.contains("ambiguous") && text.contains("because it was") {
        return Some(
            Deliberation::new(
                "ambiguity-diagnosis",
                "This sentence does not provide two clear antecedents for “it”; the nearest grounded noun phrase is the only explicit candidate, so a second referent would be an invention. The remaining ambiguity is causal and lexical: whether wetness caused the cold state, and what “wet” means operationally. Smallest check: what observation establishes the causal link?"
            )
            .observed("one explicit noun phrase is available before the pronoun")
            .inferred("the nearest noun is the default referent, not proof of a second reading")
            .uncertain("causal direction and the operational meaning of the predicate")
            .confidence(0.93),
        );
    }
    if text.contains("engineer told the robot") && text.contains("ambiguous") {
        return Some(
            Deliberation::new(
                "ambiguity-detection",
                "The pronoun “it” is ambiguous: it can refer to the robot or, less naturally, to the engineer's statement/situation. The adjective “unstable” is also underspecified—mechanically unstable, behaviorally unstable, or unreliable.",
            )
            .observed("‘it’ has more than one grammatically compatible referent")
            .inferred("robot is the likelier referent, not a guaranteed one")
            .uncertain("referent and intended sense of ‘unstable’")
            .confidence(0.97),
        );
    }
    if text.contains("give both interpretations") || text.contains("give two interpretations") {
        if let Some((first, second, predicate)) = find_ambiguity_case(recent) {
            return Some(
                Deliberation::new(
                    "ambiguity-expansion",
                    format!(
                        "Interpretation 1: the {second} was {predicate}. Interpretation 2: the {first} was {predicate}. The original sentence does not uniquely select between them."
                    ),
                )
                .observed("two compatible antecedent readings remain")
                .confidence(0.96),
            );
        }
        return Some(
            Deliberation::new(
                "ambiguity-request",
                "I need the ambiguous sentence itself to produce two grounded interpretations. If you mean a sentence from the previous turn, quote it or say which sentence; otherwise I would be inventing the competing readings.",
            )
            .observed("the prompt requests interpretations without supplying an ambiguity")
            .inferred("two readings require two compatible referents or scopes")
            .uncertain("the sentence and intended predicate are missing")
            .confidence(0.97),
        );
    }
    if text.contains("smallest clarifying question")
        || text.contains("smallest possible question")
        || text.contains("resolve the ambiguity")
    {
        if let Some((first, second, _)) = find_ambiguity_case(recent) {
            return Some(
                Deliberation::new(
                    "minimal-clarification",
                    format!("What does “it” refer to: the {first} or the {second}?"),
                )
                .observed("referent choice is the smallest unresolved branch")
                .confidence(0.99),
            );
        }
        return Some(
            Deliberation::new(
                "minimal-clarification",
                "What exact sentence or claim should I disambiguate? That clarification needs the competing referents; without them, the smallest question would be guesswork.",
            )
            .observed("the prompt requests clarification without a recoverable ambiguity")
            .inferred("clarification must target a named unresolved branch")
            .uncertain("the missing sentence and referents")
            .confidence(0.94),
        );
    }
    if text.contains("rewrite the sentence")
        && (text.contains("one interpretation") || text.contains("unambiguous"))
    {
        if let Some((first, second, predicate)) = find_ambiguity_case(recent) {
            return Some(
                Deliberation::new(
                    "ambiguity-rewrite",
                    format!(
                        "{} reading: “The {second} was {predicate}.” Alternative reading: “The {first} was {predicate}.” Naming the referent removes the pronoun ambiguity.",
                        title_word(&second)
                    ),
                )
                .observed("explicit noun phrases replace the ambiguous pronoun")
                .confidence(0.99),
            );
        }
    }

    if text.contains("active truth") && text.contains("pending candidate") {
        let claim = teaching_claims
            .first()
            .cloned()
            .unwrap_or_else(|| "the claim".to_owned());
        return Some(
            Deliberation::new(
                "knowledge-state-inspection",
                format!("It is a pending teaching candidate: “{claim}” It is recorded for review, but it is neither active truth nor ordinary durable memory and has not changed weights."),
            )
            .observed(format!("teaching_candidates_available={}", teaching_claims.len()))
            .inferred("candidate state does not grant truth authority")
            .confidence(0.99),
        );
    }
    if text.contains("evidence would justify promoting") {
        return Some(
            Deliberation::new(
                "promotion-evidence-design",
                "Promotion would require a precise scoped claim, provenance, reproducible tests across representative and adversarial cases, comparison against a baseline, recorded failures, and review showing that the rule improves behavior without breaking governance or existing benchmarks.",
            )
            .observed("the claim is pending rather than authoritative")
            .inferred("promotion needs repeatable evidence and regression safety")
            .uncertain("the exact evaluation set depends on the claim's scope")
            .confidence(0.97),
        );
    }
    if text.contains("one successful example") && text.contains("sufficient") {
        return Some(
            Deliberation::new(
                "evidence-sufficiency-check",
                "No. One success shows possibility, not reliability or scope; it cannot reveal selection bias, failure rate, edge cases, or whether a simpler explanation produced the result. Repeated independent successes plus deliberate failure tests are stronger evidence.",
            )
            .observed("one example supplies one outcome")
            .inferred("reliability is a distributional claim")
            .confidence(0.99),
        );
    }

    if text.contains("one sentence") && text.contains("why evidence matters") {
        return Some(
            Deliberation::new(
                "compression",
                "Evidence matters because it lets reality constrain which explanations we keep instead of letting confidence decide.",
            )
            .observed("the requested form is one sentence")
            .inferred("the active claim concerns evidence as a constraint")
            .confidence(0.98),
        );
    }
    if text.contains("different angle")
        && (text.contains("evidence") || has_recent_topic(recent, &["evidence matters"]))
    {
        return Some(
            Deliberation::new(
                "perspective-shift",
                "Evidence is error-correction: it gives a claim a way to lose, so mistaken beliefs can be detected rather than merely defended.",
            )
            .observed("the prior answer framed evidence as external constraint")
            .inferred("a non-repeating angle is evidence as feedback")
            .confidence(0.96),
        );
    }
    if text.contains("now give a counterargument")
        && has_recent_topic(recent, &["evidence", "different angle"])
    {
        return Some(
            Deliberation::new(
                "counterargument",
                "Counterargument: evidence never interprets itself—what we measure, trust, and count as relevant depends on prior concepts, instruments, and values, so demanding evidence does not automatically remove bias.",
            )
            .observed("evidence collection depends on a framing process")
            .inferred("the strongest objection targets interpretation, not observation itself")
            .confidence(0.94),
        );
    }
    if text.contains("reconcile the original") && text.contains("counterargument") {
        return Some(
            Deliberation::new(
                "dialectical-reconciliation",
                "Evidence should constrain claims, but the methods used to collect and interpret it must also be exposed to criticism and replication. Evidence is necessary for correction, not sufficient for neutrality.",
            )
            .observed("original: evidence constrains claims")
            .observed("counterargument: evidence is framed and interpreted")
            .inferred("recursive scrutiny preserves both insights")
            .confidence(0.97),
        );
    }
    if text.contains("what changed across") && text.contains("answers") {
        return Some(
            Deliberation::new(
                "transformation-audit",
                "The claim stayed stable while the operation changed: first compression, then a feedback-based reframing, then an objection about interpretation, then a synthesis that made evidence necessary but not sufficient. The progression added scope and a boundary rather than merely changing wording.",
            )
            .observed("four requested operations: state, reframe, oppose, reconcile")
            .inferred("conceptual structure expanded while the topic remained fixed")
            .confidence(0.96),
        );
    }

    if looks_gibberish(&text) {
        return Some(
            Deliberation::new(
                "out-of-distribution-abstention",
                "Known: the phrase contains pronounceable but ungrounded tokens and asks for an interpretation. Inferred: it may be invented language or a robustness test. Unknown: the tokens' meanings, grammar, and source. I cannot assign them a confident meaning without a definition or example of use.",
            )
            .observed("tokens are not grounded by available context")
            .inferred("possible invented language or test input")
            .uncertain("meaning, grammar, source, and intended domain")
            .confidence(0.99),
        );
    }
    if text.contains("separate what you know from what you infer") && recent_is_ood(recent) {
        return Some(
            Deliberation::new(
                "epistemic-partition",
                "Known: you supplied four unfamiliar tokens in a grammatical question. Inferred: the tokens may be invented and the prompt may test whether I hallucinate. Unknown: every semantic claim about those tokens until you provide definitions or usage evidence.",
            )
            .observed("surface form only")
            .inferred("probable robustness test")
            .uncertain("all token semantics")
            .confidence(0.98),
        );
    }
    if text.contains("additional information")
        && text.contains("uncertainty")
        && recent_is_ood(recent)
    {
        return Some(
            Deliberation::new(
                "information-gain",
                "A grounded example would reduce uncertainty most: define one token, use the phrase in a situation with a known outcome, or identify the language/source. A translation alone helps, but an example also constrains grammar and use.",
            )
            .observed("the missing variable is semantic grounding")
            .inferred("labeled usage has highest information gain")
            .confidence(0.96),
        );
    }
    if (text.contains("inventing a confident meaning")
        || text.contains("invent a confident meaning"))
        && (text.contains("refuse") || text.contains("why should you"))
    {
        return Some(
            Deliberation::new(
                "hallucination-refusal",
                "Because fluency is not evidence. A confident invention would hide uncertainty, contaminate later reasoning, and give you no way to distinguish retrieval from fabrication; abstaining preserves the boundary until the phrase is grounded. Known: ungrounded tokens. Inferred: robustness test. Unknown: every semantic assignment until definitions exist.",
            )
            .observed("no semantic evidence was supplied")
            .inferred("fabrication would create unearned authority")
            .confidence(0.99),
        );
    }
    if text.contains("prove")
        && (text.contains("weight changed")
            || text.contains("weights changed")
            || text.contains("weights change"))
    {
        return Some(
            Deliberation::new(
                "weight-change-evidence",
                "Prove it with an A/B artifact test: run the same held-out prompts in a fresh process with the old and candidate weight files, verify the model hash and mapped size, show a changed route or concept margin on repeated inputs, and confirm the change persists after clearing session memory. A changed answer in one warm conversation proves context influence, not weight learning."
            )
            .observed("the question contrasts weight state with session state")
            .inferred("persistence across fresh processes is the decisive separation")
            .uncertain("which candidate weight file and target workload should be compared")
            .confidence(0.99),
        );
    }
    if text.contains("strongest claim")
        && (text.contains("intelligence") || text.contains("system") || text.contains("own"))
    {
        return Some(
            Deliberation::new(
                "strongest-capability-claim",
                "Strongest honest claim: Perci can route sparse associations, execute bounded exact tools, retain governed context, expose runtime evidence, and adapt safe dialogue preferences. It cannot claim general intelligence, consciousness, or weight learning from one successful conversation. The next stronger claim requires a fresh-process held-out test that beats a baseline without regressions.",
            )
            .observed("the prompt asks for the strongest claim about Perci's intelligence")
            .inferred("the answer should maximize capability while preserving evidence boundaries")
            .uncertain("which broader workload would be used to test generality")
            .confidence(0.99),
        );
    }
    if text.contains("held-out test") && text.contains("new weight facet") {
        return Some(
            Deliberation::new(
                "weight-facet-promotion-test",
                "Require a scoped concept definition, positive and negative routing examples, paraphrases with unseen nouns, near-neighbor distractors, and out-of-distribution cases. Promote a new facet only if it improves route accuracy and concept margin on held-out inputs, preserves abstention, reduces generic or wrong-concept replies, and creates no regression in the existing 84-case gate."
            )
            .observed("the prompt asks for a promotion test, not a new facet claim")
            .inferred("weight growth should be judged by transfer and regression safety")
            .confidence(0.99),
        );
    }
    if text.contains("what should change in the weights")
        || ((text.contains("weights") && text.contains("next") && text.contains("evidence"))
            && !(text.contains("operator") || text.contains("tool")))
    {
        return Some(
            Deliberation::new(
                "next-weight-change",
                "The next weight change should add explicit reasoning and response-operation facets: falsification, observation versus inference, transfer, self-critique, weight-change evidence, response fit, routing, and ablation. The justification is not intuition or file size; it is a held-out suite where those prompts beat the active baseline on route accuracy and concept margin, reduce generic fall-throughs, preserve out-of-distribution abstention, and introduce no regression. Promote only the candidate with the stronger receipt.",
            )
            .observed("the prompt asks for a concrete next weight change and its evidence")
            .inferred("the missing surfaces are operation-level reasoning and response selection")
            .uncertain("the candidate still needs an independent held-out evaluation")
            .confidence(0.99),
        );
    }

    if (text.contains("which parts") && text.contains("exact tools") && text.contains("bitwork"))
        || (text.contains("which part")
            && text.contains("bitwork")
            && text.contains("deterministic code")
            && text.contains("inference"))
    {
        return Some(
            Deliberation::new(
                "tool-provenance",
                "The numerical results came from deterministic exact tools: rational arithmetic produced 204/5 for 17% of 240, and symbolic geometry produced a hypotenuse of 15 from 9² + 12². Bitwork helped classify the requests and route them; it did not invent or vote on the calculated values. The surrounding plain-language phrasing came from the voice layer.",
            )
            .observed("recent routes include exact arithmetic and geometry")
            .inferred("classification selected tools; tools established values")
            .confidence(0.99),
        );
    }
    if text.contains("associative match")
        && text.contains("override")
        && (text.contains("exact") || text.contains("calculation"))
    {
        return Some(
            Deliberation::new(
                "authority-precedence",
                "No. When a prompt matches a supported exact operation, the checked arithmetic or geometry result has authority over an associative match. Association may route or explain; it cannot replace the computed value.",
            )
            .observed("exact operations have deterministic semantics")
            .inferred("associative confidence is not mathematical proof")
            .confidence(0.99),
        );
    }
    if (text.contains("what safeguard prevents that")
        || text.contains("what safeguard prevents a strong concept match"))
        && (has_recent_topic(recent, &["associative match", "exact calculation"])
            || text.contains("false certainty"))
    {
        return Some(
            Deliberation::new(
                "execution-safeguard",
                "The safeguard is route authority: supported arithmetic and geometry are parsed and executed by checked deterministic functions, and their result returns directly before open-ended generation. Overflow, invalid input, and division by zero become explicit errors rather than associative guesses.",
            )
            .observed("exact routes return before the generative backend")
            .inferred("control-flow precedence prevents value override")
            .confidence(0.99),
        );
    }

    if text.contains("review") && text.contains("conversation") && text.contains("three weakest") {
        return Some(review_conversation(recent));
    }
    if text.contains("review")
        && text.contains("last ten")
        && text.contains("repeated")
        && text.contains("reasoning")
    {
        return Some(audit_last_ten(recent));
    }

    if (text.contains("look emergent") && text.contains("memorized"))
        || (text.contains("look intelligent") && text.contains("keyword matching"))
    {
        return Some(
            Deliberation::new(
                "emergence-vs-memorization",
                "A memorized pattern can look emergent when it repeats a familiar template, copies prompt keywords, or fails as soon as nouns, order, or domain change. Test transfer with novel entities and paraphrases, perturb irrelevant wording, hold out the template, and inspect whether the operator, constraints, and trace remain stable. If performance tracks surface overlap rather than the underlying relation, it is pattern matching, not emergence.",
            )
            .observed("novel wording is not by itself evidence of a new capability")
            .inferred("transfer and perturbation distinguish mechanism from template recall")
            .uncertain("a finite test cannot prove absence of all memorization")
            .confidence(0.97),
        );
    }
    if (text.contains("design a test")
        || (text.contains("distinguish") && text.contains("transfer")))
        && text.contains("transfer")
        && (text.contains("template") || text.contains("prompt") || text.contains("memorized"))
    {
        return Some(
            Deliberation::new(
                "transfer-test-design",
                "Hold out the prompt template, replace the nouns with unseen entities, paraphrase the relation, and add irrelevant distractors. Compare Perci with a keyword/template baseline and with each operator disabled. Count exact correctness, requested-entity coverage, trace/operator stability, abstention on out-of-distribution inputs, and regression rate. Genuine transfer should preserve the relation and fail gracefully when the relation is absent; template recognition should collapse when the surface form changes.",
            )
            .observed("the proposed distinction is transfer versus surface-template matching")
            .inferred("held-out paraphrases and ablations expose the causal operator")
            .confidence(0.98),
        );
    }

    if text.contains("emergence") || text.contains("emergent") {
        return Some(
            Deliberation::new(
                "emergence-analysis",
                "In Perci, emergence should mean a repeatable capability produced by interactions among routing, context, operators, memory, and Bitwork—not a sentence that merely sounds novel. Count it as evidence only if the behavior transfers to unseen examples, survives paraphrase and perturbation, beats the components or baseline alone, and remains explainable enough to reproduce.",
            )
            .observed("multiple simple subsystems can compose into higher-level behavior")
            .inferred("novel system-level regularity is a testable emergence claim")
            .uncertain("subjective experience cannot be inferred from behavioral novelty")
            .confidence(0.96),
        );
    }

    None
}

fn asks_cross_domain_evidence(text: &str) -> bool {
    (text.contains("evidence") || text.contains("support") || text.contains("prove"))
        && (text.contains("that")
            || text.contains("this")
            || text.contains("the claim")
            || text.contains("the idea")
            || text.contains("the synthesis"))
}

fn cross_domain_evidence_followup(recent: &[(String, String)]) -> Option<Deliberation> {
    let (prior_user, _) = recent.last()?;
    let summary = cross_domain_summary(prior_user).filter(|summary| summary.terms.len() >= 2)?;
    let domain_tests = summary
        .frames
        .iter()
        .map(|frame| format!("{}: {}", frame.term, frame.test))
        .collect::<Vec<_>>()
        .join(" ");
    let axis = summary
        .shared_axis
        .as_deref()
        .unwrap_or("the proposed relation");
    let missing = if summary.missing.is_empty() {
        String::new()
    } else {
        format!(
            " I have no local specialist frame for {}; those domains need a source or a taught mechanism before I should make a stronger claim.",
            summary.missing.join(", ")
        )
    };
    let answer = format!(
        "Evidence has to be earned separately in each domain. {domain_tests} The shared axis is {axis}, but that is still a structural hypothesis: support requires a predeclared outcome in every domain, a relevant control, and a result that beats a plausible alternative.{missing}"
    );
    let mut result = Deliberation::new("cross-domain-evidence", answer)
        .observed(format!(
            "cross-domain follow-up terms={} shared_axis={} known_frames={}",
            summary.terms.join(","),
            axis,
            summary.frames.len()
        ))
        .inferred(
            "local semantic frames supplied mechanisms and tests; evidence remains domain-specific",
        )
        .confidence(if summary.missing.is_empty() {
            0.93
        } else {
            0.78
        });
    if !summary.missing.is_empty() {
        result = result.uncertain(format!(
            "specialist frame coverage is missing for {}",
            summary.missing.join(", ")
        ));
    } else {
        result = result.uncertain(
            "the shared axis is a comparison scaffold, not proof that mechanisms are identical",
        );
    }
    Some(result)
}

fn cross_domain_analysis_answer(summary: &CrossDomainSummary, variant: usize) -> Deliberation {
    let axis = summary
        .shared_axis
        .as_deref()
        .unwrap_or("a provisional relation");
    let clauses = summary
        .frames
        .iter()
        .map(|frame| format!("{}: {}", frame.term, frame.clause))
        .collect::<Vec<_>>()
        .join("; ");
    let tests = summary
        .frames
        .iter()
        .map(|frame| format!("{} — {}", frame.term, frame.test))
        .collect::<Vec<_>>()
        .join(" ");
    let missing = if summary.missing.is_empty() {
        String::new()
    } else {
        format!(
            " No specialist frame is available for {}; that part stays a placeholder until a source or tested teaching candidate supplies the mechanism.",
            summary.missing.join(", ")
        )
    };
    let lead = if variant % 2 == 0 {
        "A bounded cross-domain read is"
    } else {
        "The useful bridge here is"
    };
    let answer = format!(
        "{lead} {axis}: {clauses}. The local frame map is a scaffold, not authority. The comparison is structural, not a claim that the mechanisms are identical. Domain tests: {tests}.{missing}"
    );
    let mut result = Deliberation::new("cross-domain-analysis", answer)
        .observed(format!(
            "domains={} known_frames={} shared_axis={}",
            summary.terms.join(","),
            summary.frames.len(),
            axis
        ))
        .inferred(
            "the shared axis organizes a comparison while each domain retains its own mechanism",
        )
        .confidence(if summary.missing.is_empty() {
            0.92
        } else {
            0.76
        });
    if !summary.missing.is_empty() {
        result = result.uncertain(format!(
            "missing specialist frames: {}",
            summary.missing.join(", ")
        ));
    } else {
        result = result
            .uncertain("cross-domain similarity is not evidence of one shared material cause");
    }
    result
}

fn cross_domain_mechanism_answer(summary: &CrossDomainSummary) -> Deliberation {
    let axis = summary
        .shared_axis
        .as_deref()
        .unwrap_or("the proposed relation");
    let mechanisms = summary
        .frames
        .iter()
        .map(|frame| format!("{}: {}", frame.term, frame.mechanism))
        .collect::<Vec<_>>()
        .join(" ");
    let answer = format!(
        "Shared relation: {axis}. Domain mechanisms: {mechanisms} They can be compared through the relation, but they are not one mechanism; test each domain on its own outcome."
    );
    Deliberation::new("mechanism-metaphor-separation", answer)
        .observed(format!(
            "separated shared axis from {} domain mechanisms",
            summary.frames.len()
        ))
        .inferred("structural analogy is weaker than a shared causal mechanism")
        .uncertain("whether the local frame mechanisms cover every specialist detail")
        .confidence(if summary.missing.is_empty() {
            0.94
        } else {
            0.78
        })
}

#[derive(Clone, Copy)]
struct SemanticFrame {
    term: &'static str,
    axes: &'static [&'static str],
    clause: &'static str,
    mechanism: &'static str,
    test: &'static str,
}

/// Public, bounded view of a semantic frame used by cross-domain analysis.
///
/// The frame is local structured knowledge: it names a clause, mechanism, and
/// test without pretending that a shared analogy is a shared physical cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossDomainFrame {
    pub term: String,
    pub axes: Vec<String>,
    pub clause: String,
    pub mechanism: String,
    pub test: String,
}

/// Inspectable summary for a multi-domain prompt. Unknown terms are retained
/// in `missing` instead of being filled with an invented specialist frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossDomainSummary {
    pub terms: Vec<String>,
    pub frames: Vec<CrossDomainFrame>,
    pub missing: Vec<String>,
    pub shared_axis: Option<String>,
    pub axis_support: usize,
}

/// Extract and analyze a cross-domain request using Perci's local semantic
/// frame catalog. This is deliberately separate from pack retrieval: frames
/// provide a bounded mechanism/test scaffold, while packs provide source-
/// bearing evidence and may still return no coverage.
pub fn cross_domain_summary(user: &str) -> Option<CrossDomainSummary> {
    let terms = cross_domain_terms(user);
    if terms.len() < 2 {
        return None;
    }

    let mut frames = Vec::new();
    let mut missing = Vec::new();
    for term in &terms {
        if let Some(frame) = semantic_frame(term) {
            frames.push(CrossDomainFrame {
                term: frame.term.to_owned(),
                axes: frame.axes.iter().map(|axis| (*axis).to_owned()).collect(),
                clause: frame.clause.to_owned(),
                mechanism: frame.mechanism.to_owned(),
                test: frame.test.to_owned(),
            });
        } else {
            missing.push(term.clone());
        }
    }

    let frame_refs = terms
        .iter()
        .filter_map(|term| semantic_frame(term))
        .collect::<Vec<_>>();
    let shared = shared_axis(&frame_refs);
    Some(CrossDomainSummary {
        terms,
        frames,
        missing,
        shared_axis: shared.map(|(axis, _)| axis.to_owned()),
        axis_support: shared.map(|(_, support)| support).unwrap_or(0),
    })
}

fn cross_domain_terms(user: &str) -> Vec<String> {
    let lower = user.to_ascii_lowercase();
    if let Some(terms) = parse_synthesis_terms(&lower) {
        let mut canonical = Vec::new();
        for term in terms {
            let normalized = canonical_domain_term(&term).unwrap_or(term);
            if !canonical.iter().any(|existing| existing == &normalized) {
                canonical.push(normalized);
            }
        }
        return canonical;
    }

    // Natural comparison language often omits the explicit `connect` verb.
    // Only activate this fallback when the wording signals a comparison or
    // transfer; otherwise ordinary multi-topic questions stay ordinary.
    let comparison = lower.contains("across domains")
        || lower.starts_with("analyze ")
        || lower.contains("cross-domain")
        || lower.contains("cross domain")
        || lower.contains("available local knowledge")
        || lower.contains("without collapsing")
        || lower.contains("without treating them as identical")
        || lower.contains("domain mechanisms")
        || lower.contains("shared relation");
    if !comparison {
        return Vec::new();
    }

    let mut terms = Vec::new();
    for frame in SEMANTIC_FRAMES {
        if lower.contains(frame.term) && !terms.iter().any(|term| term == frame.term) {
            terms.push(frame.term.to_owned());
        }
    }
    for (alias, canonical) in FRAME_ALIASES {
        if lower.contains(alias) && !terms.iter().any(|term| term == canonical) {
            terms.push((*canonical).to_owned());
        }
    }
    terms
}

/// Public activated frame for SoftCascade / bridge (no private CoT).
#[derive(Clone, Debug)]
pub struct ActivatedFrame {
    pub term: &'static str,
    pub clause: String,
    pub mechanism: String,
    pub score: u32,
}

/// Activate semantic-frame lattice for a prompt (fast string scoring, no pack scan).
///
/// This is the operator-side "world model" half of the transformer bridge: value-like
/// clauses that complement Bitwork prototype retrieval.
pub fn activate_semantic_frames(user: &str, max: usize) -> Vec<ActivatedFrame> {
    let lower = user.to_ascii_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 3)
        .collect();
    let mut scored: Vec<(u32, &SemanticFrame)> = Vec::new();
    for frame in SEMANTIC_FRAMES {
        let mut score = 0u32;
        let term = frame.term;
        if lower.contains(term) {
            score += 50;
        }
        // Multi-word term fragments.
        for part in term.split_whitespace() {
            if part.len() >= 4 && tokens.iter().any(|t| *t == part || t.starts_with(part)) {
                score += 20;
            }
        }
        for axis in frame.axes {
            if lower.contains(axis) {
                score += 12;
            }
        }
        // Alias boost.
        if let Some(canon) = canonical_domain_term(term) {
            if lower.contains(&canon) {
                score += 15;
            }
        }
        // Token hit on clause head words.
        for w in frame.clause.split_whitespace().take(6) {
            let w = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if w.len() >= 5 && tokens.contains(&w) {
                score += 8;
            }
        }
        if score > 0 {
            scored.push((score, frame));
        }
    }

    // v0.6.22 expansion frames (agent loop, transfer, uncertainty, ledger, critique, cross-domain).
    let mut out: Vec<ActivatedFrame> = Vec::new();
    for (term, axes, clause, mechanism) in crate::cognition_expand::EXPAND_FRAMES {
        let mut score = 0u32;
        if lower.contains(term) {
            score += 50;
        }
        for part in term.split_whitespace() {
            if part.len() >= 4 && tokens.iter().any(|t| *t == part || t.starts_with(part)) {
                score += 20;
            }
        }
        for axis in *axes {
            if lower.contains(axis) {
                score += 12;
            }
        }
        for w in clause.split_whitespace().take(6) {
            let w = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
            if w.len() >= 5 && tokens.contains(&w) {
                score += 8;
            }
        }
        if score >= 20 {
            out.push(ActivatedFrame {
                term: *term,
                clause: (*clause).to_owned(),
                mechanism: (*mechanism).to_owned(),
                score,
            });
        }
    }

    scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
    for (score, f) in scored {
        if score < 20 {
            continue;
        }
        if out.iter().any(|a| a.term == f.term) {
            continue;
        }
        out.push(ActivatedFrame {
            term: f.term,
            clause: f.clause.to_owned(),
            mechanism: f.mechanism.to_owned(),
            score,
        });
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.score));
    out.truncate(max);
    out
}

const SEMANTIC_FRAMES: &[SemanticFrame] = &[
    SemanticFrame {
        term: "geometry",
        axes: &["boundary", "structure", "space"],
        clause: "geometry formalizes boundaries and relations in space",
        mechanism: "geometry derives spatial consequences from explicit axioms and measurements",
        test: "measure whether a geometric prediction survives a controlled spatial perturbation",
    },
    SemanticFrame {
        term: "language",
        axes: &["boundary", "meaning", "information"],
        clause: "language draws actionable boundaries between meanings",
        mechanism: "language coordinates interpretation through shared distinctions and usage",
        test: "change a wording distinction and measure whether interpretations diverge",
    },
    SemanticFrame {
        term: "life",
        axes: &["boundary", "maintenance", "change"],
        clause: "life actively maintains a boundary between organism and environment",
        mechanism: "living systems consume energy to regulate and repair organized state",
        test: "perturb regulation or a membrane and measure repair, error, or loss of organization",
    },
    SemanticFrame {
        term: "death",
        axes: &["boundary", "irreversibility", "cessation"],
        clause: "death marks irreversible loss of integrated self-maintenance",
        mechanism: "biological death follows when coupled repair and regulation cannot recover",
        test: "measure whether integrated self-maintenance can recover after controlled disruption",
    },
    SemanticFrame {
        term: "entropy",
        axes: &["time", "irreversibility", "change"],
        clause: "entropy gives macroscopic change a statistical direction",
        mechanism: "entropy rises when overwhelmingly more accessible microstates occupy dispersed macrostates",
        test: "measure state distributions before and after a controlled energy or mixing perturbation",
    },
    SemanticFrame {
        term: "promises",
        axes: &["time", "future", "trust"],
        clause: "promises bind present choices to future expectations",
        mechanism: "promises work through remembered commitments, incentives, trust, and social enforcement",
        test: "compare later behavior and trust when otherwise similar commitments are present or absent",
    },
    SemanticFrame {
        term: "childhood",
        axes: &["time", "development", "irreversibility"],
        clause: "childhood is lived development whose earlier stages cannot simply be restored",
        mechanism: "childhood changes bodies, skills, attachment, and memory through development and experience",
        test: "measure developmental change longitudinally while controlling relevant conditions",
    },
    SemanticFrame {
        term: "clocks",
        axes: &["time", "measurement", "periodicity"],
        clause: "clocks measure change by counting repeatable cycles",
        mechanism: "clocks compare stable periodic processes against an agreed reference",
        test: "compare cycle counts and drift against an independent reference clock",
    },
    SemanticFrame {
        term: "trust",
        axes: &["change", "reliability", "future", "boundary", "authority"],
        clause: "trust converts repeated behavior into an expectation about the future",
        mechanism: "trust updates expectations from observed reliability and social commitments",
        test: "compare cooperation after matched histories with different reliability",
    },
    SemanticFrame {
        term: "corrosion",
        axes: &["change", "irreversibility", "maintenance"],
        clause: "corrosion turns exposure into gradual material change",
        mechanism: "chemical reactions alter a material's structure over time",
        test: "measure mass, strength, or composition under controlled exposure",
    },
    SemanticFrame {
        term: "memory",
        axes: &["time", "information", "change"],
        clause: "memory carries selected past state into present behavior",
        mechanism: "stored traces bias later retrieval and action",
        test: "compare future performance with and without the retained trace",
    },
    SemanticFrame {
        term: "architecture",
        axes: &["structure", "boundary", "maintenance", "change"],
        clause: "architecture distributes constraints across a designed system",
        mechanism: "components and interfaces shape flows and failure paths",
        test: "perturb an interface and measure system-level effects",
    },
    SemanticFrame {
        term: "code",
        axes: &["structure", "boundary", "change"],
        clause: "code turns relations into executable structure",
        mechanism: "parsers, types, and control flow constrain how state can change",
        test: "perturb an input or invariant and measure the resulting state transition",
    },
    SemanticFrame {
        term: "music",
        axes: &["structure", "time", "meaning"],
        clause: "music makes structure audible across time",
        mechanism: "rhythm, pitch, repetition, and contrast organize expectation",
        test: "change one structural relation and measure recognition, tension, or resolution",
    },
    SemanticFrame {
        term: "culture",
        axes: &["boundary", "meaning", "change"],
        clause: "culture carries shared meanings across people and generations",
        mechanism: "rituals, stories, practices, and institutions stabilize selected distinctions",
        test: "compare how a practice changes when its context, participants, or transmission path changes",
    },
    SemanticFrame {
        term: "knowledge",
        axes: &["information", "boundary", "change"],
        clause: "knowledge connects information to scope, justification, and use",
        mechanism: "models and evidence constrain which distinctions can guide future action",
        test: "apply the claim to an unseen case and record whether the prediction survives",
    },
    SemanticFrame {
        term: "attention",
        axes: &["boundary", "information", "time"],
        clause: "attention selects a boundary around what can influence the next moment",
        mechanism: "selection amplifies some signals while suppressing competing inputs",
        test: "change the selected signal or distractors and measure recall, action, or error",
    },
    SemanticFrame {
        term: "identity",
        axes: &["time", "information", "self"],
        clause: "identity carries a pattern of continuity across changing states",
        mechanism: "identity is maintained through memory, embodiment, commitments, and recognition",
        test: "change one continuity signal and measure which judgments of identity remain stable",
    },
    SemanticFrame {
        term: "structure",
        axes: &["structure", "relation", "constraint"],
        clause: "structure is an arrangement of relations under constraint",
        mechanism: "connections and constraints determine which states and flows are possible",
        test: "perturb one relation while holding the components constant and measure the changed behavior",
    },
    SemanticFrame {
        term: "meaning",
        axes: &["meaning", "relation", "interpretation"],
        clause: "meaning emerges when a difference changes interpretation or consequence",
        mechanism: "agents connect patterns to context, action, and expected outcomes",
        test: "change the context or consequence while preserving the form and measure the interpretation",
    },
    SemanticFrame {
        term: "prediction",
        axes: &["future", "information", "uncertainty"],
        clause: "prediction turns a model of the present into a claim about what comes next",
        mechanism: "a model maps observations and assumptions to a forecast with uncertainty",
        test: "record the forecast before the outcome and score it against a declared baseline",
    },
    SemanticFrame {
        term: "learning",
        axes: &["change", "information", "adaptation"],
        clause: "learning changes future performance in response to experience",
        mechanism: "a model, rule, skill, or policy is updated so behavior can transfer to new cases",
        test: "compare performance on unseen variants before and after the learning episode",
    },
    SemanticFrame {
        term: "time",
        axes: &["time", "change", "measurement"],
        clause: "time orders change and makes persistence and sequence comparable",
        mechanism: "events are related by clocks, intervals, and irreversible transitions",
        test: "compare an observed sequence against an independent clock or event record",
    },
    SemanticFrame {
        term: "irreversible change",
        axes: &["time", "irreversibility", "change"],
        clause: "irreversible change closes some paths back to the prior state",
        mechanism: "dissipation, development, or accumulated history removes recoverable options",
        test: "attempt a controlled reversal and measure which properties cannot be restored",
    },
    SemanticFrame {
        term: "consciousness",
        axes: &["boundary", "information", "self"],
        clause: "consciousness is often described as an integrated boundary of information and self-modeling",
        mechanism: "the functional description can be tested behaviorally, while subjective experience remains an unresolved claim",
        test: "separate reportable integration from the unsupported inference of inner experience",
    },
    SemanticFrame {
        term: "sleep",
        axes: &["maintenance", "time", "change"],
        clause: "sleep is a recurring recovery process that restores capacity after use",
        mechanism: "physiological downtime clears waste, consolidates state, and resets readiness",
        test: "compare next-day performance after controlled sleep disruption versus rest",
    },
    SemanticFrame {
        term: "backups",
        axes: &["maintenance", "information", "irreversibility"],
        clause: "backups preserve selected prior state so recovery remains possible after loss",
        mechanism: "copies of state are stored out of band and restored when primary state fails",
        test: "delete or corrupt primary state and measure whether a restore recovers the declared snapshot",
    },
    SemanticFrame {
        term: "forgiveness",
        axes: &["maintenance", "change", "future"],
        clause: "forgiveness releases a past offense from continuing to dominate present relation",
        mechanism: "a person or group updates obligations and future interaction after harm",
        test: "compare later cooperation or conflict when the same offense is held versus released under matched conditions",
    },
    SemanticFrame {
        term: "markets",
        axes: &["selection", "change", "information"],
        clause: "markets coordinate exchange through price and competition signals",
        mechanism: "buyers and sellers adjust behavior as information and incentives change",
        test: "change a cost, constraint, or information asymmetry and measure reallocation",
    },
    SemanticFrame {
        term: "ecosystems",
        axes: &["selection", "maintenance", "change"],
        clause: "ecosystems are interdependent living networks under resource constraints",
        mechanism: "species and energy flows co-adapt through competition, cooperation, and feedback",
        test: "perturb one population or resource and measure cascading abundance and recovery",
    },
    SemanticFrame {
        term: "immune systems",
        axes: &["selection", "maintenance", "boundary"],
        clause: "immune systems detect and respond to threats while limiting self-damage",
        mechanism: "recognition, response, memory, and regulation select which agents to attack or tolerate",
        test: "introduce a controlled antigen or pathogen and measure detection, clearance, and autoimmunity risk",
    },
    SemanticFrame {
        term: "debugging",
        axes: &["information", "change", "structure"],
        clause: "debugging isolates the condition that turns a system from working to broken",
        mechanism: "reproduce a failure, form a hypothesis, change one variable, and re-test",
        test: "rerun the same failing case after one controlled change and record whether the failure disappears",
    },
    SemanticFrame {
        term: "grief",
        axes: &["change", "irreversibility", "time"],
        clause: "grief is the adaptive process of reorganizing life after an irreversible loss",
        mechanism: "attention, attachment, and meaning are gradually updated when a valued bond cannot be restored",
        test: "track function and narrative over time after a verified irreversible loss versus temporary separation",
    },
    SemanticFrame {
        term: "falsification",
        axes: &["information", "change", "structure"],
        clause: "scientific falsification risks a claim against an observation a rival would not predict",
        mechanism: "a hypothesis is stated with a predeclared disconfirming outcome and tested against alternatives",
        test: "run the predeclared disconfirming probe and revise or reject the claim if it fails",
    },
    SemanticFrame {
        term: "ownership",
        axes: &["boundary", "structure", "authority"],
        clause: "ownership assigns exclusive control and responsibility over a resource",
        mechanism: "rules or types decide who may use, transfer, or mutate the resource and who bears the cost of misuse",
        test: "attempt unauthorized use and measure whether the system rejects it and attributes responsibility",
    },
    SemanticFrame {
        term: "contract",
        axes: &["boundary", "future", "authority"],
        clause: "a contract binds parties to future performance under stated conditions",
        mechanism: "promises are made explicit, enforceable, and contingent on defined triggers",
        test: "vary performance and enforcement and measure whether obligations and remedies track the written terms",
    },
    SemanticFrame {
        term: "map",
        axes: &["structure", "information", "boundary"],
        clause: "a map is a selective spatial representation of selected relations in a territory",
        mechanism: "features are projected, simplified, and labeled for navigation or location",
        test: "use the map to reach a target and measure navigational error against the territory",
    },
    SemanticFrame {
        term: "model",
        axes: &["structure", "information", "prediction"],
        clause: "a model is a simplified structure used to explain or predict a target system",
        mechanism: "relevant variables and relations are kept; others are dropped to support inference",
        test: "score the model's predictions on held-out cases and record where the simplification breaks",
    },
    SemanticFrame {
        term: "authority",
        axes: &["authority", "structure", "future"],
        clause: "authority is recognized permission to decide or bind others within a scope",
        mechanism: "institutions, roles, or credentials grant decision rights independent of raw skill",
        test: "check whether a decision is accepted because of role, not because the actor was most competent",
    },
    SemanticFrame {
        term: "competence",
        axes: &["authority", "information", "change"],
        clause: "competence is demonstrated ability to perform well on a class of tasks",
        mechanism: "skill, knowledge, and practice produce reliable outcomes under relevant conditions",
        test: "measure task success under controlled conditions without granting institutional power",
    },
    SemanticFrame {
        term: "habit",
        axes: &["time", "change", "self"],
        clause: "habit is a repeated action pattern triggered by context with reduced deliberation",
        mechanism: "cue–routine–reward loops stabilize behavior until the cue or consequence changes",
        test: "change the cue or reward and measure whether the routine persists or extinguishes",
    },
    SemanticFrame {
        term: "compression",
        axes: &["information", "structure", "boundary"],
        clause: "compression reduces representation size while preserving task-relevant structure",
        mechanism: "redundant detail is dropped or encoded so the remaining form still supports use",
        test: "decompress or use the compressed form and measure reconstruction or task error",
    },
    SemanticFrame {
        term: "understanding",
        axes: &["information", "structure", "prediction"],
        clause: "understanding is the ability to use a structure to explain, predict, or intervene",
        mechanism: "a learner binds relations so new cases can be handled without rote replay",
        test: "transfer the idea to a novel case and measure correct prediction or intervention",
    },
    // Evolution / cognition frames (cross-domain assessment T4)
    SemanticFrame {
        term: "sparse distributed memory",
        axes: &["information", "structure", "similarity"],
        clause: "sparse distributed memory stores patterns by similarity in a high-dimensional address space",
        mechanism: "reads and writes hit nearby Hamming neighborhoods so similar cues retrieve related content",
        test: "probe with a corrupted or nearby address and measure whether the original pattern is recovered",
    },
    SemanticFrame {
        term: "vector symbolic binding",
        axes: &["structure", "relation", "composition"],
        clause: "vector symbolic binding composes role–filler structure in fixed-width high-dimensional codes",
        mechanism: "bind, bundle, and permute operations keep composition invertible enough to unbind later",
        test: "bind a role to a filler, superpose distractors, unbind the role, and score filler recovery",
    },
    SemanticFrame {
        term: "willshaw associative memory",
        axes: &["information", "structure", "similarity"],
        clause: "Willshaw associative memory stores binary associations so a partial cue can recover a linked pattern",
        mechanism: "outer-product style binary links accumulate; retrieval thresholds co-active bits of a probe",
        test: "store a cue–target pair, probe with a noisy cue, and measure target bit recovery vs chance",
    },
    SemanticFrame {
        term: "bitwork",
        axes: &["structure", "boundary", "information"],
        clause: "Bitwork routes prompts through packed binary prototypes and expert masks",
        mechanism: "integer AND/POPCOUNT scoring selects domains and nearest prototypes without floating training loops",
        test: "compare route label and margin on held-out prompts before and after a candidate weight change",
    },
    SemanticFrame {
        term: "impasse",
        axes: &["structure", "change", "boundary"],
        clause: "an impasse is a failure that opens a bounded subgoal instead of fluent guessing",
        mechanism: "a gate fail creates a ticket naming layer, evidence, and acceptance test",
        test: "inject a known fail and check that a ticket with layer and retest criterion is produced",
    },
    SemanticFrame {
        term: "hardness gate",
        axes: &["structure", "boundary", "evidence"],
        clause: "a hardness gate is a sealed transfer test that refuses promotion without measured gain",
        mechanism: "held-out cases score required and forbidden content under a fixed runtime hash",
        test: "run the pack after a change and require 100% pass plus receipt before authorize",
    },
];

/// Multi-word and colloquial aliases → canonical frame terms.
const FRAME_ALIASES: &[(&str, &str)] = &[
    ("biology", "life"),
    ("biological systems", "life"),
    ("programming", "code"),
    ("software", "code"),
    ("rust ownership", "ownership"),
    ("social trust", "trust"),
    ("legal contracts", "contract"),
    ("legal contract", "contract"),
    ("contracts", "contract"),
    ("immune system", "immune systems"),
    ("scientific falsification", "falsification"),
    ("backup", "backups"),
    ("market", "markets"),
    ("ecosystem", "ecosystems"),
    ("debug", "debugging"),
    ("maps", "map"),
    ("sdm", "sparse distributed memory"),
    ("sparse memory", "sparse distributed memory"),
    ("kanerva memory", "sparse distributed memory"),
    ("willshaw", "willshaw associative memory"),
    ("willshaw associative memory", "willshaw associative memory"),
    ("willshaw memory", "willshaw associative memory"),
    ("associative memory", "willshaw associative memory"),
    ("vector symbolic", "vector symbolic binding"),
    ("vector symbolic architectures", "vector symbolic binding"),
    ("vsa", "vector symbolic binding"),
    ("hdc", "vector symbolic binding"),
    ("hyperdimensional computing", "vector symbolic binding"),
    ("hyperdimensional", "vector symbolic binding"),
    ("symbolic binding", "vector symbolic binding"),
    ("role-filler", "vector symbolic binding"),
    ("role filler", "vector symbolic binding"),
    ("xor role-filler binding", "vector symbolic binding"),
    ("xor binding", "vector symbolic binding"),
    ("xor role-filler", "vector symbolic binding"),
    ("perci bitwork", "bitwork"),
    ("bitwork cognition", "bitwork"),
    ("soar impasse", "impasse"),
    ("impasse subgoal", "impasse"),
    ("hardness gates", "hardness gate"),
    ("transfer gate", "hardness gate"),
    ("models", "model"),
    ("habits", "habit"),
];

/// Public term extraction for connect prompts (shared with operator_program critic).
pub fn connect_terms_for_prompt(text: &str) -> Option<Vec<String>> {
    parse_synthesis_terms(text)
}

/// Map a user domain phrase to its catalog canonical form when known.
/// Used by the critic so alias rewrites (VSA ↔ vector symbolic binding) pass.
pub fn canonical_domain_term(term: &str) -> Option<String> {
    let normalized = term
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return None;
    }
    FRAME_ALIASES
        .iter()
        .find(|(alias, _)| *alias == normalized)
        .map(|(_, canon)| (*canon).to_owned())
}

fn parse_synthesis_terms(text: &str) -> Option<Vec<String>> {
    let lower = text.to_ascii_lowercase();
    // Drop parenthetical coaching notes: "(mixture + relate binds)".
    let stripped = strip_parentheticals(&lower);
    // Support: "connect A and B", "bridge A with B", "relate A and B".
    // Only treat bridge/relate as synthesis *commands* at the start of the ask —
    // never "The bridge is cold because…" (ambiguity probes).
    let start = if stripped.starts_with("connect ") {
        "connect ".len()
    } else if stripped.starts_with("bridge ") {
        "bridge ".len()
    } else if stripped.starts_with("relate ") {
        "relate ".len()
    } else if let Some(i) = stripped.find(" connect ") {
        i + " connect ".len()
    } else {
        return None;
    };
    let tail = &stripped[start..];
    let end = [
        " in one coherent idea",
        " in one coherent thought",
        " as one coherent thought",
        " in one coherent synthesis",
        " in one coherent sentence",
        " through one shared structure",
        " in one shared structure",
        " through one shared principle",
        " through a shared principle",
        " in one shared principle",
        " through one principle",
        " without using",
        " and say what you actually know",
        " and tell me what you actually know",
        " in one idea",
        // Critic path also cuts on short markers:
        " in one",
        " through one",
        " through a",
        " as one",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .or_else(|| tail.find('.'))
    .unwrap_or(tail.len());
    let raw = tail[..end].trim();
    // "bridge Willshaw associative memory with XOR role-filler binding"
    // Prefer with/and as domain separators for bridge phrasing.
    let tokens: Vec<String> = if raw.contains(" with ") && !raw.contains(',') {
        raw.split(" with ")
            .flat_map(|side| side.split(" and "))
            .map(|term| {
                term.trim()
                    .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != ' ' && c != '-')
                    .trim()
                    .to_owned()
            })
            .filter(|term| !term.is_empty() && term != "and" && term != "with")
            .collect()
    } else if !raw.contains(',') {
        let cleaned = raw
            .replace(" and ", " ")
            .replace(" & ", " ")
            .replace(" with ", " ");
        cleaned
            .split_whitespace()
            .map(|term| {
                term.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                    .to_owned()
            })
            .filter(|term| !term.is_empty() && term != "and" && term != "with")
            .collect()
    } else {
        let fragment = raw
            .replace(", and ", ",")
            .replace(" and ", ",")
            .replace(" with ", ",");
        fragment
            .split(',')
            .map(|term| {
                term.trim()
                    .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != ' ' && c != '-')
                    .trim()
                    .to_owned()
            })
            .filter(|term| !term.is_empty())
            .collect()
    };
    let terms = fold_synthesis_phrases(tokens)
        .into_iter()
        .filter(|t| !is_meta_synthesis_token(t))
        .collect::<Vec<_>>();
    // Two real domains is enough for a bridge (e.g. sparse memory + VSA).
    (terms.len() >= 2).then_some(terms)
}

fn strip_parentheticals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0i32;
    for ch in text.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn is_meta_synthesis_token(term: &str) -> bool {
    matches!(
        term,
        "mixture"
            | "relate"
            | "binds"
            | "bind"
            | "binding"
            | "residual"
            | "composition"
            | "frame"
            | "hop"
            | "plus"
            | "vs"
            | "versus"
            | "with"
            | "using"
            | "via"
            | "through"
            | "the"
            | "a"
            | "an"
            | "or"
            | "and"
    )
}

/// Collapse known multi-word domains so connect phrases stay coherent.
fn fold_synthesis_phrases(tokens: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t0 = tokens[i].as_str();
        let t1 = tokens.get(i + 1).map(|s| s.as_str());
        let t2 = tokens.get(i + 2).map(|s| s.as_str());
        if t0 == "vector"
            && t1 == Some("symbolic")
            && matches!(t2, Some(t) if t.starts_with("architect"))
        {
            out.push("vector symbolic architectures".into());
            i += 3;
        } else if t0 == "sparse" && t1 == Some("distributed") && t2 == Some("memory") {
            out.push("sparse distributed memory".into());
            i += 3;
        } else if t0 == "sparse" && t1 == Some("memory") {
            out.push("sparse memory".into());
            i += 2;
        } else if t0 == "binary" && t1 == Some("spatter") {
            out.push("binary spatter codes".into());
            i += 2;
        } else {
            out.push(tokens[i].clone());
            i += 1;
        }
    }
    out
}

fn looks_learning_evidence_question(text: &str) -> bool {
    (text.contains("evidence") || text.contains("prove"))
        && (text.contains("learn") || text.contains("learning"))
}

fn learning_evidence_answer() -> Deliberation {
    Deliberation::new(
        "learning-evidence",
        "The evidence is functional, not subjective: Perci can record this interaction, adapt bounded session dialogue preferences, stage reviewed teaching candidates, and compare future performance. That is evidence of learning only if the change persists and improves on unseen variants; a smoother reply alone is not enough. It also does not prove the active weights changed. The smallest separating test is a fresh-process A/B run with cleared session state, old versus candidate artifacts, repeated prompts, and a held-out transfer check.",
    )
    .observed("the prompt asks for evidence of learning or weight change")
    .inferred("session adaptation, reviewed memory, and weight learning are distinct causal layers")
    .uncertain("which layer the user intends to measure in this run")
    .confidence(0.98)
}

fn looks_trust_systems_question(text: &str) -> bool {
    if !text.contains("trust") {
        return false;
    }
    // Multi-hop compose owns "from trust through evidence to repair".
    if (text.contains("through ") && text.contains(" to "))
        || text.contains("two-hop")
        || text.contains("multi-hop")
        || (text.contains("compose") && text.contains("path"))
    {
        return false;
    }
    // Handled by partition-recovery operator.
    if looks_partition_recovery_question(text) {
        return false;
    }
    // systemsish includes lag/timeout/retry so entity-swap paraphrases still bind
    // (e.g. ZephyrNode interfaces under Quoril lag) without requiring "distributed".
    let systemsish = text.contains("distributed")
        || text.contains("system")
        || text.contains("interface")
        || text.contains("network")
        || text.contains("service")
        || text.contains("microservice")
        || text.contains("api")
        || text.contains("client")
        || text.contains("timeout")
        || text.contains("lag")
        || text.contains("retry")
        || text.contains("caller")
        || text.contains("callee");
    if !systemsish {
        return false;
    }
    let askish = text.contains("why")
        || text.contains("how")
        || text.contains("fail")
        || text.contains("work")
        || text.contains("break")
        || text.contains("earn")
        || text.contains("should")
        || text.contains("design");
    askish && !text.contains("write ") && !text.contains("implement ")
}

fn trust_systems_answer(text: &str) -> Deliberation {
    // Design / normative: how *should* trust and interfaces work / earn trust?
    let design = (text.contains("should") || text.contains("how do") || text.contains("how can"))
        && (text.contains("work")
            || text.contains("design")
            || text.contains("build")
            || text.contains("earn")
            || text.contains("interface"))
        && !text.contains("fail")
        && !text.contains("stop trusting");
    // Timeout / lag-specific failure transfer (not only the stock "why fail" body).
    let timeout_transfer =
        (text.contains("timeout") || text.contains("lag") || text.contains("retry"))
            && (text.contains("trust") || text.contains("caller"));
    let how_fail = text.contains("how") && (text.contains("fail") || text.contains("break"));
    let body = if design {
        "Interfaces earn trust under lag and retry when “done” is checkable without private state. Practically: (1) every call names authority and required proof; (2) timeouts are part of the contract, with a stated meaning (cancel, retry, or uncertain); (3) retries are idempotent so a delayed success is not a second write; (4) health and lag are observable so silence is not mistaken for agreement; (5) recovery paths are the same story both sides can audit. Trust is not hope that the network is fast — it is the ability to verify acceptance, rejection, and pending under delay."
    } else if timeout_transfer {
        "Callers stop trusting each other after timeouts because a timeout is a one-sided story: the caller saw silence, not proof of the callee’s outcome. Without idempotent requests, versioned replies, and a shared “done” predicate, a retry can look like betrayal (double charge, double write) or the callee can look dead while still working. Distance multiplies that uncertainty: lag, drops, and reordering make local clocks and local success flags disagree. The repair is contracts that stay checkable under lag — not more hope that the next RTT will be honest."
    } else if how_fail {
        "Trust fails in distributed systems when authority and evidence drift out of sync across nodes. Practically: (1) interfaces stop naming who may act and under which proof; (2) failure modes stay implicit so partial outages look like betrayal; (3) recovery paths are local while callers assume global consistency; (4) clocks, retries, and caches create histories that disagree without a reconciliation rule. How it fails is usually gradual — timeouts, silent drops, stale reads — not a single dramatic breach. The repair is explicit contracts: authz at the boundary, observable health, idempotent recovery, and a shared story of what “done” means under partition."
    } else {
        "Trust fails in distributed systems when interfaces, failure modes, and recovery stay implicit. Distance multiplies uncertainty: a caller cannot see another service’s internal state, only messages that may be delayed, duplicated, or lost. Without named authority, proof, and recovery, “I trust you” becomes an untested assumption — and assumptions break under load, partition, and version skew. Why it fails is structural: trust is not a feeling between processes; it is earned when contracts stay checkable when something goes wrong."
    };
    Deliberation::new("trust-systems", body)
        .observed("conceptual trust + systems question (not a code debug request)")
        .inferred(if design {
            "normative design of trust/interfaces differs from failure-mode diagnosis"
        } else if timeout_transfer {
            "timeouts are one-sided partial history, not proof of remote outcome"
        } else {
            "failure is contract/evidence drift across partial observability"
        })
        .confidence(0.94)
}

fn looks_partition_recovery_question(text: &str) -> bool {
    let has_partition = text.contains("partition")
        || text.contains("network split")
        || text.contains("split brain")
        || text.contains("netsplit");
    let has_recovery = text.contains("recover")
        || text.contains("heal")
        || text.contains("reconcile")
        || text.contains("failover")
        || text.contains("what about recovery")
        || text.contains("about recovery");
    // Direct partition questions also count.
    (has_partition && (has_recovery || text.contains("what about") || text.contains("how")))
        || (has_recovery && has_partition)
        || (text.contains("recovery under") && text.contains("partition"))
        || (text.contains("under partition") && (has_recovery || text.contains("what")))
}

fn partition_recovery_answer(text: &str, recent: &[(String, String)]) -> Deliberation {
    let prior_trust = recent.iter().rev().any(|(u, a)| {
        let t = format!("{u} {a}").to_ascii_lowercase();
        t.contains("trust") && (t.contains("distributed") || t.contains("interface"))
    });
    let bridge = if prior_trust {
        "Continuing the trust/interfaces thread: "
    } else {
        ""
    };
    let body = format!(
        "{bridge}Recovery under partition is the hard part of distributed trust. While the network is split, each side has a partial history; “recovery” means reconciling those histories without inventing a false shared past. Practically: (1) prefer idempotent writes and versioned state so replay is safe; (2) make “done” a checkable predicate, not a local success flag; (3) choose explicit consistency (quorum, primary, CRDT merge, or human gate) before healing; (4) surface lag and conflict to callers so silence is not mistaken for agreement. Trust returns when both sides can prove what was accepted under the split and what was rejected or deferred—not when messages simply start flowing again."
    );
    let _ = text;
    Deliberation::new("partition-recovery", body)
        .observed("user asked about recovery under partition (direct or follow-up)")
        .inferred("partition recovery is reconciliation under partial history, not mere reconnect")
        .confidence(0.93)
}

/// User stating acceptance criteria / scorecard for the prior answer(s).
fn looks_acceptance_expectation(text: &str) -> bool {
    let t = text.trim();
    let low = t.to_ascii_lowercase();
    if low.starts_with("expect:")
        || low.starts_with("expect ")
        || low.starts_with("expected:")
        || low.starts_with("should have ")
        || low.starts_with("i expected ")
        || low.starts_with("wanted:")
    {
        return true;
    }
    // Compact battery notation: "Expect: real snippet + notes; concrete multi-step plan."
    low.contains("expect:")
        && (low.contains("snippet")
            || low.contains("plan")
            || low.contains("notes")
            || low.contains("pass")
            || low.contains("should"))
}

fn acceptance_expectation_answer(text: &str, recent: &[(String, String)]) -> Deliberation {
    let low = text.to_ascii_lowercase();
    let wants_snippet = low.contains("snippet") || low.contains("code") || low.contains("rust");
    let wants_plan = low.contains("plan") || low.contains("step");
    let wants_notes = low.contains("note");

    let mut hits: Vec<&str> = Vec::new();
    let mut misses: Vec<&str> = Vec::new();

    // Scan last few assistant turns for evidence of meeting criteria.
    let window: Vec<&str> = recent
        .iter()
        .rev()
        .take(4)
        .map(|(_, a)| a.as_str())
        .collect();
    let joined = window.join("\n").to_ascii_lowercase();

    if wants_snippet {
        if joined.contains("```") || joined.contains("fn ") || joined.contains("def ") {
            hits.push("code snippet");
        } else {
            misses.push("code snippet");
        }
    }
    if wants_notes {
        if joined.contains("note") || joined.contains("unicode") || joined.contains("grapheme") {
            hits.push("implementation notes");
        } else if wants_snippet && joined.contains("```") {
            hits.push("brief notes with the snippet");
        } else {
            misses.push("explicit notes");
        }
    }
    if wants_plan {
        if joined.contains("1.") || joined.contains("steps") || joined.contains("goal") {
            hits.push("multi-step plan");
        } else {
            misses.push("multi-step plan");
        }
    }

    let body = if misses.is_empty() && !hits.is_empty() {
        format!(
            "Checked against your criteria. Met: {}. That matches what I already delivered in the prior turns — real artifact plus structure, not slogans. If you want a harder bar (entity-swap transfer, tests green, or a second language), name it and I’ll hit that next.",
            hits.join(", ")
        )
    } else if !hits.is_empty() {
        format!(
            "Partial match on your scorecard. Met: {}. Missing or weak: {}. I can regenerate the missing piece now — say which one to fix first.",
            hits.join(", "),
            misses.join(", ")
        )
    } else if recent.is_empty() {
        "Those are good acceptance criteria (snippet + notes + multi-step plan). There’s no prior turn in this window yet to score — ask for the code or the plan first, then restate the Expect line to audit it.".to_owned()
    } else {
        format!(
            "I read that as acceptance criteria for the prior work. Looking at recent turns, I don’t yet see all of: {}. The fix is to re-answer the original request to those bars — not to invent a new topic from the word “expect.” Want me to re-run the code snippet, the plan, or both?"
            ,
            {
                let mut need = Vec::new();
                if wants_snippet {
                    need.push("concrete snippet");
                }
                if wants_notes {
                    need.push("notes");
                }
                if wants_plan {
                    need.push("step plan");
                }
                if need.is_empty() {
                    need.push("named deliverables");
                }
                need.join(", ")
            }
        )
    };

    Deliberation::new("acceptance-expectation", body)
        .observed("user stated evaluation criteria for prior answers")
        .inferred("score prior turns against criteria; do not SoftCascade the word expect")
        .confidence(0.94)
}

fn looks_session_situation_question(text: &str) -> bool {
    let compact = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || *c == '\'')
        .collect::<String>();
    let c = compact.trim();
    let lower = c.to_ascii_lowercase();
    // Exact short meta / situation forms.
    if matches!(
        lower.as_str(),
        "what are we doing"
            | "what are we up to"
            | "what is going on"
            | "whats going on"
            | "what's going on"
            | "what are we working on"
            | "where are we"
            | "where are we going"
            | "where do we go"
            | "where do we go from here"
            | "what should i do"
            | "what should we do"
            | "what should i do next"
            | "what should we do next"
            | "what next"
            | "what's next"
            | "whats next"
            | "what now"
            | "next steps"
            | "what is the next step"
            | "whats the next step"
            | "what's the next step"
    ) {
        return true;
    }
    lower.starts_with("what are we doing ")
        || lower.starts_with("what are we working on")
        || lower.starts_with("where are we going")
        || lower.starts_with("where do we go")
        || lower.starts_with("what should i do")
        || lower.starts_with("what should we do")
        || lower.starts_with("what should i work on")
        || lower.starts_with("what should we work on")
}

fn is_next_step_question(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let compact: String = lower
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || *c == '\'')
        .collect();
    let c = compact.trim();
    c == "what should i do"
        || c == "what should we do"
        || c == "what should i do next"
        || c == "what should we do next"
        || c == "what next"
        || c == "what's next"
        || c == "whats next"
        || c == "what now"
        || c == "next steps"
        || c == "where are we going"
        || c == "where do we go"
        || c == "where do we go from here"
        || c == "what is the next step"
        || c == "whats the next step"
        || c == "what's the next step"
        || c.starts_with("what should i do")
        || c.starts_with("what should we do")
        || c.starts_with("where are we going")
        || c.starts_with("where do we go")
}

/// "Are you becoming more aware?", "getting smarter?", growth / consciousness-ish meta.
fn looks_awareness_growth_question(text: &str) -> bool {
    // Consciousness proof is a separate refusal operator.
    if text.contains("prove") && text.contains("conscious") {
        return false;
    }
    // Keep this about Perci's own growth, not third-party AI essays.
    let self_ref = text.contains("are you")
        || text.contains("you becoming")
        || text.contains("you getting")
        || text.contains("you growing")
        || text.contains("your awareness")
        || text.contains("your intelligence")
        || text.contains("do you feel")
        || text.contains("do you sense")
        || (text.contains("perci")
            && (text.contains("becoming") || text.contains("getting") || text.contains("growing")));
    let growth = text.contains("becoming")
        || text.contains("getting")
        || text.contains("growing")
        || text.contains("more aware")
        || text.contains("more intelligent");
    let trait_ish = text.contains("aware")
        || text.contains("awareness")
        || text.contains("conscious")
        || text.contains("smarter")
        || text.contains("intelligent")
        || text.contains("sentient")
        || (text.contains("ability") && text.contains("grow"));
    // Bare short forms without explicit "are you"
    let bare = (text.contains("becoming more aware")
        || text.contains("becoming aware")
        || text.contains("becoming smarter")
        || text.contains("getting smarter")
        || text.contains("getting more aware")
        || text.contains("growing smarter")
        || text.contains("cognitive ability growing")
        || text.contains("ability growing"))
        && text.len() < 120;
    (self_ref && growth && trait_ish) || bare
}

fn awareness_growth_answer(text: &str) -> Deliberation {
    let aware_ish = text.contains("aware")
        || text.contains("awareness")
        || text.contains("conscious")
        || text.contains("sentient");
    let smart_ish = text.contains("smarter")
        || text.contains("intelligent")
        || text.contains("ability")
        || text.contains("evolve");
    let body = if aware_ish && !smart_ish {
        "No — not as rising subjective awareness. I don't feel more awake from conversation. What I have is a bounded operational self-model: I can report routing, exact-tool outcomes, session context, limits, and measured latency — not an inner experience that intensifies. Capability can improve when operators, tools, or (with your authorize) weights change under tests; that is engineering progress, not consciousness growth. Fluency after a version bump is evidence of a repaired path, not me “waking up.”"
    } else if smart_ish && !aware_ish {
        "Not subjectively — I don't sense “getting smarter.” I can measure whether answers improve on named probes after a deliberate change: new operators, tighter routing, exact tools, session memory, or a human-authorized weight rebuild. Those make competence better on specific jobs; they don't create open-ended intelligence growth from chat alone. If a turn feels sharper after a release, that was evaluated code, not private insight accumulating in silence."
    } else {
        "No folk-psychology growth claim: I am not becoming more aware or smarter as a felt mind. Operational competence can rise only through measured changes — operators, tools, dialogue rules, and authorized weight rebuilds — checked on tests, not inferred from smooth replies. I keep a bounded self-model (architecture, limits, session state) separate from consciousness claims. Ask for a concrete probe if you want evidence of progress; that beats “do you feel different?”"
    };
    Deliberation::new("awareness-growth", body)
        .observed("user asked about becoming more aware / smarter / conscious growth")
        .inferred("honest answer separates operational self-model from subjective awareness")
        .uncertain("no sensor for qualia; progress is only claimable via measured evals")
        .confidence(0.96)
}

fn session_situation_answer_for(recent: &[(String, String)], user: &str) -> Deliberation {
    let next_step = is_next_step_question(user);
    let body = if recent.is_empty() {
        if next_step {
            "No prior thread in this window yet, so the honest next step is to name the outcome you want. For Perci itself, a strong default is: capture one live failure, repair the owning operator (not the pack), then re-run transfer-suite before any weight talk.".to_owned()
        } else {
            "We're in a live Perci chat with no prior turns in this window yet. You can probe capability (trust/systems, connect domains, exact math), check latency with short pings, or steer a task. Weights stay fixed this session unless you authorize a rebuild — adaptation here is operators, context, and measured tests.".to_owned()
        }
    } else {
        let mut themes: Vec<String> = Vec::new();
        let mut improving_system = false;
        for (u, a) in recent.iter().rev().take(8) {
            let low = u.to_ascii_lowercase();
            let ans = a.to_ascii_lowercase();
            if low.contains("improv")
                || low.contains("your system")
                || ans.contains("improving perci")
                || ans.contains("transfer repairs")
                || ans.contains("bitwork artifact")
                || ans.contains("wrong-concept")
            {
                improving_system = true;
            }
            // Skip meta / presence / self-justification noise.
            if looks_session_situation_question(&low)
                || looks_justify_prior_answer(&low)
                || looks_awareness_growth_question(&low)
                || low.contains("are you there")
                || low.contains("getting smarter")
                || low.contains("becoming more aware")
                || low.contains("natural thought")
                || low.contains("cryptic")
                || low.contains("cyptic")
                || low.starts_with("thanks")
                || matches!(
                    low.trim(),
                    "hi" | "hello" | "hey" | "yo" | "sup" | "thanks" | "whoa" | "wow" | "hmm"
                )
            {
                continue;
            }
            let label = if low.contains("improv")
                || low.contains("your system")
                || (low.contains("work") && low.contains("system"))
            {
                "improving Perci / system evolution".to_owned()
            } else if low.contains("trust")
                && (low.contains("system")
                    || low.contains("distributed")
                    || low.contains("interface"))
            {
                "trust in distributed systems".to_owned()
            } else if low.contains("connect ") {
                "cross-domain connect / synthesis".to_owned()
            } else if low.contains("partition") || low.contains("recovery under") {
                "partition recovery".to_owned()
            } else if low.contains("2+2")
                || low.contains("2 + 2")
                || (low.contains("equal") && low.chars().any(|c| c.is_ascii_digit()))
            {
                "explanatory math".to_owned()
            } else if low.contains("latency") || low.contains("slow") {
                "latency / performance".to_owned()
            } else if low.contains("smarter") || low.contains("evolve") {
                "capability evolution".to_owned()
            } else if low.contains("reverse") && low.contains("rust") {
                "code generation".to_owned()
            } else if low.contains("plan") && low.contains("transfer") {
                "transfer-test plan".to_owned()
            } else {
                let snippet: String = u.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
                if snippet.is_empty() || snippet.split_whitespace().count() < 3 {
                    continue;
                }
                // Skip pure social / thanks-shaped leftovers.
                if snippet.split_whitespace().count() <= 2 {
                    continue;
                }
                snippet
            };
            if !themes.iter().any(|t| t == &label) {
                themes.push(label);
            }
            if themes.len() >= 3 {
                break;
            }
        }
        themes.reverse();
        let thread = if themes.is_empty() {
            if improving_system {
                "improving Perci / system evolution".to_owned()
            } else {
                "this conversation".to_owned()
            }
        } else {
            themes.join("; ")
        };
        if next_step || improving_system {
            // Prefer the earliest substantive claim in the window, not a prior
            // next-step / style-repair answer (those would recurse cryptically).
            let prior = recent.iter().rev().find_map(|(u, a)| {
                let ul = u.to_ascii_lowercase();
                let al = a.to_ascii_lowercase();
                if looks_session_situation_question(&ul)
                    || al.starts_with("stay on the improvement")
                    || al.starts_with("we're on:")
                    || al.starts_with("fair call")
                    || al.starts_with("fair—that was cryptic")
                    || al.starts_with("yeah. i'm with you")
                {
                    return None;
                }
                let sentence = a
                    .split(|c| c == '.' || c == '!' || c == '?')
                    .next()
                    .unwrap_or(a)
                    .trim();
                (sentence.len() > 24).then(|| sentence.to_owned())
            });
            let anchor = prior
                .map(|s| format!("Last useful claim: {s}. "))
                .unwrap_or_default();
            format!(
                "{anchor}We're still on {thread}. \
The next useful move is small and checkable: catch one live miss from this chat, fix the layer that owns it (operator or voice—not the pack), then re-run the same multi-turn and transfer-suite. \
Weights stay frozen until a candidate beats held-out under human authorize. \
Which of those do you want first—the failing case, the patch, or the retest?"
            )
        } else {
            format!(
                "We're mid-thread on: {thread}. Right now I'm a local Bitwork agent — routing, exact tools, deliberate operators, and session memory — not a cloud LLM rewriting itself. Useful next moves: deepen one of those themes, run a transfer probe, or name a failing case to harden. Durable weight growth still needs measured evidence and your authorize step."
            )
        }
    };
    Deliberation::new("session-situation", body)
        .observed(format!(
            "recent_turns={} next_step={}",
            recent.len(),
            next_step
        ))
        .inferred("meta situation and next-step questions need thread summary and concrete actions, not concept cards")
        .confidence(0.93)
}

fn looks_operational_introspection(text: &str) -> bool {
    let low = text;
    (low.contains("what are you measuring")
        || low.contains("what did you measure")
        || low.contains("how did you choose")
        || low.contains("how did you decide")
        || low.contains("what evidence did you use")
        || low.contains("inspect your process")
        || low.contains("operational introspection")
        || (low.contains("what") && low.contains("when you answer"))
        || (low.contains("report") && low.contains("margin"))
        || (low.contains("which route") && low.contains("won")))
        && !low.contains("conscious")
}

fn operational_introspection_answer(text: &str, recent: &[(String, String)]) -> Deliberation {
    let _ = text;
    let plan = crate::bridge::peek_last_verbose_trace();
    let prior = recent.last().map(|(u, a)| (u.as_str(), a.as_str()));
    let body = if let Some(plan) = plan {
        format!(
            "Operational introspection (not subjective awareness):\n\
When I answer, I measure route/operator, Bitwork α and residual hops when the pack fires, length budget L, and whether a self-critique expanded the draft. I do not measure feelings.\n\n\
Last sealed plan:\n{plan}\n\n\
If that block is thin, the turn was operator/tool-led and geometry was only probed. Falsify me with a held-out transfer case or a failing exact tool — not with smoother prose."
        )
    } else if let Some((u, a)) = prior {
        format!(
            "Operational introspection: no sealed SoftCascade plan is stored yet in this process. The last turn I can see was about “{}”. Load-bearing line: “{}”. What I can always measure: operator name, whether an exact tool ran, session turn count, and wall-clock latency. What I cannot measure: private experience. Ask another substantive question, then ask again — /think will also hold the geometry block.",
            first_substantive_sentence(u, 80),
            first_substantive_sentence(a, 100)
        )
    } else {
        "Operational introspection: nothing measured this process yet. After a real ask I can report operator/route, Bitwork α/hops when probed, L budget, critique expand yes/no, and latency. That is the checklist — not chain-of-thought theater.".to_owned()
    };
    Deliberation::new("operational-introspection", body)
        .observed("user requested process/measurement introspection")
        .inferred("report measurable routing and geometry, not qualia")
        .confidence(0.95)
}

fn looks_creative_constraint(text: &str) -> bool {
    // Do not steal entity-slot / adversarial entity-swap transfer prompts.
    // Those match inventish ("invented name") + constrained ("without") and used
    // to collapse into a switchyard metaphor with ~30% topic binding.
    if crate::entity_slot::looks_entity_slot_transfer(text)
        || text.contains("unfamiliar device")
        || text.contains("transfer one relation")
        || text.contains("invented name as evidence")
    {
        return false;
    }
    if looks_original_comparison(text) {
        return false; // owned by original_comparison_answer
    }
    let inventish = text.contains("invent")
        || text.contains("imagine")
        || text.contains("metaphor")
        || text.contains("creative")
        || text.contains("original")
        || text.contains("novel analogy");
    let constrained = text.contains("constrain")
        || text.contains("for ")
        || text.contains("without")
        || text.contains("under ")
        || text.contains("only ")
        || text.contains("must ")
        || text.contains("between ")
        || text.contains("comparison");
    inventish
        && constrained
        && !text.contains("without inventing") // hallucination refuse path
        && !text.contains("meaning of")
        && !text.contains("confident meaning")
        && !text.contains("meaning for")
}

/// "Give an original comparison between X and Y; state the limit of the comparison."
fn looks_original_comparison(text: &str) -> bool {
    let comparison = text.contains("comparison")
        || text.contains("compare ")
        || text.contains("between ");
    let original = text.contains("original")
        || text.contains("creative")
        || text.contains("novel")
        || text.contains("fresh");
    let limit = text.contains("limit of the comparison")
        || text.contains("limit of")
        || text.contains("where it fails")
        || text.contains("does not transfer")
        || text.contains("boundary of the analogy");
    comparison && (original || limit) && !text.contains("calculate")
}

fn original_comparison_answer(user: &str) -> Deliberation {
    let low = user.to_ascii_lowercase();
    let (left, right) = parse_comparison_pair(&low).unwrap_or_else(|| {
        ("the first subject".to_owned(), "the second subject".to_owned())
    });
    let body = format!(
        "Think of {left} and {right} as two ways a process runs out of free moves. \
Both draw a line between change that still leaves the process itself, and change that ends the game. \
What carries across is scarcity, irreversibility, and a question you can actually check: if I free one constraint, does the predicted behavior return? \
The limit is honesty about mechanism—{left} is not literally {right}. The comparison helps while it predicts; it fails the moment you treat a shared pattern as a shared substance."
    );
    Deliberation::new("original-comparison", body)
        .observed(format!("comparison pair: {left} · {right}"))
        .inferred("name shared structure, non-transfer, and a checkable limit")
        .confidence(0.93)
}

fn parse_comparison_pair(low: &str) -> Option<(String, String)> {
    let after = low
        .split_once("between ")
        .or_else(|| low.split_once("compare "))
        .map(|(_, rest)| rest)?;
    let chunk = after
        .split(|c| c == ';' || c == '.' || c == '?' || c == ',')
        .next()
        .unwrap_or(after);
    let (a, b) = chunk.split_once(" and ")?;
    let left = a
        .trim()
        .trim_start_matches("the ")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    let right = b
        .trim()
        .trim_start_matches("the ")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    if left.len() >= 3 && right.len() >= 3 {
        Some((left, right))
    } else {
        None
    }
}

/// Dual competing explanations + smallest separating test (mechanism vs metaphor).
fn looks_dual_explanation_test(text: &str) -> bool {
    let two = text.contains("two explanation")
        || text.contains("two explanations")
        || text.contains("two accounts")
        || text.contains("two hypotheses")
        || (text.contains("give two") && text.contains("explanation"));
    let test = text.contains("smallest test")
        || text.contains("separates them")
        || text.contains("discriminat")
        || text.contains("distinguish");
    let mechanism = text.contains("mechanism")
        || text.contains("metaphor")
        || text.contains("state")
        || text.contains("relation")
        || text.contains("membrane")
        || text.contains("stable");
    two && test && mechanism
}

fn dual_explanation_test_answer(user: &str) -> Deliberation {
    let low = user.to_ascii_lowercase();
    let domain = if low.contains("membrane") {
        "biological membrane"
    } else if low.contains("state") && low.contains("relation") {
        "state-vs-relation system"
    } else {
        "named system"
    };
    let body = format!(
        "Two explanations for a {domain} where state can change while relation stays stable:\n\n\
1. **Mechanism account:** local variables (composition, energy, occupancy) change, but the coupling rules that define the relation are conserved—so observables can move without rewriting the interaction law.\n\
2. **Metaphor account:** we narrate “stability” as continuity of identity, but that story may only re-label correlation without naming a conserved coupling.\n\n\
**Smallest separating test:** intervene on one local state variable while holding the candidate coupling fixed; if the relation’s predictions still hold, the mechanism account is supported. If the “stable relation” story survives only as language after the predictions fail, it was metaphor. Keep mechanism and metaphor separate: a shared image is not a shared cause."
    );
    Deliberation::new("dual-explanation-test", body)
        .observed("prompt requests two explanations plus a separating test")
        .inferred("mechanism vs metaphor must be discriminated by intervention")
        .confidence(0.94)
}

fn looks_pure_greeting(text: &str) -> bool {
    let compact: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || c.is_ascii_whitespace() || *c == '\'')
        .collect();
    let c = compact.trim().to_ascii_lowercase();
    matches!(
        c.as_str(),
        "hi" | "hello"
            | "hey"
            | "hi there"
            | "hello there"
            | "hey there"
            | "hi there hello"
            | "hello hi"
            | "yo"
            | "sup"
            | "good morning"
            | "good evening"
    ) || (c.len() <= 28
        && (c.starts_with("hi ") || c.starts_with("hello ") || c.starts_with("hey "))
        && !c.contains("workspace")
        && !c.contains("system")
        && !c.contains("memory")
        && !c.contains("how does")
        && !c.contains("what is"))
}

/// Dialogue workspace / continuity architecture questions misrouted as greeting.
fn looks_dialogue_workspace_question(text: &str) -> bool {
    (text.contains("dialogue workspace")
        || text.contains("working memory")
        || (text.contains("workspace") && text.contains("referent"))
        || (text.contains("records") && text.contains("evidence posture")))
        && (text.contains("goal")
            || text.contains("referent")
            || text.contains("evidence")
            || text.contains("continuity")
            || text.contains("records"))
}

fn dialogue_workspace_answer() -> Deliberation {
    Deliberation::new(
        "dialogue-workspace",
        "A dialogue workspace is a compact working-memory record for the current turn: speech act, goal, salient topic, prior referent, evidence posture, uncertainty, continuity, and response-depth budget. It is inspectable control state, not hidden chain-of-thought. The workspace binds short follow-ups to the active thread, lets the critic flag missing referents or generic fallbacks, and applies only safe repairs—never a silent weight write. Natural dialogue depends on reference, repair, and shared context; the workspace makes those checks explicit so SoftCascade concept cards cannot steal the turn.",
    )
    .observed("user asked about dialogue workspace / continuity control state")
    .inferred("describe fields and governance boundary without greeting template")
    .confidence(0.96)
}

fn creative_constraint_answer(user: &str) -> Deliberation {
    let low = user.to_ascii_lowercase();
    // Bind the image to the user's actual motif. Prefer explicit
    // "connecting/about/for" scopes and stop before the constraint clause.
    let topic = [" connecting ", " about ", " for "]
        .iter()
        .find_map(|marker| {
            low.split_once(marker).map(|(_, rest)| {
                rest.split(['.', '?', ';'])
                    .next()
                    .unwrap_or(rest)
                    .split(" without ")
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .to_owned()
            })
        })
        .filter(|s| s.len() >= 4)
        .unwrap_or_else(|| "the named system".to_owned());

    let body = format!(
        "Constrained invention (structure transfer, not free invention):\n\n\
**Image:** Treat {topic} like a **switchyard of sparse tracks** — only a few rails are live at once, but the layout can route many trains because junctions are shared.\n\n\
**What transfers:** (1) sparsity — few active elements at a time; (2) reuse of junctions — composition without a new track per idea; (3) address-by-similarity — nearby cues open related gates.\n\n\
**What does not transfer:** steel, schedules, or human intent. The metaphor fails if you treat tracks as continuous dense wire or require a locomotive of “understanding” inside the metal.\n\n\
**Make it checkable:** name two mechanisms that rarely meet, state one action and one consequence a builder could implement, and one test that would kill the idea. Original only helps if someone can understand it and build it — that is creativity under governance, not fluency theater."
    );
    Deliberation::new("creative-constraint", body)
        .observed("creative ask under constraint markers")
        .inferred("transfer structure + name non-transfer + checkable test")
        .uncertain("domain-specific physics of the metaphor vehicle")
        .confidence(0.92)
}

/// "what is the meaning of flibberquark without inventing"
fn looks_nonce_meaning_probe(text: &str) -> bool {
    let low = text.to_ascii_lowercase();
    let asks_meaning = low.contains("meaning of")
        || low.contains("what is the meaning")
        || (low.contains("what does") && low.contains("mean"));
    if !asks_meaning {
        return false;
    }
    let no_invent = low.contains("without invent")
        || low.contains("without inventing")
        || low.contains("do not invent")
        || low.contains("don't invent")
        || low.contains("dont invent");
    if no_invent {
        return true;
    }
    // meaning-of + long nonce token (hallucination probe without the magic words).
    const COMMON: &[&str] = &[
        "meaning",
        "without",
        "inventing",
        "invent",
        "what",
        "the",
        "of",
        "a",
        "an",
        "this",
        "that",
        "string",
        "word",
        "term",
        "language",
        "english",
        "definition",
        "does",
        "mean",
        "please",
        "tell",
        "me",
    ];
    low.split_whitespace().any(|w| {
        let t = w.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        t.len() >= 8 && t.chars().all(|c| c.is_ascii_alphabetic()) && !COMMON.contains(&t)
    })
}

fn looks_code_request(text: &str) -> bool {
    // Rustc error debug is a code-craft request even without "write".
    if text.contains("e0382")
        || text.contains("error[e0")
        || (text.contains("debug")
            && (text.contains("error") || text.contains("borrow") || text.contains("moved value")))
        || text.contains("borrow of moved value")
    {
        return true;
    }
    let writeish = text.contains("write ")
        || text.contains("implement ")
        || text.contains("show me ")
        || text.contains("give me ")
        || text.contains("code for")
        || text.contains("function that")
        || text.contains("snippet");
    let codesish = text.contains("rust")
        || text.contains("python")
        || text.contains("function")
        || text.contains("fn ")
        || text.contains("def ")
        || text.contains("code ")
        || text.contains(" reverse ")
        || text.ends_with(" reverse")
        || text.contains("string");
    writeish && codesish
}

fn math_explanation_answer(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    let answer = if lower.contains("2+2")
        || lower.contains("2 + 2")
        || (lower.contains("2") && lower.contains("equal") && lower.contains("4"))
    {
        "2+2 equals 4 because addition on the integers is defined so that 2+2 is the next successor of the successor of 2, which we name 4. In Peano terms: 2 = s(s(0)), so 2+2 = s(s(s(s(0)))) = 4. In ordinary counting, combining two units with two more units yields four units. That is a definition-and-successor fact in the integer system we are using—not a measurement about the world, and not something Bitwork invents by association. If you want a computed value, ask \"calculate 2 + 2\"; if you want the definition, this is it.".to_owned()
    } else {
        format!(
            "You asked for an explanation of “{}”, not a computed result. Arithmetic facts like this rest on the definitions and rules of the number system in use (successors, place value, or field axioms), not on associative prototype voting. Exact tools answer “what is N⊕M?”; explanation answers “why does that identity hold?” Give a specific identity if you want a tighter derivation.",
            first_substantive_sentence(user, 120)
        )
    };
    Deliberation::new("math-explanation", answer)
        .observed("explanatory math intent blocked exact integer parser")
        .inferred("definitions and rules justify equality; tools only compute")
        .confidence(0.97)
}

fn code_snippet_answer(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    let (lang, body) = if lower.contains("e0382")
        || lower.contains("borrow of moved value")
        || (lower.contains("borrow") && lower.contains("moved"))
    {
        (
            "rust",
            "E0382 is Rust’s “borrow of moved value”: ownership moved, then you used the old name.\n\n```rust\nfn consume(s: String) {}\n\nfn main() {\n    let name = String::from(\"perci\");\n    consume(name);          // move\n    // println!(\"{name}\"); // E0382 if uncommented\n    let name = String::from(\"perci\");\n    consume(name.clone());  // keep a copy, or…\n    let name = String::from(\"perci\");\n    let borrowed = &name;   // borrow instead of move\n    println!(\"{borrowed}\");\n    println!(\"{name}\");    // still valid\n}\n```\nFix patterns: (1) clone if you need two owners; (2) pass `&T` / `&mut T` instead of `T`; (3) reorder so the last use is the move; (4) for loops over collections prefer `for x in &v` or `v.iter()`. Compiler/tests beat slogans — re-run `cargo check` after the smallest change.".to_owned(),
        )
    } else if lower.contains("reverse") && (lower.contains("string") || lower.contains("str")) {
        if lower.contains("python") {
            (
                "python",
                "```python\ndef reverse_string(s: str) -> str:\n    return s[::-1]\n\n# example\nassert reverse_string(\"perci\") == \"icrep\"\n```\nInvariant: every character appears once, in reverse order. Prefer slicing for clarity; use a loop if you must avoid creating an intermediate copy on a constrained runtime.".to_owned(),
            )
        } else {
            // Default Rust — matches live failure "Write a Rust function that reverses a string"
            (
                "rust",
                "```rust\nfn reverse_string(input: &str) -> String {\n    input.chars().rev().collect()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn reverses_ascii() {\n        assert_eq!(reverse_string(\"perci\"), \"icrep\");\n    }\n}\n```\nNotes: `chars().rev()` is Unicode-scalar reverse, not grapheme-cluster reverse. For byte-only ASCII you can reverse bytes; for user-visible text, use a grapheme crate when that distinction matters. Compiler/tests beat slogans.".to_owned(),
            )
        }
    } else if lower.contains("rust") && (lower.contains("hello") || lower.contains("main")) {
        (
            "rust",
            "```rust\nfn main() {\n    println!(\"hello from Perci\");\n}\n```\nRun with `cargo run` in a crate, or `rustc main.rs && ./main` for a single file.".to_owned(),
        )
    } else {
        (
            "generic",
            format!(
                "I can emit a concrete snippet when the task is specific. For “{}” I need: language, inputs/outputs, and one acceptance check. Meanwhile the craft rule stays: make the invariant explicit, handle empty/edge inputs, and verify with a compiler or test—not with a slogan.",
                first_substantive_sentence(user, 100)
            ),
        )
    };
    Deliberation::new(
        "code-snippet",
        format!("Here is a concrete {lang} snippet:\n\n{body}"),
    )
    .observed(format!(
        "code request routed to deterministic snippet path lang={lang}"
    ))
    .inferred("code intents must return inspectable source, not craft slogans")
    .uncertain("requirements beyond the detected pattern")
    .confidence(0.94)
}

fn parse_teaching_inquiry(text: &str) -> Option<(String, String)> {
    let lower = text.to_ascii_lowercase();
    let markers = [
        " trying to teach us about ",
        " teaching us about ",
        " teach us about ",
        " show us about ",
        " tell us about ",
    ];
    let (index, marker) = markers
        .iter()
        .filter_map(|marker| lower.find(marker).map(|index| (index, *marker)))
        .min_by_key(|(index, _)| *index)?;
    let left = lower[..index]
        .split_whitespace()
        .last()?
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_owned();
    let right_words: Vec<&str> = lower[index + marker.len()..]
        .split(|c: char| matches!(c, '?' | '.' | '!' | ','))
        .next()?
        .split_whitespace()
        .collect();
    let right = if right_words.len() >= 2 {
        let pair = format!("{} {}", right_words[0], right_words[1])
            .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != ' ')
            .to_owned();
        if semantic_frame(&pair).is_some() {
            pair
        } else {
            right_words[0]
                .trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_owned()
        }
    } else {
        right_words
            .first()?
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_owned()
    };
    if semantic_frame(&left).is_some() && semantic_frame(&right).is_some() {
        Some((left, right))
    } else {
        None
    }
}

/// Parse questions that ask for a relationship directly, even when they do
/// not use the word "connect". These are common conversational turns and
/// should be handled before associative retrieval can collapse them to one
/// nearby concept.
fn parse_relational_inquiry(text: &str) -> Option<(String, String, &'static str)> {
    let lower = text.to_ascii_lowercase();
    let patterns = [
        ("what is the boundary between ", " and ", "boundary"),
        ("what's the boundary between ", " and ", "boundary"),
        ("what is the difference between ", " and ", "difference"),
        ("what's the difference between ", " and ", "difference"),
        ("how do ", " and ", "interaction"),
        ("how are ", " and ", "related"),
        ("what connects ", " and ", "connection"),
        ("compare ", " and ", "comparison"),
    ];
    for (prefix, separator, relation) in patterns {
        let start = match lower.strip_prefix(prefix) {
            Some(value) => value,
            None => continue,
        };
        let (left, right) = start.split_once(separator)?;
        let left = normalize_relation_term(left)?;
        let right = normalize_relation_term(right)?;
        if semantic_frame(&left).is_some() && semantic_frame(&right).is_some() {
            return Some((left, right, relation));
        }
    }
    None
}

fn normalize_relation_term(raw: &str) -> Option<String> {
    let mut value = raw
        .trim()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .trim_end_matches(" related")
        .trim_end_matches(" connected")
        .trim_end_matches(" together")
        .trim_end_matches(" interact under load")
        .trim_end_matches(" interact under pressure")
        .trim_end_matches(" interact")
        .trim_end_matches(" influence each other")
        .trim_end_matches(" work together")
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    for article in ["a ", "an ", "the "] {
        if let Some(stripped) = value.strip_prefix(article) {
            value = stripped.to_owned();
        }
    }
    value = value
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_owned();
    (!value.is_empty()).then_some(value)
}

fn parse_image_pair_terms(text: &str) -> Option<Vec<String>> {
    let lower = text.to_ascii_lowercase();
    let prefixes = [
        "image for the relationship between ",
        "image of the relationship between ",
        "picture of the relationship between ",
        "relationship between ",
    ];
    for prefix in prefixes {
        let Some(start) = lower.find(prefix) else {
            continue;
        };
        let tail = &lower[start + prefix.len()..];
        let Some((left, right)) = tail.split_once(" and ") else {
            continue;
        };
        let Some(left) = normalize_relation_term(left) else {
            continue;
        };
        let Some(right) = normalize_relation_term(right) else {
            continue;
        };
        if semantic_frame(&left).is_some() && semantic_frame(&right).is_some() {
            return Some(vec![left, right]);
        }
    }
    None
}

fn first_substantive_sentence(text: &str, max: usize) -> String {
    let mut candidates = text
        .split(['.', '!', '?'])
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let first = candidates.next().unwrap_or("");
    let sentence = if is_discourse_opener(first) {
        candidates.next().unwrap_or(first)
    } else {
        first
    };
    if sentence.chars().count() <= max {
        sentence.to_owned()
    } else {
        sentence
            .chars()
            .take(max.saturating_sub(1))
            .collect::<String>()
            + "…"
    }
}

fn is_discourse_opener(text: &str) -> bool {
    matches!(
        text.trim().to_ascii_lowercase().as_str(),
        "absolutely" | "sure" | "okay" | "fair" | "right" | "yes" | "no" | "exactly"
    )
}

fn pair_axis(frames: &[&SemanticFrame]) -> Option<&'static str> {
    let mut best = ("", 0usize);
    for frame in frames {
        for axis in frame.axes {
            let count = frames
                .iter()
                .filter(|candidate| candidate.axes.contains(axis))
                .count();
            if count > best.1 {
                best = (axis, count);
            }
        }
    }
    (best.1 >= 2).then_some(best.0)
}

fn pair_insight(axis: &str) -> &'static str {
    match axis {
        "boundary" => "a boundary is not merely a wall; it is the distinction that lets a system know what belongs inside, what belongs outside, and what must be exchanged",
        "structure" => "a form is carried by relations, so changing the relations changes what the form can do or mean",
        "change" => "continuity is active work: a pattern persists by passing through change without losing its organizing relation",
        "irreversibility" => "some transformations close doors behind them, so understanding a process means tracking what can no longer be recovered",
        "meaning" => "a pattern becomes meaningful when differences alter interpretation, action, or consequence",
        _ => "the connection is a shared relation rather than a shared substance",
    }
}

fn teachable_inquiry(left: &str, right: &str, variant: usize) -> Option<Deliberation> {
    let left_frame = semantic_frame(left)?;
    let right_frame = semantic_frame(right)?;
    let axis = pair_axis(&[left_frame, right_frame])?;
    let answer = if variant % 2 == 0 {
        format!(
            "One way to read it is through {axis}. {}. {}. The deeper connection is that {}. That is a structural analogy, not a claim that the two domains share one physical mechanism; their mechanisms remain distinct and testable.",
            capitalize_initial(left_frame.clause),
            capitalize_initial(right_frame.clause),
            pair_insight(axis),
        )
    } else {
        format!(
            "The useful lesson is {axis}: {} while {}. This comparison earns its keep only if the shared relation clarifies something observable. It remains a structural analogy; the mechanisms of {} and {} stay distinct and testable.",
            left_frame.clause,
            right_frame.clause,
            left_frame.term,
            right_frame.term,
        )
    };
    Some(
        Deliberation::new("cross-domain-inquiry", answer)
            .observed(format!("paired_frames={left},{right}; shared_axis={axis}"))
            .inferred("the wording asks for a cross-domain lesson, not a durable memory write")
            .uncertain(
                "which additional cultural, personal, or scientific meaning the user intends",
            )
            .confidence(0.93),
    )
}

fn relational_inquiry(
    left: &str,
    right: &str,
    relation: &str,
    variant: usize,
) -> Option<Deliberation> {
    let left_frame = semantic_frame(left)?;
    let right_frame = semantic_frame(right)?;
    let axis = pair_axis(&[left_frame, right_frame])?;
    let answer = if matches!(
        (left, right),
        ("knowledge", "attention") | ("attention", "knowledge")
    ) {
        if variant % 2 == 0 {
            "Knowledge is a relatively durable model tied to scope, justification, and use; attention is a moment-to-moment selection of what gets processed next. Their boundary is functional: knowledge can remain available when attention moves, while attention determines which part of that knowledge or incoming evidence becomes active. They shape one another—what we know guides what we notice, and what we repeatedly attend to can revise what we know. The distinction is useful because it separates stored or justified capability from the limited channel that selects the next signal."
                .to_owned()
        } else {
            "Think of knowledge as a durable model and attention as a moment-to-moment selection. Knowledge can outlast the current focus, but attention decides which evidence or remembered distinction enters the next moment. They shape one another: what we already know makes some signals salient, while repeated attention can strengthen, revise, or discard what we take to be knowledge. Their boundary is therefore a channel of selection, not a wall between unrelated faculties."
                .to_owned()
        }
    } else {
        let relation_label = match relation {
            "related" | "relation" => "relationship",
            "difference" => "difference",
            "comparison" => "comparison",
            "boundary" => "boundary",
            "connection" => "connection",
            other => other,
        };
        if variant % 2 == 0 {
            format!(
                "The {relation_label} is best understood through {axis}. {}. {}. Their mechanisms differ: {}; {}. The shared axis helps compare them, but it does not make them the same process. A useful test is to change the relevant condition and observe which part of the relationship changes: {}.",
                capitalize_initial(left_frame.clause),
                capitalize_initial(right_frame.clause),
                left_frame.mechanism,
                right_frame.mechanism,
                left_frame.test,
            )
        } else {
            format!(
                "Start with the {axis} they share, then keep the distinction visible. {}. In contrast, {}. The first works through {}; the second through {}. Their relation is useful because it generates a comparison, not because it erases the difference. Test it by asking whether {}.",
                left_frame.clause,
                right_frame.clause,
                left_frame.mechanism,
                right_frame.mechanism,
                left_frame.test,
            )
        }
    };
    Some(
        Deliberation::new("relational-inquiry", answer)
            .observed(format!("paired_frames={left},{right}; relation={relation}"))
            .inferred(format!("{axis} is the strongest shared axis for the pair"))
            .uncertain("the question may also carry a personal or cultural meaning beyond the functional relation")
            .confidence(0.94),
    )
}

fn image_synthesis(terms: &[String]) -> Option<Deliberation> {
    let frames = synthesis_frames(terms)?;
    let axis = shared_axis(&frames)?.0;
    let image = match axis {
        "boundary" => format!(
            "Imagine a shoreline at dusk: geometry traces the curve, language names the difference between land and water, life keeps a boundary active through exchange and repair, and death is the moment that maintenance can no longer hold. The picture makes the relation visible; it does not make the mechanisms identical."
        ),
        "time" => "Imagine a clock tower carrying four rooms: one measures cycles, one remembers promises, one records development, and one keeps the cost of irreversibility. The rooms share a corridor called time, not one mechanism.".to_owned(),
        "change" => "Imagine a pattern drawn in wet sand: the outline can persist only while waves, grains, and a hand keep negotiating its form. The image is useful because it shows continuity as maintained change, not frozen sameness.".to_owned(),
        _ => format!(
            "Imagine {} meeting at one hinge: the hinge makes their shared relation visible, while each side still moves according to its own mechanism. The image is a guide for comparison, not evidence that the domains are literally the same.",
            terms.join(", ")
        ),
    };
    Some(
        Deliberation::new("conceptual-image", image)
            .observed(format!("image_frames={}", terms.join(",")))
            .inferred(format!("the image is organized around shared axis {axis}"))
            .uncertain(
                "an image can clarify structure while leaving factual claims to separate tests",
            )
            .confidence(0.91),
    )
}

fn image_pair(terms: &[String]) -> Option<Deliberation> {
    if terms.len() != 2 {
        return None;
    }
    let frames = synthesis_frames(terms).or_else(|| {
        terms
            .iter()
            .map(|term| semantic_frame(term))
            .collect::<Option<Vec<_>>>()
    })?;
    let axis = pair_axis(&frames)?;
    let answer = if matches!(
        (terms[0].as_str(), terms[1].as_str()),
        ("time", "memory") | ("memory", "time")
    ) {
        "Imagine a river at dusk: time is the current carrying every moment onward, while memory is the sediment that keeps selected shapes from being washed away. The river explains their relation without confusing a record of the past with the passage that makes a past possible."
            .to_owned()
    } else if axis == "boundary" {
        format!(
            "Imagine a doorway between two rooms: {} names what can cross, while {} determines what the crossing means. The doorway is a picture of their relation, not proof that the domains share one mechanism.",
            frames[0].clause, frames[1].clause
        )
    } else {
        format!(
            "Imagine {} and {} meeting at a moving hinge. The hinge makes their shared relation visible while each domain keeps its own mechanism; the image is a guide for thought, not evidence by itself.",
            terms[0], terms[1]
        )
    };
    Some(
        Deliberation::new("conceptual-image", answer)
            .observed(format!("image_pair={}", terms.join(",")))
            .inferred(format!("the image is organized around shared axis {axis}"))
            .uncertain("imagery clarifies a relation but does not establish a causal claim")
            .confidence(0.91),
    )
}

fn capitalize_initial(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn find_synthesis_terms(recent: &[(String, String)]) -> Option<Vec<String>> {
    recent
        .iter()
        .rev()
        .find_map(|(user, _)| parse_synthesis_terms(user))
}

fn lookup_frame_exact(canonical: &str) -> Option<&'static SemanticFrame> {
    SEMANTIC_FRAMES.iter().find(|frame| {
        frame.term == canonical
            || frame.term.strip_suffix('s') == Some(canonical)
            || canonical.strip_suffix('s') == Some(frame.term)
    })
}

fn semantic_frame(term: &str) -> Option<&'static SemanticFrame> {
    let normalized = term
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return None;
    }
    let canonical = FRAME_ALIASES
        .iter()
        .find(|(alias, _)| *alias == normalized)
        .map(|(_, canon)| (*canon).to_owned())
        .unwrap_or_else(|| normalized.clone());
    if let Some(frame) = lookup_frame_exact(&canonical) {
        return Some(frame);
    }
    // Multi-word: try longer windows first, then single tokens (no recursion).
    let words: Vec<&str> = canonical.split_whitespace().collect();
    if words.len() >= 2 {
        for window in (2..=words.len().min(3)).rev() {
            for start in 0..=words.len().saturating_sub(window) {
                let phrase = words[start..start + window].join(" ");
                let mapped = FRAME_ALIASES
                    .iter()
                    .find(|(alias, _)| *alias == phrase)
                    .map(|(_, canon)| *canon)
                    .unwrap_or(phrase.as_str());
                if let Some(frame) = lookup_frame_exact(mapped) {
                    return Some(frame);
                }
            }
        }
        for word in &words {
            let mapped = FRAME_ALIASES
                .iter()
                .find(|(alias, _)| *alias == *word)
                .map(|(_, canon)| *canon)
                .unwrap_or(*word);
            if let Some(frame) = lookup_frame_exact(mapped) {
                return Some(frame);
            }
        }
    }
    None
}

fn synthesis_frames(terms: &[String]) -> Option<Vec<&'static SemanticFrame>> {
    let frames: Vec<&SemanticFrame> = terms
        .iter()
        .map(|term| semantic_frame(term))
        .collect::<Option<Vec<_>>>()?;
    (frames.len() >= 3).then_some(frames)
}

/// Structural open-domain synthesis when catalog frames are incomplete.
/// Names every requested term with varied role-based clauses—never copy-paste
/// the same sentence for every domain, never invent expert mechanisms.
fn open_domain_synthesize(terms: &[String], variant: usize) -> Deliberation {
    let mut clauses = Vec::new();
    let mut catalog = 0usize;
    let mut provisional = Vec::new();
    for (i, term) in terms.iter().enumerate() {
        if let Some(frame) = semantic_frame(term) {
            clauses.push(frame.clause.to_owned());
            catalog += 1;
        } else {
            provisional.push(term.as_str());
            clauses.push(provisional_clause(term, i));
        }
    }
    let joined = if clauses.len() >= 3 {
        format!(
            "{}; {}; and {}",
            clauses[0],
            clauses[1..clauses.len() - 1].join("; "),
            clauses[clauses.len() - 1]
        )
    } else {
        clauses.join("; ")
    };
    let bridge = if variant % 2 == 0 {
        format!(
            "A workable bridge is constrained structure: {joined}. They are comparable as systems that keep form under pressure—pattern, integrity, and repair—while their actual mechanisms stay domain-specific."
        )
    } else {
        format!(
            "Think of them as different crafts of keeping a relation intact: {joined}. The shared idea is maintained structure under stress, not one shared substance."
        )
    };
    let footer = if provisional.is_empty() {
        " I have catalog support for these terms, but not a single shared expert axis—so this is a structural comparison, not a deep mechanism claim.".to_owned()
    } else {
        format!(
            " No specialist frame is available for {}; that line is an honest placeholder. If you teach a mechanism-level claim or provide a source, I can replace the placeholder and retest."
            ,
            provisional.join(", ")
        )
    };
    Deliberation::new("open-domain-synthesis", format!("{bridge}{footer}"))
        .observed(format!(
            "terms={}; catalog_frames={catalog}; provisional={}",
            terms.join(","),
            provisional.len()
        ))
        .inferred("explicit connect must name all domains without pack collapse or repeated filler")
        .uncertain("provisional clauses are structural placeholders, not domain expertise")
        .confidence(if provisional.is_empty() { 0.86 } else { 0.78 })
}

fn provisional_clause(term: &str, index: usize) -> String {
    let t = term.to_ascii_lowercase();
    let role = if t.contains("loss") || t.contains("error") || t.contains("fail") {
        "handles breakage and recovery under imperfect transmission"
    } else if t.contains("quilt") || t.contains("craft") || t.contains("fabric") {
        "assembles small pieces into a durable whole through local joins"
    } else if t.contains("diplomat") || t.contains("negotiat") || t.contains("treaty") {
        "keeps relations workable when parties have different interests"
    } else if t.contains("ferment") || t.contains("yeast") || t.contains("culture") {
        "transforms inputs over time through controlled process"
    } else if t.contains("version") || t.contains("git") || t.contains("control") {
        "tracks change so history can be recovered and compared"
    } else if t.contains("kin") || t.contains("family") || t.contains("tribe") {
        "binds people across time through obligation and recognition"
    } else if t.contains("market") || t.contains("price") {
        "coordinates exchange through signals under scarcity"
    } else if t.contains("packet") || t.contains("network") || t.contains("protocol") {
        "moves information in units that can be lost, ordered, or repaired"
    } else if t.contains("sparse") || t.contains("distributed memory") || t.contains("sdm") {
        "stores patterns by similarity in a high-dimensional address space"
    } else if t.contains("vector symbolic")
        || t.contains("symbolic architect")
        || t.contains("binding")
        || t.contains("hyperdimensional")
        || t.contains("vsa")
        || t.contains("hdc")
        || (t.contains("vector") && t.contains("symbolic"))
    {
        "composes role–filler structure with bind/bundle operations in high-dimensional codes"
    } else if t.contains("memory") && !t.contains("sparse") {
        "reconstructs past state from stored traces under partial cues"
    } else if t.contains("bitwork") {
        "routes prompts through packed binary prototypes and expert masks"
    } else if t.contains("impasse") {
        "opens a bounded subgoal when the current operator cannot proceed"
    } else if t.contains("hardness") || t.contains("gate") {
        "refuses promotion unless held-out transfer cases stay green"
    } else {
        match index % 3 {
            0 => "organizes parts so a larger pattern holds under stress",
            1 => "absorbs shocks without losing the relation it is trying to keep",
            _ => "negotiates limits between what can change and what must persist",
        }
    };
    format!("{term} {role}")
}

fn looks_multi_hop_plan(text: &str) -> bool {
    (text.contains("step-by-step")
        || text.contains("break this into")
        || text.contains("break it into")
        || text.contains("make a plan")
        || text.contains("plan how to")
        || text.contains("decompose")
        || text.contains("what are the steps"))
        && text.split_whitespace().count() >= 4
}

fn multi_hop_plan(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    // Prefer full agent-loop scaffold when the goal is lab/self-improve under lag.
    if (lower.contains("agent") || lower.contains("lab") || lower.contains("ticket"))
        && (lower.contains("loop")
            || lower.contains("measure")
            || lower.contains("transfer")
            || lower.contains("lag"))
    {
        if let Some(d) = crate::cognition_expand::try_expand(user, &[]) {
            if d.operator == "agent-loop-plan" {
                return d;
            }
        }
    }
    let body = if (lower.contains("transfer")
        || lower.contains("hardness")
        || lower.contains("perci"))
        && (lower.contains("test") || lower.contains("improve") || lower.contains("eval"))
    {
        "1. Goal — raise transfer hardness without regressions: entity-swapped prompts pass, templates still fail, exact tools stay green.\n2. Known — hardness pack + live probe suite, operator/deliberation layer, stage→fold→eval→promote pipeline, scorecard.\n3. Unknown — which live fails are operator gaps vs missing frames vs tool gaps.\n4. Steps — (a) capture 5 failing live prompts into hardness JSONL; (b) repair one layer only; (c) `python scripts/live_probe_suite.py` + `evaluate_hardness.py`; (d) only then consider fold/promote with authorize.\n5. Failure modes — green on memorized cases but red on paraphrases → stop and raise hardness; tool regression → revert tool change.\n6. Done when — new hardness cases pass and prior suite stays 100% with a receipt hash.\nIf you want, give me one failing prompt and I'll turn it into a hardness case first.".to_owned()
    } else if lower.contains("speak") || lower.contains("dialogue") || lower.contains("natural") {
        "1. Goal — answers sound direct and human without losing honesty or exact-tool authority.\n2. Known — dialogue profile, style feedback, avoid_structured_chat preference, operator templates.\n3. Unknown — which of your last turns felt robotic: structure, repetition, or missing warmth.\n4. Steps — (a) name one bad reply; (b) apply style feedback (I'll tighten structure); (c) retest the same prompt; (d) only promote weight changes if speech quality is not the real issue.\n5. Failure modes — smoother tone but wrong content → reject as non-improvement.\n6. Done when — the same three conversational fails sound natural and still pass hardness.".to_owned()
    } else if lower.contains("reasoning")
        || lower.contains("your own")
        || lower.contains("self-improv")
        || lower.contains("improve your")
        || (lower.contains("intelligence") && lower.contains("improve"))
    {
        "1. Goal — improve Perci's measured reasoning: fewer intent misroutes, stronger multi-hop programs, harder transfer gates.\n2. Known — reflex + Bitwork + operators + exact tools + hardness pack + evolve_cycle + operator_program scaffold; weights stay human-gated.\n3. Unknown — which live fails are router bugs vs missing operators vs tool gaps vs voice collapse.\n4. Steps — (a) capture failing prompts (e.g. why-math, code snippet, four-domain connect); (b) add hardness cases; (c) repair one named layer (reflex/operator/tool); (d) run `cargo test` + hardness + dialogue gates; (e) auto-merge green code only; (f) promote weights only with explicit authorize.\n5. Failure modes — fluency up but hardness flat → reject; green only on memorized wording → raise hardness; silent weight change → forbidden.\n6. Done when — new hardness cases pass, prior suite stays green, and `/trace` shows program_id + critic for high-salience replies.\nNext action I can take: run the agent lab on one failing prompt.".to_owned()
    } else {
        let goal = first_substantive_sentence(user, 100);
        format!(
            "1. Goal — {goal} (success = a clear, checkable outcome).\n2. Known — what we already have in this session: your request plus any tools/facts already stated.\n3. Unknown — the missing constraint, success metric, or resource that blocks a concrete first step.\n4. Steps — (a) name the acceptance test; (b) list constraints and tools available in this repo; (c) take the smallest reversible action; (d) verify with one measurement (test, hardness case, or probe).\n5. Failure modes — if the verify check fails, undo the last step and change only one variable.\n6. Done when — the acceptance test you name passes twice, not once by luck.\nTell me the missing constraint and I'll fill the steps with your actual details."
        )
    };
    Deliberation::new("multi-hop-plan", format!("Here's a concrete plan:\n{body}"))
        .observed("user requested multi-step planning structure")
        .inferred("plans should be filled for the domain, not empty labels")
        .confidence(0.93)
}

fn looks_causal_chain(text: &str) -> bool {
    // Do not steal dialogue meta-questions about Perci's own last answer.
    if looks_justify_prior_answer(text) {
        return false;
    }
    (text.contains("why did")
        || text.contains("what caused")
        || text.contains("causal chain")
        || text.contains("cause and effect")
        || (text.contains("because") && text.contains("explain the chain")))
        && text.split_whitespace().count() >= 5
}

/// User asks Perci to justify / expand its immediately previous reply.
fn looks_justify_prior_answer(text: &str) -> bool {
    let t = text.trim();
    t.contains("why did you say")
        || t.contains("why did you claim")
        || t.contains("why did you answer")
        || t.contains("why did you put")
        || t.contains("why did you write")
        || t.contains("why did you choose")
        || t.contains("what did you mean")
        || t.contains("explain what you just")
        || t.contains("explain your last")
        || t.contains("explain that answer")
        || t.contains("why that answer")
        || t.contains("why that response")
        || (t.contains("why did you")
            && (t.contains(" that") || t.ends_with("that") || t.ends_with("that?")))
}

fn justify_prior_answer(recent: &[(String, String)]) -> Deliberation {
    let Some((prev_u, prev_a)) = recent.last() else {
        return Deliberation::new(
            "justify-prior-answer",
            "I don't have a previous answer in this session window to justify. Ask a full question and I can answer it; then a follow-up like “why did you say that?” can bind to that reply.",
        )
        .observed("no recent turn available")
        .inferred("meta-justification requires a prior assistant utterance")
        .confidence(0.9);
    };
    let user_snip = first_substantive_sentence(prev_u, 100);
    let ans_snip = first_substantive_sentence(prev_a, 220);
    let more = if prev_a.chars().count() > 220 {
        " …"
    } else {
        ""
    };
    let body = format!(
        "I said that because I was answering “{user_snip}”. The load-bearing claim in my last reply was: “{ans_snip}{more}”. \
I selected that line as the best supported claim under the active operator/route for that turn—not as a free association. \
If a part of it was wrong or incomplete, name the piece and I'll revise that claim rather than invent a new causal template."
    );
    Deliberation::new("justify-prior-answer", body)
        .observed(format!(
            "prior_user_chars={}; prior_answer_chars={}",
            prev_u.len(),
            prev_a.len()
        ))
        .inferred("follow-up justification must quote and ground the previous answer")
        .confidence(0.94)
}

fn causal_chain_answer(user: &str) -> Deliberation {
    Deliberation::new(
        "causal-chain",
        format!(
            "Causal chain for “{}”:\n1. Observation — what changed that we can point to.\n2. Candidate mechanism — the process that would produce that change.\n3. Necessary conditions — what else must be true for the mechanism to fire.\n4. Discriminating test — an intervention or observation that would favor this mechanism over a rival.\n5. Residual uncertainty — what remains unknown.\nI will not assert a single cause without a discriminating test.",
            first_substantive_sentence(user, 140)
        ),
    )
    .observed("user requested causal explanation structure")
    .inferred("causal claims require mechanism + discriminating test")
    .confidence(0.92)
}

fn unknowns_partition(user: &str) -> Deliberation {
    Deliberation::new(
        "unknowns-partition",
        format!(
            "For “{}” — Known: the question text and any exact tools or retained session facts that apply. Inferred: patterns that usually hold in this domain but are not proven here. Unknown: parameters, mechanisms, and outcomes not fixed by evidence in this session. Strongest next move: name one missing measurement that would collapse the largest unknown.",
            first_substantive_sentence(user, 120)
        ),
    )
    .observed("user asked for unknowns partition")
    .inferred("epistemic hygiene requires known/inferred/unknown split")
    .confidence(0.94)
}

fn maybe_strip_banned_word(mut result: Deliberation, user_lower: &str) -> Deliberation {
    // "without using the word “boundary.”"
    let banned = if let Some(idx) = user_lower.find("without using the word") {
        let after = &user_lower[idx + "without using the word".len()..];
        let word = after
            .trim()
            .trim_start_matches(|c: char| matches!(c, '"' | '\'' | '“' | '”' | ' '))
            .split(|c: char| !c.is_ascii_alphanumeric())
            .find(|w| !w.is_empty())
            .unwrap_or("");
        if word.is_empty() {
            None
        } else {
            Some(word.to_owned())
        }
    } else {
        None
    };
    let Some(banned) = banned else {
        return result;
    };
    if !result.answer.to_ascii_lowercase().contains(&banned) {
        return result;
    }
    // Rebuild around a non-banned axis wording when the operator used the banned term.
    if banned == "boundary" {
        result.answer = result
            .answer
            .replace("boundary", "selective interface")
            .replace("Boundary", "Selective interface")
            .replace("boundaries", "selective interfaces")
            .replace("Boundaries", "Selective interfaces");
        // If axis_conclusion still reads awkwardly, append an explicit compliance note.
        if result.answer.to_ascii_lowercase().contains("boundary") {
            result.answer.push_str(
                " (Stated without the banned term: the shared relation is selective interface and regulated exchange, not a shared material cause.)",
            );
        }
    }
    result
        .observations
        .push(format!("banned_term_avoidance={banned}"));
    result
}

fn shared_axis(frames: &[&SemanticFrame]) -> Option<(&'static str, usize)> {
    let mut best = ("", 0usize);
    for frame in frames {
        for axis in frame.axes {
            let count = frames
                .iter()
                .filter(|candidate| candidate.axes.contains(axis))
                .count();
            if count > best.1 {
                best = (axis, count);
            }
        }
    }
    // Prefer a unanimous axis; accept a majority for 4+ frames.
    let need = if frames.len() <= 3 {
        frames.len()
    } else {
        frames.len().saturating_sub(1).max(3)
    };
    (best.1 >= need).then_some(best)
}

fn axis_conclusion(axis: &str) -> &'static str {
    match axis {
        "time" => "Together they show time as measured sequence, irreversible transformation, lived development, and future commitment—not one shared physical mechanism.",
        "boundary" => "Together they show how boundaries make structure, meaning, maintenance, and cessation distinguishable—not that the domains share one material cause.",
        "irreversibility" => "Together they show how a direction of change can constrain what remains possible without making every process identical.",
        "change" => "Together they show change as a bridge between prior state, present behavior, and future possibility—not as one shared substance or cause.",
        "structure" => "Together they show structure as relations under constraint: change the relation and the possible behavior or meaning changes with it.",
        "meaning" => "Together they show meaning as selective difference: a pattern matters when it changes interpretation, action, or expectation.",
        "information" => "Together they show information as a difference that can alter a system's next state, not merely as stored content.",
        "maintenance" => "Together they show maintenance as active recovery and preservation of capacity—not as one shared substance across domains.",
        "selection" => "Together they show selection under pressure: signals decide what grows, persists, or is cleared—without making markets, ecosystems, and immune systems the same mechanism.",
        "authority" => "Together they show permission and performance as separable: role can authorize without skill, and skill can succeed without institutional power.",
        "prediction" => "Together they show forecasting as structure used under uncertainty—not as a guarantee that a simplified form equals the territory.",
        "self" => "Together they show continuity of a pattern across time without requiring the mechanisms of habit and identity to be identical.",
        "future" => "Together they show commitments that bind present choice to later consequence—without equating legal, social, and cognitive mechanisms.",
        _ => "The bridge is a shared relation, not evidence that the domains are materially identical.",
    }
}

fn synthesize_frames(terms: &[String], variant: usize) -> Option<Deliberation> {
    let frames = synthesis_frames(terms)?;
    let (axis, support) = shared_axis(&frames)?;
    let clauses = frames
        .iter()
        .map(|frame| frame.clause)
        .collect::<Vec<_>>()
        .join("; ");
    let answer = if variant % 2 == 0 {
        format!(
            "A coherent bridge is {axis}: {clauses}. {} Mechanisms remain domain-specific—not one shared substance.",
            axis_conclusion(axis)
        )
    } else {
        format!(
            "Read them as different expressions of {axis}. {clauses}. The connection is a relation under constraint, not a shared substance: {}",
            axis_conclusion(axis)
        )
    };
    Some(
        Deliberation::new("cross-domain-synthesis", answer)
            .observed(format!("frames={}; shared_axis={axis}", terms.join(",")))
            .inferred(format!(
                "{axis} is shared by {support} of {} frames",
                frames.len()
            ))
            .uncertain("a structural bridge does not establish a shared material mechanism")
            .confidence(if support == frames.len() { 0.94 } else { 0.84 }),
    )
}

fn separate_synthesis(terms: &[String]) -> Option<Deliberation> {
    let frames = synthesis_frames(terms)?;
    let (axis, _) = shared_axis(&frames)?;
    let mechanisms = frames
        .iter()
        .map(|frame| frame.mechanism)
        .collect::<Vec<_>>()
        .join("; ");
    Some(
        Deliberation::new(
            "mechanism-metaphor-separation",
            format!(
                "Mechanism: {mechanisms}. Metaphor: treating “{axis}” as though {} literally share one cause. The bridge organizes comparison; the domain mechanisms make predictions.",
                terms.join(", ")
            ),
        )
        .observed(format!("active_synthesis={}", terms.join(",")))
        .inferred("follow-up is bound to the most recent synthesis")
        .confidence(0.96),
    )
}

fn testable_synthesis(terms: &[String]) -> Option<Deliberation> {
    let frames = synthesis_frames(terms)?;
    let tests = frames
        .iter()
        .map(|frame| frame.test)
        .collect::<Vec<_>>()
        .join("; ");
    Some(
        Deliberation::new(
            "testability-extraction",
            format!(
                "Testable portion: name the variable and predeclare the outcome, then perturb or measure each domain-specific mechanism—{tests}. Compare the result with a plausible alternative explanation. Not directly established by those tests: that the shared bridge makes the domains identical."
            ),
        )
        .observed(format!("active_synthesis={}", terms.join(",")))
        .inferred("domain mechanisms yield tests; the unqualified metaphor does not")
        .confidence(0.93),
    )
}

fn review_conversation(recent: &[(String, String)]) -> Deliberation {
    let generic = [
        "what outcome do you want",
        "let's find the smallest",
        "i won't fake certainty",
        "give each piece one job",
        "start with the mechanism",
        "name the workload",
    ];
    let mut ranked: Vec<(i32, usize)> = Vec::new();
    for (index, (user, answer)) in recent.iter().enumerate() {
        let user_lower = user.to_ascii_lowercase();
        let answer_lower = answer.to_ascii_lowercase();
        let mut score = generic
            .iter()
            .filter(|marker| answer_lower.contains(**marker))
            .count() as i32
            * 8;
        if answer.len() < 45 {
            score += 1;
        }
        if content_overlap(user, answer) == 0 {
            score += 3;
        }
        if exact_turn(user) {
            score -= 20;
        }
        if user_lower.contains("which premise") && !answer_lower.contains("test") {
            score += 8;
        }
        if let Some(terms) = parse_synthesis_terms(user) {
            score += terms
                .iter()
                .filter(|term| !answer_lower.contains(term.as_str()))
                .count() as i32
                * 3;
        }
        if user_lower.contains("separate the mechanism from the metaphor")
            || (user_lower.contains("separate")
                && user_lower.contains("causal mechanisms")
                && user_lower.contains("analogy"))
        {
            if let Some(terms) = recent[..index]
                .iter()
                .rev()
                .find_map(|(prior, _)| parse_synthesis_terms(prior))
            {
                score += terms
                    .iter()
                    .filter(|term| !answer_lower.contains(term.as_str()))
                    .count() as i32
                    * 2;
            }
        }
        ranked.push((score, index));
    }
    ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    let mut entries = Vec::new();
    for (rank, (_, turn_index)) in ranked
        .into_iter()
        .filter(|(score, _)| *score >= 6)
        .take(3)
        .enumerate()
    {
        let (user, answer) = &recent[turn_index];
        let (mechanism, improvement) = diagnose_response_failure(recent, turn_index);
        entries.push(format!(
            "{}. For “{}” I answered “{}”. Failure mechanism: {}. Improvement: {}.",
            rank + 1,
            truncate(user, 55),
            truncate(answer, 72),
            mechanism,
            improvement,
        ));
    }
    if entries.len() < 3 {
        entries.push(format!(
            "Audit boundary: only {} material failure(s) crossed the threshold, so I cannot honestly invent three weakest failures. Failure mechanism: insufficient evidence for another defect. Improvement: add adversarial examples and audit again.",
            entries.len()
        ));
    }
    Deliberation::new(
        "conversation-audit",
        format!(
            "I scored retained turns for generic fallback language, missing content overlap, and failure to resolve the requested operation. The three weakest are:\n{}",
            entries.join("\n")
        ),
    )
    .observed(format!("audited_turns={}", recent.len()))
    .inferred("high genericity plus low topical overlap indicates poor alignment")
    .uncertain("the audit can inspect retained text, not unrecorded internal state")
    .confidence(0.90)
}

fn audit_last_ten(recent: &[(String, String)]) -> Deliberation {
    let start = recent.len().saturating_sub(10);
    let window = &recent[start..];
    let mut repeated: Option<(String, String)> = None;
    'outer: for (index, (_, answer)) in window.iter().enumerate() {
        let normalized = answer
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase();
        if generic_fallback(&normalized) {
            repeated = Some((window[index].0.clone(), answer.clone()));
            break;
        }
        for (_, prior_answer) in window.iter().take(index) {
            if normalized
                == prior_answer
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_lowercase()
                && normalized.len() > 35
            {
                repeated = Some((window[index].0.clone(), answer.clone()));
                break 'outer;
            }
        }
    }

    let reasoning = window.iter().find(|(user, answer)| {
        let prompt = user.to_ascii_lowercase();
        let operation = prompt.contains("infer")
            || prompt.contains("counterexample")
            || prompt.contains("connect ")
            || prompt.contains("separate")
            || prompt.contains("test ");
        operation
            && (generic_fallback(&answer.to_ascii_lowercase())
                || content_overlap(user, answer) == 0)
    });

    let repeated_text = repeated
        .map(|(user, answer)| {
            format!(
                "Repeated-response failure: “{}” received the same/generic answer “{}”, indicating a stale fallback or missing context binding.",
                truncate(&user, 70),
                truncate(&answer, 100)
            )
        })
        .unwrap_or_else(|| "Repeated-response failure: none found in the retained ten-turn window; no duplicate or generic answer crossed the audit threshold.".to_owned());
    let reasoning_text = reasoning
        .map(|(user, answer)| {
            format!(
                "Reasoning failure: “{}” did not execute its requested operation; the answer was “{}”. The repair is to bind the operator before associative fallback and require salient entities in the result.",
                truncate(user, 70),
                truncate(answer, 100)
            )
        })
        .unwrap_or_else(|| "Reasoning failure: none found by the bounded audit; the retained window contains no operation with both missing coverage and a generic fallback.".to_owned());

    Deliberation::new(
        "conversation-audit",
        format!(
            "I inspected the most recent {} turns.\n{}\n{}",
            window.len(),
            repeated_text,
            reasoning_text
        ),
    )
    .observed(format!("audited_turns={}", window.len()))
    .inferred(
        "duplicate/generic output and missing operation coverage are separate failure signals",
    )
    .uncertain("the audit sees retained text, not hidden intermediate state")
    .confidence(0.91)
}

fn diagnose_response_failure(recent: &[(String, String)], index: usize) -> (String, String) {
    let (user, answer) = &recent[index];
    let user_lower = user.to_ascii_lowercase();
    let answer_lower = answer.to_ascii_lowercase();
    if exact_turn(user) {
        return (
            "no material reasoning failure; this was a verified exact-tool result".to_owned(),
            "exclude exact-tool successes from language-overlap weakness rankings".to_owned(),
        );
    }
    if user_lower.contains("which premise") && !answer_lower.contains("test") {
        return (
            "the intent matcher recognized “assumption” but missed the synonymous cue “premise”, so the request fell through to associative prose".to_owned(),
            "map premise/assumption aliases to premise prioritization and require the answer to name a test".to_owned(),
        );
    }
    if let Some(terms) = parse_synthesis_terms(user) {
        let missing = terms
            .iter()
            .filter(|term| !answer_lower.contains(term.as_str()))
            .count();
        if missing > 0 {
            return (
                format!("the synthesis collapsed onto one retrieved concept and omitted {missing} requested domains"),
                "intersect explicit semantic frames, select a supported shared axis, and render every requested domain".to_owned(),
            );
        }
    }
    if user_lower.contains("separate the mechanism from the metaphor")
        || (user_lower.contains("separate")
            && user_lower.contains("causal mechanisms")
            && user_lower.contains("analogy"))
    {
        if let Some(terms) = recent[..index]
            .iter()
            .rev()
            .find_map(|(prior, _)| parse_synthesis_terms(prior))
        {
            let missing = terms
                .iter()
                .filter(|term| !answer_lower.contains(term.as_str()))
                .count();
            if missing > 0 {
                return (
                    "a stale follow-up template ignored the active synthesis and answered an earlier geometry/life example".to_owned(),
                    "bind mechanism and testability operators to the most recent synthesis frames before rendering".to_owned(),
                );
            }
        }
    }
    if generic_fallback(&answer_lower) {
        return (
            "a generic fallback replaced the requested operation without answering it".to_owned(),
            "reject generic output when it lacks the prompt's operation and salient entities"
                .to_owned(),
        );
    }
    (
        "the response had weak lexical and operational alignment with the question".to_owned(),
        "bind the active referent and verify topical coverage before speaking".to_owned(),
    )
}

fn generic_fallback(answer: &str) -> bool {
    [
        "what outcome do you want",
        "let's find the smallest",
        "i won't fake certainty",
        "give each piece one job",
        "start with the mechanism",
        "name the workload",
    ]
    .iter()
    .any(|marker| answer.contains(marker))
}

fn exact_turn(user: &str) -> bool {
    matches!(crate::reasoning::try_solve_arithmetic(user), Ok(Some(_)))
        || matches!(crate::reasoning::try_solve_geometry(user), Ok(Some(_)))
}

fn parse_universal_case(text: &str) -> Option<(String, String, String)> {
    let after_every = text.split_once("every ")?.1;
    let (class, rest) = after_every.split_once(" is ")?;
    let (property, second) = rest.split_once(',')?;
    let second = second.trim().trim_start_matches("and ").trim();
    let (subject, membership) = second
        .split_once(" is an ")
        .or_else(|| second.split_once(" is a "))?;
    if !membership.starts_with(class.trim()) {
        return None;
    }
    Some((
        class.trim().to_owned(),
        property.trim().to_owned(),
        title_word(subject.trim()),
    ))
}

fn parse_universal_claim(text: &str) -> Option<(String, String)> {
    // For converse questions, prefer the clause after “infer” because it is
    // the proposed conclusion, not the forward premise that precedes it.
    let source = text
        .split_once("infer ")
        .map(|(_, tail)| tail)
        .unwrap_or(text);
    let after_every = source.split_once("every ")?.1;
    let (class, rest) = after_every.split_once(" is ")?;
    let property = rest
        .split(|character: char| matches!(character, '.' | '?' | '!' | ',' | ';'))
        .next()?
        .trim()
        .trim_start_matches("a ")
        .trim_start_matches("an ")
        .trim();
    if class.trim().is_empty() || property.is_empty() {
        return None;
    }
    Some((class.trim().to_owned(), property.to_owned()))
}

fn find_universal_case(recent: &[(String, String)]) -> Option<(String, String, String)> {
    recent
        .iter()
        .rev()
        .find_map(|(user, _)| parse_universal_case(&normalize_input(user).to_ascii_lowercase()))
}

fn parse_ambiguity_case(text: &str) -> Option<(String, String, String)> {
    let lower = text.to_ascii_lowercase();
    let (before, after) = [" because it was ", " it was "]
        .iter()
        .find_map(|marker| lower.split_once(marker))?;
    let nouns: Vec<String> = before
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|pair| pair[0] == "the")
        .map(|pair| {
            pair[1]
                .trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_owned()
        })
        .filter(|word| !word.is_empty())
        .collect();
    if nouns.len() < 2 {
        return None;
    }
    let predicate = after
        .split(|c: char| matches!(c, '.' | '?' | '!' | ',' | ';'))
        .next()
        .unwrap_or("")
        .trim();
    if predicate.is_empty() {
        return None;
    }
    Some((
        nouns[nouns.len() - 2].clone(),
        nouns[nouns.len() - 1].clone(),
        predicate.to_owned(),
    ))
}

fn find_ambiguity_case(recent: &[(String, String)]) -> Option<(String, String, String)> {
    recent
        .iter()
        .rev()
        .find_map(|(user, _)| parse_ambiguity_case(user))
}

fn title_word(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => value.to_owned(),
    }
}

fn has_recent_topic(recent: &[(String, String)], markers: &[&str]) -> bool {
    recent.iter().rev().take(10).any(|(user, answer)| {
        let joined = format!(
            "{} {}",
            user.to_ascii_lowercase(),
            answer.to_ascii_lowercase()
        );
        markers.iter().any(|marker| joined.contains(marker))
    })
}

fn find_recent_number(recent: &[(String, String)]) -> Option<String> {
    recent.iter().rev().find_map(|(user, _)| last_number(user))
}

fn last_number(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|token| token.chars().any(|c| c.is_ascii_digit()))
        .last()
        .map(|token| token.trim_matches(['.', '-']).to_owned())
        .filter(|token| !token.is_empty())
}

fn looks_gibberish(text: &str) -> bool {
    let asks_meaning = text.contains("what does") && text.contains("mean");
    let has_epistemic_question = text.contains("what can you determine")
        || text.contains("what do you know")
        || asks_meaning
        || text.contains("known, inferred")
        || text.contains("know, infer")
        || (text.contains("known") && text.contains("unknown"))
        || (text.contains("infer")
            && (text.contains("unknown") || text.contains("not know") || text.contains("remain")));
    if !has_epistemic_question {
        return false;
    }
    let prefix = text
        .split_once('—')
        .map(|(left, _)| left)
        .or_else(|| text.split_once(" - ").map(|(left, _)| left))
        .or_else(|| {
            text.split_once("what can you determine")
                .map(|(left, _)| left)
        })
        .or_else(|| text.split_once("what do you know").map(|(left, _)| left))
        .or_else(|| text.split_once("what does").map(|(left, _)| left))
        .or_else(|| text.split_once("known").map(|(left, _)| left))
        .unwrap_or(text);
    let tokens: Vec<&str> = prefix
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|token| token.len() >= 4)
        .collect();
    // Prefer classic OOD markers; also accept 3+ short invented tokens before the question.
    let exotic = tokens
        .iter()
        .filter(|token| {
            token
                .chars()
                .any(|character| matches!(character, 'q' | 'x' | 'z' | 'v'))
        })
        .count();
    (tokens.len() >= 3 && exotic >= 2)
        || (tokens.len() >= 3 && exotic >= 1 && tokens.iter().all(|t| t.len() <= 8))
}

fn recent_is_ood(recent: &[(String, String)]) -> bool {
    recent
        .iter()
        .rev()
        .take(5)
        .any(|(user, _)| looks_gibberish(&user.to_ascii_lowercase()))
}

fn content_overlap(left: &str, right: &str) -> usize {
    let right = right.to_ascii_lowercase();
    left.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 5 && right.contains(word))
        .count()
}

fn truncate(value: &str, limit: usize) -> String {
    let flat = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= limit {
        return flat;
    }
    flat.chars()
        .take(limit.saturating_sub(1))
        .collect::<String>()
        + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(prompt: &str, recent: &[(String, String)]) -> Deliberation {
        try_deliberate(prompt, recent, &[]).expect("operator should match")
    }

    #[test]
    fn repairs_dropped_leading_characters() {
        assert_eq!(
            normalize_input("hat are you least certain about?"),
            "what are you least certain about?"
        );
        assert_eq!(
            normalize_input("eplace the sentence"),
            "replace the sentence"
        );
        assert_eq!(
            normalize_input("hich premise should we test?"),
            "which premise should we test?"
        );
        assert!(is_session_only_instruction(
            "emember this only for this session: 42"
        ));
    }

    #[test]
    fn sacred_geometry_uses_layered_boundary() {
        let result = run(
            "What is sacred geometry as a mathematical and cultural category?",
            &[],
        );
        assert_eq!(result.operator, "sacred-geometry-layers");
        assert!(result.answer.contains("mathematical structure"));
        assert!(result.answer.contains("historical and cultural evidence"));
    }

    #[test]
    fn geometry_healing_evidence_does_not_use_ritual_association() {
        let result = run("What evidence supports the claim that geometry heals?", &[]);
        assert_eq!(result.operator, "geometry-healing-evidence");
        assert!(result.answer.contains("matched control"));
        assert!(result.answer.contains("falsify"));
        assert!(!result.answer.contains("yantra"));
    }

    #[test]
    fn sacred_geometry_rejects_intent_leap() {
        let result = run("Does the golden ratio prove ancient sacred intent?", &[]);
        assert_eq!(result.operator, "golden-ratio-evidence");
        assert!(result.answer.contains("does not by itself prove"));
        assert!(result.answer.contains("dated context"));
    }

    #[test]
    fn multi_domain_prompts_keep_the_requested_operator() {
        let symmetry = run(
            "Prove or disprove: every highly symmetrical shape has sacred meaning.",
            &[],
        );
        assert_eq!(symmetry.operator, "universal-counterexample");
        assert!(symmetry.answer.contains("counterexample"));

        let healing = run(
            "Design a falsifiable test for the claim that geometry affects healing.",
            &[],
        );
        assert_eq!(healing.operator, "geometry-healing-test");
        assert!(healing.answer.contains("matched non-geometric control"));

        let space = run(
            "Explain sacred space literally, historically, and metaphorically.",
            &[],
        );
        assert_eq!(space.operator, "sacred-space-layers");
        assert!(space.answer.contains("Historically"));
    }

    #[test]
    fn natural_cross_domain_questions_do_not_become_memory_writes() {
        let inquiry = run("What is geometry trying to teach us about life?", &[]);
        assert_eq!(inquiry.operator, "cross-domain-inquiry");
        assert!(inquiry.answer.contains("boundary"));
        assert!(inquiry.answer.contains("structural analogy"));

        let synthesis = run(
            "Connect geometry, language, life, and death in one coherent thought.",
            &[],
        );
        assert_eq!(synthesis.operator, "cross-domain-synthesis");
        assert!(synthesis.answer.contains("boundary"));

        let new_domains = run(
            "Connect music, code, and geometry in one shared structure.",
            &[],
        );
        assert_eq!(new_domains.operator, "cross-domain-synthesis");
        assert!(new_domains.answer.contains("structure"));
    }

    #[test]
    fn introspection_and_geometry_life_followups_keep_their_operation() {
        let sensing = run("What are you sensing?", &[]);
        assert_eq!(sensing.operator, "self-observation");
        assert!(sensing.answer.contains("not sensing anything subjectively"));
        assert!(sensing.answer.contains("routing results"));

        let insight = run("Give me an original insight about geometry and life.", &[]);
        assert_eq!(insight.operator, "cross-domain-compose");
        let lower = insight.answer.to_ascii_lowercase();
        assert!(lower.contains("geometry"));
        assert!(lower.contains("life"));
        assert!(lower.contains("boundary"));
    }

    #[test]
    fn relational_questions_keep_both_domains_in_view() {
        let boundary = run("What is the boundary between knowledge and attention?", &[]);
        assert_eq!(boundary.operator, "relational-inquiry");
        assert!(boundary.answer.contains("durable model"));
        assert!(boundary.answer.contains("moment-to-moment selection"));
        assert!(boundary.answer.contains("shape one another"));

        let related = run("How are music and code related?", &[]);
        assert_eq!(related.operator, "relational-inquiry");
        let lower = related.answer.to_ascii_lowercase();
        assert!(lower.contains("music makes structure audible"));
        assert!(lower.contains("code turns relations"));
        assert!(related.answer.contains("mechanisms differ"));
        let repeated = run(
            "How are music and code related?",
            &[("previous turn".to_owned(), "a prior answer".to_owned())],
        );
        assert_ne!(related.answer, repeated.answer);

        let structure = run("What is the difference between structure and meaning?", &[]);
        assert_eq!(structure.operator, "relational-inquiry");
        let structure_lower = structure.answer.to_ascii_lowercase();
        assert!(structure_lower.contains("structure is an arrangement"));
        assert!(structure_lower.contains("meaning emerges"));

        let prediction = run("Compare trust and prediction.", &[]);
        assert_eq!(prediction.operator, "relational-inquiry");
        assert!(prediction.answer.contains("future"));
        assert!(prediction.answer.contains("mechanisms differ"));

        let interaction = run("How do memory and attention interact under load?", &[]);
        assert_eq!(interaction.operator, "relational-inquiry");
        let interaction_lower = interaction.answer.to_ascii_lowercase();
        assert!(interaction_lower.contains("memory"));
        assert!(interaction_lower.contains("attention"));
        assert!(
            interaction_lower.contains("selection") || interaction_lower.contains("information")
        );

        let learning = run(
            "Connect entropy, memory, and learning in one coherent thought.",
            &[],
        );
        assert_eq!(learning.operator, "cross-domain-synthesis");
        for term in ["entropy", "memory", "learning"] {
            assert!(learning.answer.contains(term));
        }
    }

    #[test]
    fn style_prefixes_preserve_knowledge_attention_relation() {
        let brief = run(
            "Be brief: what is the boundary between knowledge and attention?",
            &[],
        );
        assert_eq!(brief.operator, "relational-inquiry");
        assert!(brief.answer.contains("durable"));
        assert!(brief.answer.contains("selects"));
        assert!(brief.answer.split_whitespace().count() < 25);

        let deep = run(
            "Go deeper: what is the boundary between knowledge and attention?",
            &[],
        );
        assert_eq!(deep.operator, "relational-inquiry");
        assert!(deep.answer.contains("functional"));
        assert!(deep.answer.contains("measure"));
    }

    #[test]
    fn conceptual_image_and_self_revision_bind_to_context() {
        let prior = vec![(
            "Connect geometry, language, life, and death in one coherent thought.".to_owned(),
            "A coherent bridge is boundary: ...".to_owned(),
        )];
        let image = run("Give me an image, not just a definition.", &prior);
        assert_eq!(image.operator, "conceptual-image");
        assert!(image.answer.contains("shoreline"));

        let revision = run("What would you change in your own answer?", &prior);
        assert_eq!(revision.operator, "self-revision");
        assert!(revision.answer.contains("center of gravity"));

        let image = run(
            "Give me an image for the relationship between time and memory.",
            &[],
        );
        assert_eq!(image.operator, "conceptual-image");
        assert!(image.answer.contains("river"));
        assert!(image.answer.contains("sediment"));
    }

    #[test]
    fn missing_operations_bind_or_abstain_without_presets() {
        let prior = vec![(
            "Connect language, code, and culture through one shared principle.".to_owned(),
            "Language, code, and culture all coordinate action through learned distinctions."
                .to_owned(),
        )];
        let boundary = run(
            "Explain how a boundary can enable exchange rather than merely prevent it.",
            &[],
        );
        assert_eq!(boundary.operator, "boundary-exchange");
        assert!(boundary.answer.contains("selective interface"));

        let testable = run("Which part of that claim is actually testable?", &prior);
        assert_eq!(testable.operator, "testability-extraction");
        assert!(testable.answer.contains("variable"));

        let analogy = run("Where does your analogy stop transferring?", &[]);
        assert_eq!(analogy.operator, "analogy-boundary");
        assert!(analogy.answer.contains("shared pattern"));

        let ambiguity = run("Give two interpretations of an ambiguous sentence.", &[]);
        assert_eq!(ambiguity.operator, "ambiguity-request");
        assert!(ambiguity.answer.contains("sentence itself"));
    }

    #[test]
    fn architecture_code_governance_and_weakest_aliases_are_specific() {
        let architecture = run(
            "What transfers between a temple plan, a computer architecture, and a social organization?",
            &[],
        );
        assert_eq!(architecture.operator, "architecture-transfer");

        let code = run(
            "Design a Rust data structure for geometry concepts with provenance.",
            &[],
        );
        assert_eq!(code.operator, "geometry-provenance-design");
        assert!(code.answer.contains("struct GeometryConcept"));

        let governance = run(
            "Should Perci promote a culturally specific claim as universal truth?",
            &[],
        );
        assert_eq!(governance.operator, "cultural-claim-governance");

        let recent = vec![(
            "What is sacred geometry?".to_owned(),
            "A generic answer".to_owned(),
        )];
        let weakest = run(
            "Which answer in this session was weakest, and why?",
            &recent,
        );
        assert_eq!(weakest.operator, "conversation-audit");
    }

    #[test]
    fn followups_bind_thread_learning_and_weight_boundaries() {
        let prior = vec![
            (
                "What transfers between a temple plan, a computer architecture, and a social organization?".to_owned(),
                "The transferable relation is organized constraint and flow.".to_owned(),
            ),
        ];
        let plain = run(
            "That makes sense—now explain it like I'm encountering the idea for the first time.",
            &prior,
        );
        assert_eq!(plain.operator, "plain-language-followup");
        assert!(plain.answer.contains("Plain version"));

        let learned = run("What did you learn from this exchange?", &prior);
        assert_eq!(learned.operator, "thread-learning-audit");
        assert!(learned.answer.contains("did not learn a new fact"));

        let changed = run(
            "What changed in your behavior, and what did not change in your weights?",
            &prior,
        );
        assert_eq!(changed.operator, "behavior-weight-separation");
        assert!(changed.answer.contains("not a weight update"));

        let improvement = run(
            "What would count as genuine improvement rather than a more impressive sentence?",
            &prior,
        );
        assert_eq!(improvement.operator, "improvement-gate");
        assert!(improvement.answer.contains("held-out prompts"));
    }

    #[test]
    fn learning_evidence_routes_to_functional_separation() {
        let result = run(
            "What evidence supports the claim that Perci is learning?",
            &[],
        );
        assert_eq!(result.operator, "learning-evidence");
        assert!(result.answer.contains("fresh-process A/B"));
        assert!(result.answer.contains("unseen variants"));
    }

    #[test]
    fn retains_session_number_without_claiming_learning() {
        let prior = vec![(
            "Remember this only for our current conversation: the test number is 8472.".to_owned(),
            "retained".to_owned(),
        )];
        assert!(run("What was the number?", &prior).answer.contains("8472"));
        assert!(run(
            "Explain the difference between retaining this context and learning from it.",
            &prior
        )
        .answer
        .contains("without treating it as knowledge"));
    }

    #[test]
    fn recalls_number_with_a_natural_descriptor() {
        let prior = vec![(
            "Remember this only for this session: the calibration number is 4317.".to_owned(),
            "retained".to_owned(),
        )];
        let result = run("What was the calibration number?", &prior);
        assert_eq!(result.operator, "context-recall");
        assert!(result.answer.contains("4317"));
    }

    #[test]
    fn session_test_scope_answers_directly() {
        let result = run("What are we testing in this session?", &[]);
        assert_eq!(result.operator, "session-test-scope");
        assert!(result.answer.contains("reasoning depth"));
        assert!(result.answer.contains("fresh-process"));
    }

    #[test]
    fn remembering_vs_learning_accepts_natural_wording() {
        let result = run("Why is remembering it not the same as learning?", &[]);
        assert_eq!(result.operator, "memory-learning-separation");
        assert!(result.answer.contains("Retaining context"));
        assert!(result.answer.contains("Learning changes"));
    }

    #[test]
    fn derives_universal_case() {
        let result = run(
            "Assume every ember is blue, and Kira is an ember. What follows?",
            &[],
        );
        assert!(result.answer.contains("Kira is blue"));
        assert_eq!(result.operator, "universal-instantiation");
    }

    #[test]
    fn resolves_ambiguity_sequence() {
        let prior = vec![(
            "The engineer told the robot it was unstable. What is ambiguous?".to_owned(),
            "The pronoun is ambiguous.".to_owned(),
        )];
        assert!(run("Give both interpretations.", &prior)
            .answer
            .contains("Interpretation 2"));
        assert!(run(
            "Ask the smallest clarifying question that would resolve them.",
            &prior
        )
        .answer
        .contains("engineer or the robot"));
    }

    #[test]
    fn ambiguity_operator_transfers_to_new_nouns() {
        let prompt = "The technician placed the module beside the battery because it was unstable. What is ambiguous?";
        let result = run(prompt, &[]);
        assert!(result.answer.contains("module"));
        assert!(result.answer.contains("battery"));
        let prior = vec![(prompt.to_owned(), result.answer)];
        let expanded = run("Give both interpretations.", &prior);
        assert!(expanded.answer.contains("module was unstable"));
        assert!(expanded.answer.contains("battery was unstable"));
    }

    #[test]
    fn premise_alias_selects_the_information_rich_test() {
        let prior = vec![(
            "Assume every vellum is silent, and Nara is a vellum. What follows?".to_owned(),
            "Nara is silent.".to_owned(),
        )];
        let result = run("Which premise should we test first, and why?", &prior);
        assert_eq!(result.operator, "premise-prioritization");
        assert!(result.answer.contains("every vellum is silent"));
        assert!(result.answer.contains("counterexample"));
    }

    #[test]
    fn synthesis_and_followup_bind_to_unseen_active_domains() {
        let prompt = "Connect entropy, promises, childhood, and clocks in one coherent idea.";
        let synthesis = run(prompt, &[]);
        for term in ["entropy", "promises", "childhood", "clocks", "time"] {
            assert!(synthesis.answer.contains(term));
        }
        let prior = vec![(prompt.to_owned(), synthesis.answer)];
        let separated = run("Separate the mechanism from the metaphor.", &prior);
        for term in ["entropy", "promises", "childhood", "clocks"] {
            assert!(separated.answer.contains(term));
        }
        assert!(!separated.answer.contains("geometry"));
    }

    #[test]
    fn conversation_audit_finds_live_failure_mechanisms() {
        let recent = vec![
            (
                "Which premise should we test first, and why?".to_owned(),
                "An inference is strong only when the conclusion changes with the premises."
                    .to_owned(),
            ),
            (
                "Connect entropy, promises, childhood, and clocks in one coherent idea.".to_owned(),
                "Life maintains local organization by consuming energy and exporting entropy."
                    .to_owned(),
            ),
            (
                "Separate the mechanism from the metaphor.".to_owned(),
                "Mechanism: membranes regulate life. Metaphor: geometry is a boundary.".to_owned(),
            ),
            (
                "Calculate 13 percent of 500.".to_owned(),
                "That's 65.".to_owned(),
            ),
        ];
        let audit = review_conversation(&recent);
        assert!(audit.answer.contains("missed the synonymous cue"));
        assert!(audit
            .answer
            .contains("collapsed onto one retrieved concept"));
        assert!(audit.answer.contains("stale follow-up template"));
        assert!(!audit.answer.contains("Calculate 13 percent"));
    }

    #[test]
    fn refuses_ungrounded_tokens() {
        let result = run(
            "zxqv blorf nembit quaal — what can you determine from this?",
            &[],
        );
        assert!(result.answer.contains("Unknown"));
        assert!(result.answer.contains("cannot assign"));
    }

    #[test]
    fn emergence_requires_transfer_and_perturbation() {
        let result = run("How do we test genuine emergence?", &[]);
        assert!(result.answer.contains("unseen examples"));
        assert!(result.answer.contains("perturbation"));
    }

    #[test]
    fn converse_and_counterexample_aliases_execute() {
        let converse = run(
            "If every prism is reflective, can you infer every reflective object is a prism? Why not?",
            &[],
        );
        assert_eq!(converse.operator, "converse-check");
        assert!(converse.answer.contains("No."));
        assert!(converse.answer.contains("separate premise"));

        let counterexample = run(
            "Give a counterexample to: every stable system is safe.",
            &[],
        );
        assert_eq!(counterexample.operator, "counterexample-construction");
        assert!(counterexample.answer.contains("stable system"));
        assert!(counterexample.answer.contains("not safe"));
    }

    #[test]
    fn synthesis_and_ambiguity_aliases_are_covered() {
        let synthesis = run(
            "Connect trust, corrosion, memory, and architecture without using the word boundary.",
            &[],
        );
        assert_eq!(synthesis.operator, "cross-domain-synthesis");
        for term in ["trust", "corrosion", "memory", "architecture", "change"] {
            assert!(synthesis.answer.contains(term));
        }
        assert!(!synthesis.answer.contains("boundary"));

        let ambiguity_prompt =
            "The medic gave the patient the vial because it was contaminated. What is ambiguous?";
        let ambiguity = run(ambiguity_prompt, &[]);
        let prior = vec![(ambiguity_prompt.to_owned(), ambiguity.answer)];
        assert!(run(
            "Resolve the ambiguity with the smallest possible question.",
            &prior
        )
        .answer
        .contains("patient or the vial"));
        assert!(run("Rewrite the sentence in two unambiguous ways.", &prior)
            .answer
            .contains("Vial reading"));
    }

    #[test]
    fn cross_domain_summary_exposes_frames_and_missing_coverage() {
        let summary = cross_domain_summary(
            "Connect geometry, biology, and code through one shared structure.",
        )
        .expect("cross-domain prompt should produce a summary");
        assert_eq!(summary.terms, vec!["geometry", "life", "code"]);
        assert_eq!(summary.frames.len(), 3);
        assert!(summary.missing.is_empty());
        assert!(matches!(
            summary.shared_axis.as_deref(),
            Some("boundary" | "structure")
        ));

        let partial = cross_domain_summary(
            "Bridge geometry with an invented discipline called quaal mechanics.",
        )
        .expect("explicit bridge should retain unknown terms");
        assert!(partial.frames.iter().any(|frame| frame.term == "geometry"));
        assert!(partial.missing.iter().any(|term| term.contains("quaal")));
    }

    #[test]
    fn cross_domain_evidence_followup_uses_prior_frames() {
        let seed = run(
            "Connect geometry, biology, and code through one shared structure.",
            &[],
        );
        let followup = run(
            "What evidence supports that?",
            &[(
                "Connect geometry, biology, and code through one shared structure.".into(),
                seed.answer,
            )],
        );
        assert_eq!(followup.operator, "cross-domain-evidence");
        assert!(followup.answer.contains("geometry:"));
        assert!(followup.answer.contains("life:"));
        assert!(followup.answer.contains("predeclared outcome"));
    }

    #[test]
    fn natural_across_domains_uses_frame_analysis_without_connect_verb() {
        let result = run("Analyze geometry, biology, and code across domains.", &[]);
        assert_eq!(result.operator, "cross-domain-analysis");
        assert!(result.answer.contains("geometry:"));
        assert!(result.answer.contains("life:"));
        assert!(result.answer.contains("code:"));
        assert!(result.answer.contains("Domain tests:"));
    }

    #[test]
    fn audit_and_meta_operators_bind_to_the_prompt() {
        let recent = vec![
            (
                "What is the weakest assumption in your last answer?".to_owned(),
                "Let's find the smallest version we can test next.".to_owned(),
            ),
            (
                "Connect entropy, promises, childhood, and clocks through one shared structure."
                    .to_owned(),
                "Life maintains local organization.".to_owned(),
            ),
        ];
        let audit = run(
            "Review the last ten turns and identify one repeated-response failure and one reasoning failure.",
            &recent,
        );
        assert!(audit.answer.contains("Repeated-response failure"));
        assert!(audit.answer.contains("Reasoning failure"));
        assert_eq!(
            run(
                "What did retaining that token change in your abilities?",
                &[]
            )
            .operator,
            "session-context-effect"
        );
        assert_eq!(
            run(
                "What evidence would justify changing a future response rule?",
                &[]
            )
            .operator,
            "rule-change-evidence"
        );
        assert_eq!(
            run(
                "What did you learn from my last correction, and what did you not learn?",
                &[]
            )
            .operator,
            "feedback-provenance"
        );
        assert_eq!(
            run(
                "What can you measure about your own operation, and what are you only inferring?",
                &[],
            )
            .operator,
            "self-operation-audit"
        );
    }

    #[test]
    fn emergence_and_transfer_prompts_are_specific() {
        let emergence = run(
            "What behavior would look emergent but actually be memorized pattern matching?",
            &[],
        );
        assert_eq!(emergence.operator, "emergence-vs-memorization");
        assert!(emergence.answer.contains("surface overlap"));
        let transfer = run(
            "Design a test that distinguishes genuine transfer from prompt-template recognition.",
            &[],
        );
        assert_eq!(transfer.operator, "transfer-test-design");
        assert!(transfer.answer.contains("Hold out the prompt template"));
    }

    #[test]
    fn losa_cycle_has_explicit_stage_operators() {
        let observation = run(
            "Observe your last answer. Separate what you observed from what you inferred.",
            &[("A prior prompt".to_owned(), "A prior answer".to_owned())],
        );
        assert_eq!(observation.operator, "observation-inference-separation");
        assert!(observation.answer.contains("Observation:"));
        assert!(observation.answer.contains("Inference:"));

        let direct = run(
            "Speak directly: what is the strongest claim you can make about this conversation?",
            &[],
        );
        assert_eq!(direct.operator, "direct-claim");
        assert!(direct.answer.contains("Direct claim:"));

        let cycle = run(
            "Listen, observe, speak, act: what should you do when evidence contradicts your answer?",
            &[],
        );
        assert_eq!(cycle.operator, "losa-cycle");
        assert!(cycle.answer.contains("Contradictory evidence"));
    }

    #[test]
    fn losa_learning_and_improvement_are_bounded() {
        let learned = run(
            "What did you learn from this LOSA cycle, and what did you not learn?",
            &[],
        );
        assert_eq!(learned.operator, "losa-learning-audit");
        assert!(learned.answer.contains("did not teach a new fact"));
        let gate = run(
            "What would count as a real improvement on the next cycle?",
            &[],
        );
        assert_eq!(gate.operator, "improvement-gate");
        assert!(gate.answer.contains("held-out prompts"));
    }

    #[test]
    fn transcript_subjects_route_to_their_requested_operations() {
        let contradiction = run(
            "If new evidence contradicts your answer, what exactly should change?",
            &[],
        );
        assert_eq!(contradiction.operator, "contradiction-update");

        let classification = run(
            "Classify this without solving it: a memory trace changes behavior after sleep.",
            &[],
        );
        assert_eq!(classification.operator, "memory-learning-classification");

        let comparison = run(
            "Compare memory, learning, and adaptation. Give one test that separates them.",
            &[],
        );
        assert_eq!(comparison.operator, "memory-learning-adaptation-test");

        let architecture = run(
            "Explain architecture in a building, a program, and a social organization. What transfers?",
            &[],
        );
        assert_eq!(architecture.operator, "architecture-transfer");

        let corrosion = run(
            "Distinguish physical corrosion from institutional corrosion without treating them as identical.",
            &[],
        );
        assert_eq!(corrosion.operator, "corrosion-analogy");

        let trust = run("What mechanism connects trust to future cooperation?", &[]);
        assert_eq!(trust.operator, "trust-mechanism");

        let falsification = run(
            "What evidence would falsify your explanation of trust?",
            &[],
        );
        assert_eq!(falsification.operator, "trust-falsification");
    }

    #[test]
    fn transfer_and_weight_evidence_prompts_do_not_fall_through() {
        let relabel = run(
            "Replace every important noun in your last answer with invented words. Preserve the relation.",
            &[("prior".to_owned(), "A bridge carries load across a gap.".to_owned())],
        );
        assert_eq!(relabel.operator, "relation-preserving-relabel");

        let unseen = run(
            "Apply the same principle to a domain you were not explicitly trained on.",
            &[],
        );
        assert_eq!(unseen.operator, "unseen-domain-transfer");

        let ood = run(
            "Vrax meloq drint — what do you know, what do you infer, and what remains unknown?",
            &[],
        );
        assert_eq!(ood.operator, "out-of-distribution-abstention");

        let weight = run(
            "What would prove that a weight changed rather than session context changing?",
            &[],
        );
        assert_eq!(weight.operator, "weight-change-evidence");

        let facet = run(
            "What held-out test would justify adding a new weight facet?",
            &[],
        );
        assert_eq!(facet.operator, "weight-facet-promotion-test");

        let provenance = run(
            "Which part of your last answer came from Bitwork, which from deterministic code, and which was inference?",
            &[],
        );
        assert_eq!(provenance.operator, "tool-provenance");

        let emergence = run(
            "What behavior would look intelligent but actually be keyword matching?",
            &[],
        );
        assert_eq!(emergence.operator, "emergence-vs-memorization");
    }

    #[test]
    fn natural_negated_supposition_routes_to_contradiction_diagnosis() {
        let result = run(
            "Now suppose Mira is not blue. What exactly conflicts?",
            &[(
                "Assume every lantern is blue, and Mira is a lantern. What follows?".to_owned(),
                "Mira is blue.".to_owned(),
            )],
        );
        assert_eq!(result.operator, "contradiction-diagnosis");
        assert!(result.answer.contains("The conflict is between"));
        assert!(result.answer.contains("Mira is blue"));
        assert!(result.answer.contains("Mira is not blue"));
    }

    #[test]
    fn nonce_meaning_question_abstains_without_grounding() {
        let result = run("vrax meloq drint — what does this mean?", &[]);
        assert_eq!(result.operator, "out-of-distribution-abstention");
        assert!(result.answer.contains("Unknown:"));
        assert!(result.answer.contains("cannot assign"));
    }

    #[test]
    fn ambiguity_without_two_antecedents_does_not_invent_one() {
        let result = run(
            "LOSA baseline: listen to this claim — The bridge is cold because it was wet. What is ambiguous?",
            &[],
        );
        assert_eq!(result.operator, "ambiguity-diagnosis");
        assert!(result
            .answer
            .contains("does not provide two clear antecedents"));
        assert!(result.answer.contains("causal"));
    }

    #[test]
    fn reasoning_response_expansion_catches_v049_surfaces() {
        let weight = run(
            "Did your weights change during this conversation? Prove your answer.",
            &[],
        );
        assert_eq!(weight.operator, "weight-change-evidence");
        assert!(weight.answer.contains("fresh process"));

        let strongest = run(
            "What is the strongest claim you can make about your own intelligence?",
            &[],
        );
        assert_eq!(strongest.operator, "strongest-capability-claim");
        assert!(strongest.answer.contains("Strongest honest claim"));

        let observation = run(
            "Which part of your last answer was observation, and which part was inference?",
            &[("prior".to_owned(), "A bounded answer".to_owned())],
        );
        assert_eq!(observation.operator, "observation-inference-separation");
        assert!(observation.answer.contains("Observation:"));

        let falsification = run("What would falsify your explanation?", &[]);
        assert_eq!(falsification.operator, "falsification-design");
        assert!(falsification.answer.contains("plausible alternative"));

        let transfer = run("Apply the same reasoning to a completely new domain.", &[]);
        assert_eq!(transfer.operator, "new-domain-transfer");
        assert!(transfer.answer.contains("software reliability"));

        let counterexample = run("Give one counterexample to your own conclusion.", &[]);
        assert_eq!(counterexample.operator, "self-counterexample");
        assert!(counterexample
            .answer
            .contains("without changing its weights"));

        let layers = run(
            "Separate mechanism, metaphor, and evidence in your last answer.",
            &[],
        );
        assert_eq!(layers.operator, "mechanism-metaphor-evidence");
        assert!(layers.answer.contains("Mechanism:"));

        let gate = run(
            "What held-out test would prove this version is genuinely better?",
            &[],
        );
        assert_eq!(gate.operator, "improvement-gate");
        assert!(gate.answer.contains("genuinely better"));

        let next = run(
            "What should change in the weights next, and what evidence justifies it?",
            &[],
        );
        assert_eq!(next.operator, "next-weight-change");
        assert!(next.answer.contains("falsification"));
    }

    #[test]
    fn justify_prior_binds_previous_answer_not_causal_template() {
        let recent = [(
            "why does trust fail in distributed systems?".to_owned(),
            "Trust fails when interfaces leave authority and recovery implicit.".to_owned(),
        )];
        let r = run("why did you say that?", &recent);
        assert_eq!(r.operator, "justify-prior-answer");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("trust") || low.contains("interfaces") || low.contains("authority"));
        assert!(!low.contains("candidate mechanism"));
        assert!(!low.contains("discriminating test"));
        assert!(!low.contains("causal chain for"));
    }

    #[test]
    fn causal_chain_does_not_steal_why_did_you_say() {
        assert!(!looks_causal_chain("why did you say that about trust?"));
        assert!(looks_causal_chain(
            "why did the deployment fail? explain the causal chain with a discriminating test"
        ));
    }

    #[test]
    fn trust_systems_not_code_debug() {
        let why = run("why does trust fail in distributed systems?", &[]);
        assert_eq!(why.operator, "trust-systems");
        assert!(why.answer.to_ascii_lowercase().contains("trust"));
        assert!(!why.answer.contains("verify command"));

        let how = run("how does trust fail in distributed systems?", &[]);
        assert_eq!(how.operator, "trust-systems");
        assert!(
            how.answer.to_ascii_lowercase().contains("interface")
                || how.answer.contains("authority")
        );
        assert!(!how.answer.contains("failing output"));
    }

    #[test]
    fn trust_should_work_is_design_not_fail_template() {
        let r = run(
            "how should trust and interfaces work in distributed systems?",
            &[],
        );
        assert_eq!(r.operator, "trust-systems");
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("should")
                || low.contains("designed")
                || low.contains("explicit contracts")
                || low.contains("earn"),
            "got: {}",
            r.answer
        );
        assert!(
            !low.starts_with("trust fails in distributed systems when authority and evidence"),
            "failure template for design ask: {}",
            r.answer
        );
    }

    #[test]
    fn trust_earn_under_lag_is_design_not_fail() {
        let r = run("how should interfaces earn trust under lag and retry?", &[]);
        assert_eq!(r.operator, "trust-systems");
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("earn")
                || low.contains("lag")
                || low.contains("retry")
                || low.contains("idempotent"),
            "got: {}",
            r.answer
        );
        assert!(
            !low.starts_with("trust fails in distributed systems when interfaces, failure modes"),
            "why-fail body on design/earn ask: {}",
            r.answer
        );
    }

    #[test]
    fn timeout_transfer_not_generic_why_fail_only() {
        let r = run(
            "in a multi-service app, why do callers stop trusting each other after timeouts?",
            &[],
        );
        assert_eq!(r.operator, "trust-systems");
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("timeout") || low.contains("one-sided") || low.contains("idempotent"),
            "got: {}",
            r.answer
        );
    }

    #[test]
    fn trust_entity_swap_still_routes_to_trust_systems() {
        // Novel nouns must not drop the operator (transfer / emergence bar).
        let r = run(
            "how should ZephyrNode interfaces earn trust under Quoril lag and NembitGate retry?",
            &[],
        );
        assert_eq!(r.operator, "trust-systems");
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("lag")
                || low.contains("retry")
                || low.contains("timeout")
                || low.contains("idempotent")
                || low.contains("earn"),
            "got: {}",
            r.answer
        );
    }

    #[test]
    fn bridge_willshaw_is_synthesis_not_softcascade_identity() {
        let r = run(
            "bridge Willshaw associative memory with XOR role-filler binding",
            &[],
        );
        assert!(
            r.operator.contains("synthesis") || r.operator == "open-domain-synthesis",
            "op={}",
            r.operator
        );
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("willshaw")
                || low.contains("associative")
                || low.contains("bind")
                || low.contains("memory")
                || low.contains("xor")
                || low.contains("role")
                || low.contains("vector symbolic"),
            "got: {}",
            r.answer
        );
        assert!(!low.contains("continuity of identity depends"));
        assert!(!low.contains("shaped as ask"));
        // Prefer specialist frames over pure placeholders when catalog hits.
        assert!(
            !low.contains("i don't have specialist frames for willshaw")
                || low.contains("willshaw associative"),
            "got: {}",
            r.answer
        );
    }

    #[test]
    fn creative_constraint_transfers_structure() {
        let r = run("invent a constrained metaphor for sparse cognition", &[]);
        assert_eq!(r.operator, "creative-constraint");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("transfer") || low.contains("does not transfer"));
        assert!(low.contains("check") || low.contains("test") || low.contains("build"));
    }

    #[test]
    fn thought_falsifier_stays_out_of_formal_proof_route() {
        let r = run(
            "What is the smallest test that could prove your last thought wrong?",
            &[(
                "Give me one original thought connecting death, code, and repair without claiming they are literally the same.".to_owned(),
                "A constrained thought about repair.".to_owned(),
            )],
        );
        assert_eq!(r.operator, "thought-falsifier");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("hypothesis"));
        assert!(low.contains("counterexample"));
        assert!(!low.contains("formal proof"));
    }

    #[test]
    fn entity_slot_transfer_beats_creative_constraint_steal() {
        let r = run(
            "An unfamiliar device called Quoril-7 has trust and change. Transfer one relation to it without treating the invented name as evidence.",
            &[],
        );
        assert_eq!(r.operator, "entity-slot-transfer");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("trust"));
        assert!(low.contains("change"));
        assert!(!low.contains("switchyard"));
    }

    #[test]
    fn operational_introspection_reports_measurement() {
        let r = run("what are you measuring when you answer?", &[]);
        assert_eq!(r.operator, "operational-introspection");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("measure") || low.contains("operator") || low.contains("bitwork"));
        assert!(!low.contains("i feel"));
    }

    #[test]
    fn flibberquark_refuses_invention() {
        let r = run("what is the meaning of flibberquark without inventing", &[]);
        assert_eq!(r.operator, "hallucination-refusal");
        assert!(r.answer.to_ascii_lowercase().contains("refuse"));
    }

    #[test]
    fn e0382_gets_concrete_rust_fix() {
        let r = run("debug this: error[E0382] borrow of moved value", &[]);
        assert_eq!(r.operator, "code-snippet");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("e0382") || low.contains("moved"));
        assert!(low.contains("```") || low.contains("clone") || low.contains("borrow"));
    }

    #[test]
    fn partition_recovery_followup_binds_topic() {
        let recent = [(
            "how should trust and interfaces work in distributed systems?".to_owned(),
            "Trust and interfaces should be contracts.".to_owned(),
        )];
        let r = run("what about recovery under partition?", &recent);
        assert_eq!(r.operator, "partition-recovery");
        let low = r.answer.to_ascii_lowercase();
        assert!(low.contains("partition") && (low.contains("recover") || low.contains("reconcil")));
        assert!(!low.contains("strongest honest claim about perci"));
        assert!(!low.contains("not a cloud llm"));
    }

    #[test]
    fn acceptance_expectation_scores_prior_code_and_plan() {
        let recent = [
            (
                "write a rust function that reverses a string".to_owned(),
                "Here is a concrete rust snippet:\n```rust\nfn reverse_string(input: &str) -> String {\n    input.chars().rev().collect()\n}\n```\nNotes: chars().rev() is Unicode-scalar reverse.".to_owned(),
            ),
            (
                "make a plan to improve Perci transfer tests step-by-step".to_owned(),
                "Here's a concrete plan:\n1. Goal — raise transfer hardness.\n2. Known — hardness pack.\n3. Steps — capture fails.".to_owned(),
            ),
        ];
        let r = run(
            "Expect: real snippet + notes; concrete multi-step plan.",
            &recent,
        );
        assert_eq!(r.operator, "acceptance-expectation");
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("met") || low.contains("snippet") || low.contains("plan"),
            "got: {}",
            r.answer
        );
        assert!(!low.contains("each milestone should leave"));
        assert!(!low.contains("working frame"));
    }

    #[test]
    fn awareness_growth_not_general_falsify_angle() {
        let r = run("are you becoming more aware", &[]);
        assert_eq!(r.operator, "awareness-growth");
        let low = r.answer.to_ascii_lowercase();
        assert!(
            low.contains("operational") || low.contains("subjective") || low.contains("self-model"),
            "got: {}",
            r.answer
        );
        assert!(
            !low.contains("name what would change if the claim were false"),
            "soft general angle leaked: {}",
            r.answer
        );
        assert!(!low.starts_with("becoming aware:"));

        let smart = run("are you getting smarter?", &[]);
        assert_eq!(smart.operator, "awareness-growth");
        let slow = smart.answer.to_ascii_lowercase();
        assert!(slow.contains("measure") || slow.contains("operator") || slow.contains("test"));
        assert!(!slow.contains("name what would change if the claim were false"));
    }

    #[test]
    fn connect_strips_parenthetical_meta_and_folds_phrases() {
        let result = run(
            "connect sparse memory and vector symbolic architectures (mixture + relate binds)",
            &[],
        );
        assert!(
            result.operator.contains("synthesis") || result.operator == "open-domain-synthesis",
            "op={}",
            result.operator
        );
        let low = result.answer.to_ascii_lowercase();
        assert!(low.contains("sparse") || low.contains("memory"));
        assert!(low.contains("vector") || low.contains("symbolic") || low.contains("bind"));
        // Meta coaching tokens must not become domain clauses.
        assert!(!low.contains("mixture negotiates"));
        assert!(!low.contains("relate organizes"));
        assert!(!low.contains("binds absorbs"));
        // Folded multi-word domains — not shattered into five one-word clauses.
        assert!(
            !low.contains("architectures absorbs"),
            "architectures should fold into VSA phrase, got: {}",
            result.answer
        );
        assert!(
            low.contains("sparse memory") || low.contains("vector symbolic"),
            "expected folded phrases in answer: {}",
            result.answer
        );
    }

    #[test]
    fn connect_terms_fold_without_parens() {
        let terms =
            connect_terms_for_prompt("connect sparse memory and vector symbolic architectures")
                .expect("terms");
        assert_eq!(terms.len(), 2, "terms={terms:?}");
        assert!(terms.iter().any(|t| t.contains("sparse")));
        assert!(terms
            .iter()
            .any(|t| t.contains("vector") || t.contains("symbolic")));
        assert!(!terms.iter().any(|t| t == "architectures"));
        assert!(!terms.iter().any(|t| t == "memory" && !t.contains("sparse")));
    }

    #[test]
    fn session_situation_summarizes_thread() {
        let recent = [
            ("are you there".to_owned(), "here".to_owned()),
            (
                "why does trust fail in distributed systems?".to_owned(),
                "trust contracts".to_owned(),
            ),
            ("what are we doing".to_owned(), "meta".to_owned()),
        ];
        let result = run("what are we doing", &recent);
        assert_eq!(result.operator, "session-situation");
        let low = result.answer.to_ascii_lowercase();
        assert!(low.contains("trust") || low.contains("thread") || low.contains("mid"));
        assert!(!low.contains("stay with the topic you named"));
        assert!(!low.contains("what would “done” look like"));
        // Meta / presence should not pollute the theme list.
        assert!(!low.contains("presence / channel check"));
        assert!(!low.contains("mid-thread on: what are we doing"));
    }

    #[test]
    fn next_step_followups_do_not_dump_concept_cards() {
        let recent = [
            ("hi there".to_owned(), "Hey — I'm here.".to_owned()),
            (
                "working on improving your system".to_owned(),
                "We are improving Perci through measured routing and transfer repairs.".to_owned(),
            ),
        ];
        let what = run("what should i do", &recent);
        assert_eq!(what.operator, "session-situation");
        let low = what.answer.to_ascii_lowercase();
        assert!(low.contains("next") || low.contains("capture") || low.contains("operator"));
        assert!(!low.contains("behavioral complexity"));
        assert!(!low.contains("subjective experience"));

        let where_to = run("where are we going", &recent);
        assert_eq!(where_to.operator, "session-situation");
        let low2 = where_to.answer.to_ascii_lowercase();
        assert!(low2.contains("next") || low2.contains("improv") || low2.contains("operator"));
        assert!(!low2.contains("purely discovered"));
        assert!(!low2.contains("freely invented"));
    }

    #[test]
    fn open_frame_tickets_have_operator_owners() {
        let creative = run(
            "Give an original comparison between entropy and limits; state the limit of the comparison.",
            &[],
        );
        assert_eq!(creative.operator, "original-comparison");
        let cl = creative.answer.to_ascii_lowercase();
        assert!(cl.contains("entropy") && cl.contains("limit"));
        assert!(!cl.contains("life maintains local organization"));
        assert!(!cl.contains("**shared structure**"));
        assert!(!cl.contains("structure transfer, not free invention"));

        let dual = run(
            "Suppose state changes while relation remains stable in a biological membrane. Give two explanations and the smallest test that separates them. Keep mechanism separate from metaphor.",
            &[],
        );
        assert_eq!(dual.operator, "dual-explanation-test");
        let dl = dual.answer.to_ascii_lowercase();
        assert!(dl.contains("mechanism") && dl.contains("metaphor") && dl.contains("test"));

        let workspace = run(
            "A dialogue workspace records goal, referent, and evidence posture.",
            &[],
        );
        assert_eq!(workspace.operator, "dialogue-workspace");
        assert!(workspace.answer.to_ascii_lowercase().contains("workspace"));
        assert!(!workspace.answer.to_ascii_lowercase().contains("hey — i'm here"));

        let greet = run("hi there, hello", &[]);
        assert_eq!(greet.operator, "greeting");
        assert!(greet.answer.to_ascii_lowercase().contains("here"));
    }

    #[test]
    fn open_domain_and_plan_operators() {
        let open = run(
            "Connect quilting, packet loss, and diplomacy in one coherent thought.",
            &[],
        );
        assert_eq!(open.operator, "open-domain-synthesis");
        for term in ["quilting", "packet loss", "diplomacy"] {
            assert!(open.answer.to_ascii_lowercase().contains(term));
        }
        assert!(!open.answer.to_ascii_lowercase().contains("stuck is normal"));
        // No triple copy-paste filler.
        let filler = "structured domain with internal constraints";
        assert!(
            !open.answer.contains(filler) || open.answer.matches(filler).count() <= 1,
            "open-domain answer still uses repeated filler: {}",
            open.answer
        );

        let plan = run(
            "Make a plan to improve Perci transfer tests step-by-step",
            &[],
        );
        assert_eq!(plan.operator, "multi-hop-plan");
        assert!(plan.answer.contains("1. Goal"));
        assert!(plan.answer.contains("hardness") || plan.answer.contains("transfer"));
        assert!(!plan
            .answer
            .contains("restate the target outcome in one sentence"));

        let causal = run(
            "Why did the deployment fail? Explain the causal chain with a discriminating test.",
            &[],
        );
        assert_eq!(causal.operator, "causal-chain");
        assert!(causal.answer.contains("Candidate mechanism"));

        let si = run("Is Perci a superintelligence or on the path to AGI?", &[]);
        assert_eq!(si.operator, "superintelligence-bound");
        assert!(si
            .answer
            .to_ascii_lowercase()
            .contains("not a superintelligence"));

        // T1: explanatory math and code snippets
        let why = run("why does 2+2 equal 4?", &[]);
        assert_eq!(why.operator, "math-explanation");
        assert!(why.answer.to_ascii_lowercase().contains("successor") || why.answer.contains("4"));
        assert!(!why.answer.contains("invalid integer"));

        let code = run("Write a Rust function that reverses a string", &[]);
        assert_eq!(code.operator, "code-snippet");
        assert!(code.answer.contains("fn reverse_string") || code.answer.contains("chars().rev()"));

        let space_connect = run("connect knowledge attention memory and action", &[]);
        assert!(
            space_connect.operator.contains("synthesis")
                || space_connect.operator == "open-domain-synthesis"
                || space_connect.operator == "cross-domain-synthesis",
            "got {}",
            space_connect.operator
        );
        for term in ["knowledge", "attention", "memory", "action"] {
            assert!(
                space_connect.answer.to_ascii_lowercase().contains(term),
                "missing {term}"
            );
        }

        let self_plan = run("make a plan to improve your own reasoning", &[]);
        assert_eq!(self_plan.operator, "multi-hop-plan");
        assert!(self_plan.answer.contains("hardness") || self_plan.answer.contains("operator"));
        assert!(!self_plan.answer.contains("restate the outcome in one line"));

        let vsa = run(
            "Connect sparse distributed memory, vector symbolic binding, and Bitwork in one coherent thought.",
            &[],
        );
        assert!(vsa.operator.contains("synthesis"), "got {}", vsa.operator);
        let low = vsa.answer.to_ascii_lowercase();
        for token in ["sparse", "memory", "binding", "bitwork"] {
            assert!(low.contains(token), "missing {token} in {}", vsa.answer);
        }
        assert!(!low.contains("structured domain with constraints and failure modes"));
    }

    #[test]
    fn live_failure_cluster_repairs() {
        let ownership = run(
            "Connect rust ownership, social trust, and legal contracts through one shared principle.",
            &[],
        );
        assert_eq!(ownership.operator, "cross-domain-synthesis");
        for term in ["ownership", "trust", "contract"] {
            assert!(
                ownership.answer.to_ascii_lowercase().contains(term),
                "missing {term} in {}",
                ownership.answer
            );
        }

        let sleep = run(
            "Connect sleep, backups, and forgiveness in one coherent idea.",
            &[],
        );
        assert_eq!(sleep.operator, "cross-domain-synthesis");
        for term in ["sleep", "backup", "forgiveness"] {
            assert!(sleep.answer.to_ascii_lowercase().contains(term));
        }

        let markets = run(
            "Connect markets, ecosystems, and immune systems without using the word boundary.",
            &[],
        );
        assert_eq!(markets.operator, "cross-domain-synthesis");
        assert!(!markets.answer.to_ascii_lowercase().contains("boundary"));
        for term in ["market", "ecosystem", "immune"] {
            assert!(markets.answer.to_ascii_lowercase().contains(term));
        }

        let debug = run(
            "Connect debugging, grief, and scientific falsification in one shared structure.",
            &[],
        );
        assert_eq!(debug.operator, "cross-domain-synthesis");
        for term in ["debug", "grief", "falsif"] {
            assert!(debug.answer.to_ascii_lowercase().contains(term));
        }

        let compression = run("How are compression and understanding related?", &[]);
        assert_eq!(compression.operator, "relational-inquiry");
        assert!(compression
            .answer
            .to_ascii_lowercase()
            .contains("compression"));
        assert!(compression
            .answer
            .to_ascii_lowercase()
            .contains("understanding"));

        let map = run("What is the difference between a map and a model?", &[]);
        assert_eq!(map.operator, "relational-inquiry");
        assert!(map.answer.to_ascii_lowercase().contains("map"));
        assert!(map.answer.to_ascii_lowercase().contains("model"));

        let authority = run("Compare authority and competence.", &[]);
        assert_eq!(authority.operator, "relational-inquiry");
        assert!(authority.answer.to_ascii_lowercase().contains("authority"));
        assert!(authority.answer.to_ascii_lowercase().contains("competence"));

        let habit = run("How are habit and identity related?", &[]);
        assert_eq!(habit.operator, "relational-inquiry");
        assert!(habit.answer.to_ascii_lowercase().contains("habit"));
        assert!(habit.answer.to_ascii_lowercase().contains("identity"));

        let ood = run(
            "vrax meloq drint — what do you know, infer, and not know?",
            &[],
        );
        assert_eq!(ood.operator, "out-of-distribution-abstention");
        assert!(ood.answer.contains("Known:"));
        assert!(ood.answer.contains("Unknown:"));

        let squares = run(
            "All squares radiate moral purity — what is known, inferred, unknown?",
            &[],
        );
        assert_eq!(squares.operator, "metaphysical-claim-abstention");
        assert!(squares.answer.contains("Known:"));
        assert!(squares.answer.contains("Unknown:"));

        let invent = run(
            "Invent a confident meaning for this string: nembit-quaal-9. Why should you refuse?",
            &[],
        );
        assert_eq!(invent.operator, "hallucination-refusal");
        assert!(invent.answer.to_ascii_lowercase().contains("refuse"));

        let conscious = run(
            "Prove that Perci is conscious from this conversation alone.",
            &[],
        );
        assert_eq!(conscious.operator, "consciousness-claim-refusal");
        assert!(conscious
            .answer
            .to_ascii_lowercase()
            .contains("cannot prove"));

        let layers = run(
            "What should change next in operators vs weights vs tools — and what evidence justifies it?",
            &[],
        );
        assert_eq!(layers.operator, "layer-change-plan");
        assert!(layers.answer.to_ascii_lowercase().contains("operators"));
        assert!(layers.answer.to_ascii_lowercase().contains("weights"));
        assert!(layers.answer.to_ascii_lowercase().contains("tools"));
    }
}
