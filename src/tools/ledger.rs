//! Step ledger: a hash-chained, append-only record of every tool verdict in
//! one run (an MCP session, or a harness run named by `PERCI_RUN_ID`).
//!
//! One JSON line per entry in `PERCI_PROFILE/ledger/<run_id>.jsonl`:
//!
//! ```json
//! {"schema":"perci.ledger.v1","run_id":"r1","seq":3,"ts_unix":1760700000,
//!  "tool":"perci.agent.gate_step","phase":"post","step":"shell:cargo test",
//!  "status":"pass","input_sha256":"…","output_sha256":"…","receipt_sha256":"…",
//!  "prev_hash":"…","hash":"…"}
//! ```
//!
//! `hash = sha256(canonical JSON of the entry without "hash")`; `prev_hash`
//! is the previous entry's hash, or `sha256("perci.ledger.v1|<run_id>")` for
//! the first one, so entries cannot be edited, reordered, dropped from the
//! middle or moved between runs without [`verify_lines`] noticing. Dropping
//! entries from the *end* leaves a valid shorter chain: pin the head hash
//! (`ledger_verify` with `expect_head`) to detect that.
//!
//! Sequence rules (`"sequence"` in the policy) are evaluated against the
//! entries of the current run by [`check_sequence`]; verdicts cite the
//! entries they relied on as `ledger:<run_id>#<seq>` evidence.

use super::{arg_str, opt_str, try_arg, with_summary, Ctx, Tool};
use crate::verdict::{canonical_json, Confidence, Evidence, Outcome, Reason, Status, Verdict};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, Write};
use std::path::PathBuf;

pub const SCHEMA: &str = "perci.ledger.v1";
/// Ledger lines longer than this are rejected when reading.
const MAX_LINE: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub schema: String,
    pub run_id: String,
    pub seq: u64,
    pub ts_unix: u64,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default)]
    pub step: String,
    pub status: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub receipt_sha256: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cache_hit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub prev_hash: String,
    pub hash: String,
}

impl Entry {
    /// `sha256(canonical JSON without "hash")`.
    pub fn compute_hash(&self) -> String {
        let mut v = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(o) = v.as_object_mut() {
            o.remove("hash");
        }
        crate::doctor::sha256_hex(canonical_json(&v).as_bytes())
    }
    /// `ledger:<run_id>#<seq>`.
    pub fn reference(&self) -> String {
        format!("ledger:{}#{}", self.run_id, self.seq)
    }
    pub fn evidence(&self) -> Evidence {
        Evidence::new("ledger_entry", self.reference()).hashed(self.hash.clone())
    }
    /// Token used by the temporal router: `tool[:phase][:step]` (wire name).
    pub fn token(&self) -> String {
        token(&self.tool, self.phase.as_deref(), &self.step)
    }
}

pub fn token(tool: &str, phase: Option<&str>, step: &str) -> String {
    let mut t = super::wire_name(tool);
    if let Some(p) = phase.filter(|p| !p.is_empty()) {
        t.push(':');
        t.push_str(p);
    }
    if !step.is_empty() {
        t.push(':');
        t.push_str(step);
    }
    t.replace(['\n', '\r', '|'], " ")
}

pub fn genesis(run_id: &str) -> String {
    crate::doctor::sha256_hex(format!("{SCHEMA}|{run_id}").as_bytes())
}

pub fn valid_run_id(id: &str) -> bool {
    (1..=96).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && !id.starts_with('.')
}

pub fn ledger_dir() -> PathBuf {
    crate::paths::profile_file("ledger")
}

pub fn run_path(run_id: &str) -> PathBuf {
    ledger_dir().join(format!("{run_id}.jsonl"))
}

/// Run ids with a ledger file, sorted.
pub fn list_runs() -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(ledger_dir())
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|n| n.strip_suffix(".jsonl"))
                        .filter(|n| valid_run_id(n))
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// One problem found by [`verify_lines`].
#[derive(Clone, Debug, Serialize)]
pub struct Problem {
    pub line: usize,
    pub code: &'static str,
    pub detail: String,
}

/// Result of verifying one run's ledger.
#[derive(Clone, Debug, Serialize)]
pub struct Verified {
    pub run_id: String,
    pub ok: bool,
    pub entries: usize,
    pub head: String,
    pub problems: Vec<Problem>,
    #[serde(skip)]
    pub parsed: Vec<Entry>,
}

