//! `perci.eval`: extension slot for bug2eval (regression proof for fixes).
//!
//! Track B ships the bug2eval-backed implementation. Until it registers
//! itself with [`super::register`] (same name, `perci.eval`), this placeholder
//! answers `abstain` with `NOT_INSTALLED`, so harnesses get a clear verdict
//! instead of an unknown-tool error. It is off by default.

use super::{Ctx, Tool};
use crate::verdict::Outcome;
use serde_json::{json, Value};

pub struct Eval;

impl Tool for Eval {
    fn name(&self) -> &'static str {
        "perci.eval"
    }
    fn title(&self) -> &'static str {
        "Run a bug2eval case"
    }
    fn description(&self) -> &'static str {
        "Validate or run a bug2eval case (the bug reproduces before the fix and is gone after). Placeholder until the bug2eval integration is installed."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["case"],
            "properties": {
                "case": {"type": "string", "description": "Path to a .b2e case."},
                "mode": {"enum": ["validate", "run"], "default": "validate"}
            }
        })
    }
    fn default_enabled(&self) -> bool {
        false
    }
    fn call(&self, _ctx: &Ctx, _args: &Value) -> Outcome {
        Outcome::abstain(
            "NOT_INSTALLED",
            "perci.eval needs the bug2eval integration, which is not installed in this build",
        )
    }
}
