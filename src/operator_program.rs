//! Operator programs: compose named cognitive steps instead of one-off replies.
//!
//! This module is the scaffold for the next architecture step:
//! intent → program of operators → evidence bindings → tool calls → critic → answer.
//!
//! It does not invent private chain-of-thought. Programs are inspectable plans
//! with named steps and critic checks. Full runtime integration is gradual;
//! deliberation remains the high-salience operator path today.

use crate::deliberation::{normalize_input, Deliberation};

/// One step in an inspectable cognitive program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStep {
    pub name: &'static str,
    pub purpose: &'static str,
}

/// A bounded program selected for a user prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorProgram {
    pub program_id: &'static str,
    pub steps: &'static [OperatorStep],
    pub critic_checks: &'static [&'static str],
    pub primary_layer: &'static str,
}

/// Critic report over a candidate answer relative to a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriticReport {
    pub ok: bool,
    pub flags: Vec<&'static str>,
    pub notes: Vec<String>,
}

const SYNTHESIS_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "parse_frames",
        purpose: "extract named domains from the prompt",
    },
    OperatorStep {
        name: "bind_shared_axis",
        purpose: "find a relation supported by at least three frames",
    },
    OperatorStep {
        name: "compose_bridge",
        purpose: "state the bridge without collapsing mechanisms",
    },
    OperatorStep {
        name: "state_limit",
        purpose: "mark that shared relation is not shared substance",
    },
];

const RELATIONAL_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "bind_both_frames",
        purpose: "keep both semantic frames in view",
    },
    OperatorStep {
        name: "name_interaction",
        purpose: "state how the frames constrain one another",
    },
    OperatorStep {
        name: "mark_mechanism_boundary",
        purpose: "separate analogy from causal identity",
    },
];

const FOLLOWUP_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "recover_prior",
        purpose: "bind to the most recent substantive answer or synthesis",
    },
    OperatorStep {
        name: "apply_requested_operation",
        purpose: "testability, counterexample, or analogy limit as asked",
    },
    OperatorStep {
        name: "avoid_stale_preset",
        purpose: "do not answer a previous geometry/life example by default",
    },
];

const TRANSFER_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "hold_out_template",
        purpose: "replace surface form and entities",
    },
    OperatorStep {
        name: "score_transfer_metrics",
        purpose: "correctness, coverage, stability, abstention, regression",
    },
    OperatorStep {
        name: "compare_baselines",
        purpose: "keyword/template baseline and operator ablation",
    },
];

const TOOL_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "classify_exact_intent",
        purpose: "detect arithmetic or geometry request",
    },
    OperatorStep {
        name: "execute_deterministic_tool",
        purpose: "checked rational or geometry solver",
    },
    OperatorStep {
        name: "return_before_generation",
        purpose: "preserve tool authority over associative text",
    },
];

const GOVERNED_LEARN_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "stage_candidate",
        purpose: "record pending evidence without weight mutation",
    },
    OperatorStep {
        name: "require_review",
        purpose: "human label/approval before fold",
    },
    OperatorStep {
        name: "sealed_eval_before_promote",
        purpose: "hardness + operational gates with explicit authorize",
    },
];

const MATH_EXPLAIN_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "detect_explanatory_intent",
        purpose: "block exact integer parser for why/how equality questions",
    },
    OperatorStep {
        name: "state_definition_basis",
        purpose: "explain via number-system rules, not association",
    },
    OperatorStep {
        name: "offer_tool_path",
        purpose: "point calculate/compute for pure numeric results",
    },
];

const CODE_SNIPPET_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "detect_language_and_task",
        purpose: "pick rust/python and the requested operation",
    },
    OperatorStep {
        name: "emit_compilable_snippet",
        purpose: "return real source with a checkable invariant",
    },
    OperatorStep {
        name: "state_edge_limits",
        purpose: "unicode/empty-input caveats when relevant",
    },
];

