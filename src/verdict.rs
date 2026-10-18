//! `perci.verdict.v1`: the one reply shape every Perci tool returns.
//!
//! A verdict says **pass**, **fail**, **abstain** or **error**, why, on what
//! evidence, and carries a receipt with two SHA-256 hashes:
//!
//! * `input_sha256`  = sha256(canonical JSON of `{"tool", "args"}`)
//! * `output_sha256` = sha256(canonical JSON of `{"status", "result", "reasons", "evidence"}`)
//! * `receipt_sha256` = sha256(canonical JSON of `{"schema", "tool",
//!   "input_sha256", "output_sha256", "perci_version", "build_id", "weights",
//!   "config"}`): the receipt id. It binds the question, the answer and the
//!   engine/pack that produced it, so two different inputs that happen to get
//!   the same output (two reads both allowed) never share a receipt id.
//!
//! Canonical JSON means sorted object keys, no insignificant whitespace, and
//! floats written as strings, so the same input on the same build and
//! weights gives the same `output_sha256` on any machine. `elapsed_us`,
//! `config` and `weights` are reported but never hashed.
//!
//! `confidence` is categorical, never a made-up float: `exact` for proven
//! computation, `bounded` for closed-world relation results and hash-checked
//! evidence, `heuristic` for routing and keyword posture.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub const SCHEMA: &str = "perci.verdict.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Fail,
    Abstain,
    Error,
}

impl Status {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Abstain => "abstain",
            Self::Error => "error",
        }
    }

    /// Process exit code for `perci tool …` (hook-friendly):
    /// 0 pass, 1 fail, 4 abstain, 3 error. (2 is reserved for config errors.)
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Pass => 0,
            Self::Fail => 1,
            Self::Abstain => 4,
            Self::Error => 3,
        }
    }

    /// Worst-of combination used by orchestrators: fail > error > abstain > pass.
    pub fn worst(self, other: Status) -> Status {
        fn rank(s: Status) -> u8 {
            match s {
                Status::Pass => 0,
                Status::Abstain => 1,
                Status::Error => 2,
                Status::Fail => 3,
            }
        }
        if rank(other) > rank(self) {
            other
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Exact,
    Bounded,
    Heuristic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<[usize; 2]>,
}

impl Reason {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            detail: detail.into(),
            span: None,
        }
    }

    pub fn at(mut self, start: usize, end: usize) -> Self {
        self.span = Some([start, end]);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// computation | file | memory_record | memory_note | policy_rule | ledger_entry | ledger_head | temporal_model | b2e_case | heldout_item | step_log
    pub kind: String,
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
}

impl Evidence {
    pub fn new(kind: &str, reference: impl Into<String>) -> Self {
        Self {
            kind: kind.to_owned(),
            reference: reference.into(),
            sha256: None,
            line: None,
        }
    }

    pub fn hashed(mut self, sha256: impl Into<String>) -> Self {
        self.sha256 = Some(sha256.into());
        self
    }

    pub fn line(mut self, line: u64) -> Self {
        self.line = Some(line);
        self
    }
}

/// What a tool computes before it is sealed into a [`Verdict`].
#[derive(Clone, Debug)]
pub struct Outcome {
    pub status: Status,
    pub result: Value,
    pub reasons: Vec<Reason>,
    pub evidence: Vec<Evidence>,
    pub confidence: Confidence,
    pub deterministic: bool,
    /// Weight or index identities used (reported in the receipt, not hashed).
    pub weights: BTreeMap<String, String>,
}

impl Outcome {
    pub fn new(status: Status, result: Value) -> Self {
        Self {
            status,
            result,
            reasons: Vec::new(),
            evidence: Vec::new(),
            confidence: Confidence::Exact,
            deterministic: true,
            weights: BTreeMap::new(),
        }
    }

    pub fn pass(result: Value) -> Self {
        Self::new(Status::Pass, result)
    }

    pub fn fail(result: Value) -> Self {
        Self::new(Status::Fail, result)
    }

    pub fn abstain(code: &str, detail: impl Into<String>) -> Self {
        Self::new(Status::Abstain, json!({})).reason(Reason::new(code, detail))
    }

    pub fn error(code: &str, detail: impl Into<String>) -> Self {
        Self::new(Status::Error, json!({})).reason(Reason::new(code, detail))
    }

