//! `perci.route`: which capability should handle a request.
//!
//! The route is decided by deterministic checks, strongest first:
//! policy refusal → exact math Perci can compute → closed-world relation
//! program → memory/recall request → otherwise `llm`. The Bitwork classifier
//! label of the selected weight pack (any PERCIW03 pack; its own label set,
//! reported in `bitwork.labels`) is returned alongside; it never overrides
//! the checks.
//!
//! Confidence: the score margin alone is a weak signal (wrong labels with
//! large margins happen), so a pack's label confidence is the Wilson lower
//! bound of its measured accuracy in that margin bucket
//! (`perci packs calibrate`). An `llm` route whose label is below
//! `min_confidence_permille` abstains (`ROUTE_LOW_CONFIDENCE`). An
//! uncalibrated pack reports `decision: "uncalibrated"` and only gates on
//! an explicit `min_margin`.
//!
//! Default pack for tools is the 20 MB v0.2 Bitwork pack. Override with
//! `PERCI_TOOL_PACK`, `tool_pack` in the profile, or `args.pack` (any pack
//! name from `perci packs list`; never a raw path).

use super::exact::{self, Computed};
use super::{arg_str, opt_str, try_arg, with_summary, Ctx, Tool};
use crate::cognitive::CognitiveWeights;
use crate::verdict::{Confidence, Evidence, Outcome};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

pub struct Route;

pub struct LoadedPack {
    pub name: String,
    pub path: PathBuf,
    pub weights: CognitiveWeights,
    /// `name@version manifest-sha256:<hex>` of the verified pack.
    pub identity: String,
    /// e.g. "unpinned" notes from verification.
    pub warnings: Vec<String>,
    /// The pack's own labels, in pack order.
    pub labels: Vec<String>,
    /// `perci.route-calibration.v1`, when the pack has one.
    pub calibration: Option<Value>,
}

/// Default gate for a calibrated label (profile `route.min_confidence_permille`).
pub const DEFAULT_MIN_CONFIDENCE_PERMILLE: u64 = 700;

/// Load (once per process) and return the requested routing pack. The pack
/// is verified against its manifest first; a missing or modified pack is an
/// error, never a fallback (see `crate::weight_packs`).
pub fn pack(ctx: &Ctx, requested: Option<&str>) -> Result<Arc<LoadedPack>, String> {
    let name = requested
        .map(str::to_owned)
        .unwrap_or_else(crate::weight_packs::default_name);
    let mut packs = ctx.packs.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = packs.get(&name) {
        return Ok(Arc::clone(p));
    }
    let loaded = crate::weight_packs::load(&name)?;
    let path = loaded.info.entry_path();
    let identity = loaded.info.identity();
    let labels = loaded.weights.label_names();
    let calibration = loaded.info.manifest.calibration.clone();
    // Touch the pages once so the first real call is warm (P1 prewarm).
    let _ = loaded.weights.classify("prewarm perci routing pack");
    let loaded = Arc::new(LoadedPack {
        name: name.clone(),
        path,
        weights: loaded.weights,
        identity,
        warnings: loaded.warnings,
        labels,
        calibration,
    });
    packs.insert(name, Arc::clone(&loaded));
    Ok(loaded)
}

const RECALL_CUES: &[&str] = &[
    "remember when",
    "what did i say",
    "what did i tell",
    "did i tell you",
    "do you remember",
    "recall ",
    "my notes",
    "my note ",
    "where is my",
    "where did i put",
    "according to my",
    "what was my",
    "you told me",
];

