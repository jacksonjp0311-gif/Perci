//! bug2eval integration: validate or run Perci's regression cases and return a
//! verdict with a receipt. This is the library entry point for the
//! `perci.eval` tool and for `perci eval`.
//!
//! bug2eval (github.com/jacksonjp0311-gif/bug2eval, Python, zero dependencies)
//! is called as a subprocess: `bug2eval validate --json CASE` proves the case's
//! verifier fails on the broken snapshot and passes on the fixed one;
//! `bug2eval run --json CASE --agent-cmd …` lets an agent repair the broken
//! snapshot and scores it with the same verifier.
//!
//! Safety: a case's verifier executes code (it builds and runs Perci). Only
//! cases inside the allowlisted folders are accepted: `PERCI_HOME/evals/b2e`
//! plus any folders in `PERCI_B2E_CASES` (path-list separated). `run` executes
//! an agent command and is refused unless the caller opts in explicitly.
//!
//! Command: `PERCI_BUG2EVAL` holds the command prefix (e.g. `py -3.12 -m
//! bug2eval`); default `bug2eval`.

use crate::doctor::{sha256_file, sha256_hex};
use serde::Serialize;
use serde_json::Value;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const SCHEMA: &str = "perci.eval.v1";

/// Options for one evaluation.
#[derive(Clone, Debug)]
pub struct EvalOptions {
    /// Wall-clock limit for the whole bug2eval call (both snapshots).
    pub timeout: Duration,
    /// Command prefix for bug2eval; `None` uses `PERCI_BUG2EVAL` or `bug2eval`.
    pub command: Option<Vec<String>>,
    /// Folders a case must live in; empty uses the defaults described above.
    pub allowed_dirs: Vec<PathBuf>,
    /// Required for [`run_case`] (it executes the agent command).
    pub allow_agent_run: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(1800),
            command: None,
            allowed_dirs: Vec::new(),
            allow_agent_run: false,
        }
    }
}

/// Hashes and identity that make a verdict reproducible and auditable.
#[derive(Clone, Debug, Serialize)]
pub struct EvalReceipt {
    /// SHA-256 of the `.b2e` file (or of `metadata.json` for a case folder).
    pub case_sha256: String,
    /// SHA-256 of bug2eval's JSON output with timing fields removed.
    pub output_sha256: String,
    pub bug2eval_version: String,
    pub argv: Vec<String>,
    pub perci_build_id: String,
    pub elapsed_ms: u64,
}

/// Outcome of [`validate_case`] / [`run_case`] (serialises as `perci.eval.v1`).
#[derive(Clone, Debug, Serialize)]
pub struct EvalVerdict {
    pub schema: &'static str,
    pub case_id: String,
    pub case_path: String,
    /// `"validate"` or `"run"`.
    pub mode: String,
    /// `"pass"`, `"fail"`, `"invalid"` (malformed case) or `"error"`.
    pub status: String,
    /// bug2eval's `valid` (validate) or `passed` (run) flag.
    pub valid: Option<bool>,
    pub before_exit: Option<i64>,
    pub after_exit: Option<i64>,
    pub verifier_intact: Option<bool>,
    pub reasons: Vec<String>,
    pub exit_code: Option<i32>,
    pub receipt: EvalReceipt,
    /// bug2eval's full JSON output (verifier stdout/stderr included).
    pub bug2eval: Value,
}

impl EvalVerdict {
    pub fn passed(&self) -> bool {
        self.status == "pass"
    }
}

/// Folder of Perci's packed cases: `PERCI_HOME/evals/b2e`.
pub fn default_case_dir() -> PathBuf {
    crate::paths::asset("evals/b2e")
}

/// `*.b2e` files in `dir`, sorted by name.
pub fn list_cases(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut cases: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "b2e"))
        .collect();
    cases.sort();
    Ok(cases)
}

/// Prove the case: verifier fails on before and passes on after.
pub fn validate_case(case: &Path, opts: &EvalOptions) -> EvalVerdict {
    evaluate(case, "validate", &[], opts)
}

/// Give `agent_cmd` the broken snapshot (placeholders `{workspace}`,
/// `{task_file}`, `{prompt_file}`), then score it with the case verifier.
pub fn run_case(case: &Path, agent_cmd: &str, opts: &EvalOptions) -> EvalVerdict {
    if !opts.allow_agent_run {
        return error_verdict(
            case,
            "run",
            "running an agent command requires allow_agent_run (perci eval run --allow-agent)",
        );
    }
    let extra = vec![
        "--agent-cmd".to_owned(),
        agent_cmd.to_owned(),
        "--no-save".to_owned(),
    ];
    evaluate(case, "run", &extra, opts)
}