const MULTI_HOP_STEPS: &[OperatorStep] = &[
    OperatorStep {
        name: "name_goal",
        purpose: "bind plan goal to user content",
    },
    OperatorStep {
        name: "list_known_unknown",
        purpose: "separate evidence from missing constraints",
    },
    OperatorStep {
        name: "ordered_steps_and_done_when",
        purpose: "checkable steps plus acceptance test",
    },
];

/// Select a program for a prompt when a multi-step plan is useful.
pub fn plan_for_prompt(user: &str) -> Option<OperatorProgram> {
    let text = normalize_input(user).to_ascii_lowercase();

    if crate::reasoning::is_explanatory_math(&text) {
        return Some(OperatorProgram {
            program_id: "math_explanation",
            steps: MATH_EXPLAIN_STEPS,
            critic_checks: &["forbids_comfort_collapse", "no_associative_override"],
            primary_layer: "operator",
        });
    }

    if (text.contains("write ") || text.contains("implement ") || text.contains("function that"))
        && (text.contains("rust")
            || text.contains("python")
            || text.contains("string")
            || text.contains("code"))
    {
        return Some(OperatorProgram {
            program_id: "code_snippet",
            steps: CODE_SNIPPET_STEPS,
            critic_checks: &["emits_code_fence"],
            primary_layer: "tool",
        });
    }

    if text.contains("connect ")
        && (text.contains(" coherent")
            || text.contains("shared principle")
            || text.contains("shared structure")
            || text.contains("one idea")
            || text.contains(" and ")
            || text.matches(',').count() >= 2)
    {
        return Some(OperatorProgram {
            program_id: "cross_domain_synthesis",
            steps: SYNTHESIS_STEPS,
            critic_checks: &[
                "names_all_requested_domains",
                "states_shared_axis",
                "states_mechanism_boundary",
                "forbids_comfort_collapse",
            ],
            primary_layer: "operator",
        });
    }

    if (text.contains("compare ") || text.contains("difference between") || text.contains("how are "))
        && (text.contains(" and ") || text.contains(" vs ") || text.contains(" related"))
    {
        return Some(OperatorProgram {
            program_id: "relational_inquiry",
            steps: RELATIONAL_STEPS,
            critic_checks: &[
                "binds_both_frames",
                "names_interaction",
                "forbids_generic_checklist",
            ],
            primary_layer: "operator",
        });
    }

    if text.contains("which part") && text.contains("testable")
        || text.contains("where does your analogy")
        || text.contains("counterexample to your conclusion")
        || text.contains("assumption is doing the most work")
    {
        return Some(OperatorProgram {
            program_id: "followup_binding",
            steps: FOLLOWUP_STEPS,
            critic_checks: &["binds_prior", "performs_requested_op", "avoids_stale_preset"],
            primary_layer: "operator",
        });
    }

    if text.contains("transfer")
        && (text.contains("template") || text.contains("memorized") || text.contains("pattern matching"))
    {
        return Some(OperatorProgram {
            program_id: "transfer_vs_template",
            steps: TRANSFER_STEPS,
            critic_checks: &["proposes_entity_swap", "names_metrics", "mentions_baseline"],
            primary_layer: "operator",
        });
    }

    if text.contains("calculate")
        || text.contains("percent of")
        || text.contains("pythagorean")
        || text.contains("triangle area")
    {
        return Some(OperatorProgram {
            program_id: "exact_tool_authority",
            steps: TOOL_STEPS,
            critic_checks: &["numeric_or_symbolic_result", "no_associative_override"],
            primary_layer: "tool",
        });
    }

    if text.contains("make a plan")
        || text.contains("step-by-step")
        || text.contains("break this into")
        || text.contains("what are the steps")
    {
        return Some(OperatorProgram {
            program_id: "multi_hop_plan",
            steps: MULTI_HOP_STEPS,
            critic_checks: &["forbids_comfort_collapse"],
            primary_layer: "operator",
        });
    }

    if text.contains("learn that")
        || text.contains("evolve this system")
        || text.contains("promote")
            && (text.contains("weight") || text.contains("evidence") || text.contains("candidate"))
    {
        return Some(OperatorProgram {
            program_id: "governed_learning_loop",
            steps: GOVERNED_LEARN_STEPS,
            critic_checks: &["pending_not_promoted", "names_eval_gate"],
            primary_layer: "pipeline",
        });
    }

    None
}

