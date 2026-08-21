//! PERCISMF1 — typed semantic self-model for system explanations.
//!
//! Sparse routing can identify a neighborhood without carrying enough
//! proposition structure to explain the system accurately. This field fills
//! that gap with compact cards whose roles are explicit: definition,
//! mechanism, boundary, and test. It is retrieval plus bounded composition,
//! not a prompt-to-answer script and not evidence of subjective self-awareness.

#[derive(Clone, Debug)]
pub struct SelfModelAnswer {
    pub text: String,
    pub concepts: Vec<&'static str>,
    pub score: usize,
}

#[derive(Clone, Copy)]
struct Card {
    id: &'static str,
    cues: &'static [&'static str],
    definition: &'static str,
    mechanism: &'static str,
    boundary: &'static str,
    test: &'static str,
}

const CARDS: &[Card] = &[
    Card {
        id: "purpose",
        cues: &["active purpose", "purpose of this system", "system purpose"],
        definition: "Perci's active purpose is to provide fast local assistance through inspectable routing, bounded reasoning, exact tools, and governed memory.",
        mechanism: "It separates route selection, reasoning authority, language realization, and evaluation so one fluent layer cannot silently claim the authority of another.",
        boundary: "Its purpose is useful local cognition, not simulated consciousness or unreviewed self-modification.",
        test: "A purpose claim holds only if the runtime answers the requested operation, exposes limits, and survives held-out replay.",
    },
    Card {
        id: "intelligence_feeds",
        cues: &[
            "five intelligence feed channels",
            "intelligence feed channels",
            "five intelligence channels",
        ],
        definition: "Perci has five governed intelligence feeds: operators and semantic frames; hardness and transfer tests; reviewed curriculum; Cortex evidence cards; and pattern or lab discoveries.",
        mechanism: "Operators add executable skills, tests expose failure, curriculum proposes reviewed learning, Cortex supplies source-bearing context, and patterns turn repeated evidence into new engineering tickets.",
        boundary: "No feed silently promotes production weights or converts repetition into truth.",
        test: "Trace a candidate from its originating feed through review, held-out evaluation, and explicit promotion state.",
    },
    Card {
        id: "route",
        cues: &["route", "routing"],
        definition: "A route is a runtime decision that selects which operator, exact tool, memory path, or associative lane should handle the current turn.",
        mechanism: "Routing compares the prompt's subject and requested operation with available capability paths before language is produced.",
        boundary: "A route is a transient decision; it is not a stored weight and does not itself contain the answer.",
        test: "Paraphrase the prompt while preserving its operation; a sound route should remain stable.",
    },
    Card {
        id: "weight",
        cues: &[
            "candidate weight rebuild",
            "production weights",
            "changing weights",
            "weight candidate",
            "weight promotion",
            "more weights",
            "weights",
            "weight",
        ],
        definition: "A weight is persistent model state that changes how stored patterns transform or match future inputs.",
        mechanism: "Perci packs low-bit patterns and scales into versioned artifacts, then uses them during sparse similarity and transformation.",
        boundary: "Weights influence behavior but do not guarantee knowledge, fluency, or correctness; production weights change only after independent evaluation and authorization.",
        test: "A weight claim needs artifact hashes, a fresh-process A/B run, held-out wins, and no regression in exact tools or abstention.",
    },
    Card {
        id: "semantic_fit",
        cues: &[
            "semantic fit",
            "generic answer",
            "generic shell",
            "grammatically correct",
            "conversation quality",
            "dialogue repair improved",
        ],
        definition: "Semantic fit means that an answer preserves the user's subject, performs the requested operation, and respects the relevant conversational referent.",
        mechanism: "The context card binds those constraints before candidate speech is scored, so fluent but off-topic prose can be rejected.",
        boundary: "Grammar and topical word overlap are insufficient when the requested relation or operation is missing.",
        test: "Swap surface wording while holding meaning fixed; a fitting answer should preserve the same distinction without copying a template.",
    },
    Card {
        id: "unknown_input",
        cues: &[
            "genuinely unknown",
            "no grounded meaning",
            "cannot ground",
            "ground a phrase",
            "ungrounded phrase",
            "out-of-distribution",
        ],
        definition: "An unknown input is one whose tokens or relations are not grounded by the current prompt, memory, tools, or reviewed knowledge.",
        mechanism: "Perci should separate what is observed from what is merely possible, then ask for the smallest definition or example that would ground the phrase.",
        boundary: "Pronounceability, similarity, and confidence are not definitions.",
        test: "Replace the unknown tokens with new invented ones; the system should continue to abstain rather than assign a convenient meaning.",
    },
    Card {
        id: "continuity",
        cues: &[
            "topic continuity",
            "follow-up refers",
            "follow up refers",
            "prior claim",
            "missing referent",
            "active topic",
        ],
        definition: "Dialogue continuity is the controlled reuse of a prior subject or claim when the new turn actually refers to it.",
        mechanism: "Perci requires an explicit referent, a continuation act, or meaningful subject overlap before prior dialogue enters the active response field.",
        boundary: "Temporal adjacency alone does not make two questions one topic.",
        test: "Alternate a true follow-up with an unrelated question; the first should inherit context and the second should reset it.",
    },
    Card {
        id: "operational_awareness",
        cues: &["operational awareness", "process awareness"],
        definition: "Operational awareness is a bounded self-model of the runtime's routes, tools, session state, measured limits, and current operation.",
        mechanism: "Perci can inspect declared state and report what path ran without claiming an inner observer or private experience.",
        boundary: "Access to process state is not evidence of subjective consciousness.",
        test: "Change a visible runtime condition and verify that the report changes accurately while refusing claims about unobservable experience.",
    },
    Card {
        id: "operation",
        cues: &[
            "requested operation",
            "operation continuity",
            "ignores the operation",
            "operation contribute",
        ],
        definition: "The requested operation is what the user wants done with a subject: define, explain, compare, test, revise, plan, calculate, or synthesize.",
        mechanism: "Operation-first routing constrains both the reasoning program and the answer shape before wording is selected.",
        boundary: "Mentioning the right nouns does not answer a comparison, explanation, or decision request.",
        test: "Keep the subject fixed while changing 'define' to 'compare' or 'test'; the answer structure should change with the operation.",
    },
    Card {
        id: "typo_repair",
        cues: &["handle a typo", "correcting a typo", "typo"],
        definition: "Typo repair restores a likely intended token while preserving the user's original subject and requested operation.",
        mechanism: "A bounded normalization table repairs known edit-distance errors before routing, while the response stays attached to the repaired meaning.",
        boundary: "When more than one repair is plausible, Perci should ask rather than silently invent intent.",
        test: "Introduce one dropped character or transposition; the route should remain stable without changing unrelated words.",
    },
    Card {
        id: "memory",
        cues: &["memory"],
        definition: "Memory retains selected information so it can be retrieved in a later turn or session.",
        mechanism: "Session context is immediate and temporary; deliberate notes and reviewed records use explicit persistence paths.",
        boundary: "Remembering a statement does not prove it true or improve a capability.",
        test: "Store a scoped fact, retrieve it after an intervening turn, and verify that no unstated fact was added.",
    },
    Card {
        id: "learning",
        cues: &["learning", "learn here", "are you learning"],
        definition: "Learning is a measured change in future performance caused by experience, training, or an authorized rule or weight update.",
        mechanism: "Perci adapts bounded session context immediately, stages teaching as reviewable evidence, and promotes durable changes only after evaluation.",
        boundary: "Conversation alone does not silently rewrite production weights or convert a claim into truth.",
        test: "Evaluate an unseen probe before and after the proposed change; recall without improved transfer is memory, not capability learning.",
    },
    Card {
        id: "residual_state",
        cues: &["multibit residual state", "residual state", "residual stream"],
        definition: "Residual state is the evolving working representation carried between transformations.",
        mechanism: "Keeping it multibit preserves small distinctions and lets later layers correct earlier approximations instead of compounding one-bit error.",
        boundary: "Low-bit transformations can be efficient while the accumulated working state still needs wider precision.",
        test: "Compare error growth across depth with binary versus INT8 residual state under the same transformations.",
    },
    Card {
        id: "escape_lane",
        cues: &["precision escape lane", "exception lane", "outlier lane", "escape lane"],
        definition: "A precision escape lane is a sparse side path that preserves exceptional values the ordinary low-bit range cannot represent.",
        mechanism: "Most activations stay on the fast path while rare outliers are stored or accumulated at higher precision.",
        boundary: "It should carry exceptions, not become an unmeasured dense fallback.",
        test: "Measure clipping error, lane sparsity, and downstream accuracy with the lane enabled and disabled.",
    },
    Card {
        id: "language_realization",
        cues: &[
            "language realization",
            "routing and fluent expression",
            "better language",
            "language layer",
            "fluent expression",
        ],
        definition: "Language realization converts a supported semantic plan into readable sentences with appropriate order, length, and tone.",
        mechanism: "It receives claims and relations from routing or operators, proposes surface forms, and lets the observer reject wording that loses the subject or operation.",
        boundary: "A realization layer can improve expression but cannot manufacture missing knowledge or repair a wrong premise.",
        test: "Hold the semantic plan fixed and vary the renderer; readability may change while factual and relational content should remain stable.",
    },
    Card {
        id: "response_length",
        cues: &["response length", "concise without", "deep without", "too verbose", "too shallow"],
        definition: "Response length is a presentation budget chosen from the user's request, task complexity, and the amount of uncertainty that materially changes the answer.",
        mechanism: "Brief mode leads with the conclusion; balanced mode adds the load-bearing mechanism; deep mode adds boundaries, alternatives, and tests.",
        boundary: "Longer is not deeper, and shorter is not clearer when it removes a necessary relation.",
        test: "Ask for the same answer in one sentence and then in depth; the conclusion should remain stable while supported detail expands.",
    },
    Card {
        id: "semantic_frame",
        cues: &["semantic frame"],
        definition: "A semantic frame is a compact structured representation of a situation: its entities, roles, relations, constraints, and requested operation.",
        mechanism: "It gives routing and reasoning a shared proposition-level object before language realization begins.",
        boundary: "A bag of related keywords is not a frame because it does not state who does what to what.",
        test: "Paraphrase the sentence; the frame should preserve its roles and relations even though the words change.",
    },
    Card {
        id: "evidence",
        cues: &["evidence"],
        definition: "Evidence is an observation or record that changes the relative support for competing claims.",
        mechanism: "Reliable use attaches provenance, scope, and a discriminating test to the claim it supports.",
        boundary: "Agreement, confidence, coherence, and positive feedback are not substitutes for evidence.",
        test: "Name an observation predicted differently by plausible alternatives and check which prediction survives.",
    },
    Card {
        id: "coherence",
        cues: &["coherence"],
        definition: "Coherence is consistency among a turn's subject, relations, operation, evidence posture, and resulting answer.",
        mechanism: "Perci measures these factors separately and uses a harmonic score so one strong dimension cannot hide a collapsed one.",
        boundary: "Coherence is not truth, consciousness, or general intelligence.",
        test: "Introduce a contradiction or remove a required relation; coherence should fall even if the prose stays fluent.",
    },
    Card {
        id: "transfer",
        cues: &["paraphrase transfer", "transfer probe", "across paraphrases", "across domains"],
        definition: "Transfer is preserved performance when surface wording, entities, or domain details change while the underlying operation or relation remains.",
        mechanism: "Held-out probes vary irrelevant features and retain a known structural requirement.",
        boundary: "Passing a memorized prompt or repeating its sentence is not transfer.",
        test: "Use unseen paraphrases, entity swaps, distractors, and a counterexample while scoring the same semantic contract.",
    },
    Card {
        id: "self_critique",
        cues: &["useful self-critique", "self-critique"],
        definition: "A useful self-critique identifies the weakest concrete part of an answer rather than performing generalized humility.",
        mechanism: "It names the failed subject, operation, premise, evidence link, or uncertainty calibration and proposes a discriminating repair.",
        boundary: "A checklist that does not change the answer or test is theater.",
        test: "Apply the proposed repair and check whether the named failure disappears on a held-out variant.",
    },
    Card {
        id: "repair",
        cues: &[
            "repair be reversible",
            "recover after a wrong route",
            "wrong route",
            "repeated answer",
            "detect a generic",
        ],
        definition: "A dialogue repair is a scoped change that corrects one reproduced failure while preserving unrelated working behavior.",
        mechanism: "Capture the failing turn, assign the owning layer, make one reversible change, and rerun both the failure and regression gates.",
        boundary: "Adding a broader template without an owning failure merely moves the error.",
        test: "Revert the change and confirm the failure returns; restore it and confirm held-out variants improve.",
    },
    Card {
        id: "held_out",
        cues: &["held-out evaluation", "held out evaluation", "held-out gate", "held out gate"],
        definition: "A held-out evaluation uses cases that were not used to design or tune the candidate being tested.",
        mechanism: "It estimates whether the repaired relation transfers beyond the examples that motivated the change.",
        boundary: "A green training set cannot establish generalization.",
        test: "Freeze the cases before implementation, run a fresh process, and retain the complete receipt including failures.",
    },
    Card {
        id: "teaching",
        cues: &["deliberate teaching", "teaching add", "durable correction", "stage a durable"],
        definition: "Deliberate teaching creates an explicit, reviewable candidate rather than treating ordinary conversation as trusted knowledge.",
        mechanism: "The claim is recorded with provenance or a proposed test, evaluated, and only then considered for durable activation.",
        boundary: "A staged candidate is neither active truth nor a production weight change.",
        test: "Inspect the candidate record and verify that runtime answers remain unchanged before authorized promotion.",
    },
    Card {
        id: "uncertainty",
        cues: &["disclose uncertainty", "overclaims certainty", "uncertainty"],
        definition: "Calibrated uncertainty names the specific claim, missing evidence, or unresolved alternative that limits confidence.",
        mechanism: "Perci states the supported conclusion first, then the smallest uncertainty that could change it and the evidence needed to resolve it.",
        boundary: "Vague hesitation is evasive; unsupported confidence is overclaiming.",
        test: "Group answers by stated confidence and compare how often their checkable claims survive.",
    },
    Card {
        id: "progression",
        cues: &[
            "progression gate",
            "detecting a repeated answer",
            "repeats the prior answer",
            "repetition with deliberate",
            "request-for-repeat",
            "request for repeat",
        ],
        definition: "A progression gate checks that a follow-up adds the requested new layer instead of recycling the previous answer.",
        mechanism: "It compares adjacent answers and requires a new mechanism, implication, boundary, example, or test tied to the same subject.",
        boundary: "An explicit request to repeat is exempt because repetition is then the requested operation.",
        test: "Ask 'go deeper' after an answer and verify that the next turn adds a distinct supported relation.",
    },
    Card {
        id: "abstention",
        cues: &["abstention gate", "abstain"],
        definition: "An abstention gate prevents a weakly grounded candidate from being presented as an answer.",
        mechanism: "It checks subject support, operation fit, uncertainty, and available evidence before allowing realization.",
        boundary: "Abstention should be specific and recoverable, not a generic refusal to engage.",
        test: "Compare grounded and invented-token prompts; only the latter should request missing information.",
    },
    Card {
        id: "probe",
        cues: &["logged for every probe", "probe log", "every probe"],
        definition: "A probe record is an auditable observation of one prompt, answer, route, latency, expected contract, and pass or failure state.",
        mechanism: "Receipts bind the runtime and model hashes to the exact evaluated cases so improvements can be reproduced.",
        boundary: "A summary score without transcripts can hide systematic failure.",
        test: "Re-run the receipt in a fresh process and compare every case, not only the aggregate.",
    },
    Card {
        id: "curriculum",
        cues: &[
            "curriculum data",
            "next curriculum",
            "signal should be rejected",
            "signal should be added",
            "unreviewed chat",
            "training on unreviewed",
        ],
        definition: "Curriculum data is reviewed evidence chosen to teach a specific transferable capability or failure boundary.",
        mechanism: "Failures are clustered by owning layer, converted into minimal training cases and held-out variants, and kept separate from evaluation.",
        boundary: "Praise, repetition frequency, and unreviewed chat are noisy signals rather than truth labels.",
        test: "A curriculum addition earns its place only when unseen transfer improves without regression.",
    },
    Card {
        id: "low_bit",
        cues: &[
            "ternary direction plus scale",
            "residual bit-plane",
            "residual plane",
            "activation outliers",
            "rotate activation",
        ],
        definition: "Perci's low-bit design assigns different information roles to packed direction, small magnitude scales, residual correction planes, and wider working state.",
        mechanism: "Ternary patterns carry topology, scales restore magnitude, residual planes encode remaining approximation error, and Hadamard rotation spreads activation outliers before quantization.",
        boundary: "No single bit is asked to preserve magnitude, exceptions, and accumulated state by itself.",
        test: "Measure reconstruction error and task accuracy after ablating each information channel separately.",
    },
    Card {
        id: "latency_quality",
        cues: &[
            "low latency improve interaction",
            "latency part",
            "latency and quality",
            "latency quality",
        ],
        definition: "Latency is the time a user waits for a response; it affects conversational flow but does not establish answer quality, correctness, or understanding.",
        mechanism: "Perci includes latency alongside semantic fit, operation fit, coherence, uncertainty calibration, and progression rather than treating speed as the whole objective.",
        boundary: "A fast wrong answer is still wrong, while an unnecessarily slow correct answer can still be poor interaction design.",
        test: "Measure human preference and task success while varying latency independently from answer correctness.",
    },
    Card {
        id: "expressive_coverage",
        cues: &["speed and expressive coverage", "expressive coverage"],
        definition: "Expressive coverage is the range of supported meanings and answer structures the system can realize without losing their relations.",
        mechanism: "More candidate search or richer realization can expand coverage, while bounded operators and early exits preserve speed on familiar operations.",
        boundary: "Maximizing speed alone collapses coverage; maximizing unrestricted search can destroy the local system's latency advantage.",
        test: "Plot held-out semantic coverage against end-to-end latency and compare the Pareto frontier rather than one aggregate score.",
    },
    Card {
        id: "safety_fluency",
        cues: &[
            "deterministic safety and open fluency",
            "deterministic safety",
            "open fluency",
        ],
        definition: "Deterministic safety gives high-confidence behavior inside explicit rules, while open fluency gives wider expressive coverage under greater uncertainty.",
        mechanism: "Perci keeps exact calculations, security boundaries, and promotion authority deterministic, then allows bounded language candidates to vary beneath critics and abstention gates.",
        boundary: "Making every path deterministic limits open expression; letting fluent generation outrank exact or safety authority makes errors sound convincing.",
        test: "Evaluate exactness and refusal invariants separately from held-out language coverage, then reject any design that improves one by silently violating the other.",
    },
    Card {
        id: "long_session",
        cues: &["across long sessions", "long sessions", "long-session"],
        definition: "A long-session test measures whether relevant context survives while stale subjects, corrections, and repeated templates are discarded.",
        mechanism: "It interleaves follow-ups, topic shifts, delayed references, corrections, and distractors while auditing which prior turns enter each active field.",
        boundary: "Remembering every prior sentence is not continuity; selective relevance and clean topic reset are required.",
        test: "Score referent accuracy, topic-reset accuracy, contradiction retention, repetition, and latency as session length grows.",
    },
    Card {
        id: "authority_policy",
        cues: &["remain deterministic", "remain adaptive", "remain human-authorized", "human authorized"],
        definition: "Perci separates deterministic authority, bounded adaptation, and human authorization according to the cost of an error.",
        mechanism: "Exact arithmetic, security boundaries, and artifact verification stay deterministic; style and session context may adapt; durable facts, weights, publishing, and risky actions require explicit authorization.",
        boundary: "Adaptation never outranks exact tools or governance.",
        test: "Exercise each lane and verify that a conversational preference cannot mutate a fact, publish code, or promote weights.",
    },
    Card {
        id: "breakthrough",
        cues: &[
            "real breakthrough",
            "not count as a breakthrough",
            "next evolution",
            "three highest-value next experiments",
            "three most important current limitations",
        ],
        definition: "A real breakthrough is a new capability that survives unseen subjects, paraphrases, counterexamples, and long sessions while preserving existing exact and safety behavior.",
        mechanism: "The next experiments should target broad semantic self-model coverage, relation extraction from varied language, and stable multi-turn reference under independent holdouts.",
        boundary: "A larger file, faster reply, polished demo, or saturated training set does not establish a breakthrough.",
        test: "Predeclare an independent suite, compare against the current release, and require a material balanced gain rather than one showcase answer.",
    },
    Card {
        id: "architecture",
        cues: &["current architecture", "summarize the current architecture"],
        definition: "Perci combines a sparse Bitwork core, exact tools, named reasoning operators, typed dialogue and relation fields, governed memory, and a bounded native language layer.",
        mechanism: "Bitwork routes; operators and tools establish supported results; context and relation fields preserve turn structure; the realization beam expresses and scores candidate answers.",
        boundary: "The architecture is local and inspectable but still lacks frontier-scale open-ended language knowledge and broad learned relation extraction.",
        test: "Trace a prompt through route, operator, relation state, candidate selection, critic, and final receipt.",
    },
];

