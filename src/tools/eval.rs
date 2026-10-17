//! `perci.eval`: bug2eval regression proof through [`crate::b2e`].
//!
//! * `validate` (default): the case's verifier must fail on the broken
//!   snapshot and pass on the fixed one ([`b2e::validate_case`]).
//! * `validate_all`: every case in `PERCI_HOME/evals/b2e` ([`b2e::validate_all`]).
//! * `run`: an agent command repairs the broken snapshot and the same
//!   verifier scores it ([`b2e::run_case`]). It executes that command, so it
//!   needs the operator's opt-in `PERCI_EVAL_ALLOW_AGENT=1`; the calling agent
//!   cannot grant it to itself.
//!
//! Cases must live in the allowlisted folders (`PERCI_HOME/evals/b2e`,
//! `PERCI_B2E_CASES`); verifiers build and run code. Off by default.

use super::{opt_str, try_arg, with_summary, Ctx, Tool};
use crate::b2e::{self, EvalOptions, EvalVerdict};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

pub struct Eval;

const MAX_TIMEOUT_S: u64 = 1800;

/// A path, or a case id such as `B2E-PERCI-001` in the default case folder.
pub fn resolve_case(arg: &str) -> PathBuf {
    let p = PathBuf::from(arg);
    if p.exists() || arg.contains('/') || arg.contains('\\') || arg.ends_with(".b2e") {
        return p;
    }
    b2e::default_case_dir().join(format!("{arg}.b2e"))
}

fn status_of(v: &EvalVerdict) -> Status {
    match v.status.as_str() {
        "pass" => Status::Pass,
        "fail" => Status::Fail,
        _ => Status::Error,
    }
}

/// The verdict JSON without wall-clock fields, so equal runs hash equally.
fn stable(v: &EvalVerdict) -> Value {
    let mut doc = serde_json::to_value(v).unwrap_or(Value::Null);
    if let Some(r) = doc.get_mut("receipt").and_then(Value::as_object_mut) {
        r.remove("elapsed_ms");
    }
    doc
}

fn evidence(v: &EvalVerdict) -> Vec<Evidence> {
    let mut out = Vec::new();
    if !v.receipt.case_sha256.is_empty() {
        out.push(
            Evidence::new("b2e_case", v.case_path.clone()).hashed(v.receipt.case_sha256.clone()),
        );
    }
    if !v.receipt.output_sha256.is_empty() {
        out.push(
            Evidence::new(
                "b2e_output",
                format!("bug2eval {}", v.receipt.bug2eval_version),
            )
            .hashed(v.receipt.output_sha256.clone()),
        );
    }
    out
}

fn reason_code(v: &EvalVerdict) -> &'static str {
    match (v.mode.as_str(), v.status.as_str()) {
        (_, "invalid") => "CASE_INVALID",
        (_, "error") => "EVAL_ERROR",
        ("run", _) => "FIX_REJECTED",
        _ => "CASE_NOT_PROVEN",
    }
}