/// Check every line: parse, schema, run id, contiguous seq from 1, prev links
/// and recomputed hashes.
pub fn verify_lines(run_id: &str, text: &str) -> Verified {
    let mut problems = Vec::new();
    let mut parsed = Vec::new();
    let mut prev = genesis(run_id);
    let mut expect_seq = 1u64;
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > MAX_LINE {
            problems.push(Problem {
                line: n,
                code: "LINE_TOO_LONG",
                detail: format!("{} bytes", line.len()),
            });
            break;
        }
        let e: Entry = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(err) => {
                problems.push(Problem {
                    line: n,
                    code: "PARSE",
                    detail: err.to_string(),
                });
                break;
            }
        };
        if e.schema != SCHEMA {
            problems.push(Problem {
                line: n,
                code: "SCHEMA",
                detail: e.schema.clone(),
            });
        }
        if e.run_id != run_id {
            problems.push(Problem {
                line: n,
                code: "RUN_MISMATCH",
                detail: format!("entry belongs to run {}", e.run_id),
            });
        }
        if e.seq != expect_seq {
            problems.push(Problem {
                line: n,
                code: "SEQ_GAP",
                detail: format!(
                    "seq {} where {expect_seq} was expected (reordered or removed)",
                    e.seq
                ),
            });
        }
        if e.prev_hash != prev {
            problems.push(Problem {
                line: n,
                code: "BROKEN_CHAIN",
                detail: format!("seq {}: prev_hash does not match the previous entry", e.seq),
            });
        }
        let actual = e.compute_hash();
        if actual != e.hash {
            problems.push(Problem {
                line: n,
                code: "HASH_MISMATCH",
                detail: format!("seq {}: entry was edited after it was written", e.seq),
            });
        }
        prev = e.hash.clone();
        expect_seq = e.seq + 1;
        parsed.push(e);
    }
    Verified {
        run_id: run_id.to_owned(),
        ok: problems.is_empty(),
        entries: parsed.len(),
        head: parsed
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| genesis(run_id)),
        problems,
        parsed,
    }
}

pub fn verify_run(run_id: &str) -> io::Result<Verified> {
    match std::fs::read_to_string(run_path(run_id)) {
        Ok(t) => Ok(verify_lines(run_id, &t)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(verify_lines(run_id, "")),
        Err(e) => Err(e),
    }
}

/// The ledger of the run this process records into.
pub struct RunLedger {
    pub run_id: String,
    pub path: PathBuf,
    pub entries: Vec<Entry>,
    /// Set when the existing file failed verification; nothing is appended.
    pub broken: Option<String>,
}

impl RunLedger {
    pub fn open(run_id: &str) -> io::Result<Self> {
        if !valid_run_id(run_id) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid run id {run_id:?} (1-96 chars of [A-Za-z0-9._-])"),
            ));
        }
        let v = verify_run(run_id)?;
        let broken = (!v.ok).then(|| {
            v.problems
                .iter()
                .map(|p| format!("line {} {}: {}", p.line, p.code, p.detail))
                .collect::<Vec<_>>()
                .join("; ")
        });
        Ok(Self {
            run_id: run_id.to_owned(),
            path: run_path(run_id),
            entries: v.parsed,
            broken,
        })
    }

    pub fn head(&self) -> String {
        self.entries
            .last()
            .map(|e| e.hash.clone())
            .unwrap_or_else(|| genesis(&self.run_id))
    }

    /// Append one entry (flushed before returning).
    pub fn append(
        &mut self,
        tool: &str,
        phase: Option<&str>,
        step: &str,
        status: &str,
        verdict: &Verdict,
        note: Option<String>,
    ) -> io::Result<Entry> {
        if let Some(why) = &self.broken {
            return Err(io::Error::other(format!(
                "ledger {} failed verification, not appending: {why}",
                self.run_id
            )));
        }
        let receipt = serde_json::to_value(&verdict.receipt).unwrap_or(Value::Null);
        let mut e = Entry {
            schema: SCHEMA.into(),
            run_id: self.run_id.clone(),
            seq: self.entries.last().map(|e| e.seq + 1).unwrap_or(1),
            ts_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            tool: tool.to_owned(),
            phase: phase.map(str::to_owned),
            step: step.to_owned(),
            status: status.to_owned(),
            input_sha256: verdict.receipt.input_sha256.clone(),
            output_sha256: verdict.receipt.output_sha256.clone(),
            receipt_sha256: crate::doctor::sha256_hex(canonical_json(&receipt).as_bytes()),
            cache_hit: verdict.receipt.cache_hit,
            note,
            prev_hash: self.head(),
            hash: String::new(),
        };
        e.hash = e.compute_hash();
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(&e).map_err(io::Error::other)? + "\n";
        f.write_all(line.as_bytes())?;
        f.flush()?;
        self.entries.push(e.clone());
        Ok(e)
    }
}