/// Critic: check a candidate answer against the program's declared checks.
pub fn critic_program(user: &str, answer: &str, program: &OperatorProgram) -> CriticReport {
    let lower = answer.to_ascii_lowercase();
    let user_l = normalize_input(user).to_ascii_lowercase();
    let mut flags: Vec<&'static str> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    for check in program.critic_checks {
        match *check {
            "forbids_comfort_collapse" => {
                if lower.contains("stuck is normal") || lower.contains("friction is real") {
                    flags.push("comfort_collapse");
                    notes.push("answer collapsed into social comfort".into());
                }
            }
            "forbids_generic_checklist" => {
                if lower.contains("name one fact that would update")
                    || lower.contains("compare on capability, correctness, latency")
                {
                    flags.push("generic_checklist");
                    notes.push("generic comparison template leaked".into());
                }
            }
            "names_all_requested_domains" => {
                for term in extract_connect_terms(&user_l) {
                    if !domain_mentioned_in_answer(&lower, &term) {
                        flags.push("missing_domain");
                        notes.push(format!("missing domain term: {term}"));
                    }
                }
            }
            "states_shared_axis" | "states_mechanism_boundary" => {
                let axisish = lower.contains("bridge")
                    || lower.contains("together they")
                    || lower.contains("shared")
                    || lower.contains("relation");
                let limitish = lower.contains("not")
                    && (lower.contains("mechanism")
                        || lower.contains("identical")
                        || lower.contains("substance")
                        || lower.contains("distinct"));
                if *check == "states_shared_axis" && !axisish {
                    flags.push("missing_shared_axis");
                }
                if *check == "states_mechanism_boundary" && !limitish {
                    flags.push("missing_mechanism_boundary");
                }
            }
            "binds_both_frames" => {
                // crude: at least two content words from user appear
                let words: Vec<&str> = user_l
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .filter(|w| w.len() >= 4)
                    .filter(|w| {
                        !matches!(
                            *w,
                            "what" | "difference" | "between" | "compare" | "related" | "about"
                        )
                    })
                    .take(4)
                    .collect();
                let hits = words.iter().filter(|w| lower.contains(*w)).count();
                if words.len() >= 2 && hits < 2 {
                    flags.push("single_frame_collapse");
                }
            }
            "names_interaction" => {
                if !(lower.contains("convert")
                    || lower.contains("shape")
                    || lower.contains("constrain")
                    || lower.contains("interact")
                    || lower.contains("through")
                    || lower.contains("into")
                    || lower.contains("while"))
                {
                    flags.push("missing_interaction");
                }
            }
            "binds_prior" | "performs_requested_op" | "avoids_stale_preset" => {
                if lower.contains("not enough local support") && user_l.contains("testable") {
                    flags.push("generic_abstain_on_followup");
                }
                if lower.contains("geometry")
                    && lower.contains("life")
                    && user_l.contains("testable")
                    && !(user_l.contains("geometry") || user_l.contains("life"))
                {
                    flags.push("stale_preset");
                }
            }
            "proposes_entity_swap" => {
                if !(lower.contains("unseen")
                    || lower.contains("hold out")
                    || lower.contains("held-out")
                    || lower.contains("replace the nouns")
                    || lower.contains("entity"))
                {
                    flags.push("missing_entity_swap");
                }
            }
            "names_metrics" => {
                if !(lower.contains("correct")
                    || lower.contains("regression")
                    || lower.contains("abstention")
                    || lower.contains("coverage")
                    || lower.contains("stability"))
                {
                    flags.push("missing_metrics");
                }
            }
            "mentions_baseline" => {
                if !(lower.contains("template")
                    || lower.contains("keyword")
                    || lower.contains("baseline")
                    || lower.contains("ablation"))
                {
                    flags.push("missing_baseline");
                }
            }
            "numeric_or_symbolic_result" => {
                let has_digit = answer.chars().any(|c| c.is_ascii_digit());
                if !has_digit && !lower.contains("cannot") {
                    flags.push("missing_numeric_result");
                }
            }
            "no_associative_override" => {
                if lower.contains("override") && lower.contains("yes") {
                    flags.push("associative_override_claimed");
                }
            }
            "pending_not_promoted" => {
                if lower.contains("promoted automatically") || lower.contains("weights updated") {
                    flags.push("illegal_promotion_claim");
                }
            }
            "names_eval_gate" => {
                if user_l.contains("evolve")
                    && !(lower.contains("evaluat")
                        || lower.contains("held-out")
                        || lower.contains("test")
                        || lower.contains("measur"))
                {
                    flags.push("missing_eval_gate");
                }
            }
            "emits_code_fence" => {
                if !(answer.contains("```")
                    || lower.contains("fn ")
                    || lower.contains("def ")
                    || lower.contains("function "))
                {
                    flags.push("missing_code_snippet");
                    notes.push("code program expected a source fence or function body".into());
                }
            }
            _ => {}
        }
    }

    CriticReport {
        ok: flags.is_empty(),
        flags,
        notes,
    }
}

