//! Agent-level tools: bounded verification loops a harness delegates to Perci.
//!
//! * `perci.agent.check_claims`: split an answer into sentences, check each
//!   checkable claim (math, relations against supplied premises), abstain on
//!   the rest, and report coverage.
//! * `perci.agent.gate_step`: a before/after hook around one harness tool
//!   call (policy before; exit code, math and hygiene after). ≤ 5 ms.
//! * `perci.agent.verify_patch`: apply a diff in a scratch git worktree,
//!   policy-check the touched paths, run an allowlisted verifier with a
//!   time budget, and report pass/fail with hashed logs. The caller's tree
//!   is never touched (checked before and after). Off by default.

use super::{arg_str, opt_str, opt_str_list, try_arg, with_summary, Ctx, Tool};
use crate::verdict::{Confidence, Evidence, Outcome, Reason, Status};
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ---- check_claims ------------------------------------------------------------

pub struct CheckClaims;

const MAX_SENTENCES: usize = 32;

const PROVENANCE_CUES: &[&str] = &[
    "according to",
    "the readme says",
    "the docs say",
    "you told me",
    "you said",
    "your notes",
    "as noted in",
    "as documented in",
    "per the",
    "the log shows",
];

/// Split into sentences on `. ! ?` followed by whitespace, and on newlines.
pub fn sentences(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (i, c) in text.char_indices() {
        let end_here = match c {
            '\n' => true,
            '.' | '!' | '?' => bytes.get(i + 1).is_none_or_ws(),
            _ => false,
        };
        if end_here {
            let s = &text[start..=i];
            if !s.trim().is_empty() {
                out.push((start + (s.len() - s.trim_start().len()), s.trim()));
            }
            start = i + 1;
        }
    }
    if start < text.len() && !text[start..].trim().is_empty() {
        let s = &text[start..];
        out.push((start + (s.len() - s.trim_start().len()), s.trim()));
    }
    out
}

trait NoneOrWs {
    fn is_none_or_ws(&self) -> bool;
}
impl NoneOrWs for Option<&u8> {
    fn is_none_or_ws(&self) -> bool {
        self.map_or(true, |b| b.is_ascii_whitespace())
    }
}

/// Typed relation edges in a sentence. Plain "X is Y" edges are left out:
/// in ordinary prose they are descriptions, not dependency claims.
fn relational_edges(s: &str) -> Vec<crate::relation_field::RelationEdge> {
    crate::relation_field::parse_program(s)
        .edges
        .into_iter()
        .filter(|e| e.relation != crate::relation_field::RelationKind::IsA)
        .collect()
}