// ---- steps --------------------------------------------------------------------

const WRAPPERS: &[&str] = &["sudo", "env", "nice", "nohup", "time", "doas", "command"];
const WITH_SUBCOMMAND: &[&str] = &[
    "git",
    "cargo",
    "npm",
    "pnpm",
    "yarn",
    "go",
    "kubectl",
    "helm",
    "terraform",
    "docker",
    "make",
    "fly",
    "flyctl",
    "gh",
    "pip",
    "pip3",
    "uv",
    "poetry",
    "dotnet",
    "mvn",
    "gradle",
    "bun",
    "deno",
    "systemctl",
];

/// A short, stable label for an action: `shell:cargo test`, `shell:rm`,
/// `shell:python -m pytest`, `write`, `delete`, `network`, `push`.
pub fn action_step(action: &Value) -> String {
    let kind = action
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase();
    match kind.as_str() {
        "shell" | "exec" | "command" => {
            let argv: Vec<String> = match action.get("argv").and_then(Value::as_array) {
                Some(a) => a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                None => {
                    let cmd = action.get("command").and_then(Value::as_str).unwrap_or("");
                    let first = super::policy::segments(cmd)
                        .into_iter()
                        .next()
                        .unwrap_or_default();
                    super::policy::tokenize(&first)
                }
            };
            let mut i = 0;
            while i < argv.len()
                && (super::policy::is_env_assignment(&argv[i])
                    || WRAPPERS.contains(&super::policy::program_name(&argv[i]).as_str()))
            {
                i += 1;
            }
            let Some(prog) = argv.get(i) else {
                return "shell".into();
            };
            let mut prog = super::policy::program_name(prog);
            if matches!(
                prog.as_str(),
                "python3" | "py" | "python3.12" | "python3.11"
            ) {
                prog = "python".into();
            }
            let rest = &argv[i + 1..];
            if prog == "python" && rest.first().map(String::as_str) == Some("-m") {
                if let Some(m) = rest.get(1) {
                    return format!("shell:python -m {}", m.to_lowercase());
                }
            }
            if WITH_SUBCOMMAND.contains(&prog.as_str()) {
                if let Some(sub) = super::policy::subcommand(&prog, rest) {
                    return format!("shell:{prog} {sub}");
                }
            }
            format!("shell:{prog}")
        }
        "write" | "edit" | "create" => "write".into(),
        "delete" | "remove" => "delete".into(),
        "network" | "fetch" | "http" => "network".into(),
        "" => String::new(),
        other => other.to_owned(),
    }
}

/// `(phase, step)` recorded for a tool call.
pub fn step_of(tool: &str, args: &Value) -> (Option<String>, String) {
    let phase = args.get("phase").and_then(Value::as_str).map(str::to_owned);
    match tool {
        "perci.agent.gate_step" => (
            phase,
            args.get("action").map(action_step).unwrap_or_default(),
        ),
        "perci.policy_check" => (
            None,
            args.get("action").map(action_step).unwrap_or_default(),
        ),
        "perci.agent.verify_patch" => (None, "patch".into()),
        _ => (None, String::new()),
    }
}

/// Classes (from the policy's `sequence.classes`) a step belongs to. A step
/// matches a pattern when equal to it or when it starts with the pattern
/// followed by a space; a step equal to a class name belongs to that class.
pub fn classes_of(policy: &Value, step: &str) -> Vec<String> {
    let mut out = Vec::new();
    if step.is_empty() {
        return out;
    }
    if let Some(classes) = policy
        .pointer("/sequence/classes")
        .and_then(Value::as_object)
    {
        for (name, pats) in classes {
            let hit = step == name
                || pats.as_array().is_some_and(|a| {
                    a.iter().filter_map(Value::as_str).any(|p| {
                        step == p || (step.starts_with(p) && step[p.len()..].starts_with(' '))
                    })
                });
            if hit {
                out.push(name.clone());
            }
        }
    }
    out
}

/// One sequence-rule finding.
#[derive(Clone, Debug)]
pub struct SequenceFinding {
    pub rule: String,
    pub posture: super::policy::Posture,
    pub satisfied: bool,
    pub detail: String,
    pub evidence: Vec<Evidence>,
}

