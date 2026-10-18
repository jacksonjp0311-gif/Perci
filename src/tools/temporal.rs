//! The Temporal Router (`perci.next_step`, alias `temporal_route`): Perci learns the step sequences of
//! the system it serves from its own ledgers and tells the main model what
//! usually comes next, which sequence rules the next step would trip, and
//! which planned calls it can answer from cache.
//!
//! Deterministic and integer-only: an order-3 n-gram over ledger tokens
//! (`tool[:phase][:step]`) with back-off to shorter contexts. Every
//! suggestion carries its evidence counts (how often this continuation
//! followed this context, out of how many, and how those steps ended).
//!
//! The model is rebuilt from every ledger that verifies by
//! `perci ledger learn` (→ `PERCI_PROFILE/temporal/model.json`); runs whose
//! chain is broken are skipped and reported. The current run's own entries
//! are added live, so suggestions improve within a session.

use super::ledger::{self, Entry};
use super::{opt_str, try_arg, with_summary, Ctx, Tool};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const SCHEMA: &str = "perci.temporal.v1";
/// Context length (tokens) of the longest n-gram.
pub const MAX_CONTEXT: usize = 2;
const SEP: &str = " | ";
const START: &str = "<start>";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Outcomes {
    pub pass: u64,
    pub fail: u64,
    pub abstain: u64,
    pub error: u64,
}

