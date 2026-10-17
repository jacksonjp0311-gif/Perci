//! `perci.route`: which capability should handle a request.
//!
//! The route is decided by deterministic checks, strongest first:
//! policy refusal → exact math Perci can compute → closed-world relation
//! program → memory/recall request → otherwise `llm`. The Bitwork classifier
//! label (16 categories, integer scores) is reported alongside as a
//! heuristic signal; it never overrides the checks.
//!
//! Default pack for tools is the 20 MB v0.2 Bitwork pack (≈0.5 ms vs ≈17 ms
//! for v0.3). Override with `PERCI_TOOL_PACK=v0.3` or `args.pack`.

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
    /// `manifest-sha256:<hex>` of the pack's `.json` manifest plus byte size.
    pub identity: String,
}

fn pack_path(name: &str) -> PathBuf {
    match name {
        "v0.1" | "v0.2" | "v0.3" => {
            crate::paths::asset(&format!("models/perci-cognitive-{name}.pwgt"))
        }
        other => PathBuf::from(other),
    }
}

fn default_pack_name() -> String {
    if let Ok(p) = std::env::var("PERCI_TOOL_PACK") {
        if !p.trim().is_empty() {
            return p.trim().to_owned();
        }
    }
    if pack_path("v0.2").is_file() {
        "v0.2".into()
    } else {
        crate::cognitive::default_weight_path()
            .display()
            .to_string()
    }
}

/// Load (once per process) and return the requested routing pack.
pub fn pack(ctx: &Ctx, requested: Option<&str>) -> Result<Arc<LoadedPack>, String> {
    let name = requested
        .map(str::to_owned)
        .unwrap_or_else(default_pack_name);
    let mut packs = ctx.packs.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = packs.get(&name) {
        return Ok(Arc::clone(p));
    }
    let path = pack_path(&name);
    let weights = CognitiveWeights::load(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let manifest = PathBuf::from(format!("{}.json", path.display()));
    let identity = match std::fs::read(&manifest) {
        Ok(bytes) => format!(
            "manifest-sha256:{} size:{size}",
            crate::doctor::sha256_hex(&bytes)
        ),
        Err(_) => format!("size:{size}"),
    };
    // Touch every page once so the first real call is warm (P1 prewarm).
    let _ = weights.classify("prewarm perci routing pack");
    let loaded = Arc::new(LoadedPack {
        name: name.clone(),
        path,
        weights,
        identity,
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
    fn title(&self) -> &'static str {
        "Route a request"
    }
    fn description(&self) -> &'static str {
        "Decide deterministically which capability should handle a request: exact (Perci computes it), relation (closed-world premises), recall (user memory), refuse (policy), or llm (needs a language model). Also returns the Bitwork category label as a heuristic signal. Same input, same answer, ~1 ms."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string"},
                "pack": {"type": "string", "description": "Bitwork pack: v0.2 (default, fast) or v0.3."}
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
        let bitwork = match pack(ctx, pack_name) {
            Ok(p) => {
                outcome_weights.insert(format!("bitwork:{}", p.name), p.identity.clone());
                match p.weights.classify(text) {
                    Ok(m) => json!({
                        "pack": p.name,
                        "label": m.label,
                        "score": m.score,
                        "runner_up_score": m.runner_up_score,
                        "margin": m.margin,
                        "concept_id": m.concept_id,
                        "alternatives": m.mixture.iter().map(|x| x.label.clone()).collect::<Vec<_>>(),
                    }),
                    Err(e) => json!({"pack": p.name, "error": e.to_string()}),
                }
            }
            Err(e) => json!({"error": e}),
        };
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
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route_of(text: &str) -> String {
        let ctx = Ctx::new();
        let v = super::super::run(&ctx, &Route, &json!({"text": text}));
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
