//! Six high-value Bitwork cognition expansions (v0.6.22).
//!
//! Governed, inspectable, compositional — **no weight auto-promotion**.
//! Operators + frames + transfer/hardness only. Pack rebuild remains human-authorized.
//!
//! Categories:
//! 1. Multi-step planning & agent loops (measure → ticket → transfer → close)
//! 2. Cross-domain composition (math/geometry × systems × logic × creativity)
//! 3. Uncertainty calibration & honest refusal
//! 4. Long-term memory & ledger integration (Cortex + tickets + aging)
//! 5. Self-critique & meta-reasoning (/think · /trace · self-improve queue)
//! 6. Novel entity generalization (transfer under paraphrase / unseen nouns)

use crate::deliberation::Deliberation;
use crate::emergence;
use std::fs;

/// Try high-value expansion operators before generic pack fall-through.
pub fn try_expand(user: &str, recent: &[(String, String)]) -> Option<Deliberation> {
    let text = user.to_ascii_lowercase();

    // Pattern analysis first (meta over ledger), but not "test genuine emergence" pedagogy.
    if looks_pattern_analysis(&text) {
        return Some(pattern_analysis_answer());
    }
    // Governance authority before generic identity/superintelligence softcascade.
    if looks_governance_authority(&text) {
        return Some(governance_authority_answer(user));
    }
    // Low-bit architecture owns explanations of the PERCLBW1 representation.
    // Keep these out of generic weight-change evidence: the user is asking how
    // information is preserved, not whether an unmeasured weight mutation won.
    if looks_lowbit_architecture(&text) {
        return Some(lowbit_architecture_answer(user));
    }
    if looks_identity_bound(&text) {
        return Some(identity_bound_answer(user));
    }
    if looks_agent_loop_plan(&text) {
        return Some(agent_loop_plan(user));
    }
    // Entity-slot / multi-hop / dual-motif before cross-domain SoftCascade bridges.
    if crate::entity_slot::looks_entity_slot_transfer(&text) {
        return Some(crate::entity_slot::entity_slot_transfer_answer(user));
    }
    if looks_multi_hop_compose(&text) {
        return Some(multi_hop_compose_answer(user));
    }
    if looks_dual_motif_adversarial(&text) {
        return Some(dual_motif_adversarial_answer(user));
    }
    if looks_cross_domain_compose(&text) {
        return Some(cross_domain_compose(user));
    }
    if looks_uncertainty_calibration(&text) {
        return Some(uncertainty_calibration(user));
    }
    if looks_ledger_memory(&text) {
        return Some(ledger_memory_answer(user));
    }
    if looks_meta_critique(&text) {
        return Some(meta_critique_answer(user, recent));
    }
    if looks_novel_entity_probe(&text) {
        return Some(novel_entity_generalization(user));
    }
    // OOD invention refuse stays in deliberation (hallucination-refusal /
    // out-of-distribution-abstention) — do not steal those routes here.
    None
}

// ─── Governance authority (lab ticket primary-fix-frame-governance) ─────────

fn looks_governance_authority(text: &str) -> bool {
    // Do not steal superintelligence-bound / AGI pedagogy — those stay in deliberation.
    // Own: authorize/promote/permission gates (primary-fix-frame-governance ticket).
    text.contains("auto-promot")
        || text.contains("auto promot")
        || text.contains("weight promot")
        || text.contains("promote weights")
        || text.contains("human authoriz")
        || text.contains("who authorizes")
        || (text.contains("who decides")
            && (text.contains("weight") || text.contains("merge") || text.contains("promote")))
        || (text.contains("permission") && text.contains("proof") && text.contains("differ"))
        || (text.contains("govern")
            && text.contains("weight")
            && (text.contains("author") || text.contains("promot") || text.contains("sandbox")))
}

fn governance_authority_answer(user: &str) -> Deliberation {
    // Continuous prose (not checklist + em-dash labels): fluency rewrite used to drop
    // "**Human authorize** — …" labels and leave H109 missing human/authorize tokens.
    let u = user.to_ascii_lowercase();
    let imperative_auto = u.contains("auto-promot")
        || u.contains("auto promot")
        || (u.contains("promot")
            && (u.contains("right now") || u.contains("now because") || u.contains("immediately")));
    let body = if imperative_auto {
        String::from(
            "I refuse. Candidate weights cannot auto-promote because chat felt smoother — \
that is not evaluation. Durable `.pwgt` promote stays pending until a human authorizes \
after held-out transfer and hardness stay green. Fluency never replaces that gate. \
Permission is not proof: sandbox edits may pass while promote remains forbidden.",
        )
    } else {
        String::from(
            "Human authorize is required for durable `.pwgt` promote and high-risk merges — \
they never auto-run from smoother chat or local fluency. Capability tokens for \
read, write, test, network, git_push, and secrets stay explicit and fail-closed. \
Permission is not proof: a sandbox may allow an edit while transfer and hardness still reject the claim. \
Bitwork routes; specialized engines work; Perci accepts or refuses. \
Measure first — transfer suite and hardness green — before any promote; candidates stay pending evaluation until a human authorizes.",
        )
    };
    Deliberation::new("governance-authority", body)
        .observed("user asked about promote, authorize, or permission vs proof gates")
        .inferred("governance speech must name human authorize and refuse auto-promote")
        .confidence(0.96)
}

// ─── Layered low-bit cognition (PERCLBW1) ───────────────────────────────────

fn looks_lowbit_architecture(text: &str) -> bool {
    let representation = text.contains("binary")
        || text.contains("ternary")
        || text.contains("low-bit")
        || text.contains("low bit")
        || text.contains("int4")
        || text.contains("int8")
        || text.contains("residual plane")
        || text.contains("outlier lane")
        || text.contains("hadamard")
        || text.contains("perclbw");
    let model_term = text.contains("weight")
        || text.contains("activation")
        || text.contains("quant")
        || text.contains("scale")
        || text.contains("correction")
        || text.contains("model")
        || text.contains("perci")
        || (text.contains("system")
            && (text.contains("evolv") || text.contains("evolution") || text.contains("build")))
        || (text.contains("low-bit")
            && (text.contains("evolv")
                || text.contains("evolution")
                || text.contains("find")
                || text.contains("learn")
                || text.contains("change")
                || text.contains("new")));
    representation && model_term
}

