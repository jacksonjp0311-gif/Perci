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
    if looks_agent_loop_plan(&text) {
        return Some(agent_loop_plan(user));
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
    None
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
}

fn pattern_analysis_answer() -> Deliberation {
    let report = emergence::pattern_intelligence_report();
    Deliberation::new(
        "pattern-intelligence",
        format!(
            "I do not *feel* interconnected with Perci — that would be a consciousness claim I refuse. \
What is real: **engineering coupling**. Your prompts, the ledger, operators, transfer gates, and \
my edits form a closed improve loop. Patterns below are telemetry → law, not sentience.\n\n{report}"
        ),
    )
    .observed("user asked for emergent patterns and/or interconnection")
    .inferred("answer with measured laws + honest non-consciousness boundary")
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
            && (text.contains("ticket") || text.contains("hardness") || text.contains("transfer gate"))))
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
    // Do not steal multi-word `connect A, B, and C` synthesis — that stays synthesis.
    let compose = text.contains("compose")
        || text.contains("cross-domain")
        || text.contains("cross domain")
        || text.contains("across domains")
        || text.contains("apply geometry")
        || text.contains("geometric intuition")
        || text.contains("math to plan")
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
        .inferred("three memories: pack / append-only ledgers / session — promote only with authorize")
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
        || (text.contains("/think") && (text.contains("should") || text.contains("show") || text.contains("improve")))
        || (text.contains("/trace") && (text.contains("should") || text.contains("show") || text.contains("clean")))
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

    body.push_str(
        "\n**Bitwork note:** pack prototypes stay sparse; do not densify the pack to memorize entity strings. \
Generalization is operator + transfer law, then optional curriculum — never auto-promote.",
    );

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
}