/// Evaluate every sequence rule whose `before` class contains `step`
/// against `history` (entries strictly before the step).
pub fn check_sequence(policy: &Value, step: &str, history: &[Entry]) -> Vec<SequenceFinding> {
    let classes = classes_of(policy, step);
    // Distinct steps are few; classify each once.
    let mut memo: std::collections::HashMap<&str, Vec<String>> = std::collections::HashMap::new();
    for e in history {
        if !memo.contains_key(e.step.as_str()) {
            memo.insert(e.step.as_str(), classes_of(policy, &e.step));
        }
    }
    let in_class = |e: &Entry, c: &str| {
        memo.get(e.step.as_str())
            .is_some_and(|v| v.iter().any(|x| x == c))
    };
    let mut out = Vec::new();
    let Some(rules) = policy.pointer("/sequence/rules").and_then(Value::as_array) else {
        return out;
    };
    for rule in rules {
        let before = rule.get("before").and_then(Value::as_str).unwrap_or("");
        if !classes.iter().any(|c| c == before) {
            continue;
        }
        let id = rule
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("sequence")
            .to_owned();
        let requires = rule.get("requires").and_then(Value::as_str).unwrap_or("");
        let want_status = rule.get("status").and_then(Value::as_str).unwrap_or("pass");
        let want_phase = rule.get("phase").and_then(Value::as_str);
        let not_before = rule.get("not_before").and_then(Value::as_str);
        let posture = super::policy::Posture::parse(
            rule.get("posture")
                .and_then(Value::as_str)
                .unwrap_or("propose_and_verify"),
        );
        let reason = rule
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("sequence rule");
        let last_change = not_before.and_then(|nb| history.iter().rev().find(|e| in_class(e, nb)));
        let floor = last_change.map(|e| e.seq).unwrap_or(0);
        let found = history.iter().rev().find(|e| {
            e.seq > floor
                && e.status == want_status
                && want_phase.is_none_or_eq(e.phase.as_deref())
                && in_class(e, requires)
        });
        let mut evidence = Vec::new();
        let (satisfied, detail) = match found {
            Some(e) => {
                evidence.push(e.evidence());
                (
                    true,
                    format!(
                        "{id}: {requires} {want_status} at {} ({})",
                        e.reference(),
                        e.step
                    ),
                )
            }
            None => {
                if let Some(c) = last_change {
                    evidence.push(c.evidence());
                }
                let since = match (not_before, last_change) {
                    (Some(nb), Some(c)) => {
                        format!(" after the last {nb} ({}, {})", c.reference(), c.step)
                    }
                    (Some(_), None) | (None, _) => String::new(),
                };
                (
                    false,
                    format!(
                        "{id}: {reason} — no {requires} step with status {want_status}{}{since} among {} earlier entr{} of this run",
                        want_phase.map(|p| format!(" (phase {p})")).unwrap_or_default(),
                        history.len(),
                        if history.len() == 1 { "y" } else { "ies" }
                    ),
                )
            }
        };
        out.push(SequenceFinding {
            rule: id,
            posture,
            satisfied,
            detail,
            evidence,
        });
    }
    out
}

trait OptEq {
    fn is_none_or_eq(&self, other: Option<&str>) -> bool;
}
impl OptEq for Option<&str> {
    fn is_none_or_eq(&self, other: Option<&str>) -> bool {
        match self {
            None => true,
            Some(w) => other == Some(*w),
        }
    }
}