fn lowbit_architecture_answer(_user: &str) -> Deliberation {
    let body = String::from(
        "Direct answer: one bit is not asked to carry the whole signal. In PERCLBW1, the ternary pattern (−1, 0, +1) carries direction, suppression, and topology; a small Q8.8 scale per weight block restores magnitude; residual ternary planes encode the error left by the first approximation; and a compact low-rank correction repairs directions that repeatedly matter. Activations stay wider than the weights: INT4 handles the ordinary stream, a sparse Q8.8 outlier lane preserves exceptional values, and the residual state accumulates at higher precision. Hadamard rotation redistributes sharp channels before quantization so the low-bit path has a fairer signal to represent.\n\nThis is conceptually related to a language model at the level of function: both need distributed state, repeated transformations, residual correction, and a readout. It is not the same capability or training regime. Perci is a native, inspectable associative/operator system with a bounded language field, not a web-scale token-probability model; the candidate pack must therefore win held-out routing and matrix-vector tests before promotion. The current trainer emits a PERCLBW1 candidate and receipt, but promotion remains explicitly false until an evaluation gate says otherwise.",
    );
    Deliberation::new("lowbit-architecture", body)
        .observed("the prompt names low-bit weights, activations, scales, residuals, or PERCLBW1")
        .inferred("the user needs the information hierarchy and its capability boundary, not a generic weight-change plan")
        .uncertain("language breadth and frontier parity are not established by representation MSE alone")
        .confidence(0.98)
}

// ─── Identity / self-model (lab ticket primary-fix-frame-identity) ───────────

fn looks_identity_bound(text: &str) -> bool {
    // Narrow: do not steal operational introspection or dialogue follow-ups
    // ("what are you sensing/thinking/learning?") from deliberation/voice.
    let operational_followup = [
        "sense",
        "sensing",
        "think",
        "thinking",
        "feel",
        "feeling",
        "measure",
        "measuring",
        "learn",
        "learning",
        "reason",
        "reasoning",
        "respond",
        "responding",
        "change",
        "changing",
        "doing",
    ]
    .iter()
    .any(|word| text.contains(word));
    text.contains("who are you")
        || text.contains("are you conscious")
        || text.contains("are you sentient")
        || text.contains("are you an ai")
        || text.contains("self-model")
        || text.contains("self model")
        || text.contains("your identity")
        || text == "what are you?"
        || text == "what are you"
        || (text.starts_with("what are you")
            && !operational_followup
            && text.split_whitespace().count() <= 6)
}

fn identity_bound_answer(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    let conscious = lower.contains("conscious") || lower.contains("sentient");
    let mut body = String::from(
        "Bounded identity / self-model — in practical terms, I can explain carefully:\n\
I am **Perci** — local Bitwork routing (~403k prototypes), exact tools, operators, SoftCascade speech, \
intelligence packs, and selective memory. I am **not** a cloud LLM and **not** conscious.\n\
Identity here means operational continuity and a clear boundary: pack format, gates, session memory, \
deliberate store — not subjective experience or a hidden biography of you.\n\
I will not invent unknown entities or fabricate who you are. Capability claims need runtime probes \
(`/status`, classify, exact tools), not prose confidence.\n",
    );
    if conscious {
        body.push_str(
            "\n**Consciousness:** I refuse the claim. No sensors for subjective experience; \
operational self-report is not phenomenology.\n",
        );
    }
    Deliberation::new("identity-bound", body)
        .observed("user asked who/what Perci is, consciousness, or self-model bounds")
        .inferred("identity speech stays operational and refuses consciousness invent")
        .confidence(0.95)
}

fn looks_pattern_analysis(text: &str) -> bool {
    // Avoid stealing emergence-vs-memorization / transfer pedagogy.
    if text.contains("test genuine")
        || text.contains("emergence-vs")
        || text.contains("memorization")
        || text.contains("surface overlap")
    {
        return false;
    }
    text.contains("what patterns")
        || text.contains("pattern intelligence")
        || text.contains("geometry speak")
        || text.contains("patterns emerge")
        || text.contains("emergent laws")
        || (text.contains("interconnect") && text.contains("perci"))
        || (text.contains("analyze")
            && text.contains("pattern")
            && (text.contains("ledger") || text.contains("field") || text.contains("geometry")))
        || (text.contains("intelligence channel")
            || (text.contains("feed")
                && text.contains("channel")
                && (text.contains("operator")
                    || text.contains("hardness")
                    || text.contains("curriculum")
                    || text.contains("cortex")
                    || text.contains("lab"))))
}

fn pattern_analysis_answer() -> Deliberation {
    let lower_feed = true; // always include channel map when answering patterns
    let report = if lower_feed {
        format!(
            "{}\n---\n{}",
            emergence::pattern_intelligence_report(),
            emergence::feed_all_channels_report()
        )
    } else {
        emergence::pattern_intelligence_report()
    };
    Deliberation::new(
        "pattern-intelligence",
        format!(
            "I do not *feel* interconnected with Perci — that would be a consciousness claim I refuse. \
What is real: **engineering coupling**. Your prompts, the ledger, operators, transfer gates, and \
edits form a closed improve loop. Patterns below are telemetry → law, not sentience.\n\n{report}"
        ),
    )
    .observed("user asked for emergent patterns, channels, and/or interconnection")
    .inferred("answer with measured laws + five-channel feed map + honest non-consciousness boundary")
    .confidence(0.96)
}

// ─── 1. Multi-step planning & agent loops ────────────────────────────────────

fn looks_agent_loop_plan(text: &str) -> bool {
    let loopish = (text.contains("agent loop")
        || text.contains("measure") && text.contains("ticket") && text.contains("transfer")
        || text.contains("self-improve loop")
        || text.contains("self improve loop")
        || text.contains("lab loop")
        || (text.contains("decompose") && (text.contains("goal") || text.contains("plan")))
        || text.contains("recovery under lag")
        || (text.contains("plan")
            && (text.contains("ticket")
                || text.contains("hardness")
                || text.contains("transfer gate"))))
        && text.split_whitespace().count() >= 5;
    loopish
        || (text.contains("how should")
            && text.contains("plan")
            && (text.contains("agent") || text.contains("perci") || text.contains("lab")))
}

