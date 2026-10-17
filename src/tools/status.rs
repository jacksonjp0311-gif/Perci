//! `perci.status`: health, versions, weight and policy identity.

use super::{Ctx, Tool};
use crate::verdict::{Confidence, Evidence, Outcome, Status as VStatus};
use serde_json::{json, Value};

pub struct Status;

impl Tool for Status {
    fn name(&self) -> &'static str {
        "perci.status"
    }
    fn title(&self) -> &'static str {
        "Perci health"
    }
    fn description(&self) -> &'static str {
        "Report Perci's version, home and data folders, weight files, policy hash, enabled tools and problems (same data as `perci doctor --json`). deep=true also SHA-256s every weight file (~1 s)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {"deep": {"type": "boolean", "default": false}},
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let deep = args.get("deep").and_then(Value::as_bool).unwrap_or(false);
        let report = crate::doctor::run(deep);
        let mut doctor = crate::doctor::to_json(&report);
        let policy = super::policy::loaded(ctx);
        let tools: Vec<String> = super::enabled_from_env()
            .iter()
            .map(|t| t.name().to_owned())
            .collect();
        if let Some(obj) = doctor.as_object_mut() {
            obj.insert("tools".into(), json!(tools));
            obj.insert("verdict_schema".into(), json!(crate::verdict::SCHEMA));
            obj.insert(
                "policy".into(),
                json!({"source": policy.source, "sha256": policy.sha256, "valid": policy.policy.is_ok()}),
            );
            let (backend, why) = ctx.memory_backend();
            obj.insert(
                "recall".into(),
                json!({"schema": crate::cited_memory::SCHEMA, "backend": backend, "unavailable": why,
                       "journal": crate::cited_memory::default_journal().display().to_string()}),
            );
            obj.insert("eval_schema".into(), json!(crate::b2e::SCHEMA));
            let profile_dir = crate::profile::dir();
            let profile_cfg = crate::profile::config_checked();
            obj.insert(
                "profile".into(),
                json!({
                    "dir": profile_dir.display().to_string(),
                    "from": if std::env::var_os("PERCI_PROFILE").is_some_and(|v| !v.is_empty()) { "PERCI_PROFILE" } else { "PERCI_DATA" },
                    "name": profile_cfg.as_ref().ok().and_then(|c| c.get("name")).cloned(),
                    "config_error": profile_cfg.as_ref().err(),
                    "inside_git_checkout": crate::profile::inside_git_checkout().map(|p| p.display().to_string()),
                }),
            );
            let packs: Vec<Value> = crate::weight_packs::discover()
                .iter()
                .map(|r| match r {
                    Ok(i) => crate::weight_packs::list_row(i),
                    Err(e) => json!({"error": e}),
                })
                .collect();
            obj.insert(
                "weight_packs".into(),
                json!({"default": crate::weight_packs::default_name(), "packs": packs}),
            );
            let (entries, hits, misses) = ctx.cache_stats();
            obj.insert(
                "ledger".into(),
                json!({"run_id": ctx.run_id(), "dir": super::ledger::ledger_dir().display().to_string(),
                       "cache": {"entries": entries, "hits": hits, "misses": misses}}),
            );
            obj.insert(
                "summary".into(),
                json!(if report.ok() {
                    "healthy".to_owned()
                } else {
                    format!("{} problem(s)", report.problems.len())
                }),
            );
        }
        Outcome::new(
            if report.ok() {
                VStatus::Pass
            } else {
                VStatus::Fail
            },
            doctor,
        )
        .confidence(Confidence::Exact)
        .evidence(
            Evidence::new("policy_rule", format!("policy:{}", policy.source))
                .hashed(policy.sha256.clone()),
        )
        .nondeterministic()
    }
}