impl Tool for CheckClaims {
    fn name(&self) -> &'static str {
        "perci.agent.check_claims"
    }
    fn title(&self) -> &'static str {
        "Check every claim in an answer"
    }
    fn description(&self) -> &'static str {
        "Split an answer into sentences and check each one Perci can: arithmetic (exact recompute) and relations like \"X depends on Y\" (against supplied premises). Sentences citing sources are flagged as needing evidence; the rest abstain. fail if any claim is wrong; pass if all checkable claims pass and coverage ≥ min_coverage; else abstain. ≤ 32 sentences."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["answer"],
            "properties": {
                "answer": {"type": "string"},
                "premises": {"type": "array", "items": {"type": "string"}},
                "min_coverage": {"type": "number", "minimum": 0, "maximum": 1, "default": 0}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, _ctx: &Ctx, args: &Value) -> Outcome {
        let answer = try_arg!(arg_str(args, "answer"));
        let premises = try_arg!(opt_str_list(args, "premises"));
        let min_cov_pm = (args
            .get("min_coverage")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
            * 1000.0)
            .round() as u64;
        let premise_edges = crate::relation_field::parse_program(&premises.join(". ")).edges;

        let mut rows = Vec::new();
        let mut reasons = Vec::new();
        let (mut passed, mut failed, mut total) = (0u64, 0u64, 0u64);
        for (offset, s) in sentences(answer).into_iter().take(MAX_SENTENCES) {
            total += 1;
            let math = super::check_math::extract(s);
            let lower = s.to_lowercase();
            let (kind, verdict, note) = if !math.is_empty()
                && math.iter().any(|c| c.status != Status::Abstain)
            {
                let bad: Vec<_> = math.iter().filter(|c| c.status == Status::Fail).collect();
                if bad.is_empty() {
                    (
                        "math",
                        Status::Pass,
                        format!("{} claim(s) exact", math.len()),
                    )
                } else {
                    for c in &bad {
                        reasons.push(
                            Reason::new("MATH_MISMATCH", format!("{}: {}", c.lhs, c.note))
                                .at(offset + c.span[0], offset + c.span[1]),
                        );
                    }
                    (
                        "math",
                        Status::Fail,
                        bad.iter()
                            .map(|c| c.note.clone())
                            .collect::<Vec<_>>()
                            .join("; "),
                    )
                }
            } else if !premise_edges.is_empty() && !relational_edges(s).is_empty() {
                let edges = relational_edges(s);
                let mut st = Status::Pass;
                let mut notes = Vec::new();
                for e in &edges {
                    let same = |p: &crate::relation_field::RelationEdge| {
                        p.relation == e.relation
                            && crate::relation_field::same_node(&p.subject, &e.subject)
                            && crate::relation_field::same_node(&p.object, &e.object)
                    };
                    if premise_edges
                        .iter()
                        .any(|p| same(p) && p.negated == e.negated)
                    {
                        notes.push(format!("\"{}\" is a supplied premise", e.proposition()));
                    } else if premise_edges
                        .iter()
                        .any(|p| same(p) && p.negated != e.negated)
                    {
                        st = Status::Fail;
                        notes.push(format!(
                            "\"{}\" contradicts a supplied premise",
                            e.proposition()
                        ));
                        reasons.push(
                            Reason::new("CONTRADICTS_PREMISE", e.proposition())
                                .at(offset, offset + s.len()),
                        );
                    } else {
                        st = st.worst(Status::Abstain);
                        notes.push(format!("\"{}\" is not among the premises", e.proposition()));
                    }
                }
                ("relation", st, notes.join("; "))
            } else if PROVENANCE_CUES.iter().any(|c| lower.contains(c)) {
                (
                    "provenance",
                    Status::Abstain,
                    "cites a source: verify with perci.verify evidence (file/line + sha256)"
                        .to_owned(),
                )
            } else {
                (
                    "other",
                    Status::Abstain,
                    "not a claim Perci can check".to_owned(),
                )
            };
            match verdict {
                Status::Pass => passed += 1,
                Status::Fail => failed += 1,
                _ => {}
            }
            rows.push(json!({"span": [offset, offset + s.len()], "text": crate::util::text::truncate(s, 200), "kind": kind, "verdict": verdict.as_str(), "note": note}));
        }
        let checked = passed + failed;
        let coverage_pm = (checked * 1000).checked_div(total).unwrap_or(0);
        let status = if failed > 0 {
            Status::Fail
        } else if passed > 0 && coverage_pm >= min_cov_pm {
            Status::Pass
        } else {
            if passed > 0 {
                reasons.push(Reason::new(
                    "LOW_COVERAGE",
                    format!("checked {checked} of {total} sentences, below min_coverage"),
                ));
            } else {
                reasons.push(Reason::new(
                    "NOTHING_CHECKABLE",
                    "no sentence had a claim Perci can check",
                ));
            }
            Status::Abstain
        };
        let mut o = Outcome::new(
            status,
            with_summary(
                json!({
                    "claims": rows,
                    "sentences": total,
                    "checked": checked,
                    "failed": failed,
                    "coverage_permille": coverage_pm,
                }),
                format!("{checked}/{total} sentence(s) checkable, {failed} wrong"),
            ),
        )
        .confidence(if premise_edges.is_empty() {
            Confidence::Exact
        } else {
            Confidence::Bounded
        })
        .evidence(
            Evidence::new("computation", "perci.agent.check_claims")
                .hashed(crate::doctor::sha256_hex(answer.as_bytes())),
        );
        for r in reasons {
            o = o.reason(r);
        }
        o
    }
}

// ---- gate_step ---------------------------------------------------------------

pub struct GateStep;