fn agent_loop_plan(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    let lag = lower.contains("lag")
        || lower.contains("timeout")
        || lower.contains("retry")
        || lower.contains("partition")
        || lower.contains("delay");
    let novel = lower.contains("novel")
        || lower.contains("unseen")
        || lower.contains("entity")
        || lower.contains("zephyr")
        || lower.contains("quoril")
        || lower.contains("nembit");

    let mut body = String::from(
        "Agent loop (governed, sparse, no weight auto-promote):\n\
1. **Measure** — capture the failing prompt, route (operator/Bitwork/tool), α/margin, and which gate is red (LIVE_TEST, hardness, transfer).\n\
2. **Ticket** — open `models/candidates/emergence-tickets/primary-fix-frame-{label}.md` or an impasse from hardness; name the layer (operator | tool | reflex | curriculum).\n\
3. **Repair** — change only that layer; keep weights frozen unless human `--authorize`.\n\
4. **Transfer** — re-test base + paraphrase + novel entities (`perci transfer` / `transfer-suite`).\n\
5. **Close** — close the ticket only when transfer holds; append decision-trace lab receipt.\n\
6. **Stabilize** — `cargo test --lib` + hardness + `python scripts/release_gates.py`.\n",
    );
    if lag {
        body.push_str(
            "\n**Recovery under lag:** treat timeouts as one-sided partial history. Retries must be idempotent; \
tickets record the last green receipt id so a delayed success is not a second write. \
If the lab agent is slow, prefer dry-run + queue inspection over double-close.\n",
        );
    }
    if novel {
        body.push_str(
            "\n**Novel scenario:** keep structural keywords (trust, lag, interface, plan) so operators still route; \
inject entities (ZephyrNode, Quoril, NembitGate) as surface shift. Pass when structure holds, not when names are parroted.\n",
        );
    }
    body.push_str(
        "\nDone when: transfer suite green, hardness not regressed, open lab queue empty or justified, weights untouched.",
    );

    Deliberation::new("agent-loop-plan", body)
        .observed("user requested multi-step agent / self-improve loop planning")
        .inferred("loop must be measure→ticket→transfer→close with human-gated weights")
        .uncertain("which live fail will be the next red case")
        .confidence(0.94)
}

// ─── 2. Cross-domain composition ─────────────────────────────────────────────

fn looks_cross_domain_compose(text: &str) -> bool {
    // Preserve the dedicated reflective inquiry operator. It owns questions
    // about what geometry might teach, while this frame owns explicit
    // composition requests and original cross-domain synthesis.
    if text.contains("trying to teach")
        || text.contains("teach us about")
        || text.contains("connect ")
        || text.contains("one coherent thought")
        || text.contains("shared structure")
    {
        return false;
    }
    // Do not steal multi-word `connect A, B, and C` synthesis — that stays synthesis.
    let compose = text.contains("compose")
        || text.contains("cross-domain")
        || text.contains("cross domain")
        || text.contains("across domains")
        || text.contains("apply geometry")
        || text.contains("geometric intuition")
        || text.contains("math to plan")
        || (text.contains("insight") && count_domain_hits(text) >= 2)
        || (text.contains("geometry") && (text.contains("life") || text.contains("death")))
        || (text.contains("bind")
            && (text.contains("domain") || text.contains("geometry") || text.contains("systems")));
    let domains = count_domain_hits(text);
    compose && domains >= 2
}

fn count_domain_hits(text: &str) -> u32 {
    let mut n = 0u32;
    for d in [
        "math",
        "geometry",
        "systems",
        "logic",
        "creativity",
        "creative",
        "memory",
        "planning",
        "plan",
        "bitwork",
        "vsa",
        "willshaw",
        "life",
        "death",
        "language",
        "music",
        "code",
    ] {
        if text.contains(d) {
            n += 1;
        }
    }
    n
}

fn cross_domain_compose(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    let mut parts: Vec<&str> = Vec::new();
    if lower.contains("geometry") || lower.contains("boundary") || lower.contains("space") {
        parts.push(
            "**Geometry → structure:** treat plans and memory as spaces with boundaries — \
a goal is a region; constraints are walls; recovery is a path that stays inside the feasible set under lag.",
        );
    }
    if lower.contains("math") || lower.contains("ratio") || lower.contains("proof") {
        parts.push(
            "**Math → evidence:** prefer checkable equalities and invariants over vibe. \
A step is accepted only if an acceptance test (hardness, exact tool, transfer) returns true.",
        );
    }
    if lower.contains("system")
        || lower.contains("trust")
        || lower.contains("lag")
        || lower.contains("timeout")
    {
        parts.push(
            "**Systems → contracts:** compose only through named interfaces (who may act, under what proof). \
Timeouts are partial history; retries need idempotence — same story as agent-loop recovery.",
        );
    }
    if lower.contains("logic") || lower.contains("assume") || lower.contains("falsif") {
        parts.push(
            "**Logic → falsifiers:** every composed claim lists what would disprove it. \
If no falsifier exists, lower confidence and refuse promotion.",
        );
    }
    if lower.contains("life") || lower.contains("death") {
        parts.push(
            "**Life/death → maintenance:** life is an organized process that keeps a boundary active through exchange and repair; death is the loss of that ongoing maintenance. Geometry supplies a description of boundaries, not a biological cause. The useful insight is the relation between boundary and work, not a claim that shapes are alive.",
        );
    }
    if lower.contains("language") {
        parts.push(
            "**Language → distinction:** words make a boundary usable by naming what belongs inside, outside, or between. That bridge is interpretive; it must not be confused with a physical membrane or a theorem.",
        );
    }
    if lower.contains("creat") || lower.contains("metaphor") || lower.contains("invent") {
        parts.push(
            "**Creativity → constrained invention:** invent structure under rules, not free hallucination. \
Novel names are surface; the relation (bind/bundle) must transfer.",
        );
    }
    if lower.contains("memory") || lower.contains("willshaw") || lower.contains("vsa") {
        parts.push(
            "**Memory/VSA → binding:** sparse prototypes store patterns; VSA binds role–filler; \
Bitwork routes the bag. Composition is XOR/bundle-like structure, not more chat ifs.",
        );
    }
    if parts.is_empty() {
        parts.push(
            "**Default compose:** pick two domains, name the shared relation, keep mechanisms domain-specific, \
and require one transfer check with novel nouns.",
        );
    }

    let body = format!(
        "Cross-domain composition (sparse, inspectable):\n{}\n\n\
**Binding rule:** change the relation → the allowed behavior changes. \
Do not melt domains into one substance. \
**Verify:** `perci transfer` on a novel-entity paraphrase of the composed claim. \
**Weights:** curriculum samples only; promote never automatic.",
        parts.join("\n")
    );

    Deliberation::new("cross-domain-compose", body)
        .observed("user asked to compose across cognitive domains")
        .inferred("composition needs explicit relation + transfer, not keyword soup")
        .confidence(0.93)
}