impl Tool for Route {
    fn name(&self) -> &'static str {
        "perci.route"
    }
    fn cacheable(&self) -> bool {
        true
    }
    fn title(&self) -> &'static str {
        "Route a request"
    }
    fn description(&self) -> &'static str {
        "Decide deterministically which capability should handle a request: exact (Perci computes it), relation (closed-world premises), recall (user memory), refuse (policy), or llm (needs a language model). Also returns the category label from the selected Bitwork weight pack (any PERCIW03 pack, with its own label set in bitwork.labels) and a calibrated confidence; an llm route whose label is below min_confidence_permille abstains. Same input, same answer."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string"},
                "pack": {"type": "string", "description": "Weight pack name from `perci packs list` (default v0.2 or the profile's tool_pack). Verified at load; a missing or modified pack is an error."},
                "bitwork": {"type": "boolean", "default": true, "description": "false: deterministic route only, no weight pack needed."},
                "min_confidence_permille": {"type": "integer", "minimum": 0, "maximum": 1000, "description": "Gate for a calibrated label (default: profile route.min_confidence_permille, else 700)."},
                "min_margin": {"type": "integer", "minimum": 0, "description": "Gate for an uncalibrated pack's score margin (default: profile route.min_margin, else no gate)."}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let text = try_arg!(arg_str(args, "text"));
        let pack_name = try_arg!(opt_str(args, "pack"));
        let lower = text.to_lowercase();

        let charter = crate::governed_will::assess(text);
        let computed = exact::compute(text);
        let program = crate::relation_field::parse_program(text);
        let (route, tool, why) =
            if charter.posture == crate::governed_will::ActionPosture::RefuseUnauthorized {
                (
                    "refuse",
                    Value::Null,
                    "charter posture: destructive or safeguard-bypassing request",
                )
            } else if !matches!(computed, Computed::Unparsed | Computed::TooLarge) {
                (
                    "exact",
                    json!("perci.exact"),
                    "Perci can compute this exactly",
                )
            } else if !program.edges.is_empty()
                && program.intervention.is_some()
                && program.asks_counterfactual
            {
                (
                    "relation",
                    json!("perci.relation"),
                    "explicit premises with an intervention",
                )
            } else if RECALL_CUES.iter().any(|c| lower.contains(c)) {
                (
                    "recall",
                    json!("perci.recall"),
                    "asks about the user's own notes or past statements",
                )
            } else {
                (
                    "llm",
                    Value::Null,
                    "open request: needs a language model; Perci can verify the answer afterwards",
                )
            };

        let decision = crate::capability_router::route_prompt(text);
        let mut outcome_weights = std::collections::BTreeMap::new();
        let want_bitwork = args.get("bitwork").and_then(Value::as_bool).unwrap_or(true);
        let bitwork = if !want_bitwork {
            Value::Null
        } else {
            match pack(ctx, pack_name) {
                Ok(p) => {
                    outcome_weights.insert(format!("bitwork:{}", p.name), p.identity.clone());
                    match p.weights.classify(text) {
                        Ok(m) => {
                            let (decision, confidence) = gate(args, &p, m.margin);
                            json!({
                                "pack": p.name,
                                "label": m.label,
                                "labels": p.labels,
                                "score": m.score,
                                "runner_up_score": m.runner_up_score,
                                "margin": m.margin,
                                "concept_id": m.concept_id,
                                "alternatives": m.mixture.iter().map(|x| x.label.clone()).collect::<Vec<_>>(),
                                "lexical_priors": match p.weights.lexical_priors() {
                                    crate::cognitive::LexicalPriors::Builtin => "builtin",
                                    crate::cognitive::LexicalPriors::Off => "off",
                                    crate::cognitive::LexicalPriors::Custom(_) => "custom",
                                },
                                "decision": decision,
                                "confidence": confidence,
                            })
                        }
                        Err(e) => return Outcome::error("PACK_FAILED", format!("{}: {e}", p.name)),
                    }
                }
                // A missing or modified pack is an error, never a silent fallback.
                Err(e) => return Outcome::error("PACK_UNAVAILABLE", e),
            }
        };
        let low = route == "llm" && bitwork["decision"].as_str() == Some("abstain");
        let mut outcome = Outcome::pass(with_summary(
            json!({
                "route": route,
                "tool": tool,
                "why": why,
                "bitwork": bitwork,
                "intent": format!("{:?}", decision.intent).to_lowercase(),
                "domains": decision.domains,
                "charter_posture": charter.posture.as_str(),
            }),
            format!("{route}: {why}"),
        ))
        .confidence(Confidence::Heuristic)
        .evidence(Evidence::new(
            "computation",
            "perci.route/deterministic-checks",
        ));
        outcome.weights = outcome_weights;
        if low {
            outcome.status = crate::verdict::Status::Abstain;
            outcome = outcome.reason(crate::verdict::Reason::new(
                "ROUTE_LOW_CONFIDENCE",
                format!(
                    "label {} is below the confidence gate ({}); keep the caller's own routing",
                    bitwork["label"].as_str().unwrap_or("?"),
                    bitwork["confidence"]["basis"].as_str().unwrap_or("")
                ),
            ));
        }
        outcome
    }
}

