//! `perci.relation`: closed-world "what follows if…" over supplied premises.
//!
//! Wraps `relation_field` (signed relation graph, intervention, propagation).
//! It reasons only over the premises given; it never adds world knowledge.
//! Optionally checks a caller's claim about which nodes are affected.

use super::{opt_str, opt_str_list, try_arg, with_summary, Ctx, Tool};
use crate::relation_field::{self, RelationProgram};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};

pub struct Relation;

impl Tool for Relation {
    fn name(&self) -> &'static str {
        "perci.relation"
    }
    fn title(&self) -> &'static str {
        "Closed-world consequences"
    }
    fn description(&self) -> &'static str {
        "Given premises like \"Deploy depends on build. Build depends on tests.\" and an intervention (\"tests fail\"), return which supplied relations are impacted and which stay invariant, using only the premises. Pass claimed_impacted / claimed_invariant to check an answer: fail if it says something breaks that doesn't follow, or misses something that does."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "premises": {"type": "array", "items": {"type": "string"}, "description": "One relation per item, e.g. \"Deploy depends on build\"."},
                "intervention": {"type": "object", "properties": {
                    "target": {"type": "string"}, "change": {"type": "string", "description": "e.g. \"fails\" (default)"}
                }, "required": ["target"]},
                "text": {"type": "string", "description": "Alternative to premises+intervention: one prose program ending in a question like \"If tests fail, what changes?\""},
                "claimed_impacted": {"type": "array", "items": {"type": "string"}, "description": "Nodes the caller says are affected."},
                "claimed_invariant": {"type": "array", "items": {"type": "string"}, "description": "Nodes the caller says are unaffected."}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let premises = try_arg!(opt_str_list(args, "premises"));
        let text = try_arg!(opt_str(args, "text"));
        let claimed_impacted = try_arg!(opt_str_list(args, "claimed_impacted"));
        let claimed_invariant = try_arg!(opt_str_list(args, "claimed_invariant"));
        let intervention = args.get("intervention");
        if text.is_none() && !premises.is_empty() {
            let Some(target) = intervention
                .and_then(|i| i.get("target"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return Outcome::error("BAD_ARGS", "premises need intervention.target");
            };
            let change = intervention
                .and_then(|i| i.get("change"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("fails");
            let text = build_text(&premises, target, change);
            let mut program = relation_field::parse_program(&text);
            // The caller named the intervention; don't make the parser guess it.
            program.intervention = Some(crate::relation_field::Intervention {
                target: target.to_lowercase(),
                change: change.to_owned(),
            });
            program.asks_counterfactual = true;
            program.asks_invariant = true;
            return evaluate_program(program, &text, &claimed_impacted, &claimed_invariant);
        }
        match text {
            Some(t) => evaluate(t, &claimed_impacted, &claimed_invariant),
            None => Outcome::error("BAD_ARGS", "give premises+intervention or text"),
        }
    }
}

/// Turn structured premises into the program text the field parses.
pub fn build_text(premises: &[String], target: &str, change: &str) -> String {
    let mut s = String::new();
    for p in premises {
        let p = p.trim().trim_end_matches(['.', ';']);
        if !p.is_empty() {
            s.push_str(p);
            s.push_str(". ");
        }
    }
    s.push_str(&format!(
        "If {target} {change}, what changes and what remains invariant?"
    ));
    s
}

pub fn evaluate(text: &str, claimed_impacted: &[String], claimed_invariant: &[String]) -> Outcome {
    evaluate_program(
        relation_field::parse_program(text),
        text,
        claimed_impacted,
        claimed_invariant,
    )
}

pub fn evaluate_program(
    program: RelationProgram,
    text: &str,
    claimed_impacted: &[String],
    claimed_invariant: &[String],
) -> Outcome {
    let edges: Vec<Value> = program
        .edges
        .iter()
        .map(|e| {
            json!({
                "subject": e.subject,
                "relation": e.relation.phrase(),
                "object": e.object,
                "negated": e.negated,
                "proposition": e.proposition(),
            })
        })
        .collect();
    let evidence = Evidence::new("computation", "perci.relation/closed-world")
        .hashed(crate::doctor::sha256_hex(text.as_bytes()));
    if program.edges.is_empty() {
        return Outcome::abstain(
            "NO_RELATIONS",
            "no relation like \"A depends on B\" was found in the premises",
        )
        .confidence(Confidence::Bounded)
        .evidence(evidence);
    }
    let Some(intervention) = program.intervention.clone() else {
        return Outcome::abstain(
            "NO_INTERVENTION",
            "no intervention like \"if B fails\" was found",
        )
        .with_result(json!({"edges": edges}))
        .confidence(Confidence::Bounded)
        .evidence(evidence);
    };
    let affected = relation_field::affected_nodes(&program);
    let Some(exec) = relation_field::execute_parsed(program, text) else {
        return Outcome::abstain("NO_PROGRAM", "premises did not form an executable program")
            .with_result(json!({"edges": edges}))
            .confidence(Confidence::Bounded)
            .evidence(evidence);
    };
    let mut reasons = Vec::new();
    let mut failed = false;
    for c in claimed_impacted {
        if !affected.iter().any(|n| relation_field::same_node(n, c)) {
            failed = true;
            reasons.push(Reason::new(
                "DOES_NOT_FOLLOW",
                format!("{c:?} is claimed to be affected, but no supplied relation carries the change to it"),
            ));
        }
    }
    for c in claimed_invariant {
        if affected.iter().any(|n| relation_field::same_node(n, c)) {
            failed = true;
            reasons.push(Reason::new(
                "MISSED_CONSEQUENCE",
                format!("{c:?} is claimed to be unaffected, but it loses support through the supplied relations"),
            ));
        }
    }
    let checked = !claimed_impacted.is_empty() || !claimed_invariant.is_empty();
    let status = if failed { Status::Fail } else { Status::Pass };
    let summary = if exec.impacted.is_empty() {
        format!(
            "{} {} touches no supplied relation; nothing follows",
            intervention.target, intervention.change
        )
    } else {
        format!(
            "if {} {}: affected {}; invariant {}",
            intervention.target,
            intervention.change,
            affected.join(", "),
            if exec.invariant.is_empty() {
                "none".to_owned()
            } else {
                exec.invariant.join("; ")
            }
        )
    };
    let mut outcome = Outcome::new(
        status,
        with_summary(
            json!({
                "edges": edges,
                "intervention": {"target": intervention.target, "change": intervention.change},
                "affected": affected,
                "impacted": exec.impacted,
                "invariant": exec.invariant,
                "contradictions": exec.contradictions,
                "no_conclusion": exec.impacted.is_empty(),
                "claim_checked": checked,
                "explanation": exec.text,
            }),
            summary,
        ),
    )
    .confidence(Confidence::Bounded)
    .evidence(evidence);
    for r in reasons {
        outcome = outcome.reason(r);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    fn premises(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn deploy_chain_propagates_two_hops() {
        let text = build_text(
            &premises(&[
                "Deploy depends on build",
                "Build depends on tests",
                "Docs depend on nothing",
            ]),
            "tests",
            "fail",
        );
        let o = evaluate(&text, &[], &[]);
        assert_eq!(o.status, Status::Pass, "{:?}", o.reasons);
        let affected: Vec<String> = serde_json::from_value(o.result["affected"].clone()).unwrap();
        assert!(affected.iter().any(|a| a.contains("build")), "{affected:?}");
        assert!(
            affected.iter().any(|a| a.contains("deploy")),
            "{affected:?}"
        );
    }

    #[test]
    fn claims_are_checked() {
        let text = build_text(
            &premises(&["Trust depends on verification", "Memory preserves identity"]),
            "verification",
            "fails",
        );
        let ok = evaluate(&text, &["trust".into()], &["identity".into()]);
        assert_eq!(ok.status, Status::Pass, "{:?}", ok.reasons);
        let wrong = evaluate(&text, &["identity".into()], &[]);
        assert_eq!(wrong.status, Status::Fail);
        assert_eq!(wrong.reasons[0].code, "DOES_NOT_FOLLOW");
        let missed = evaluate(&text, &[], &["trust".into()]);
        assert_eq!(missed.status, Status::Fail);
        assert_eq!(missed.reasons[0].code, "MISSED_CONSEQUENCE");
    }

    #[test]
    fn prose_without_relations_abstains() {
        assert_eq!(
            evaluate("The sky is nice today.", &[], &[]).status,
            Status::Abstain
        );
    }
}