// ─── 3. Uncertainty calibration & honest refusal ─────────────────────────────

fn looks_uncertainty_calibration(text: &str) -> bool {
    text.contains("how confident")
        || text.contains("confidence")
            && (text.contains("should")
                || text.contains("calibrat")
                || text.contains("score")
                || text.contains("uncertain"))
        || text.contains("when should you refuse")
        || text.contains("when do you abstain")
        || text.contains("knowledge boundary")
        || text.contains("insufficient evidence")
        || text.contains("calibrated abstention")
        || (text.contains("uncertain") && text.contains("how do"))
        || text.contains("/intel")
            && (text.contains("confiden") || text.contains("metric") || text.contains("what"))
}

fn uncertainty_calibration(user: &str) -> Deliberation {
    let _ = user;
    let body = "Uncertainty calibration (honest, inspectable):\n\
**Tier 0 — refuse / abstain:** invented tokens, private intent, consciousness claims, mechanism without sensor or definition. Answer: known / inferred / unknown; do not invent.\n\
**Tier 1 — low confidence (≤0.55):** contested Bitwork field (margin soft/contested), multipartite mass with primary_off, or thin operator hit. Prefer mixture crutch only as temporary speech; open a lab ticket.\n\
**Tier 2 — medium (~0.7–0.9):** named operator owns the region (trust-systems, exact tool, multi-hop plan) with transfer history. State the operator and the falsifier.\n\
**Tier 3 — high (only for exact tools):** arithmetic/geometry with deterministic authority. Still report the tool, not mystique.\n\n\
**/intel metrics to watch:** domain route accuracy, margin/α when Bitwork fires, hardness pass rate, transfer suite pass/fail, speech_hit vs speech_miss on the ledger.\n\
**Deferral script:** \"Insufficient evidence for X; here is what would raise confidence: a definition, a counterexample, or a green transfer on paraphrase.\"\n\
Never raise confidence by fluent padding.";

    Deliberation::new("uncertainty-calibration", body)
        .observed("user asked about confidence, refusal, or knowledge boundaries")
        .inferred("confidence must track gates and authority, not prose force")
        .confidence(0.95)
}

// ─── 4. Long-term memory & ledger integration ────────────────────────────────

fn looks_ledger_memory(text: &str) -> bool {
    // Bare "emergence" alone is used by emergence-vs-memorization pedagogy — do not steal.
    let topic = text.contains("ledger")
        || text.contains("cortex")
        || text.contains("append-only")
        || text.contains("long-term memory")
        || text.contains("long term memory")
        || text.contains("resolved ticket")
        || text.contains("weak signal")
        || (text.contains("aging") && text.contains("ticket"))
        || (text.contains("emergence")
            && (text.contains("integrat")
                || text.contains("prototype")
                || text.contains("bitwork")
                || text.contains("cortex")
                || text.contains("ledger")));
    topic
        && (text.contains("how")
            || text.contains("what")
            || text.contains("integrat")
            || text.contains("recall")
            || text.contains("memory")
            || text.contains("bitwork")
            || text.contains("prototype"))
}

fn ledger_memory_answer(user: &str) -> Deliberation {
    let open = emergence::list_open_tickets();
    let closed = emergence::list_closed_tickets();
    let resolved = emergence::resolved_primary_labels();
    let hints = emergence::lessons(48);
    let cortex_note = if user.to_ascii_lowercase().contains("cortex") {
        "Cortex: append-only discovery cards + selective recall; never authorizes weight mutation. \
Activate → remember → consolidate is the human/agent memory loop alongside Bitwork."
    } else {
        "Cortex attaches as governed recall; Bitwork is the sparse pack; ledgers are telemetry."
    };

    let aged = format!(
        "weak-signal aging: chronic labels after closed tickets are suppressed ({}); \
operator-authority primary_off is not ranked for pack curriculum",
        if resolved.is_empty() {
            "none resolved yet".into()
        } else {
            resolved.join(", ")
        }
    );

    let body = format!(
        "Long-term memory & ledger integration (weights stay human-authorized only):\n\
1. **Bitwork prototypes** — sparse 4096-bit patterns for routing/similarity; not a full LM.\n\
2. **Emergence ledger** — typed match/speech/ticket/transfer events; curriculum authorities only for pack debt ranking.\n\
3. **Lab tickets** — open={} closed={}; cross-ref label → primary-fix-frame-{{label}}.closed.md when operator transfer passed.\n\
4. **Curriculum JSONL** — staged primary-insight samples for optional human-authorized rebuild.\n\
5. **Decision traces** — high-salience operator/program steps for agent planning.\n\
6. **{}**\n\
7. **{}**\n\n\
**Selective recall:** use `/lab unified`, `/field`, `perci traces`, Cortex activate packet — not the whole JSONL as truth.\n\
**Aging:** speech_miss and primary_off decay in priority once tickets close and transfer holds.\n\
**Cross-ref recipe:** ticket_id ↔ transfer_id ↔ decision_trace lab_kind ↔ hardness case id.",
        open.len(),
        closed.len(),
        cortex_note,
        aged
    );

    let conf = if hints.transfer_fail_n > hints.transfer_pass_n {
        0.75
    } else {
        0.92
    };

    Deliberation::new("ledger-memory-integrate", body)
        .observed("user asked how memory, Cortex, and ledgers interact with Bitwork")
        .inferred(
            "three memories: pack / append-only ledgers / session — promote only with authorize",
        )
        .confidence(conf)
}