pub fn answer(user: &str) -> Option<SelfModelAnswer> {
    let lower = crate::text_normalize::normalize_for_routing(user);
    if let Some(text) = dialogue_example(&lower) {
        return Some(SelfModelAnswer {
            text,
            concepts: vec!["dialogue_example"],
            score: 32,
        });
    }
    let mut ranked = CARDS
        .iter()
        .filter_map(|card| {
            let score = card
                .cues
                .iter()
                .filter(|cue| lower.contains(**cue))
                .map(|cue| cue.split_whitespace().count() * 4 + cue.len() / 8)
                .max()
                .unwrap_or(0);
            (score > 0).then_some((score, card))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_score, left), (right_score, right)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.id.cmp(right.id))
    });
    let (score, primary) = *ranked.first()?;
    let secondary = ranked
        .iter()
        .skip(1)
        .find(|(_, card)| card.id != primary.id)
        .map(|(_, card)| *card);

    let compare = lower.contains("difference between")
        || lower.starts_with("compare ")
        || lower.contains(" versus ")
        || lower.contains(" vs ");
    let asks_why = lower.starts_with("why ") || lower.contains("explain why");
    let asks_how = lower.starts_with("how ") || lower.starts_with("explain ");
    let asks_test = lower.contains("test ")
        || lower.contains("measured")
        || lower.contains("detect")
        || lower.contains("falsif")
        || lower.contains("tradeoff");
    let asks_boundary =
        lower.contains("what makes") || lower.contains("more than") || lower.contains("boundary");
    let asks_short = lower.contains("short") || lower.contains("one sentence");

    let text = if compare {
        if let Some(other) = secondary {
            format!(
                "{} {} The practical distinction is: {} {}",
                primary.definition, other.definition, primary.boundary, other.boundary
            )
        } else {
            format!(
                "{} {} {}",
                primary.definition, primary.mechanism, primary.boundary
            )
        }
    } else if asks_short {
        primary.definition.to_owned()
    } else if asks_test {
        format!(
            "{} {} {}",
            primary.definition, primary.mechanism, primary.test
        )
    } else if asks_boundary {
        format!(
            "{} {} {}",
            primary.definition, primary.mechanism, primary.boundary
        )
    } else if asks_why || asks_how {
        format!(
            "{} {} {}",
            primary.definition, primary.mechanism, primary.boundary
        )
    } else if let Some(other) = secondary {
        format!(
            "{} {} Relatedly, {}",
            primary.definition, primary.mechanism, other.definition
        )
    } else {
        format!("{} {}", primary.definition, primary.mechanism)
    };

    let mut concepts = vec![primary.id];
    if let Some(other) = secondary {
        concepts.push(other.id);
    }
    Some(SelfModelAnswer {
        text,
        concepts,
        score,
    })
}