impl Tool for GateStep {
    fn name(&self) -> &'static str {
        "perci.agent.gate_step"
    }
    fn title(&self) -> &'static str {
        "Gate one harness step"
    }
    fn description(&self) -> &'static str {
        "Hook around one tool call in a harness. phase=pre: policy-check the action before it runs, including sequence rules against this run's ledger such as deploy-after-tests (pass = run it, abstain = ask the user, fail = don't). phase=post: check the result (exit code vs expected, math claims in the output, empty or leaked output). ≤ 5 ms."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["phase", "action"],
            "properties": {
                "phase": {"enum": ["pre", "post"]},
                "action": {"type": "object", "description": "Same shape as perci.policy_check action."},
                "workspace": {"type": "string"},
                "request_text": {"type": "string"},
                "output": {"type": "string", "description": "post: the tool's output or the model's summary of it."},
                "exit_code": {"type": "integer"},
                "expect_exit_code": {"type": "integer", "default": 0},
                "run_id": {"type": "string", "description": "Ledger to check sequence rules against (default: the current run)."}
            },
            "additionalProperties": false
        })
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let phase = try_arg!(arg_str(args, "phase"));
        let Some(action) = args.get("action").filter(|a| a.is_object()) else {
            return Outcome::error("BAD_ARGS", "missing action object");
        };
        match phase {
            "pre" => {
                let mut pargs = json!({"action": action});
                for k in ["workspace", "request_text"] {
                    if let Some(v) = args.get(k) {
                        pargs[k] = v.clone();
                    }
                }
                let o = super::policy::PolicyCheck.call(ctx, &pargs);
                if o.status == Status::Error {
                    return o;
                }
                let mut result = o.result.clone();
                result["phase"] = json!("pre");
                result["proceed"] = json!(o.status == Status::Pass);
                let o = Outcome { result, ..o };
                // Sequence rules against this run's ledger (cites the entries used).
                let run_id = try_arg!(opt_str(args, "run_id"));
                let step = super::ledger::action_step(action);
                super::ledger::apply_sequence(ctx, run_id, &step, o)
            }
            "post" => {
                let output = try_arg!(opt_str(args, "output")).unwrap_or("");
                let expect = args
                    .get("expect_exit_code")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                let code = args.get("exit_code").and_then(Value::as_i64);
                let mut status = Status::Pass;
                let mut reasons = Vec::new();
                if let Some(c) = code {
                    if c != expect {
                        status = Status::Fail;
                        reasons.push(Reason::new(
                            "EXIT_CODE",
                            format!("exit code {c}, expected {expect}"),
                        ));
                    }
                }
                let math = super::check_math::check(output);
                if math.status == Status::Fail {
                    status = Status::Fail;
                    reasons.extend(math.reasons.clone());
                }
                if !output.is_empty() {
                    let h = super::verify::hygiene(output);
                    if h.status == Status::Fail {
                        status = Status::Fail;
                        reasons.extend(h.reasons);
                    }
                } else if code.is_none() {
                    status = Status::Abstain;
                    reasons.push(Reason::new(
                        "NOTHING_TO_CHECK",
                        "post phase needs exit_code or output",
                    ));
                }
                let mut o = Outcome::new(
                    status,
                    with_summary(
                        json!({"phase": "post", "exit_code": code, "math": math.result}),
                        match status {
                            Status::Pass => "step result checks out".to_owned(),
                            Status::Fail => "step result has problems".to_owned(),
                            _ => "nothing to check".to_owned(),
                        },
                    ),
                )
                .evidence(
                    Evidence::new("step_log", "output")
                        .hashed(crate::doctor::sha256_hex(output.as_bytes())),
                );
                for r in reasons {
                    o = o.reason(r);
                }
                o
            }
            other => Outcome::error(
                "BAD_ARGS",
                format!("phase must be pre or post, got {other:?}"),
            ),
        }
    }
}

// ---- verify_patch ------------------------------------------------------------

pub struct VerifyPatch;

const VERIFIER_PROGRAMS: &[&str] = &["cargo", "python", "python3", "py", "pytest", "bug2eval"];
const MAX_TIMEOUT_S: u64 = 600;
const LOG_EXCERPT: usize = 2000;

struct StepLog {
    argv: Vec<String>,
    code: Option<i32>,
    output: String,
    timed_out: bool,
}