impl Outcomes {
    fn add(&mut self, status: &str) {
        match status {
            "pass" => self.pass += 1,
            "fail" => self.fail += 1,
            "abstain" => self.abstain += 1,
            _ => self.error += 1,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Model {
    pub schema: String,
    /// context (tokens joined by " | ", "" for none) → next token → count.
    pub counts: BTreeMap<String, BTreeMap<String, u64>>,
    /// How steps of each token ended.
    pub outcomes: BTreeMap<String, Outcomes>,
    /// run id → (entries learned, head hash).
    pub runs: BTreeMap<String, (u64, String)>,
    pub skipped: Vec<String>,
    pub transitions: u64,
    /// Last tokens seen by [`Model::push`] (live models only).
    #[serde(skip)]
    tail: Vec<String>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            schema: SCHEMA.into(),
            ..Self::default()
        }
    }

    /// Add one run's tokens (with `<start>` padding so first steps are learned).
    pub fn add_sequence(&mut self, tokens: &[(String, String)]) {
        let mut hist: Vec<String> = vec![START.into(); MAX_CONTEXT];
        for (tok, status) in tokens {
            for k in 0..=MAX_CONTEXT {
                let ctx = hist[hist.len() - k..].join(SEP);
                *self
                    .counts
                    .entry(ctx)
                    .or_default()
                    .entry(tok.clone())
                    .or_default() += 1;
            }
            self.outcomes.entry(tok.clone()).or_default().add(status);
            self.transitions += 1;
            hist.push(tok.clone());
        }
    }

    /// Append one step to a live model (keeps its own context tail).
    pub fn push(&mut self, tok: String, status: &str) {
        if self.tail.is_empty() {
            self.tail = vec![START.into(); MAX_CONTEXT];
        }
        for k in 0..=MAX_CONTEXT {
            let ctx = self.tail[self.tail.len() - k..].join(SEP);
            *self
                .counts
                .entry(ctx)
                .or_default()
                .entry(tok.clone())
                .or_default() += 1;
        }
        self.outcomes.entry(tok.clone()).or_default().add(status);
        self.transitions += 1;
        self.tail.push(tok);
        if self.tail.len() > MAX_CONTEXT {
            self.tail.remove(0);
        }
    }

    pub fn add_entries(&mut self, entries: &[Entry]) {
        let toks: Vec<(String, String)> = entries
            .iter()
            .map(|e| (e.token(), e.status.clone()))
            .collect();
        self.add_sequence(&toks);
    }

    /// Longest context with observations: `(context tokens, continuations)`.
    pub fn predict(&self, history: &[String]) -> (Vec<String>, Vec<(String, u64)>, u64) {
        let mut padded: Vec<String> = vec![START.into(); MAX_CONTEXT];
        padded.extend(history.iter().cloned());
        for k in (0..=MAX_CONTEXT).rev() {
            let ctx_toks = padded[padded.len() - k..].to_vec();
            if let Some(next) = self.counts.get(&ctx_toks.join(SEP)) {
                let total: u64 = next.values().sum();
                if total == 0 {
                    continue;
                }
                let mut v: Vec<(String, u64)> = next.iter().map(|(t, c)| (t.clone(), *c)).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                return (ctx_toks, v, total);
            }
        }
        (Vec::new(), Vec::new(), 0)
    }
}

/// [`Model::predict`] over the sum of several models' counts.
pub fn predict_merged(
    models: &[&Model],
    history: &[String],
) -> (Vec<String>, Vec<(String, u64)>, u64) {
    let mut padded: Vec<String> = vec![START.into(); MAX_CONTEXT];
    padded.extend(history.iter().cloned());
    for k in (0..=MAX_CONTEXT).rev() {
        let ctx_toks = padded[padded.len() - k..].to_vec();
        let key = ctx_toks.join(SEP);
        let mut merged: BTreeMap<&str, u64> = BTreeMap::new();
        for m in models {
            if let Some(next) = m.counts.get(&key) {
                for (t, c) in next {
                    *merged.entry(t.as_str()).or_default() += c;
                }
            }
        }
        let total: u64 = merged.values().sum();
        if total == 0 {
            continue;
        }
        let mut v: Vec<(String, u64)> =
            merged.into_iter().map(|(t, c)| (t.to_owned(), c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        return (ctx_toks, v, total);
    }
    (Vec::new(), Vec::new(), 0)
}

pub fn model_path() -> PathBuf {
    crate::paths::profile_file("temporal/model.json")
}

/// Rebuild the model from every verified ledger.
pub fn learn() -> Model {
    let mut m = Model::new();
    for run in ledger::list_runs() {
        match ledger::verify_run(&run) {
            Ok(v) if v.ok => {
                m.add_entries(&v.parsed);
                m.runs.insert(run, (v.entries as u64, v.head));
            }
            Ok(v) => m.skipped.push(format!(
                "{run}: chain broken ({})",
                v.problems.first().map(|p| p.code).unwrap_or("?")
            )),
            Err(e) => m.skipped.push(format!("{run}: {e}")),
        }
    }
    m
}

pub fn learn_and_save() -> Result<(PathBuf, Model), String> {
    let m = learn();
    let path = model_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string(&m).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok((path, m))
}

/// The saved model (or an empty one), loaded once per process.
pub fn load_saved() -> (Model, String) {
    match std::fs::read(model_path()) {
        Ok(b) => match serde_json::from_slice::<Model>(&b) {
            Ok(m) if m.schema == SCHEMA => (m, crate::doctor::sha256_hex(&b)),
            _ => (Model::new(), "invalid".into()),
        },
        Err(_) => (Model::new(), "none".into()),
    }
}

/// Tools whose verdicts are cached (same input → same verdict).
fn cache_eligible(tool: &str) -> bool {
    matches!(
        tool,
        "perci.exact"
            | "perci.check_math"
            | "perci.relation"
            | "perci.route"
            | "perci.policy_check"
    )
}

pub struct NextStep;

impl Tool for NextStep {
    fn name(&self) -> &'static str {
        "perci.next_step"
    }
    fn title(&self) -> &'static str {
        "Temporal Router: predict the next step"
    }
    fn description(&self) -> &'static str {
        "Perci's Temporal Router (alias temporal_route). Learns from this system's own hash-chained step ledgers (only chains that verify): given the current run (or an explicit history), returns the steps that usually come next with their counts, warns when a likely or planned step would break a sequence rule (e.g. deploy before tests passed), and flags planned calls Perci can answer from cache or that already failed with the same input. Abstains (NO_HISTORY) when it has not seen this context. Advisory, deterministic, < 1 ms."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "run_id": {"type": "string", "description": "Default: the current run."},
                "history": {"type": "array", "items": {"type": "string"}, "description": "Explicit step tokens instead of the run ledger (stateless use)."},
                "planned": {"type": "object", "description": "What you are about to do: {\"action\": {...}} for a harness step, or {\"tool\": \"exact\", \"args\": {...}} for a Perci call.", "properties": {
                    "action": {"type": "object"}, "tool": {"type": "string"}, "args": {"type": "object"}
                }},
                "limit": {"type": "integer", "minimum": 1, "maximum": 20, "default": 5}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(5)
            .clamp(1, 20) as usize;
        let explicit: Option<Vec<String>> =
            args.get("history").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            });
        let run = match try_arg!(opt_str(args, "run_id")) {
            Some(r) if ledger::valid_run_id(r) => Some(r.to_owned()),
            Some(r) => return Outcome::error("BAD_ARGS", format!("invalid run_id {r:?}")),
            None => ctx.run_id(),
        };
        // Resolve the run before taking the ledger lock (answer must not
        // call back into ctx.run_id()).
        let this_run = ctx.run_id();
        let current = run.is_none() || run == this_run;
        let run = run.or(this_run);
        let out = ctx.with_ledger_entries(run.as_deref(), |entries| {
            self.answer(
                ctx,
                args,
                limit,
                explicit.as_deref(),
                run.as_deref(),
                current,
                entries,
            )
        });
        match out {
            Some(o) => o,
            None => self.answer(
                ctx,
                args,
                limit,
                explicit.as_deref(),
                run.as_deref(),
                false,
                &[],
            ),
        }
    }
}