    pub fn reason(mut self, reason: Reason) -> Self {
        self.reasons.push(reason);
        self
    }

    pub fn evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn nondeterministic(mut self) -> Self {
        self.deterministic = false;
        self
    }

    pub fn with_result(mut self, result: Value) -> Self {
        self.result = result;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Receipt id: binds tool + input + output + engine and weight identity
    /// (see the module docs). Added in 0.11.8; empty when parsing an older
    /// verdict that did not carry it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub receipt_sha256: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub perci_version: String,
    pub build_id: String,
    #[serde(default)]
    pub weights: BTreeMap<String, String>,
    #[serde(default)]
    pub config: BTreeMap<String, String>,
    pub deterministic: bool,
    pub confidence: Confidence,
    #[serde(default)]
    pub elapsed_us: u64,
    /// Answered from the per-process verdict cache (same input hash and
    /// config). Not hashed, like `elapsed_us`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cache_hit: bool,
    /// Ledger entry that recorded this verdict: `ledger:<run_id>#<seq>`.
    /// Not hashed (it is assigned after sealing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub schema: String,
    pub tool: String,
    pub status: Status,
    pub result: Value,
    pub reasons: Vec<Reason>,
    pub evidence: Vec<Evidence>,
    pub receipt: Receipt,
}

/// Environment variables that can change a tool's output. They are copied
/// into every receipt so a replay can reproduce the conditions.
pub const OUTPUT_ENV: &[&str] = &[
    "PERCI_ASCII",
    "PERCI_WEIGHTS",
    "PERCI_TOOL_PACK",
    "PERCI_POLICY",
    "PERCI_AGENT",
];

pub fn output_config() -> BTreeMap<String, String> {
    OUTPUT_ENV
        .iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| ((*k).to_owned(), v)))
        .collect()
}

impl Verdict {
    /// Seal an [`Outcome`] into a verdict with hashed receipt.
    pub fn seal(tool: &str, args: &Value, outcome: Outcome, elapsed_us: u64) -> Self {
        let input_sha256 = input_hash(tool, args);
        let reasons = outcome.reasons;
        let evidence = outcome.evidence;
        let output_sha256 = output_hash(outcome.status, &outcome.result, &reasons, &evidence);
        let config = output_config();
        let receipt_sha256 = receipt_hash(
            tool,
            &input_sha256,
            &output_sha256,
            crate::branding::build_id(),
            &outcome.weights,
            &config,
        );
        Verdict {
            schema: SCHEMA.to_owned(),
            tool: tool.to_owned(),
            status: outcome.status,
            result: outcome.result,
            reasons,
            evidence,
            receipt: Receipt {
                receipt_sha256,
                input_sha256,
                output_sha256,
                perci_version: env!("CARGO_PKG_VERSION").to_owned(),
                build_id: crate::branding::build_id().to_owned(),
                weights: outcome.weights,
                config,
                deterministic: outcome.deterministic,
                confidence: outcome.confidence,
                elapsed_us,
                cache_hit: false,
                ledger: None,
            },
        }
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self)
            .unwrap_or_else(|e| json!({"schema": SCHEMA, "error": e.to_string()}))
    }

    /// One-line human summary for `content[0].text` in MCP replies.
    pub fn summary(&self) -> String {
        let mut s = format!("{} {}", self.tool, self.status.as_str().to_uppercase());
        if let Some(text) = self.result.get("summary").and_then(Value::as_str) {
            s.push_str(": ");
            s.push_str(text);
        }
        for r in self.reasons.iter().take(3) {
            s.push_str(&format!(" | {}: {}", r.code, r.detail));
        }
        s
    }

    /// Recompute `output_sha256` from the body. A receipt that does not match
    /// means the verdict was edited after Perci issued it.
    pub fn receipt_matches(&self) -> bool {
        let output_ok = output_hash(self.status, &self.result, &self.reasons, &self.evidence)
            == self.receipt.output_sha256;
        let id_ok = self.receipt.receipt_sha256.is_empty()
            || receipt_hash(
                &self.tool,
                &self.receipt.input_sha256,
                &self.receipt.output_sha256,
                &self.receipt.build_id,
                &self.receipt.weights,
                &self.receipt.config,
            ) == self.receipt.receipt_sha256;
        output_ok && id_ok
    }
}