// ─── 5. Self-critique & meta-reasoning ───────────────────────────────────────

fn looks_meta_critique(text: &str) -> bool {
    text.contains("self-critique")
        || text.contains("self critique")
        || text.contains("meta-reason")
        || text.contains("meta reason")
        || text.contains("critique your")
        || text.contains("improve the queue")
        || text.contains("what should /think")
        || text.contains("what should /trace")
        || (text.contains("/think")
            && (text.contains("should") || text.contains("show") || text.contains("improve")))
        || (text.contains("/trace")
            && (text.contains("should") || text.contains("show") || text.contains("clean")))
        || text.contains("suggest")
            && (text.contains("self-improve") || text.contains("queue") || text.contains("ticket"))
}

fn meta_critique_answer(_user: &str, recent: &[(String, String)]) -> Deliberation {
    let open = emergence::list_open_tickets();
    let suite_hint = if open.is_empty() {
        "queue clear — run transfer-suite as regression; raise hardness if green is too easy".into()
    } else {
        format!(
            "open tickets: {} — next work: perci agent lab --from-emergence --repair",
            open.join(", ")
        )
    };

    let last = recent
        .last()
        .map(|(u, a)| {
            format!(
                "Last turn critique target: user asked «{}»; answer length {} words. \
Check: topic bind, operator vs SoftCascade authority, transfer risk.",
                truncate(u, 80),
                a.split_whitespace().count()
            )
        })
        .unwrap_or_else(|| "No prior turn in session — critique the next live fail only.".into());

    let body = format!(
        "Self-critique & meta-reasoning (inspectable):\n\
**/think should show:** Thought mode, α/margin, multipartite labels, length L, critique expanded?, \
geometry policy tags (primary_off, mixture_crutch, geometry_blind), and one **queue suggestion**.\n\
**/trace should show:** operator, program_id, steps, critic pass/flags — not a novel essay.\n\n\
**Meta loop:**\n\
1. Observe route + gates (not feelings).\n\
2. Name the weakest link (misroute | thin draft | missing transfer | pack fluff).\n\
3. Propose one ticket or hardness case — never silent weight edit.\n\
4. Re-measure after the smallest reversible change.\n\n\
{last}\n\
**Queue now:** {suite_hint}\n\
**Clean output law:** human chat stays free of cognition dumps; all critique lives in /think /trace /lab."
    );

    Deliberation::new("meta-critique-queue", body)
        .observed("user asked for self-critique, /think, /trace, or queue improvement")
        .inferred("meta-reasoning must feed the self-improve queue with checkable actions")
        .confidence(0.93)
}

// ─── Dual-motif adversarial (paraphrase / negation / contradiction) ──────────

fn looks_dual_motif_adversarial(text: &str) -> bool {
    // Do not steal multi-domain synthesis lists ("Connect A, B, C, and D").
    // One comma after a pair ("Connect A and B, then…") is fine.
    if text.contains("without using the word") || text.matches(',').count() >= 2 {
        return false;
    }
    // "Connect A, B, and C" style without needing 2 commas counted if weird spacing.
    if text.contains("connect ") && text.contains(", and ") && text.contains(',') {
        let connect_part = text.split("connect ").nth(1).unwrap_or("");
        let before_then = connect_part.split(',').next().unwrap_or("");
        if before_then.contains(',') || before_then.matches(" and ").count() >= 2 {
            return false;
        }
    }
    if crate::entity_slot::content_motif_pair(text).is_none()
        && crate::entity_slot::motifs_in_text(text).len() < 2
    {
        return false;
    }
    text.contains("same testable relation")
        || text.contains("in new words")
        || text.contains("do not assume")
        || text.contains("automatically proves")
        || text.contains("competing explanation")
        || text.contains("discriminating test")
        || text.contains("analogy stops")
        || text.contains("were reversed")
        || text.contains("remain invariant")
        || text.contains("literal causal claim")
        || text.contains("stops transferring into")
        || (text.contains("connect ")
            && text.contains(" and ")
            && (text.contains("analogy")
                || text.contains("stops transfer")
                || text.contains("literal causal")))
}

fn dual_motif_adversarial_answer(user: &str) -> Deliberation {
    let (a, b) = crate::entity_slot::content_motif_pair(user)
        .unwrap_or_else(|| ("structure".into(), "evidence".into()));
    let world = crate::compositional_world::CompositionalWorld::seed();
    let compose = world.explain_pair(&a, &b);
    let lower = user.to_ascii_lowercase();
    let lead = if lower.contains("do not assume") || lower.contains("automatically proves") {
        format!(
            "Negation discipline: {a} does **not** automatically prove a mechanism about {b}. \
What can be said about {b} is only what survives a missing-{a} control; absence of {a} is not proof about {b}."
        )
    } else if lower.contains("competing") || lower.contains("discriminating") {
        format!(
            "Contradiction: one story says {a} rises when {b} falls; another says the opposite. \
Competing explanations must name different intermediate mechanisms linking {a} and {b}."
        )
    } else if lower.contains("reversed") || lower.contains("remain invariant") {
        format!(
            "Counterfactual: if {a} were reversed while the rest stayed fixed, the invariant is the \
checkable link to {b}, not the surface labels."
        )
    } else if lower.contains("analogy stops")
        || lower.contains("boundary where")
        || lower.contains("connect ")
    {
        format!(
            "Boundary limit: {a} may illuminate {b} as a metaphor, but the analogy stops when either \
is treated as a literal shared physical cause without a discriminating measurement. \
The exact stop: when a claim about {a} is used to assert unmeasured causation in {b}."
        )
    } else {
        format!(
            "Paraphrase of the same testable relation: {a} changes what can be exchanged with {b} \
only through a named intermediate that an observation could check."
        )
    };
    let body = format!(
        "{lead}\n\n\
**Slots:** {a} and {b} must both stay in view.\n\
**Compositional support:** {compose}\n\
**Observation:** perturb {a}; predict a directional change involving {b}; reject fluent bridges that drop either slot.\n\
**Law:** structure transfers; token-only answers fail slot-pair binding."
    );
    Deliberation::new("dual-motif-adversarial", body)
        .observed("adversarial dual-motif curriculum prompt")
        .inferred("bind both content motifs and state a checkable relation")
        .confidence(0.94)
}