impl Tool for Eval {
    fn name(&self) -> &'static str {
        "perci.eval"
    }
    fn title(&self) -> &'static str {
        "Run a bug2eval case"
    }
    fn description(&self) -> &'static str {
        "Regression proof with bug2eval. validate: the case's verifier fails on the broken snapshot and passes on the fixed one. validate_all: every packed case. run: an agent command repairs the broken snapshot and the verifier scores it (needs the operator's PERCI_EVAL_ALLOW_AGENT=1). Slow (builds Perci per snapshot). Off by default."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "case": {"type": "string", "description": "Case id (B2E-PERCI-001) or path to a .b2e inside the allowed case folders. Not used by validate_all."},
                "mode": {"enum": ["validate", "validate_all", "run"], "default": "validate"},
                "agent_cmd": {"type": "string", "description": "run only: command with {workspace}, {task_file}, {prompt_file} placeholders."},
                "timeout_s": {"type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_S, "default": MAX_TIMEOUT_S}
            },
            "additionalProperties": false
        })
    }
    fn default_enabled(&self) -> bool {
        false
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let mode = try_arg!(opt_str(args, "mode")).unwrap_or("validate");
        let case = try_arg!(opt_str(args, "case"));
        let timeout = args
            .get("timeout_s")
            .and_then(Value::as_u64)
            .unwrap_or(MAX_TIMEOUT_S)
            .clamp(1, MAX_TIMEOUT_S);
        let mut opts = EvalOptions {
            timeout: Duration::from_secs(timeout),
            ..EvalOptions::default()
        };
        let outcome = match mode {
            "validate_all" => {
                let verdicts = match b2e::validate_all(&b2e::default_case_dir(), &opts) {
                    Ok(v) => v,
                    Err(e) => return Outcome::error("EVAL_ERROR", e.to_string()),
                };
                if verdicts.is_empty() {
                    return Outcome::abstain("NO_CASES", "no .b2e cases in the case folder");
                }
                let status = verdicts
                    .iter()
                    .map(status_of)
                    .fold(Status::Pass, Status::worst);
                let passed = verdicts.iter().filter(|v| v.passed()).count();
                let mut o = Outcome::new(
                    status,
                    with_summary(
                        json!({"schema": b2e::SCHEMA, "mode": mode, "cases": verdicts.iter().map(stable).collect::<Vec<_>>()}),
                        format!("{passed}/{} case(s) proven", verdicts.len()),
                    ),
                );
                for v in verdicts.iter().filter(|v| !v.passed()) {
                    o = o.reason(Reason::new(
                        reason_code(v),
                        format!("{}: {}", v.case_id, v.reasons.join("; ")),
                    ));
                }
                for v in &verdicts {
                    for e in evidence(v) {
                        o = o.evidence(e);
                    }
                }
                o
            }
            "validate" | "run" => {
                let Some(case) = case else {
                    return Outcome::error("BAD_ARGS", "case is required");
                };
                let path = resolve_case(case);
                let v = if mode == "run" {
                    let Some(cmd) = try_arg!(opt_str(args, "agent_cmd")) else {
                        return Outcome::error("BAD_ARGS", "run needs agent_cmd");
                    };
                    if std::env::var("PERCI_EVAL_ALLOW_AGENT").ok().as_deref() != Some("1") {
                        return Outcome::fail(json!({"status": "refused"})).reason(Reason::new(
                            "AGENT_RUN_NOT_ALLOWED",
                            "run executes agent_cmd; the operator must start Perci with PERCI_EVAL_ALLOW_AGENT=1",
                        ));
                    }
                    opts.allow_agent_run = true;
                    b2e::run_case(&path, cmd, &opts)
                } else {
                    b2e::validate_case(&path, &opts)
                };
                let status = status_of(&v);
                let summary = match (mode, status) {
                    ("validate", Status::Pass) => {
                        format!("{}: bug reproduces before, gone after", v.case_id)
                    }
                    ("run", Status::Pass) => {
                        format!("{}: the agent's fix passes the verifier", v.case_id)
                    }
                    _ => format!("{}: {}", v.case_id, v.status),
                };
                let mut o = Outcome::new(status, with_summary(stable(&v), summary));
                if status != Status::Pass {
                    o = o.reason(Reason::new(reason_code(&v), v.reasons.join("; ")));
                }
                for e in evidence(&v) {
                    o = o.evidence(e);
                }
                o
            }
            other => return Outcome::error("BAD_ARGS", format!("unknown mode {other}")),
        };
        // Verifier output (timings, temp paths) can differ between runs.
        outcome.confidence(Confidence::Bounded).nondeterministic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_cases_outside_the_allowlist_and_unopted_runs() {
        let ctx = Ctx::new();
        let v = Eval.call(&ctx, &json!({"case": "/etc/passwd"}));
        assert_eq!(v.status, Status::Error);
        assert_eq!(v.reasons[0].code, "EVAL_ERROR");
        let v = Eval.call(
            &ctx,
            &json!({"case": "B2E-PERCI-001", "mode": "run", "agent_cmd": "true"}),
        );
        if std::env::var("PERCI_EVAL_ALLOW_AGENT").ok().as_deref() != Some("1") {
            assert_eq!(v.status, Status::Fail);
            assert_eq!(v.reasons[0].code, "AGENT_RUN_NOT_ALLOWED");
        }
        assert_eq!(
            resolve_case("B2E-PERCI-001"),
            b2e::default_case_dir().join("B2E-PERCI-001.b2e")
        );
    }
}