fn run_step(
    argv: &[String],
    cwd: &Path,
    timeout: Duration,
    envs: &[(&str, String)],
) -> std::io::Result<StepLog> {
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn()?;
    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let t_out = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_end(&mut s);
        }
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = Vec::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_end(&mut s);
        }
        s
    });
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(st) = child.try_wait()? {
            break Some(st);
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut output = t_out.join().unwrap_or_default();
    output.extend(t_err.join().unwrap_or_default());
    Ok(StepLog {
        argv: argv.to_vec(),
        code: status.and_then(|s| s.code()),
        output: String::from_utf8_lossy(&output).into_owned(),
        timed_out,
    })
}

fn git(repo: &Path, args: &[&str]) -> std::io::Result<StepLog> {
    let argv: Vec<String> = std::iter::once("git".to_owned())
        .chain(args.iter().map(|s| (*s).to_owned()))
        .collect();
    run_step(&argv, repo, Duration::from_secs(120), &[])
}

fn tail(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        s.to_owned()
    } else {
        format!("…{}", chars[chars.len() - n..].iter().collect::<String>())
    }
}

/// Paths a unified diff touches (`+++ b/x`, `--- a/x`, renames).
pub fn diff_paths(diff: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in diff.lines() {
        let p = line
            .strip_prefix("+++ ")
            .or_else(|| line.strip_prefix("--- "))
            .or_else(|| line.strip_prefix("rename to "))
            .or_else(|| line.strip_prefix("rename from "));
        if let Some(p) = p {
            let p = p.split('\t').next().unwrap_or(p).trim();
            if p == "/dev/null" {
                continue;
            }
            let p = p
                .strip_prefix("a/")
                .or_else(|| p.strip_prefix("b/"))
                .unwrap_or(p);
            if !out.iter().any(|x| x == p) {
                out.push(p.to_owned());
            }
        }
    }
    out
}

fn changed_lines(diff: &str) -> u64 {
    diff.lines()
        .filter(|l| {
            (l.starts_with('+') && !l.starts_with("+++"))
                || (l.starts_with('-') && !l.starts_with("---"))
        })
        .count() as u64
}

struct ScratchGuard {
    repo: PathBuf,
    dir: PathBuf,
}
impl Drop for ScratchGuard {
    fn drop(&mut self) {
        let d = self.dir.display().to_string();
        let _ = git(&self.repo, &["worktree", "remove", "--force", &d]);
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = git(&self.repo, &["worktree", "prune"]);
    }
}