impl NextStep {
    #[allow(clippy::too_many_arguments)]
    fn answer(
        &self,
        ctx: &Ctx,
        args: &Value,
        limit: usize,
        explicit: Option<&[String]>,
        run: Option<&str>,
        current: bool,
        entries: &[Entry],
    ) -> Outcome {
        let history: Vec<String> = match explicit {
            Some(h) => h.to_vec(),
            None => entries
                .iter()
                .rev()
                .take(MAX_CONTEXT + 1)
                .rev()
                .map(Entry::token)
                .collect(),
        };
        let run = run.map(str::to_owned);
        let (prediction, model_info) = ctx.temporal(|saved, sha| {
            // Live: the current run's transitions unless the saved model
            // already learned this run; another run is rebuilt from its file.
            let skip_live = run.as_ref().is_some_and(|r| saved.runs.contains_key(r));
            let rebuilt;
            let compute = |live: &Model| {
                let models = [saved, live];
                let (context, next, total) = predict_merged(&models, &history);
                let rows: Vec<Value> = next
                    .iter()
                    .take(limit)
                    .map(|(tok, c)| {
                        let mut o = Outcomes::default();
                        for m in &models {
                            if let Some(x) = m.outcomes.get(tok) {
                                o.pass += x.pass;
                                o.fail += x.fail;
                                o.abstain += x.abstain;
                                o.error += x.error;
                            }
                        }
                        json!({"step": tok, "count": c, "of": total, "permille": c * 1000 / total.max(1),
                               "outcomes": {"pass": o.pass, "fail": o.fail, "abstain": o.abstain, "error": o.error}})
                    })
                    .collect();
                (
                    (context, rows, total),
                    json!({"model_sha256": sha, "runs_learned": saved.runs.len(),
                           "transitions": saved.transitions + live.transitions,
                           "live_run_added": !skip_live && live.transitions > 0}),
                )
            };
            let empty = Model::new();
            if skip_live {
                compute(&empty)
            } else if current {
                ctx.live_model(compute)
            } else {
                let mut m = Model::new();
                m.add_entries(entries);
                rebuilt = m;
                compute(&rebuilt)
            }
        });
        let (context, rows, total) = prediction;

        let mut o = Outcome::new(
            if rows.is_empty() {
                Status::Abstain
            } else {
                Status::Pass
            },
            Value::Null,
        )
        .confidence(Confidence::Heuristic);
        let mut warnings = Vec::new();
        let mut hints = Vec::new();

        // Sequence rules: likely next steps and the planned one.
        let policy = super::policy::loaded(ctx).policy.as_ref().ok();
        if let Some(policy) = &policy {
            for row in &rows {
                let tok = row["step"].as_str().unwrap_or("");
                let step = step_of_token(tok);
                for f in ledger::check_sequence(policy, &step, entries) {
                    if !f.satisfied {
                        warnings.push(json!({"code": "SEQUENCE_WARNING", "step": tok, "rule": f.rule, "detail": f.detail}));
                    }
                }
            }
        }
        if let Some(planned) = args.get("planned").filter(|p| p.is_object()) {
            if let Some(action) = planned.get("action").filter(|a| a.is_object()) {
                let step = ledger::action_step(action);
                if let Some(policy) = &policy {
                    for f in ledger::check_sequence(policy, &step, entries) {
                        if f.satisfied {
                            for e in &f.evidence {
                                o = o.evidence(e.clone());
                            }
                        } else {
                            o.status = o.status.worst(f.posture.status());
                            o = o.reason(Reason::new(
                                "SEQUENCE_REQUIRES",
                                format!("planned {step}: {}", f.detail),
                            ));
                            for e in &f.evidence {
                                o = o.evidence(e.clone());
                            }
                        }
                    }
                }
            }
            if let Some(tool_name) = planned.get("tool").and_then(Value::as_str) {
                let tools = super::catalog();
                if let Some(t) = super::resolve(&tools, tool_name) {
                    let pargs = planned.get("args").cloned().unwrap_or(json!({}));
                    let ih = crate::verdict::input_hash(t.name(), &pargs);
                    if let Some(prev) = entries.iter().rev().find(|e| e.input_sha256 == ih) {
                        if prev.status == "fail" || prev.status == "error" {
                            hints.push(json!({"code": "REPEAT_FAILURE", "detail": format!("the same input {} at {}; change something before retrying", prev.status, prev.reference())}));
                            o = o.evidence(prev.evidence());
                        } else if cache_eligible(t.name()) {
                            hints.push(json!({"code": "CACHED", "detail": format!("identical input already answered at {} ({}); Perci will return the cached verdict", prev.reference(), prev.status)}));
                            o = o.evidence(prev.evidence());
                        } else {
                            hints.push(json!({"code": "ALREADY_RAN", "detail": format!("identical input at {} ({}); result may differ now (not cacheable)", prev.reference(), prev.status)}));
                        }
                    } else if cache_eligible(t.name()) && ctx.cache_contains(&ih) {
                        hints.push(json!({"code": "CACHED", "detail": "identical input is in this process's verdict cache"}));
                    }
                }
            }
        }
        // Loops: the same step three times running.
        if history.len() >= 3 {
            let n = history.len();
            if history[n - 1] == history[n - 2] && history[n - 2] == history[n - 3] {
                hints.push(json!({"code": "LOOP", "detail": format!("{} three times in a row", history[n - 1])}));
            }
        }
        if rows.is_empty() {
            o = o.reason(Reason::new("NO_HISTORY", "no learned transitions for this context yet (run perci ledger learn after a few runs)"));
        }
        for w in &warnings {
            o = o.reason(Reason::new(
                "SEQUENCE_WARNING",
                format!(
                    "{}: {}",
                    w["step"].as_str().unwrap_or(""),
                    w["detail"].as_str().unwrap_or("")
                ),
            ));
        }
        let top = rows
            .first()
            .and_then(|r| r["step"].as_str())
            .unwrap_or("")
            .to_owned();
        o.result = with_summary(
            json!({
                "run_id": run,
                "history_tail": history.iter().rev().take(MAX_CONTEXT).rev().collect::<Vec<_>>(),
                "context": context,
                "order": context.len() + 1,
                "observations": total,
                "next": rows,
                "warnings": warnings,
                "hints": hints,
                "model": model_info,
            }),
            if top.is_empty() {
                "no suggestion yet".to_owned()
            } else {
                format!("likely next: {top}")
            },
        );
        o.evidence(Evidence::new(
            "temporal_model",
            model_path().display().to_string(),
        ))
        .nondeterministic()
    }
}