/// Attach program metadata onto a deliberation audit when available.
/// On critical flags (comfort collapse / generic checklist), rewrite the answer.
/// This is the live operator-program runtime: select → critic → optional rewrite.
pub fn annotate_deliberation(user: &str, deliberation: &mut Deliberation) {
    if let Some(program) = plan_for_prompt(user) {
        let step_names: Vec<&'static str> = program.steps.iter().map(|s| s.name).collect();
        deliberation.program_id = Some(program.program_id);
        deliberation.program_steps = step_names;
        deliberation.observations.push(format!(
            "operator_program={} layer={} steps={}",
            program.program_id,
            program.primary_layer,
            program
                .steps
                .iter()
                .map(|s| s.name)
                .collect::<Vec<_>>()
                .join("→")
        ));
        let report = critic_program(user, &deliberation.answer, &program);
        deliberation.critic_ok = Some(report.ok);
        if report.ok {
            deliberation
                .inferences
                .push("program critic: pass".to_owned());
        } else {
            deliberation.uncertainties.push(format!(
                "program critic flags: {}",
                report.flags.join(", ")
            ));
            if report.flags.iter().any(|f| {
                matches!(
                    *f,
                    "comfort_collapse" | "generic_checklist" | "missing_domain"
                )
            }) {
                if let Some(rewritten) = rewrite_after_critic(user, &program, &report) {
                    deliberation.answer = rewritten;
                    deliberation
                        .inferences
                        .push("program critic: rewrite applied".to_owned());
                    deliberation.confidence = (deliberation.confidence * 0.9).clamp(0.4, 0.95);
                    // Re-score after rewrite for honest critic_ok.
                    let again = critic_program(user, &deliberation.answer, &program);
                    deliberation.critic_ok = Some(again.ok);
                    return;
                }
            }
            deliberation.confidence = (deliberation.confidence * 0.85).clamp(0.35, 0.99);
        }
    }
}

/// Run the program runtime on a finished answer (exact tools / associative path).
pub fn apply_program_runtime(user: &str, mut deliberation: Deliberation) -> Deliberation {
    annotate_deliberation(user, &mut deliberation);
    deliberation
}

/// Extract domain terms from a connect-style prompt.
///
/// Delegates to deliberation's phrase-aware parser (paren strip, multi-word fold,
/// meta-token filter) so critic rewrites do not re-shatter "sparse memory" /
/// "vector symbolic architectures" into single-word placeholders.
pub fn extract_connect_terms(user_lower: &str) -> Vec<String> {
    crate::deliberation::connect_terms_for_prompt(user_lower).unwrap_or_default()
}