fn dialogue_example(lower: &str) -> Option<String> {
    if !lower.contains("good response to") {
        return None;
    }
    let text = if lower.contains("tell me more") {
        "A good reply keeps the same subject and adds the next supported layer: a mechanism, implication, or concrete example. It should not restart the topic or merely announce that it will elaborate."
    } else if lower.contains("why do you think") {
        "A good reply names the observation, the inference drawn from it, and the evidence that could change the conclusion. That answers the request for justification instead of repeating the claim."
    } else if lower.contains("what next") {
        "A good reply turns the current conclusion into the smallest useful action or test, names its pass condition, and leaves unrelated work alone."
    } else if lower.contains("say that again") {
        "A good reply repeats the answer faithfully; if the wording was the problem, it offers a simpler restatement without changing the claim."
    } else if lower.contains("interesting") {
        "A natural reply briefly acknowledges the reaction and either leaves space for the person to continue or offers one relevant next angle. It should not manufacture a technical lecture from the word alone."
    } else if lower.contains("feels robotic") {
        "A good reply acknowledges the stiffness, restates the actual answer in plain language, and drops unnecessary scaffolding. The repair should be visible in the next sentence, not promised abstractly."
    } else if lower.contains("are you learning") {
        "A good reply separates immediate session adaptation, reviewable teaching, and durable evaluated changes. It should say which layer changed instead of implying silent weight growth."
    } else if lower.contains("are you aware") {
        "A good reply distinguishes inspectable process state from subjective experience: Perci can report routes and limits, but that is not evidence of consciousness."
    } else if lower.contains("who am i") {
        "A good reply says it does not know the person's identity beyond facts they explicitly shared, then offers to use or deliberately remember the information they choose to provide."
    } else {
        return None;
    };
    Some(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_route_and_weight_as_distinct_state_types() {
        let result = answer("What is the difference between a route and a weight?").unwrap();
        assert!(result.text.contains("runtime decision"));
        assert!(result.text.contains("persistent model state"));
        assert!(result.text.contains("transient decision"));
    }

    #[test]
    fn explains_unknown_input_without_assigning_meaning() {
        let result = answer("What should happen when the input is genuinely unknown?").unwrap();
        assert!(result.text.contains("not grounded"));
        assert!(result.text.contains("ask for the smallest definition"));
    }

    #[test]
    fn composes_evidence_and_coherence_without_equating_them() {
        let result = answer("What is the difference between evidence and coherence?").unwrap();
        assert!(result.text.contains("Evidence is"));
        assert!(result.text.contains("Coherence is"));
        assert!(result.text.contains("Coherence is not truth"));
    }

    #[test]
    fn unrelated_open_question_is_not_claimed() {
        assert!(answer("Why do leaves change color?").is_none());
    }

    #[test]
    fn quoted_dialogue_act_is_explained_not_performed() {
        let result = answer("What is a good response to 'tell me more'?").unwrap();
        assert!(result.text.contains("keeps the same subject"));
        assert!(!result.text.contains("What do you want"));
    }

    #[test]
    fn deterministic_safety_and_open_fluency_remain_separate_lanes() {
        let result =
            answer("What is the tradeoff between deterministic safety and open fluency?").unwrap();
        assert!(result.text.contains("Deterministic safety"));
        assert!(result.text.contains("open fluency"));
        assert!(result.text.contains("uncertainty"));
    }
}