/// `agent_gate_step:pre:shell:git push` → `shell:git push`;
/// `ledger_append:tests` → `tests`; `agent_verify_patch:patch` → `patch`.
pub fn step_of_token(tok: &str) -> String {
    let mut parts = tok.splitn(2, ':');
    let _tool = parts.next();
    let rest = parts.next().unwrap_or("");
    for phase in ["pre:", "post:", "check:"] {
        if let Some(r) = rest.strip_prefix(phase) {
            return r.to_owned();
        }
    }
    rest.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ngram_backoff_and_counts() {
        let mut m = Model::new();
        let seq = |s: &[&str]| {
            s.iter()
                .map(|t| (t.to_string(), "pass".to_string()))
                .collect::<Vec<_>>()
        };
        for _ in 0..3 {
            m.add_sequence(&seq(&["write", "test", "push"]));
        }
        m.add_sequence(&seq(&["write", "test", "write", "test", "push"]));
        let (ctx, next, total) = m.predict(&["write".into(), "test".into()]);
        assert_eq!(ctx, vec!["write".to_string(), "test".to_string()]);
        assert_eq!(next[0], ("push".to_string(), 4));
        assert_eq!(total, 5);
        // Unseen context backs off to the unigram distribution.
        let (ctx, next, _) = m.predict(&["deploy".into(), "zzz".into()]);
        assert!(ctx.is_empty());
        assert!(!next.is_empty());
        // First step of a run.
        let (_, next, _) = m.predict(&[]);
        assert_eq!(next[0].0, "write");
        assert_eq!(
            step_of_token("agent_gate_step:pre:shell:git push"),
            "shell:git push"
        );
        assert_eq!(step_of_token("ledger_append:tests"), "tests");
    }
}