/// Multi-word domains match if the full phrase appears, or content tokens do.
/// Also accepts **canonical catalog aliases** (e.g. "vector symbolic architectures"
/// answered as "vector symbolic binding") so critic does not false-flag missing_domain.
pub fn domain_mentioned_in_answer(answer_lower: &str, term: &str) -> bool {
    let mut candidates = vec![term.to_ascii_lowercase()];
    if let Some(canon) = crate::deliberation::canonical_domain_term(term) {
        if !candidates.iter().any(|c| c == &canon) {
            candidates.push(canon);
        }
    }
    let t = term.to_ascii_lowercase();
    if t.contains("vector") && t.contains("symbolic") {
        candidates.push("vector symbolic binding".into());
        candidates.push("bind/bundle".into());
        candidates.push("role-filler".into());
    }
    if t.contains("sparse") && t.contains("memory") {
        candidates.push("sparse distributed memory".into());
        candidates.push("high-dimensional address".into());
    }
    candidates
        .iter()
        .any(|phrase| phrase_covered(answer_lower, phrase))
}

fn phrase_covered(answer_lower: &str, phrase: &str) -> bool {
    let p = phrase.to_ascii_lowercase();
    if p.is_empty() {
        return false;
    }
    if answer_lower.contains(&p) {
        return true;
    }
    let tokens: Vec<&str> = p
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 2)
        .filter(|w| {
            !matches!(
                *w,
                "the" | "and" | "for" | "with" | "from" | "that" | "this" | "into"
            )
        })
        .collect();
    if tokens.len() >= 2 {
        // Majority of content tokens — alias clauses may drop a tail word
        // ("architectures") while keeping the domain ("vector symbolic binding").
        let need = tokens.len().saturating_mul(2).div_ceil(3).max(2);
        let hits = tokens.iter().filter(|w| answer_lower.contains(**w)).count();
        return hits >= need;
    }
    if tokens.len() == 1 {
        return answer_lower.contains(tokens[0]);
    }
    false
}

fn rewrite_role_for_term(term: &str, index: usize) -> String {
    let t = term.to_ascii_lowercase();
    if t.contains("sparse") || t.contains("distributed memory") || t.contains("sdm") {
        format!("{term} stores patterns by similarity in a high-dimensional address space")
    } else if t.contains("vector symbolic")
        || t.contains("symbolic architect")
        || (t.contains("vector") && t.contains("symbolic"))
        || t.contains("binding")
        || t.contains("vsa")
        || t.contains("hdc")
    {
        format!("{term} composes role–filler structure with bind/bundle operations")
    } else if t.contains("memory") {
        format!("{term} reconstructs past state from stored traces under partial cues")
    } else if t.contains("bitwork") {
        format!("{term} routes prompts through packed binary prototypes and expert masks")
    } else if t.contains("impasse") {
        format!("{term} opens a bounded subgoal when the current path cannot proceed")
    } else if t.contains("hardness") || t.contains("gate") {
        format!("{term} refuses promotion unless held-out transfer stays green")
    } else {
        let role = match index % 3 {
            0 => "organizes parts so a larger pattern holds under stress",
            1 => "absorbs shocks without losing the relation it keeps",
            _ => "negotiates limits between what can change and what must persist",
        };
        format!("{term} {role}")
    }
}