/// The receipt id (`receipt_sha256`): input, output and engine identity in
/// one hash. `perci_version` is taken from `build_id`'s semver prefix so the
/// two can never disagree.
pub fn receipt_hash(
    tool: &str,
    input_sha256: &str,
    output_sha256: &str,
    build_id: &str,
    weights: &BTreeMap<String, String>,
    config: &BTreeMap<String, String>,
) -> String {
    let version = build_id.split('+').next().unwrap_or(build_id);
    sha256_canonical(&json!({
        "schema": SCHEMA,
        "tool": tool,
        "input_sha256": input_sha256,
        "output_sha256": output_sha256,
        "perci_version": version,
        "build_id": build_id,
        "weights": weights,
        "config": config,
    }))
}

pub fn input_hash(tool: &str, args: &Value) -> String {
    sha256_canonical(&json!({"tool": tool, "args": args}))
}

pub fn output_hash(
    status: Status,
    result: &Value,
    reasons: &[Reason],
    evidence: &[Evidence],
) -> String {
    sha256_canonical(&json!({
        "status": status.as_str(),
        "result": result,
        "reasons": reasons,
        "evidence": evidence,
    }))
}

pub fn sha256_canonical(value: &Value) -> String {
    crate::doctor::sha256_hex(canonical_json(value).as_bytes())
}

/// Canonical JSON: sorted keys, compact, floats as strings.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => {
            out.push_str(&serde_json::to_string(value).unwrap_or_default())
        }
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                out.push_str(&n.to_string());
            } else {
                // Floats differ across platforms in their last digits; hash them as text.
                out.push_str(&serde_json::to_string(&n.to_string()).unwrap_or_default());
            }
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_canonical(&map[*k], out);
            }
            out.push('}');
        }
    }
}

/// JSON Schema for a verdict, used as the MCP `outputSchema`.
pub fn json_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://perci.local/schemas/verdict.v1.json",
        "type": "object",
        "required": ["schema", "tool", "status", "result", "reasons", "evidence", "receipt"],
        "properties": {
            "schema": {"const": SCHEMA},
            "tool": {"type": "string"},
            "status": {"enum": ["pass", "fail", "abstain", "error"]},
            "result": {"type": "object"},
            "reasons": {"type": "array", "items": {
                "type": "object", "required": ["code", "detail"],
                "properties": {
                    "code": {"type": "string"},
                    "detail": {"type": "string"},
                    "span": {"type": "array", "items": {"type": "integer"}, "minItems": 2, "maxItems": 2}
                }
            }},
            "evidence": {"type": "array", "items": {
                "type": "object", "required": ["kind", "ref"],
                "properties": {
                    "kind": {"type": "string", "description": "computation | file | memory_record | memory_note | policy_rule | ledger_entry | ledger_head | temporal_model | b2e_case | heldout_item | step_log (open set: new kinds may be added)"},
                    "ref": {"type": "string"},
                    "sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                    "line": {"type": "integer"}
                }
            }},
            "receipt": {
                "type": "object",
                "required": ["input_sha256", "output_sha256", "perci_version", "build_id", "deterministic", "confidence"],
                "properties": {
                    "receipt_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$", "description": "receipt id: sha256 over tool, input_sha256, output_sha256, perci_version, build_id, weights and config"},
                    "input_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                    "output_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                    "perci_version": {"type": "string"},
                    "build_id": {"type": "string"},
                    "weights": {"type": "object", "additionalProperties": {"type": "string"}},
                    "config": {"type": "object", "additionalProperties": {"type": "string"}},
                    "deterministic": {"type": "boolean"},
                    "confidence": {"enum": ["exact", "bounded", "heuristic"]},
                    "elapsed_us": {"type": "integer"},
                    "cache_hit": {"type": "boolean"},
                    "ledger": {"type": "string", "description": "ledger:<run_id>#<seq> of the entry that recorded this verdict"}
                }
            }
        }
    })
}