/// Apply sequence findings to an outcome: unmet rules make the posture
/// stricter (never looser) and add `SEQUENCE_REQUIRES` reasons; met rules add
/// `SEQUENCE_OK` evidence. Without a run ledger the rules cannot be checked,
/// which is reported as `SEQUENCE_UNCHECKED` (needs approval).
pub fn apply_sequence(ctx: &Ctx, run_id: Option<&str>, step: &str, mut o: Outcome) -> Outcome {
    let policy = match &super::policy::loaded(ctx).policy {
        Ok(p) => p,
        Err(_) => return o,
    };
    if let Some(err) = policy.pointer("/sequence/error").and_then(Value::as_str) {
        o.status = o.status.worst(Status::Abstain);
        o = o.reason(Reason::new(
            "SEQUENCE_RULES_INVALID",
            format!("sequence rules could not be loaded, so no step can be cleared: {err}"),
        ));
        if let Some(p) = o.result.get_mut("proceed") {
            *p = json!(false);
        }
        return o;
    }
    if classes_of(policy, step).is_empty() {
        return o;
    }
    let findings = match ctx.with_ledger_entries(run_id, |h| check_sequence(policy, step, h)) {
        Some(f) => f,
        None => {
            let applicable: Vec<SequenceFinding> = check_sequence(policy, step, &[]);
            if applicable.is_empty() {
                return o;
            }
            o.status = o.status.worst(Status::Abstain);
            o = o.reason(Reason::new(
                "SEQUENCE_UNCHECKED",
                format!(
                    "{} applies to {step} but there is no run ledger (use perci mcp, or set PERCI_RUN_ID)",
                    applicable.iter().map(|f| f.rule.as_str()).collect::<Vec<_>>().join(", ")
                ),
            ));
            return o;
        }
    };
    let mut seq_rows = Vec::new();
    for f in findings {
        seq_rows.push(json!({"rule": f.rule, "satisfied": f.satisfied, "detail": f.detail}));
        if f.satisfied {
            for e in f.evidence {
                o = o.evidence(e);
            }
        } else {
            o.status = o.status.worst(f.posture.status());
            o = o.reason(Reason::new("SEQUENCE_REQUIRES", f.detail));
            for e in f.evidence {
                o = o.evidence(e);
            }
        }
    }
    if let Some(obj) = o.result.as_object_mut() {
        obj.insert("sequence".into(), json!(seq_rows));
        if let Some(p) = obj.get_mut("proceed") {
            *p = json!(o.status == Status::Pass);
        }
    }
    o
}

// ---- tools --------------------------------------------------------------------

fn entry_row(e: &Entry) -> Value {
    serde_json::to_value(e).unwrap_or(Value::Null)
}

fn requested_run(ctx: &Ctx, args: &Value) -> Result<String, Outcome> {
    match opt_str(args, "run_id")? {
        Some(r) if valid_run_id(r) => Ok(r.to_owned()),
        Some(r) => Err(Outcome::error("BAD_ARGS", format!("invalid run_id {r:?}"))),
        None => ctx.run_id().ok_or_else(|| {
            Outcome::error(
                "NO_RUN",
                "no current run: pass run_id, or start perci mcp / set PERCI_RUN_ID",
            )
        }),
    }
}

pub struct LedgerAppend;

impl Tool for LedgerAppend {
    fn name(&self) -> &'static str {
        "perci.ledger_append"
    }
    fn title(&self) -> &'static str {
        "Record a harness step"
    }
    fn description(&self) -> &'static str {
        "Record a step the harness took outside Perci (e.g. ran the tests, deployed, made a backup) in this run's hash-chained ledger, so sequence rules ('deploy requires tests passed earlier') and next_step can see it. Every Perci tool call is recorded automatically; use this for everything else."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["step"],
            "properties": {
                "step": {"type": "string", "description": "Step label or class: tests, deploy, backup, code_change, or an action label such as 'shell:cargo test'."},
                "status": {"enum": ["pass", "fail", "abstain", "error"], "default": "pass", "description": "Outcome of the step."},
                "phase": {"type": "string", "description": "e.g. post (the step finished)."},
                "action": {"type": "object", "description": "Optional action object; its label is used when step is omitted."},
                "note": {"type": "string", "maxLength": 200}
            },
            "additionalProperties": false
        })
    }
    fn read_only(&self) -> bool {
        false
    }
    fn idempotent(&self) -> bool {
        false
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let step = try_arg!(arg_str(args, "step")).trim().to_owned();
        let status = try_arg!(opt_str(args, "status")).unwrap_or("pass");
        if !matches!(status, "pass" | "fail" | "abstain" | "error") {
            return Outcome::error("BAD_ARGS", "status must be pass, fail, abstain or error");
        }
        let note = try_arg!(opt_str(args, "note")).map(|n| crate::util::text::truncate(n, 200));
        let phase = try_arg!(opt_str(args, "phase")).map(str::to_owned);
        if ctx.run_id().is_none() {
            return Outcome::error(
                "NO_RUN",
                "no current run: start perci mcp or set PERCI_RUN_ID",
            );
        }
        // The entry itself is written by the recorder (super::run_recorded)
        // with this step, phase, status and note.
        Outcome::pass(with_summary(
            json!({"step": step, "status": status, "phase": phase, "note": note}),
            format!("recorded {step} ({status})"),
        ))
        .nondeterministic()
    }
}