/// Validate every case in `dir`.
pub fn validate_all(dir: &Path, opts: &EvalOptions) -> io::Result<Vec<EvalVerdict>> {
    Ok(list_cases(dir)?
        .iter()
        .map(|case| validate_case(case, opts))
        .collect())
}

fn command_prefix(opts: &EvalOptions) -> Vec<String> {
    if let Some(cmd) = opts.command.clone().filter(|c| !c.is_empty()) {
        return cmd;
    }
    match std::env::var("PERCI_BUG2EVAL") {
        Ok(v) if !v.trim().is_empty() => v.split_whitespace().map(str::to_owned).collect(),
        _ => vec!["bug2eval".to_owned()],
    }
}

fn allowed_dirs(opts: &EvalOptions) -> Vec<PathBuf> {
    if !opts.allowed_dirs.is_empty() {
        return opts.allowed_dirs.clone();
    }
    let mut dirs = vec![default_case_dir()];
    if let Some(extra) = std::env::var_os("PERCI_B2E_CASES") {
        dirs.extend(std::env::split_paths(&extra));
    }
    dirs
}

/// Canonical case path if it is inside an allowlisted folder.
pub fn check_allowed(case: &Path, opts: &EvalOptions) -> Result<PathBuf, String> {
    let real = std::fs::canonicalize(case).map_err(|e| format!("{}: {e}", case.display()))?;
    for dir in allowed_dirs(opts) {
        if let Ok(dir) = std::fs::canonicalize(&dir) {
            if real.starts_with(&dir) && real != dir {
                return Ok(real);
            }
        }
    }
    Err(format!(
        "{} is outside the allowed case folders (PERCI_HOME/evals/b2e, PERCI_B2E_CASES)",
        case.display()
    ))
}

fn evaluate(case: &Path, mode: &str, extra: &[String], opts: &EvalOptions) -> EvalVerdict {
    let real = match check_allowed(case, opts) {
        Ok(p) => p,
        Err(e) => return error_verdict(case, mode, &e),
    };
    let prefix = command_prefix(opts);
    let mut argv = prefix.clone();
    argv.push(mode.to_owned());
    argv.push("--json".to_owned());
    argv.extend(extra.iter().cloned());
    argv.push(real.to_string_lossy().into_owned());
    let started = Instant::now();
    let output = run_with_timeout(&argv, opts.timeout);
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let case_sha256 = case_hash(&real);
    let version = bug2eval_version(&prefix);
    let (code, stdout, stderr) = match output {
        Ok(o) => o,
        Err(e) => {
            let mut v = error_verdict(&real, mode, &format!("could not run bug2eval: {e}"));
            v.receipt.case_sha256 = case_sha256;
            v.receipt.argv = argv;
            v.receipt.elapsed_ms = elapsed_ms;
            return v;
        }
    };
    let json: Value = serde_json::from_str(stdout.trim()).unwrap_or_else(
        |_| serde_json::json!({"error": "bug2eval printed no JSON", "stderr": tail(&stderr, 2000)}),
    );
    let valid = json
        .get("valid")
        .or_else(|| json.get("passed"))
        .and_then(Value::as_bool);
    let status = match (code, valid) {
        (Some(0), Some(true)) => "pass",
        (_, Some(false)) | (Some(1), _) => "fail",
        (Some(2), _) => "invalid",
        _ => "error",
    };
    let exit_of = |side: &str| {
        json.get(side)
            .or_else(|| json.get(format!("{side}_verify")))
            .and_then(|s| s.get("exit_code"))
            .and_then(Value::as_i64)
    };
    let mut reasons: Vec<String> = json
        .get("reasons")
        .and_then(Value::as_array)
        .map(|r| {
            r.iter()
                .map(|x| {
                    x.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| x.to_string())
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(err) = json.get("error") {
        reasons.push(
            err.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| err.to_string()),
        );
    }
    if code.is_none() {
        reasons.push(format!("timed out after {} s", opts.timeout.as_secs()));
    }
    EvalVerdict {
        schema: SCHEMA,
        case_id: json
            .get("case_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| case_stem(&real)),
        case_path: real.to_string_lossy().into_owned(),
        mode: mode.to_owned(),
        status: status.to_owned(),
        valid,
        before_exit: exit_of("before"),
        after_exit: exit_of("after"),
        verifier_intact: json
            .get("verifier_integrity")
            .and_then(|v| v.get("intact"))
            .and_then(Value::as_bool),
        reasons,
        exit_code: code,
        receipt: EvalReceipt {
            case_sha256,
            output_sha256: sha256_hex(canonical_without_timing(&json).as_bytes()),
            bug2eval_version: version,
            argv,
            perci_build_id: crate::branding::build_id().to_owned(),
            elapsed_ms,
        },
        bug2eval: json,
    }
}

/// Spawn `argv`, collect stdout/stderr on threads, kill it at the deadline.
/// `Ok((None, …))` means it timed out.
fn run_with_timeout(
    argv: &[String],
    timeout: Duration,
) -> io::Result<(Option<i32>, String, String)> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut out = child.stdout.take().expect("piped stdout");
    let mut err = child.stderr.take().expect("piped stderr");
    let out_t = thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let err_t = thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + timeout;
    let code = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status.code().unwrap_or(3));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(50));
    };
    let stdout = out_t.join().unwrap_or_default();
    let stderr = err_t.join().unwrap_or_default();
    Ok((code, stdout, stderr))
}