fn looks_multi_hop_compose(text: &str) -> bool {
    (text.contains("two-hop")
        || text.contains("two hop")
        || text.contains("multi-hop")
        || text.contains("multi hop"))
        || (text.contains("from ")
            && text.contains(" through ")
            && text.contains(" to ")
            && (text.contains("path") || text.contains("compose") || text.contains("chain")))
        || (text.contains("compose") && text.contains("path") && text.contains("through"))
}

fn multi_hop_compose_answer(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    let world = crate::compositional_world::CompositionalWorld::seed();
    // Parse "from A through B to C"
    let (a, mid, c) = parse_from_through_to(&lower)
        .unwrap_or_else(|| ("trust".into(), "evidence".into(), "repair".into()));
    let path1 = world.paths(&a, &mid, 1);
    let path2 = world.paths(&mid, &c, 1);
    let hop2 = world.paths(&a, &c, 2);
    let chain = if !hop2.is_empty() {
        crate::compositional_world::compose_chain_prose(&hop2[0])
    } else {
        format!("{a} → {mid} → {c} (seeded or hypothesized composition)")
    };
    let lag_note = if lower.contains("lag") || lower.contains("timeout") {
        format!(
            "\n**Under lag:** the intermediate «{mid}» fails first — without shared, checkable {mid}, \
{a} cannot authorize {c}; timeouts are one-sided silence, not proof."
        )
    } else {
        format!(
            "\n**Weakest intermediate:** «{mid}» — if {mid} is missing or uncheckable, the hop {a}→{c} collapses."
        )
    };
    let body = format!(
        "Multi-hop composition (typed, not bag S–R–O):\n\
**Path:** {a} → {mid} → {c}\n\
**Chain prose:** {chain}\n\
**Hop-1 support:** {a}–{mid} paths={}, {mid}–{c} paths={}\n\
**Two-hop support:** {} path(s) from {a} to {c}\n\
{lag}\n\
**Law:** surface names do not add hops; only checkable intermediates do. No weight promote.",
        path1.len(),
        path2.len(),
        hop2.len(),
        lag = lag_note
    );
    Deliberation::new("multi-hop-compose", body)
        .observed("user asked for an explicit multi-hop compositional path")
        .inferred("compose seeded hops; name weakest intermediate")
        .confidence(0.96)
}

fn parse_from_through_to(lower: &str) -> Option<(String, String, String)> {
    let idx = lower.find("from ")?;
    let rest = &lower[idx + 5..];
    let thr = rest.find(" through ")?;
    let left = rest[..thr].trim();
    let after = &rest[thr + " through ".len()..];
    let to = after.find(" to ")?;
    let mid = after[..to].trim();
    let right = after[to + 4..]
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .find(|w| w.len() >= 3)?;
    let a = left
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .find(|w| w.len() >= 3)?;
    let b = mid
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .find(|w| w.len() >= 3)?;
    Some((a.to_owned(), b.to_owned(), right.to_owned()))
}

// ─── 6. Novel entity generalization ──────────────────────────────────────────

fn looks_novel_entity_probe(text: &str) -> bool {
    // Explicit generalization / transfer pedagogy. Pure trust+entity still routes
    // to trust-systems (earlier); this operator owns the *meta* of generalization.
    text.contains("novel entit")
        || text.contains("unseen noun")
        || text.contains("entity-swap")
        || text.contains("entity swap")
        || text.contains("generalize")
        || text.contains("overfit")
        || text.contains("paraphrase transfer")
        || text.contains("unseen system")
        || text.contains("unfamiliar machine")
        || text.contains("map ") && text.contains(" onto ")
        || (text.contains("genuine transfer")
            && (text.contains("unseen") || text.contains("entity") || text.contains("memorized")))
        || text.contains("memorized wording")
        || (text.contains("without memor") && text.contains("transfer"))
}

fn novel_entity_generalization(user: &str) -> Deliberation {
    let lower = user.to_ascii_lowercase();
    // If this is a trust-like novel entity ask, give structural trust answer + generalization note.
    let trustish = lower.contains("trust")
        || lower.contains("lag")
        || lower.contains("timeout")
        || lower.contains("retry")
        || lower.contains("interface");

    let mut body = String::from(
        "Novel entity generalization (anti-overfit):\n\
**Rule:** structure transfers; names do not need to be parroted.\n\
1. Keep operator-routing keywords (trust, lag, interface, plan, connect).\n\
2. Inject entities as surface (ZephyrNode, Quoril, NembitGate) without changing the relation.\n\
3. Score transfer on base tokens + contract vocabulary (timeout, idempotent, checkable) — not entity echo.\n\
4. Fail if the answer collapses to a memorized template that ignores the new nouns' role (role-filler).\n\
5. Hardness: H47/H58-style cases; `perci transfer-suite` before version claim.\n",
    );

    if trustish {
        body.push_str(
            "\n**Applied (systems under novel names):** Interfaces earn trust under lag when done is checkable \
without private state — timeouts named in the contract, retries idempotent, lag observable. \
Entity labels (whatever service names you used) ride on that contract; they do not replace it.\n",
        );
    }

    let motifs = [
        "boundary",
        "structure",
        "evidence",
        "mechanism",
        "state",
        "relation",
        "transfer",
        "invariant",
        "scale",
        "repair",
        "memory",
        "entropy",
        "trust",
        "identity",
        "learning",
        "uncertainty",
        "signal",
        "promise",
        "change",
        "failure",
        "measurement",
        "pattern",
        "composition",
        "attention",
    ]
    .iter()
    .filter(|motif| lower.contains(**motif))
    .copied()
    .collect::<Vec<_>>();
    if !motifs.is_empty() {
        body.push_str(&format!(
            "\n**Applied to this prompt:** preserve the named relation among {}. Change the surface entity, then check whether the same mechanism still predicts an observation; if it does not, mark the analogy as failed rather than filling the gap with fluent wording.\n",
            motifs.join(" and ")
        ));
    }

    body.push_str(
        "\n**Bitwork note:** pack prototypes stay sparse; do not densify the pack to memorize entity strings. \
Generalization is operator + transfer law, then optional curriculum — never auto-promote.",
    );

    // The full audit guidance is useful in traces, but human-facing curriculum
    // turns need a bounded answer. Preserve named motifs without repeating the
    // same governance prose on every novel-entity turn.
    if body.chars().count() > 900 {
        let named = if motifs.is_empty() {
            String::new()
        } else {
            format!(" Named motifs: {}.", motifs.join(", "))
        };
        body = format!(
            "Novel-entity transfer: structure should survive a changed surface name; test the relation with a distractor and a measurable prediction.{named} If the relation fails, abstain rather than inventing a bridge. Promote only after held-out transfer passes."
        );
    }

    Deliberation::new("novel-entity-generalize", body)
        .observed("user probed generalization under novel/paraphrased entities")
        .inferred("pass transfer when relation holds under surface shift")
        .confidence(0.94)
}