pub struct LedgerVerify;

impl Tool for LedgerVerify {
    fn name(&self) -> &'static str {
        "perci.ledger_verify"
    }
    fn title(&self) -> &'static str {
        "Verify a run ledger"
    }
    fn description(&self) -> &'static str {
        "Recompute every hash in a run's step ledger: pass = intact; fail = an entry was edited, reordered, removed or moved from another run (or the head differs from expect_head, i.e. entries were cut from the end)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "run_id": {"type": "string", "description": "Default: the current run."},
                "expect_head": {"type": "string", "description": "Head hash you saved earlier; detects truncation."}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let run = match requested_run(ctx, args) {
            Ok(r) => r,
            Err(o) => return o,
        };
        let expect = try_arg!(opt_str(args, "expect_head"));
        let v = match verify_run(&run) {
            Ok(v) => v,
            Err(e) => return Outcome::error("LEDGER_UNREADABLE", e.to_string()),
        };
        let mut o = Outcome::new(
            if v.ok { Status::Pass } else { Status::Fail },
            with_summary(
                json!({"run_id": run, "entries": v.entries, "head": v.head, "problems": v.problems}),
                if v.ok {
                    format!("{} entries, chain intact", v.entries)
                } else {
                    format!("{} problem(s)", v.problems.len())
                },
            ),
        )
        .confidence(Confidence::Exact)
        .evidence(Evidence::new("ledger_head", format!("ledger:{run}")).hashed(v.head.clone()))
        .nondeterministic();
        for p in &v.problems {
            o = o.reason(Reason::new(
                p.code,
                format!("line {}: {}", p.line, p.detail),
            ));
        }
        if let Some(h) = expect {
            if !h.eq_ignore_ascii_case(&v.head) {
                o.status = Status::Fail;
                o = o.reason(Reason::new(
                    "HEAD_MISMATCH",
                    "head differs from expect_head: entries were removed from the end or the run forked",
                ));
            }
        }
        o
    }
}

pub struct LedgerShow;

impl Tool for LedgerShow {
    fn name(&self) -> &'static str {
        "perci.ledger_show"
    }
    fn title(&self) -> &'static str {
        "Show a run ledger"
    }
    fn description(&self) -> &'static str {
        "Return the last entries of a run's step ledger (tool, step, status, hashes), oldest first, plus whether the chain verifies."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "run_id": {"type": "string"},
                "last": {"type": "integer", "minimum": 1, "maximum": 500, "default": 20}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let run = match requested_run(ctx, args) {
            Ok(r) => r,
            Err(o) => return o,
        };
        let last = args
            .get("last")
            .and_then(Value::as_u64)
            .unwrap_or(20)
            .clamp(1, 500) as usize;
        let v = match verify_run(&run) {
            Ok(v) => v,
            Err(e) => return Outcome::error("LEDGER_UNREADABLE", e.to_string()),
        };
        let skip = v.parsed.len().saturating_sub(last);
        let rows: Vec<Value> = v.parsed.iter().skip(skip).map(entry_row).collect();
        let mut o = Outcome::new(
            if v.entries == 0 { Status::Abstain } else { Status::Pass },
            with_summary(
                json!({"run_id": run, "entries": v.entries, "verified": v.ok, "head": v.head, "shown": rows}),
                format!("{} entries ({})", v.entries, if v.ok { "chain intact" } else { "CHAIN BROKEN" }),
            ),
        )
        .nondeterministic();
        if v.entries == 0 {
            o = o.reason(Reason::new("EMPTY", "no entries in this run yet"));
        }
        if !v.ok {
            o = o.reason(Reason::new("CHAIN_BROKEN", "run ledger_verify for details"));
        }
        o
    }
}