fn bug2eval_version(prefix: &[String]) -> String {
    let mut argv = prefix.to_vec();
    argv.push("--version".to_owned());
    match run_with_timeout(&argv, Duration::from_secs(30)) {
        Ok((Some(0), out, err)) => {
            let text = if out.trim().is_empty() { err } else { out };
            text.split_whitespace()
                .last()
                .unwrap_or("unknown")
                .to_owned()
        }
        _ => "unknown".to_owned(),
    }
}

fn case_hash(path: &Path) -> String {
    if path.is_dir() {
        sha256_file(&path.join("metadata.json")).unwrap_or_default()
    } else {
        sha256_file(path).unwrap_or_default()
    }
}

fn case_stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Sorted-key JSON without `duration_seconds`/`elapsed`/timestamps, so the hash
/// only changes when an outcome or verifier output changes.
fn canonical_without_timing(value: &Value) -> String {
    fn strip(v: &Value) -> Value {
        match v {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut out = serde_json::Map::new();
                for k in keys {
                    let lower = k.to_ascii_lowercase();
                    if lower.contains("duration")
                        || lower.contains("elapsed")
                        || lower.ends_with("_at")
                        || lower == "timestamp"
                        || lower == "workspace"
                    {
                        continue;
                    }
                    out.insert(k.clone(), strip(&map[k]));
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(strip).collect()),
            other => other.clone(),
        }
    }
    serde_json::to_string(&strip(value)).unwrap_or_default()
}

fn tail(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    chars[chars.len().saturating_sub(n)..].iter().collect()
}

fn error_verdict(case: &Path, mode: &str, reason: &str) -> EvalVerdict {
    EvalVerdict {
        schema: SCHEMA,
        case_id: case_stem(case),
        case_path: case.to_string_lossy().into_owned(),
        mode: mode.to_owned(),
        status: "error".to_owned(),
        valid: None,
        before_exit: None,
        after_exit: None,
        verifier_intact: None,
        reasons: vec![reason.to_owned()],
        exit_code: None,
        receipt: EvalReceipt {
            case_sha256: String::new(),
            output_sha256: String::new(),
            bug2eval_version: String::new(),
            argv: Vec::new(),
            perci_build_id: crate::branding::build_id().to_owned(),
            elapsed_ms: 0,
        },
        bug2eval: Value::Null,
    }
}

/// `perci eval list | validate [CASE…] | run CASE --agent-cmd CMD --allow-agent` `[--json]`.
/// Exit code: 0 all pass, 1 any fail/invalid/error, 2 usage.
pub fn run_cli(args: &[String]) -> io::Result<i32> {
    let json = args.iter().any(|a| a == "--json");
    let allow_agent = args.iter().any(|a| a == "--allow-agent");
    let mut rest = Vec::new();
    let mut agent_cmd = None;
    let mut timeout = None;
    let mut iter = args
        .iter()
        .filter(|a| !matches!(a.as_str(), "--json" | "--allow-agent"));
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--agent-cmd" => agent_cmd = iter.next().cloned(),
            "--timeout" => timeout = iter.next().and_then(|v| v.parse::<u64>().ok()),
            other => rest.push(other.to_owned()),
        }
    }
    let opts = EvalOptions {
        timeout: Duration::from_secs(timeout.unwrap_or(1800)),
        allow_agent_run: allow_agent,
        ..EvalOptions::default()
    };
    let sub = rest.first().map(String::as_str).unwrap_or("list");
    let cases: Vec<PathBuf> = if rest.len() > 1 {
        rest[1..].iter().map(|c| resolve_case_arg(c)).collect()
    } else {
        list_cases(&default_case_dir()).unwrap_or_default()
    };
    let verdicts: Vec<EvalVerdict> = match sub {
        "list" => {
            for c in &cases {
                println!("{}", c.display());
            }
            return Ok(0);
        }
        "validate" => cases
            .iter()
            .map(|c| {
                if !json {
                    eprintln!("validating {} …", case_stem(c));
                }
                validate_case(c, &opts)
            })
            .collect(),
        "run" => {
            let (Some(cmd), Some(case)) = (agent_cmd, rest.get(1)) else {
                eprintln!("usage: perci eval run CASE --agent-cmd \"agent {{workspace}} {{task_file}}\" --allow-agent");
                return Ok(2);
            };
            vec![run_case(&resolve_case_arg(case), &cmd, &opts)]
        }
        _ => {
            eprintln!("usage: perci eval list | validate [CASE…] | run CASE --agent-cmd CMD --allow-agent [--json] [--timeout SECS]");
            return Ok(2);
        }
    };
    let passed = verdicts.iter().filter(|v| v.passed()).count();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "perci.eval.summary.v1",
                "passed": passed,
                "total": verdicts.len(),
                "verdicts": verdicts,
            }))
            .map_err(io::Error::other)?
        );
    } else {
        for v in &verdicts {
            println!(
                "{:<16} {:<8} before={:<4} after={:<4} {:>6} ms  {}",
                v.case_id,
                v.status,
                v.before_exit.map_or("-".into(), |c| c.to_string()),
                v.after_exit.map_or("-".into(), |c| c.to_string()),
                v.receipt.elapsed_ms,
                v.reasons.join("; ")
            );
        }
        println!("{passed}/{} passed", verdicts.len());
    }
    Ok(i32::from(passed != verdicts.len()))
}