impl Tool for VerifyPatch {
    fn name(&self) -> &'static str {
        "perci.agent.verify_patch"
    }
    fn title(&self) -> &'static str {
        "Verify a patch in a scratch worktree"
    }
    fn description(&self) -> &'static str {
        "Prove a fix before claiming it: applies a unified diff in a throwaway git worktree at base_ref, policy-checks every touched path, runs an allowlisted verifier (cargo/python/pytest/bug2eval) with a time budget, and returns pass (verifier exit 0), fail (with the failing output tail) or abstain (diff doesn't apply, policy needs approval, budget exhausted). Your working tree is never modified. Off by default."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["repo", "diff", "verify"],
            "properties": {
                "repo": {"type": "string", "description": "Path to the git repository."},
                "base_ref": {"type": "string", "default": "HEAD"},
                "diff": {"type": "string", "description": "Unified diff (git diff format)."},
                "verify": {"type": "object", "required": ["argv"], "properties": {
                    "argv": {"type": "array", "items": {"type": "string"}},
                    "timeout_s": {"type": "integer", "minimum": 1, "maximum": MAX_TIMEOUT_S}
                }}
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
    fn default_enabled(&self) -> bool {
        false
    }
    fn call(&self, ctx: &Ctx, args: &Value) -> Outcome {
        let repo = PathBuf::from(try_arg!(arg_str(args, "repo")));
        let base = try_arg!(opt_str(args, "base_ref"))
            .unwrap_or("HEAD")
            .to_owned();
        let diff = match args.get("diff").and_then(Value::as_str) {
            Some(d) if !d.trim().is_empty() && d.len() <= 1024 * 1024 => d.to_owned(),
            _ => return Outcome::error("BAD_ARGS", "diff must be a non-empty string up to 1 MiB"),
        };
        let argv: Vec<String> = args["verify"]["argv"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let timeout_s = args["verify"]["timeout_s"]
            .as_u64()
            .unwrap_or(MAX_TIMEOUT_S)
            .clamp(1, MAX_TIMEOUT_S);
        if std::env::var("PERCI_AGENT").ok().as_deref() == Some("0")
            || repo.join(".perci/agent.lock").exists()
        {
            return Outcome::abstain(
                "AGENT_DISABLED",
                "agent kill switch is on (PERCI_AGENT=0 or .perci/agent.lock)",
            );
        }
        let Some(program) = argv.first().map(|p| {
            p.rsplit(['/', '\\'])
                .next()
                .unwrap_or(p)
                .trim_end_matches(".exe")
                .to_lowercase()
        }) else {
            return Outcome::error("BAD_ARGS", "verify.argv is empty");
        };
        if !VERIFIER_PROGRAMS.contains(&program.as_str()) {
            return Outcome::fail(json!({"stage": "verifier"})).reason(Reason::new(
                "VERIFIER_NOT_ALLOWED",
                format!(
                    "{program} is not an allowed verifier ({})",
                    VERIFIER_PROGRAMS.join(", ")
                ),
            ));
        }
        if base.starts_with('-') {
            return Outcome::error("BAD_ARGS", "base_ref must not start with '-'");
        }
        let repo_root = match git(&repo, &["rev-parse", "--show-toplevel"]) {
            Ok(l) if l.code == Some(0) => PathBuf::from(l.output.trim()),
            _ => {
                return Outcome::error(
                    "NOT_A_REPO",
                    format!("{} is not a git repository", repo.display()),
                )
            }
        };

        let mut steps: Vec<StepLog> = Vec::new();
        let mut evidence = Vec::new();

        // 1. policy on every touched path
        let paths = diff_paths(&diff);
        let action = json!({"kind": "write", "paths": paths, "diff_stats": {"changed_lines": changed_lines(&diff)}});
        let policy = super::policy::PolicyCheck.call(
            ctx,
            &json!({"action": action, "workspace": repo_root.display().to_string()}),
        );
        if policy.status != Status::Pass {
            let mut o = Outcome::new(
                policy.status,
                with_summary(
                    json!({"stage": "policy", "paths": paths, "policy": policy.result}),
                    "patch touches paths that need approval or are protected",
                ),
            );
            o.reasons = policy.reasons;
            o.evidence = policy.evidence;
            return o.nondeterministic();
        }

        // 2. caller tree snapshot (must be identical afterwards)
        let before = git(&repo_root, &["status", "--porcelain=v1", "-uall"])
            .map(|l| l.output)
            .unwrap_or_default();

        // 3. scratch worktree
        let id = &crate::verdict::input_hash("perci.agent.verify_patch", args)[..12];
        let scratch_root = crate::paths::data_file("agent/scratch");
        let _ = std::fs::create_dir_all(&scratch_root);
        let dir = scratch_root.join(format!("{id}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let dir_s = dir.display().to_string();
        let add = match git(
            &repo_root,
            &[
                "-c",
                "core.hooksPath=/dev/null",
                "worktree",
                "add",
                "--detach",
                &dir_s,
                &base,
            ],
        ) {
            Ok(l) => l,
            Err(e) => return Outcome::error("WORKTREE_FAILED", e.to_string()),
        };
        let guard = ScratchGuard {
            repo: repo_root.clone(),
            dir: dir.clone(),
        };
        if add.code != Some(0) {
            return Outcome::abstain("WORKTREE_FAILED", tail(&add.output, 400)).nondeterministic();
        }
        steps.push(add);

        // 4. apply
        let patch_file = scratch_root.join(format!("{id}-{}.diff", std::process::id()));
        if let Err(e) = std::fs::write(&patch_file, &diff) {
            return Outcome::error("WRITE_FAILED", e.to_string());
        }
        let pf = patch_file.display().to_string();
        let check = git(&dir, &["apply", "--check", "--whitespace=nowarn", &pf]);
        let applied = match check {
            Ok(l) if l.code == Some(0) => {
                steps.push(l);
                git(&dir, &["apply", "--whitespace=nowarn", &pf]).ok()
            }
            Ok(l) => {
                let detail = tail(&l.output, 600);
                steps.push(l);
                let _ = std::fs::remove_file(&patch_file);
                drop(guard);
                return Outcome::abstain("DIFF_DOES_NOT_APPLY", detail).nondeterministic();
            }
            Err(e) => return Outcome::error("GIT_FAILED", e.to_string()),
        };
        let _ = std::fs::remove_file(&patch_file);
        if let Some(l) = applied {
            steps.push(l);
        }

        // 5. verifier with budget, offline
        let cache = crate::paths::data_file("agent/target-cache");
        let envs = [
            ("CARGO_NET_OFFLINE", "true".to_owned()),
            (
                "CARGO_TARGET_DIR",
                std::env::var("PERCI_AGENT_TARGET_DIR")
                    .unwrap_or_else(|_| cache.display().to_string()),
            ),
            ("PERCI_AGENT_SCRATCH", "1".to_owned()),
        ];
        let run = run_step(&argv, &dir, Duration::from_secs(timeout_s), &envs);
        let run = match run {
            Ok(r) => r,
            Err(e) => {
                drop(guard);
                return Outcome::abstain("VERIFIER_MISSING", format!("{}: {e}", argv[0]))
                    .nondeterministic();
            }
        };
        let verdict_status = if run.timed_out {
            Status::Abstain
        } else if run.code == Some(0) {
            Status::Pass
        } else {
            Status::Fail
        };
        let excerpt = tail(&run.output, LOG_EXCERPT);
        steps.push(run);
        drop(guard);

        // 6. caller tree unchanged?
        let after = git(&repo_root, &["status", "--porcelain=v1", "-uall"])
            .map(|l| l.output)
            .unwrap_or_default();
        let untouched = before == after;

        for (i, s) in steps.iter().enumerate() {
            evidence.push(
                Evidence::new(
                    "step_log",
                    format!(
                        "step{}: {} → {}",
                        i + 1,
                        s.argv.join(" "),
                        s.code
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "killed".into())
                    ),
                )
                .hashed(crate::doctor::sha256_hex(s.output.as_bytes())),
            );
        }
        let mut o = Outcome::new(
            if untouched {
                verdict_status
            } else {
                Status::Error
            },
            with_summary(
                json!({
                    "stage": "verify",
                    "verifier": argv,
                    "exit_code": steps.last().and_then(|s| s.code),
                    "timed_out": steps.last().is_some_and(|s| s.timed_out),
                    "output_tail": excerpt,
                    "paths": paths,
                    "caller_tree_untouched": untouched,
                    "base_ref": base,
                }),
                match verdict_status {
                    Status::Pass => "verifier passed on the patched tree".to_owned(),
                    Status::Fail => "verifier failed on the patched tree".to_owned(),
                    _ => format!("verifier exceeded the {timeout_s} s budget"),
                },
            ),
        )
        .nondeterministic();
        if verdict_status == Status::Fail {
            o = o.reason(Reason::new(
                "VERIFIER_FAILED",
                format!(
                    "{} exited {}",
                    argv.join(" "),
                    steps
                        .last()
                        .and_then(|s| s.code)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "?".into())
                ),
            ));
        }
        if verdict_status == Status::Abstain {
            o = o.reason(Reason::new(
                "BUDGET_EXHAUSTED",
                format!("killed after {timeout_s} s"),
            ));
        }
        if !untouched {
            o = o.reason(Reason::new(
                "CALLER_TREE_CHANGED",
                "git status of the caller's tree changed during verification",
            ));
        }
        o.evidence = evidence;
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentence_split() {
        let s = sentences("17% of 240 is 41. Deploy depends on build!\nOk? v1.2 is out");
        let texts: Vec<&str> = s.iter().map(|x| x.1).collect();
        assert_eq!(
            texts,
            vec![
                "17% of 240 is 41.",
                "Deploy depends on build!",
                "Ok?",
                "v1.2 is out"
            ]
        );
    }

    #[test]
    fn check_claims_mixed_answer() {
        let ctx = Ctx::new();
        let v = super::super::run(
            &ctx,
            &CheckClaims,
            &json!({
                "answer": "The build takes 3 * 4 = 12 minutes. Deploy depends on build. According to the README, the port is 8080. Rust is fun.",
                "premises": ["Deploy depends on build", "Build depends on tests"]
            }),
        );
        assert_eq!(v.status, Status::Pass, "{:?}", v.result);
        assert_eq!(v.result["checked"], 2);
        assert_eq!(v.result["sentences"], 4);
        let kinds: Vec<&str> = v.result["claims"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, vec!["math", "relation", "provenance", "other"]);

        let bad = super::super::run(
            &ctx,
            &CheckClaims,
            &json!({"answer": "So 6 * 7 = 43. Fine.", "min_coverage": 0.5}),
        );
        assert_eq!(bad.status, Status::Fail);
        let low = super::super::run(
            &ctx,
            &CheckClaims,
            &json!({"answer": "6 * 7 = 42. A. B. C.", "min_coverage": 0.5}),
        );
        assert_eq!(low.status, Status::Abstain);
    }

    #[test]
    fn gate_step_pre_and_post() {
        let ctx = Ctx::new();
        let pre = super::super::run(
            &ctx,
            &GateStep,
            &json!({"phase": "pre", "action": {"kind": "shell", "command": "git push --force"}}),
        );
        assert_eq!(pre.status, Status::Fail);
        assert_eq!(pre.result["proceed"], false);
        let ok = super::super::run(
            &ctx,
            &GateStep,
            &json!({"phase": "pre", "action": {"kind": "shell", "command": "cargo test"}}),
        );
        assert_eq!(ok.status, Status::Pass);
        let post = super::super::run(
            &ctx,
            &GateStep,
            &json!({"phase": "post", "action": {"kind": "shell", "command": "cargo test"}, "exit_code": 101, "output": "test result: FAILED"}),
        );
        assert_eq!(post.status, Status::Fail);
        let post_ok = super::super::run(
            &ctx,
            &GateStep,
            &json!({"phase": "post", "action": {"kind": "shell", "command": "cargo test"}, "exit_code": 0, "output": "test result: ok. 5 passed"}),
        );
        assert_eq!(post_ok.status, Status::Pass);
    }

    #[test]
    fn diff_path_extraction() {
        let d = "diff --git a/src/x.rs b/src/x.rs\n--- a/src/x.rs\n+++ b/src/x.rs\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(diff_paths(d), vec!["src/x.rs"]);
        assert_eq!(changed_lines(d), 2);
    }

    #[test]
    fn verify_patch_end_to_end_in_a_temp_repo() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let py = ["python3", "python"].into_iter().find(|p| {
            Command::new(p)
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
        });
        let Some(py) = py else { return };
        let repo = std::env::temp_dir().join(format!("perci-vp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(
            repo.join("src/calc.py"),
            "def add(a, b):\n    return a - b\n",
        )
        .unwrap();
        std::fs::write(repo.join("tests_check.py"), "import sys\nsys.path.insert(0, 'src')\nfrom calc import add\nassert add(2, 3) == 5, add(2, 3)\nprint('ok')\n").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "-A"],
            vec![
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-qm",
                "init",
            ],
        ] {
            assert_eq!(git(&repo, &args).unwrap().code, Some(0));
        }
        let ctx = Ctx::new();
        let fix = "--- a/src/calc.py\n+++ b/src/calc.py\n@@ -1,2 +1,2 @@\n def add(a, b):\n-    return a - b\n+    return a + b\n";
        let wrong = "--- a/src/calc.py\n+++ b/src/calc.py\n@@ -1,2 +1,2 @@\n def add(a, b):\n-    return a - b\n+    return a * b\n";
        let call = |diff: &str| {
            super::super::run(
                &ctx,
                &VerifyPatch,
                &json!({"repo": repo.display().to_string(), "diff": diff, "verify": {"argv": [py, "tests_check.py"], "timeout_s": 60}}),
            )
        };
        let pass = call(fix);
        assert_eq!(
            pass.status,
            Status::Pass,
            "{:?} {:?}",
            pass.reasons,
            pass.result
        );
        assert_eq!(pass.result["caller_tree_untouched"], true);
        let fail = call(wrong);
        assert_eq!(fail.status, Status::Fail, "{:?}", fail.result);
        // Caller tree still has the bug and no stray files.
        assert!(std::fs::read_to_string(repo.join("src/calc.py"))
            .unwrap()
            .contains("a - b"));
        let protected = call("--- a/.git/config\n+++ b/.git/config\n@@ -1 +1 @@\n-a\n+b\n");
        assert_eq!(protected.status, Status::Fail);
        let not_allowed = super::super::run(
            &ctx,
            &VerifyPatch,
            &json!({"repo": repo.display().to_string(), "diff": fix, "verify": {"argv": ["bash", "-c", "true"]}}),
        );
        assert_eq!(not_allowed.status, Status::Fail);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