/// Structural check of a verdict value against [`json_schema`] (no external
/// validator dependency). Returns every violation found.
pub fn validate(value: &Value) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    let empty = Map::new();
    let obj = value.as_object().unwrap_or_else(|| {
        errs.push("verdict is not an object".into());
        &empty
    });
    let is_hex64 = |s: &str| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    };
    for key in [
        "schema", "tool", "status", "result", "reasons", "evidence", "receipt",
    ] {
        if !obj.contains_key(key) {
            errs.push(format!("missing {key}"));
        }
    }
    if obj.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        errs.push("schema must be perci.verdict.v1".into());
    }
    if !obj.get("tool").is_some_and(Value::is_string) {
        errs.push("tool must be a string".into());
    }
    match obj.get("status").and_then(Value::as_str) {
        Some("pass" | "fail" | "abstain" | "error") => {}
        _ => errs.push("status must be pass|fail|abstain|error".into()),
    }
    if !obj.get("result").is_some_and(Value::is_object) {
        errs.push("result must be an object".into());
    }
    for r in obj
        .get("reasons")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if !r.get("code").is_some_and(Value::is_string)
            || !r.get("detail").is_some_and(Value::is_string)
        {
            errs.push(format!("reason needs code+detail: {r}"));
        }
    }
    const KINDS: &[&str] = &[
        "computation",
        "file",
        "memory_note",
        "policy_rule",
        "b2e_case",
        "heldout_item",
        "step_log",
    ];
    for e in obj
        .get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if !e
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|k| KINDS.contains(&k))
        {
            errs.push(format!("evidence kind invalid: {e}"));
        }
        if !e.get("ref").is_some_and(Value::is_string) {
            errs.push(format!("evidence needs ref: {e}"));
        }
        if let Some(h) = e.get("sha256") {
            if !h.as_str().is_some_and(is_hex64) {
                errs.push(format!("evidence sha256 not 64 lowercase hex: {e}"));
            }
        }
    }
    match obj.get("receipt").and_then(Value::as_object) {
        None => errs.push("receipt must be an object".into()),
        Some(rc) => {
            for key in ["input_sha256", "output_sha256"] {
                if !rc.get(key).and_then(Value::as_str).is_some_and(is_hex64) {
                    errs.push(format!("receipt.{key} must be 64 lowercase hex"));
                }
            }
            for key in ["perci_version", "build_id"] {
                if !rc.get(key).is_some_and(Value::is_string) {
                    errs.push(format!("receipt.{key} must be a string"));
                }
            }
            if !rc.get("deterministic").is_some_and(Value::is_boolean) {
                errs.push("receipt.deterministic must be a boolean".into());
            }
            match rc.get("confidence").and_then(Value::as_str) {
                Some("exact" | "bounded" | "heuristic") => {}
                _ => errs.push("receipt.confidence must be exact|bounded|heuristic".into()),
            }
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_sorts_keys_and_stringifies_floats() {
        let v = json!({"b": 1, "a": {"d": [true, null], "c": 0.5}});
        assert_eq!(
            canonical_json(&v),
            r#"{"a":{"c":"0.5","d":[true,null]},"b":1}"#
        );
    }

    #[test]
    fn seal_is_stable_and_validates() {
        let args = json!({"expr": "17% of 240"});
        let make = |elapsed| {
            Verdict::seal(
                "perci.exact",
                &args,
                Outcome::pass(json!({"value": "204/5", "decimal": "40.8"}))
                    .evidence(Evidence::new("computation", "exact_expr").hashed("0".repeat(64))),
                elapsed,
            )
        };
        let a = make(3);
        let b = make(99);
        assert_eq!(
            a.receipt.output_sha256, b.receipt.output_sha256,
            "elapsed_us must not be hashed"
        );
        assert_eq!(a.receipt.input_sha256, b.receipt.input_sha256);
        assert!(a.receipt_matches());
        validate(&a.to_value()).unwrap();

        let mut tampered = a.clone();
        tampered.result = json!({"value": "41"});
        assert!(!tampered.receipt_matches());
    }

    #[test]
    fn validate_reports_violations() {
        let errs = validate(&json!({"schema": "nope", "status": "maybe"})).unwrap_err();
        assert!(errs.iter().any(|e| e.contains("schema")));
        assert!(errs.iter().any(|e| e.contains("status")));
        assert!(errs.iter().any(|e| e.contains("receipt")));
    }

    #[test]
    fn worst_status_ordering() {
        assert_eq!(Status::Pass.worst(Status::Abstain), Status::Abstain);
        assert_eq!(Status::Abstain.worst(Status::Fail), Status::Fail);
        assert_eq!(Status::Fail.worst(Status::Pass), Status::Fail);
        assert_eq!(Status::Error.worst(Status::Abstain), Status::Error);
    }
}