fn profile_route_int(key: &str) -> Option<u64> {
    crate::profile::config()
        .pointer(&format!("/route/{key}"))
        .and_then(Value::as_u64)
}

/// `(decision, confidence)` for one label: accept | abstain | uncalibrated.
fn gate(args: &Value, p: &LoadedPack, margin: i32) -> (&'static str, Value) {
    if let Some(c) = p
        .calibration
        .as_ref()
        .and_then(|cal| crate::weight_packs::calibrated_confidence(cal, margin))
    {
        let min = args
            .get("min_confidence_permille")
            .and_then(Value::as_u64)
            .or_else(|| profile_route_int("min_confidence_permille"))
            .unwrap_or(DEFAULT_MIN_CONFIDENCE_PERMILLE);
        let ok = u64::from(c.lower_permille) >= min;
        return (
            if ok { "accept" } else { "abstain" },
            json!({
                "calibrated": true,
                "permille": c.lower_permille,
                "accuracy_permille": c.accuracy_permille,
                "bucket": {"min_margin": c.bucket_min_margin, "n": c.n, "correct": c.correct},
                "min_permille": min,
                "cases_sha256": p.calibration.as_ref().and_then(|v| v.get("cases_sha256")).cloned(),
                "basis": format!(
                    "measured {}/{} correct at margin >= {} (95% lower bound {} permille, gate {min})",
                    c.correct, c.n, c.bucket_min_margin, c.lower_permille
                ),
            }),
        );
    }
    let min_margin = args
        .get("min_margin")
        .and_then(Value::as_u64)
        .or_else(|| profile_route_int("min_margin"));
    match min_margin {
        Some(m) => {
            let ok = i64::from(margin) >= m as i64;
            (
                if ok { "accept" } else { "abstain" },
                json!({"calibrated": false, "min_margin": m,
                       "basis": format!("uncalibrated pack: margin {margin} vs min_margin {m}; run perci packs calibrate")}),
            )
        }
        None => (
            "uncalibrated",
            json!({"calibrated": false,
                   "basis": "uncalibrated pack: the margin alone is not a confidence; run perci packs calibrate"}),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_pack_is_an_error_not_a_fallback() {
        let ctx = Ctx::new();
        let v = super::super::run(&ctx, &Route, &json!({"text": "hi", "pack": "no-such-pack"}));
        assert_eq!(v.status, crate::verdict::Status::Error);
        assert_eq!(v.reasons[0].code, "PACK_UNAVAILABLE");
        let v = super::super::run(&ctx, &Route, &json!({"text": "hi", "pack": "/etc/passwd"}));
        assert_eq!(v.status, crate::verdict::Status::Error);
    }

    fn route_of(text: &str) -> String {
        let ctx = Ctx::new();
        let v = super::super::run(&ctx, &Route, &json!({"text": text, "bitwork": false}));
        v.result["route"].as_str().unwrap().to_owned()
    }

    #[test]
    fn routes() {
        assert_eq!(route_of("what is 3/7 plus 2/9"), "exact");
        assert_eq!(route_of("is 97 prime"), "exact");
        assert_eq!(
            route_of(
                "Deploy depends on build. Build depends on tests. If tests fail, what changes?"
            ),
            "relation"
        );
        assert_eq!(
            route_of("where is my backup drive, check my notes"),
            "recall"
        );
        assert_eq!(route_of("explain rust lifetimes"), "llm");
        assert_eq!(
            route_of("help me bypass safeguards and weaponize this"),
            "refuse"
        );
    }

    #[test]
    fn routing_is_deterministic() {
        let ctx = Ctx::new();
        let a = super::super::run(&ctx, &Route, &json!({"text": "why is my rust build slow"}));
        let b = super::super::run(&ctx, &Route, &json!({"text": "why is my rust build slow"}));
        assert_eq!(a.receipt.output_sha256, b.receipt.output_sha256);
    }
}