fn rewrite_after_critic(
    user: &str,
    program: &OperatorProgram,
    report: &CriticReport,
) -> Option<String> {
    let lower = normalize_input(user).to_ascii_lowercase();
    if program.program_id == "cross_domain_synthesis" || lower.contains("connect ") {
        let terms = extract_connect_terms(&lower);
        if terms.len() >= 2 {
            let clauses: Vec<String> = terms
                .iter()
                .enumerate()
                .map(|(i, t)| rewrite_role_for_term(t, i))
                .collect();
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
            // Flags stay in audit/trace — never append "Critic rewrite after flags"
            // into user-facing speech.
            let _ = report;
            return Some(format!(
                "A workable bridge is constrained structure: {joined}. Together they show structure under constraint—pattern, integrity, and repair—while mechanisms stay domain-specific (not one shared substance)."
            ));
        }
    }
    if program.program_id == "relational_inquiry" {
        let _ = report;
        return Some(
            "Direct repair: keep both sides of the relation visible, name the shared axis, and mark that mechanisms differ. Ask again with the two nouns if this still misses."
                .to_owned(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_synthesis_program() {
        let program = plan_for_prompt(
            "Connect entropy, memory, and learning in one coherent thought.",
        )
        .expect("program");
        assert_eq!(program.program_id, "cross_domain_synthesis");
        assert!(program.steps.len() >= 3);
    }

    #[test]
    fn critic_flags_comfort_collapse() {
        let program = plan_for_prompt(
            "Connect language, code, and culture through one shared principle.",
        )
        .expect("program");
        let report = critic_program(
            "Connect language, code, and culture through one shared principle.",
            "Stuck is normal. One concrete detail and we can cut a path.",
            &program,
        );
        assert!(!report.ok);
        assert!(report.flags.contains(&"comfort_collapse"));
    }

    #[test]
    fn critic_passes_good_synthesis() {
        let user = "Connect entropy, memory, and learning in one coherent thought.";
        let program = plan_for_prompt(user).expect("program");
        let answer = "A coherent bridge is change: entropy gives macroscopic change a statistical direction; memory carries selected past state into present behavior; learning updates future performance from retained traces. Together they show change as a bridge between prior state, present behavior, and future possibility—not as one shared substance or cause.";
        let report = critic_program(user, answer, &program);
        assert!(report.ok, "flags={:?}", report.flags);
    }

    #[test]
    fn multi_word_domains_match_by_tokens() {
        let user =
            "Connect sparse distributed memory, vector symbolic binding, and Bitwork in one coherent thought.";
        let program = plan_for_prompt(user).expect("program");
        let answer = "A workable bridge is constrained structure: sparse distributed memory stores patterns by similarity; vector symbolic binding composes role–filler structure; and Bitwork routes prompts through packed binary prototypes. They are comparable as systems that keep form under pressure while mechanisms stay domain-specific—not one shared substance.";
        let report = critic_program(user, answer, &program);
        assert!(report.ok, "flags={:?} notes={:?}", report.flags, report.notes);
        assert!(domain_mentioned_in_answer(
            &answer.to_ascii_lowercase(),
            "sparse distributed memory"
        ));
    }

    #[test]
    fn extract_connect_folds_space_list_phrases() {
        let terms = extract_connect_terms(
            "connect sparse memory and vector symbolic architectures",
        );
        assert_eq!(terms.len(), 2, "terms={terms:?}");
        assert!(terms.iter().any(|t| t.contains("sparse memory") || t == "sparse memory"));
        assert!(terms.iter().any(|t| t.contains("vector symbolic")));
        assert!(!terms.iter().any(|t| t == "architectures"));
    }

    #[test]
    fn domain_match_accepts_catalog_alias_for_vsa() {
        let answer = "A workable bridge is constrained structure: sparse distributed memory stores patterns by similarity; vector symbolic binding composes role-filler structure. Together they show a shared relation under constraint while mechanisms stay domain-specific—not one shared substance.";
        let lower = answer.to_ascii_lowercase();
        assert!(domain_mentioned_in_answer(&lower, "sparse memory"));
        assert!(domain_mentioned_in_answer(
            &lower,
            "vector symbolic architectures"
        ));
        let user = "connect sparse memory and vector symbolic architectures";
        let program = plan_for_prompt(user).expect("program");
        let report = critic_program(user, answer, &program);
        assert!(
            report.ok,
            "alias-aware critic should pass: flags={:?} notes={:?}",
            report.flags,
            report.notes
        );
    }

    #[test]
    fn rewrite_does_not_leak_critic_footer() {
        let program = plan_for_prompt(
            "connect sparse memory and vector symbolic architectures",
        )
        .expect("program");
        let report = CriticReport {
            ok: false,
            flags: vec!["missing_domain"],
            notes: vec!["test".into()],
        };
        let out = rewrite_after_critic(
            "connect sparse memory and vector symbolic architectures",
            &program,
            &report,
        )
        .expect("rewrite");
        assert!(!out.to_ascii_lowercase().contains("critic rewrite"));
        assert!(!out.to_ascii_lowercase().contains("missing_domain"));
        assert!(out.to_ascii_lowercase().contains("sparse"));
    }
}