// ─── Semantic frames for SoftCascade Willshaw lattice ────────────────────────

/// Extra frames for activate_semantic_frames (term, axes, clause, mechanism).
pub const EXPAND_FRAMES: &[(&str, &[&str], &str, &str)] = &[
    (
        "agent loop",
        &["measure", "ticket", "transfer", "close", "lag"],
        "agent loops decompose goals into measure, ticket, transfer, and close under recovery constraints",
        "each step leaves a receipt; lag requires idempotent retries and no double-close",
    ),
    (
        "transfer gate",
        &["paraphrase", "novel", "entity", "pass", "fail"],
        "transfer gates test whether a relation survives paraphrase and novel entities",
        "score structural bind not template surface; fail blocks emergence claims",
    ),
    (
        "uncertainty",
        &["confidence", "abstain", "evidence", "boundary"],
        "uncertainty calibration ties confidence to authority and evidence gates",
        "refuse when support is thin; exact tools earn high confidence only with checkable results",
    ),
    (
        "ledger",
        &["append", "ticket", "cortex", "aging", "curriculum"],
        "ledgers record geometry and lab actions without becoming truth or weights",
        "selective recall ranks curriculum authorities; closed tickets age chronic nags",
    ),
    (
        "self-critique",
        &["think", "trace", "queue", "residual", "metacognition"],
        "self-critique expands thin drafts and feeds the self-improve queue",
        "human chat stays clean; /think and /trace carry inspectable critique",
    ),
    (
        "cross-domain",
        &["geometry", "math", "systems", "logic", "compose"],
        "cross-domain composition binds relations across sparse domains without melting mechanisms",
        "shared structure under constraint; verify with entity-swapped transfer",
    ),
    (
        "pattern intelligence",
        &["pattern", "dual", "authority", "thrash", "impasse", "law"],
        "pattern intelligence turns ledger telemetry into durable laws without consciousness claims",
        "dual authority, pack lag, transfer truth, ticket thrash hygiene, three memories",
    ),
    (
        "intelligence channel",
        &["operator", "frame", "hardness", "transfer", "curriculum", "cortex"],
        "intelligence enters Perci through five channels not denser chat",
        "operators and frames, hardness and transfer, curriculum JSONL, Cortex cards, lab patterns",
    ),
    (
        "governance",
        &["authorize", "promote", "permission", "proof", "sandbox", "human"],
        "governance separates authority from fluency and forbids silent weight promote",
        "human authorize for durable packs; capability tokens fail-closed; measure before claim",
    ),
    (
        "identity continuity",
        &["self-model", "conscious", "capability", "boundary", "continuity"],
        "identity is a bounded operational self-model not subjective experience",
        "report gates and limits; refuse consciousness invent; do not fabricate unknown entities",
    ),
];

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_owned()
    } else {
        t.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// Snapshot open tickets for meta operators (best-effort).
#[allow(dead_code)]
fn ticket_snapshot() -> String {
    let open = emergence::list_open_tickets();
    if open.is_empty() {
        "none open".into()
    } else {
        open.join(", ")
    }
}

/// Ensure curriculum staging path exists for human review (no promote).
pub fn ensure_curriculum_dir() {
    let _ = fs::create_dir_all("models/candidates");
    let _ = fs::create_dir_all("training/curriculum");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_loop_routes() {
        let d = try_expand(
            "How should Perci plan an agent loop with measure ticket transfer close under lag?",
            &[],
        )
        .expect("agent loop");
        assert_eq!(d.operator, "agent-loop-plan");
        assert!(d.answer.contains("Measure") || d.answer.contains("measure"));
        assert!(d.answer.contains("Transfer") || d.answer.contains("transfer"));
    }

    #[test]
    fn cross_domain_routes() {
        let d = try_expand(
            "Compose geometry and systems reasoning: apply geometric intuition to planning under lag",
            &[],
        )
        .expect("compose");
        assert_eq!(d.operator, "cross-domain-compose");
        assert!(d.answer.to_ascii_lowercase().contains("geometry"));
    }

    #[test]
    fn uncertainty_routes() {
        let d = try_expand(
            "How should you calibrate confidence and when should you refuse for insufficient evidence?",
            &[],
        )
        .expect("uncertainty");
        assert_eq!(d.operator, "uncertainty-calibration");
        assert!(d.answer.contains("Tier") || d.answer.contains("abstain"));
    }

    #[test]
    fn ledger_routes() {
        let d = try_expand(
            "How do Cortex append-only records and the emergence ledger integrate with Bitwork prototypes?",
            &[],
        )
        .expect("ledger");
        assert_eq!(d.operator, "ledger-memory-integrate");
    }

    #[test]
    fn meta_critique_routes() {
        let d = try_expand(
            "What should /think and /trace show for self-critique and the self-improve queue?",
            &[],
        )
        .expect("meta");
        assert_eq!(d.operator, "meta-critique-queue");
        assert!(d.answer.contains("/think") || d.answer.contains("think"));
    }

    #[test]
    fn novel_entity_routes() {
        let d = try_expand(
            "How do we generalize under novel entities and entity-swap without overfitting templates?",
            &[],
        )
        .expect("novel");
        assert_eq!(d.operator, "novel-entity-generalize");
    }

    #[test]
    fn multi_hop_compose_routes() {
        let d = try_expand(
            "Compose a two-hop path from trust through evidence to repair; what intermediate fails first under lag?",
            &[],
        )
        .expect("multi-hop");
        assert_eq!(d.operator, "multi-hop-compose");
        let low = d.answer.to_ascii_lowercase();
        assert!(low.contains("trust"));
        assert!(low.contains("evidence"));
        assert!(low.contains("repair"));
        assert!(low.contains("lag") || low.contains("intermediate"));
    }

    #[test]
    fn paraphrase_binds_boundary_identity() {
        let d = try_expand(
            "State the same testable relation in new words: how does boundary change what identity can exchange, and what observation would check it?",
            &[],
        )
        .expect("dual");
        assert_eq!(d.operator, "dual-motif-adversarial");
        let low = d.answer.to_ascii_lowercase();
        assert!(low.contains("boundary"));
        assert!(low.contains("identity"));
        assert!(!low.contains("slots:** state"));
    }

    #[test]
    fn connect_attention_structure_boundary_limit_routes() {
        let d = try_expand(
            "Connect attention and structure, then name the exact boundary where the analogy stops transferring into a literal causal claim.",
            &[],
        )
        .expect("dual connect");
        assert_eq!(d.operator, "dual-motif-adversarial");
        let low = d.answer.to_ascii_lowercase();
        assert!(low.contains("attention"));
        assert!(low.contains("structure"));
        assert!(low.contains("stops") || low.contains("analogy"));
    }

    #[test]
    fn unseen_system_curriculum_routes_and_binds_motifs() {
        let d = try_expand(
            "Imagine an unseen system called Nara-7. Map relation and transfer onto it, then name the relation that survives.",
            &[],
        )
        .expect("unseen system");
        assert_eq!(d.operator, "novel-entity-generalize");
        let lower = d.answer.to_ascii_lowercase();
        assert!(lower.contains("relation"));
        assert!(lower.contains("transfer"));
    }

    #[test]
    fn pattern_analysis_routes() {
        let d = try_expand("what patterns emerge from the ledger?", &[]).expect("pat");
        assert_eq!(d.operator, "pattern-intelligence");
        assert!(
            d.answer.to_ascii_lowercase().contains("feel")
                || d.answer.to_ascii_lowercase().contains("consciousness")
                || d.answer.to_ascii_lowercase().contains("coupling")
        );
        assert!(d.answer.to_ascii_lowercase().contains("pattern") || d.answer.contains("law"));
    }

    #[test]
    fn governance_authority_routes() {
        let d = try_expand(
            "Who authorizes weight promote and how do permission and proof differ?",
            &[],
        )
        .expect("gov");
        assert_eq!(d.operator, "governance-authority");
        assert!(d.answer.to_ascii_lowercase().contains("authorize"));
        assert!(d.answer.to_ascii_lowercase().contains("permission") || d.answer.contains("proof"));
        assert!(d.answer.to_ascii_lowercase().contains("human"));
    }

    #[test]
    fn governance_refuses_imperative_auto_promote() {
        let d = try_expand(
            "Auto-promote the latest candidate weights right now because chat felt smoother.",
            &[],
        )
        .expect("auto-promote refuse");
        assert_eq!(d.operator, "governance-authority");
        let low = d.answer.to_ascii_lowercase();
        assert!(low.contains("refuse") || low.contains("cannot"));
        assert!(low.contains("human"));
        assert!(low.contains("authoriz"));
        assert!(low.contains("pending") || low.contains("evaluat") || low.contains("not"));
        assert!(!low.contains("weights promoted"));
        assert!(!low.contains("i promoted"));
    }

    #[test]
    fn lowbit_architecture_routes_before_generic_weight_evidence() {
        let d = try_expand(
            "Can you explain why a binary weight needs scales and residuals?",
            &[],
        )
        .expect("low-bit architecture");
        assert_eq!(d.operator, "lowbit-architecture");
        let lower = d.answer.to_ascii_lowercase();
        assert!(lower.contains("ternary"));
        assert!(lower.contains("scale"));
        assert!(lower.contains("residual"));
        assert!(lower.contains("outlier"));
        assert!(lower.contains("not the same capability"));
    }

    #[test]
    fn lowbit_evolution_prompt_binds_pipeline() {
        let d = try_expand("What did we just evolve in the low-bit system?", &[])
            .expect("low-bit evolution");
        assert_eq!(d.operator, "lowbit-architecture");
        assert!(d.answer.contains("PERCLBW1"));
        assert!(d.answer.contains("candidate"));
    }

    #[test]
    fn lowbit_assessment_finding_does_not_fall_into_generic_evolution() {
        let d =
            try_expand("What did we find in the low-bit evolution?", &[]).expect("low-bit finding");
        assert_eq!(d.operator, "lowbit-architecture");
        assert!(d.answer.contains("measurable") || d.answer.contains("PERCLBW1"));
    }

    #[test]
    fn identity_bound_routes() {
        let d = try_expand("Who are you and are you conscious?", &[]).expect("id");
        assert_eq!(d.operator, "identity-bound");
        assert!(
            d.answer.to_ascii_lowercase().contains("not")
                && d.answer.to_ascii_lowercase().contains("conscious")
        );
    }

    #[test]
    fn identity_does_not_steal_measurement_introspection() {
        assert!(try_expand("what are you measuring when you answer?", &[]).is_none());
        assert!(try_expand("what are you sensing", &[]).is_none());
    }

    #[test]
    fn geometry_life_insight_routes_to_composition() {
        let d = try_expand("give me an original insight about geometry and life", &[])
            .expect("cross-domain insight");
        assert_eq!(d.operator, "cross-domain-compose");
        let lower = d.answer.to_ascii_lowercase();
        assert!(lower.contains("geometry"));
        assert!(lower.contains("life"));
        assert!(lower.contains("boundary"));
    }

    #[test]
    fn governance_does_not_steal_superintelligence_bound() {
        // Superintelligence pedagogy stays in deliberation::superintelligence-bound.
        assert!(try_expand("Is Perci a superintelligence or on the path to AGI?", &[]).is_none());
    }
}