/// `perci ledger list | show RUN | verify [RUN…|--all] | learn` `[--json]`.
pub fn run_cli(args: &[String]) -> i32 {
    let json_out = args.iter().any(|a| a == "--json");
    let rest: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|a| *a != "--json")
        .collect();
    match rest.first().copied().unwrap_or("list") {
        "list" => {
            let runs = list_runs();
            if json_out {
                let rows: Vec<Value> = runs
                    .iter()
                    .map(|r| {
                        let v = verify_run(r);
                        match v {
                            Ok(v) => json!({"run_id": r, "entries": v.entries, "ok": v.ok, "head": v.head}),
                            Err(e) => json!({"run_id": r, "error": e.to_string()}),
                        }
                    })
                    .collect();
                println!(
                    "{}",
                    json!({"dir": ledger_dir().display().to_string(), "runs": rows})
                );
            } else {
                println!("ledger dir: {}", ledger_dir().display());
                for r in &runs {
                    match verify_run(r) {
                        Ok(v) => println!(
                            "{:<40} {:>5} entries  {}",
                            r,
                            v.entries,
                            if v.ok { "ok" } else { "BROKEN" }
                        ),
                        Err(e) => println!("{r:<40} error {e}"),
                    }
                }
            }
            0
        }
        "show" => {
            let Some(run) = rest.get(1) else {
                eprintln!("usage: perci ledger show RUN_ID [--json]");
                return 2;
            };
            match verify_run(run) {
                Ok(v) => {
                    if json_out {
                        let rows: Vec<Value> = v.parsed.iter().map(entry_row).collect();
                        println!(
                            "{}",
                            json!({"run_id": run, "ok": v.ok, "head": v.head, "entries": rows, "problems": v.problems})
                        );
                    } else {
                        for e in &v.parsed {
                            println!(
                                "#{:<4} {:<8} {:<48} {}",
                                e.seq,
                                e.status,
                                e.token(),
                                &e.hash[..12]
                            );
                        }
                        println!(
                            "{} entries, {}",
                            v.entries,
                            if v.ok { "chain intact" } else { "CHAIN BROKEN" }
                        );
                        for p in &v.problems {
                            println!("  line {} {}: {}", p.line, p.code, p.detail);
                        }
                    }
                    i32::from(!v.ok)
                }
                Err(e) => {
                    eprintln!("perci ledger show: {e}");
                    1
                }
            }
        }
        "verify" => {
            let runs: Vec<String> = if rest.len() > 1 && rest[1] != "--all" {
                rest[1..].iter().map(|s| s.to_string()).collect()
            } else {
                list_runs()
            };
            let mut ok = true;
            let mut rows = Vec::new();
            for r in &runs {
                match verify_run(r) {
                    Ok(v) => {
                        ok &= v.ok;
                        if !json_out {
                            println!(
                                "{} {r}: {} entries",
                                if v.ok { "OK  " } else { "FAIL" },
                                v.entries
                            );
                            for p in &v.problems {
                                println!("     line {} {}: {}", p.line, p.code, p.detail);
                            }
                        }
                        rows.push(serde_json::to_value(&v).unwrap_or(Value::Null));
                    }
                    Err(e) => {
                        ok = false;
                        rows.push(json!({"run_id": r, "error": e.to_string()}));
                    }
                }
            }
            if json_out {
                println!("{}", json!({"ok": ok, "runs": rows}));
            }
            i32::from(!ok)
        }
        "learn" | "refresh" => match super::temporal::learn_and_save() {
            Ok((path, m)) => {
                if json_out {
                    println!(
                        "{}",
                        json!({"model": path.display().to_string(), "runs": m.runs.len(), "skipped": m.skipped, "transitions": m.transitions})
                    );
                } else {
                    println!(
                        "learned {} transition(s) from {} verified run(s) → {}",
                        m.transitions,
                        m.runs.len(),
                        path.display()
                    );
                    for s in &m.skipped {
                        println!("  skipped {s}");
                    }
                }
                0
            }
            Err(e) => {
                eprintln!("perci ledger learn: {e}");
                1
            }
        },
        _ => {
            eprintln!(
                "usage: perci ledger list | show RUN | verify [RUN…|--all] | learn  [--json]"
            );
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_verdict(tool: &str, status: Status) -> Verdict {
        Verdict::seal(tool, &json!({"x": 1}), Outcome::new(status, json!({})), 1)
    }

    fn temp_run(tag: &str) -> RunLedger {
        let dir = std::env::temp_dir().join(format!("perci-ledger-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        RunLedger {
            run_id: tag.into(),
            path: dir.join(format!("{tag}.jsonl")),
            entries: Vec::new(),
            broken: None,
        }
    }

    #[test]
    fn chain_detects_edit_reorder_drop_and_move() {
        let mut l = temp_run("chain");
        for i in 0..4 {
            let v = fake_verdict(
                "perci.exact",
                if i == 2 { Status::Fail } else { Status::Pass },
            );
            l.append("perci.exact", None, "", v.status.as_str(), &v, None)
                .unwrap();
        }
        let text = std::fs::read_to_string(&l.path).unwrap();
        assert!(verify_lines("chain", &text).ok);
        let lines: Vec<&str> = text.lines().collect();
        // Edit: flip a status.
        let edited = text.replacen("\"status\":\"fail\"", "\"status\":\"pass\"", 1);
        assert!(verify_lines("chain", &edited)
            .problems
            .iter()
            .any(|p| p.code == "HASH_MISMATCH"));
        // Reorder.
        let reordered = [lines[0], lines[2], lines[1], lines[3]].join("\n");
        let v = verify_lines("chain", &reordered);
        assert!(v
            .problems
            .iter()
            .any(|p| p.code == "SEQ_GAP" || p.code == "BROKEN_CHAIN"));
        // Drop from the middle.
        let dropped = [lines[0], lines[1], lines[3]].join("\n");
        assert!(!verify_lines("chain", &dropped).ok);
        // Moved to another run.
        assert!(verify_lines("other", &text)
            .problems
            .iter()
            .any(|p| p.code == "RUN_MISMATCH" || p.code == "BROKEN_CHAIN"));
        // Truncation keeps a valid chain but changes the head.
        let full = verify_lines("chain", &text);
        let cut = verify_lines("chain", &lines[..3].join("\n"));
        assert!(cut.ok && cut.head != full.head);
        let _ = std::fs::remove_dir_all(l.path.parent().unwrap());
    }

    #[test]
    fn steps_and_classes() {
        assert_eq!(
            action_step(&json!({"kind": "shell", "command": "cargo test --release && echo ok"})),
            "shell:cargo test"
        );
        assert_eq!(
            action_step(
                &json!({"kind": "shell", "argv": ["sudo", "RUST_LOG=1", "git", "-C", "x", "push", "origin"]})
            ),
            "shell:git push"
        );
        assert_eq!(
            action_step(&json!({"kind": "shell", "command": "python3 -m pytest -q"})),
            "shell:python -m pytest"
        );
        assert_eq!(
            action_step(&json!({"kind": "shell", "command": "rm -rf build"})),
            "shell:rm"
        );
        assert_eq!(
            action_step(&json!({"kind": "write", "paths": ["a"]})),
            "write"
        );
        let policy: Value = serde_json::from_str(super::super::policy::EMBEDDED_POLICY).unwrap();
        assert!(classes_of(&policy, "shell:cargo test").contains(&"tests".to_owned()));
        assert!(classes_of(&policy, "shell:git push").contains(&"deploy".to_owned()));
        assert!(classes_of(&policy, "tests").contains(&"tests".to_owned()));
        assert!(classes_of(&policy, "shell:cargo testx").is_empty());
    }

    #[test]
    fn deploy_requires_tests_after_last_change() {
        let policy: Value = serde_json::from_str(super::super::policy::EMBEDDED_POLICY).unwrap();
        let mut l = temp_run("seq");
        fn add(l: &mut RunLedger, tool: &str, phase: Option<&str>, step: &str, status: &str) {
            let v = fake_verdict(tool, Status::Pass);
            l.append(tool, phase, step, status, &v, None).unwrap();
        }
        add(
            &mut l,
            "perci.agent.gate_step",
            Some("post"),
            "shell:cargo test",
            "pass",
        );
        add(
            &mut l,
            "perci.agent.gate_step",
            Some("pre"),
            "write",
            "pass",
        );
        let f = check_sequence(&policy, "shell:git push", &l.entries);
        let deploy = f
            .iter()
            .find(|f| f.rule == "seq.deploy_requires_tests")
            .unwrap();
        assert!(
            !deploy.satisfied,
            "tests ran before the last write: {}",
            deploy.detail
        );
        assert!(
            deploy.detail.contains("after the last code_change"),
            "{}",
            deploy.detail
        );
        add(
            &mut l,
            "perci.agent.gate_step",
            Some("pre"),
            "shell:cargo test",
            "pass",
        );
        let f = check_sequence(&policy, "shell:git push", &l.entries);
        assert!(!f[0].satisfied, "a pre-gate is not a test run");
        add(
            &mut l,
            "perci.agent.gate_step",
            Some("post"),
            "shell:cargo test",
            "pass",
        );
        let f = check_sequence(&policy, "shell:git push", &l.entries);
        assert!(f[0].satisfied, "{}", f[0].detail);
        assert_eq!(f[0].evidence[0].reference, format!("ledger:seq#{}", 4));
        let _ = std::fs::remove_dir_all(l.path.parent().unwrap());
    }
}