/// `B2E-PERCI-001` → `PERCI_HOME/evals/b2e/B2E-PERCI-001.b2e`; paths pass through.
fn resolve_case_arg(arg: &str) -> PathBuf {
    let p = PathBuf::from(arg);
    if p.exists() {
        return p;
    }
    default_case_dir().join(format!("{arg}.b2e"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_outside_the_allowlist_are_refused_without_running() {
        let dir = std::env::temp_dir().join(format!("perci-b2e-allow-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let case = dir.join("X.b2e");
        std::fs::write(&case, b"not a zip").unwrap();
        let opts = EvalOptions {
            allowed_dirs: vec![dir.join("elsewhere")],
            command: Some(vec!["definitely-not-a-real-binary".into()]),
            ..EvalOptions::default()
        };
        let v = validate_case(&case, &opts);
        assert_eq!(v.status, "error");
        assert!(
            v.reasons[0].contains("outside the allowed"),
            "{:?}",
            v.reasons
        );
        assert!(v.receipt.argv.is_empty(), "nothing may be executed");
    }

    #[test]
    fn agent_run_requires_opt_in() {
        let v = run_case(Path::new("x.b2e"), "true", &EvalOptions::default());
        assert_eq!(v.status, "error");
        assert!(v.reasons[0].contains("allow_agent_run"));
    }

    #[test]
    fn missing_bug2eval_is_an_error_not_a_pass() {
        let dir = std::env::temp_dir().join(format!("perci-b2e-missing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let case = dir.join("Y.b2e");
        std::fs::write(&case, b"zip").unwrap();
        let opts = EvalOptions {
            allowed_dirs: vec![dir.clone()],
            command: Some(vec!["definitely-not-a-real-binary".into()]),
            ..EvalOptions::default()
        };
        let v = validate_case(&case, &opts);
        assert_eq!(v.status, "error");
        assert!(!v.passed());
    }

    #[test]
    fn output_hash_ignores_timing() {
        let a =
            serde_json::json!({"valid": true, "before": {"exit_code": 1, "duration_seconds": 3.1}});
        let b =
            serde_json::json!({"before": {"duration_seconds": 9.9, "exit_code": 1}, "valid": true});
        assert_eq!(canonical_without_timing(&a), canonical_without_timing(&b));
    }

    /// Validates a real packed case when bug2eval is installed; skipped
    /// otherwise unless PERCI_REQUIRE_BUG2EVAL=1. Slow (builds Perci twice),
    /// so it only runs when PERCI_B2E_SLOW=1.
    #[test]
    fn packed_case_validates_with_bug2eval() {
        if std::env::var("PERCI_B2E_SLOW").as_deref() != Ok("1") {
            eprintln!("skipping: set PERCI_B2E_SLOW=1 to validate a packed case");
            return;
        }
        let case = default_case_dir().join("B2E-PERCI-001.b2e");
        let v = validate_case(&case, &EvalOptions::default());
        if v.status == "error" && std::env::var("PERCI_REQUIRE_BUG2EVAL").as_deref() != Ok("1") {
            eprintln!("skipping: {:?}", v.reasons);
            return;
        }
        assert!(v.passed(), "{:?} {:?}", v.status, v.reasons);
        assert_eq!(v.before_exit, Some(1));
        assert_eq!(v.after_exit, Some(0));
        assert_eq!(v.receipt.case_sha256.len(), 64);
    }
}
