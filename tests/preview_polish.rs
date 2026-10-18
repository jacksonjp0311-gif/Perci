//! 0.11.8 pre-share fixes, end to end through the `perci` binary:
//! `perci tool` never waits on stdin, receipt ids bind the input, the gate's
//! status / posture / summary agree, and sequence-rule noise is gone without
//! loosening any refusal.

use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("perci-prev-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn perci(data: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_perci"));
    cmd.env("PERCI_HOME", env!("CARGO_MANIFEST_DIR"))
        .env("PERCI_DATA", data.join("data"))
        .env("PERCI_PROFILE", data.join("profile"))
        .env("PERCI_AGENTSEARCH", "off")
        .env_remove("PERCI_POLICY")
        .env_remove("PERCI_TOOL_PACK")
        .env_remove("PERCI_WEIGHT_PACKS")
        .env_remove("PERCI_TOOLS")
        .env_remove("PERCI_RUN_ID")
        .env_remove("PERCI_LEDGER");
    cmd
}

fn tool(data: &Path, name: &str, args: Value, run: Option<&str>) -> Value {
    let mut c = perci(data);
    if let Some(r) = run {
        c.env("PERCI_RUN_ID", r);
    }
    let out = c
        .args(["tool", name, "--json", &args.to_string()])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "{name}: {e}\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn codes(v: &Value) -> Vec<String> {
    v["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["code"].as_str().unwrap().to_owned())
        .collect()
}

/// status is exactly posture.status(); the summary names the posture.
fn assert_consistent(v: &Value) {
    let posture = v["result"]["posture"].as_str().unwrap();
    let want = match posture {
        "allow" => "pass",
        "propose_and_verify" => "abstain",
        "refuse" => "fail",
        other => panic!("unknown posture {other}"),
    };
    assert_eq!(v["status"], want, "{v}");
    assert!(
        v["result"]["summary"]
            .as_str()
            .unwrap()
            .starts_with(posture),
        "{v}"
    );
}

#[test]
fn tool_never_waits_on_stdin() {
    let d = temp("stdin");
    // A bare --json means {}.
    let t = Instant::now();
    let out = perci(&d)
        .args(["tool", "status", "--json"])
        .stdin(Stdio::piped()) // open, never written, never closed
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let out = out.wait_with_output().unwrap();
    assert!(t.elapsed() < Duration::from_secs(10));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["tool"], "perci.status");

    // No --json and a pipe that stays open and silent: {} after the wait.
    let t = Instant::now();
    let mut child = perci(&d)
        .env("PERCI_STDIN_WAIT_MS", "200")
        .args(["tool", "status"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take(); // keep it open
    let out = child.wait_with_output().unwrap();
    drop(stdin);
    assert!(t.elapsed() < Duration::from_secs(10));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["tool"], "perci.status");
    assert!(String::from_utf8_lossy(&out.stderr).contains("using {}"));

    // Piped arguments still work, including a slow writer after the first byte.
    let mut child = perci(&d)
        .env("PERCI_STDIN_WAIT_MS", "2000")
        .args(["tool", "exact"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"{\"expr\":\"17% of 240\",").unwrap();
    stdin.flush().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    stdin.write_all(b"\"claimed\":\"41\"}").unwrap();
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "fail");
    assert_eq!(v["result"]["decimal"], "40.8");

    // Empty stdin (closed at once) is {}.
    let out = perci(&d)
        .args(["tool", "status"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["tool"], "perci.status");
}

#[test]
fn receipt_id_binds_the_input() {
    let d = temp("receipt");
    let a = tool(
        &d,
        "policy_check",
        json!({"action": {"kind": "shell", "command": "ls docs"}}),
        None,
    );
    let b = tool(
        &d,
        "policy_check",
        json!({"action": {"kind": "shell", "command": "ls src"}}),
        None,
    );
    // Same answer, different questions.
    assert_eq!(a["receipt"]["output_sha256"], b["receipt"]["output_sha256"]);
    assert_ne!(a["receipt"]["input_sha256"], b["receipt"]["input_sha256"]);
    let (ra, rb) = (
        &a["receipt"]["receipt_sha256"],
        &b["receipt"]["receipt_sha256"],
    );
    assert_eq!(ra.as_str().unwrap().len(), 64);
    assert_ne!(ra, rb);
    // Deterministic: the same call gives the same receipt id.
    let a2 = tool(
        &d,
        "policy_check",
        json!({"action": {"kind": "shell", "command": "ls docs"}}),
        None,
    );
    assert_eq!(a2["receipt"]["receipt_sha256"], *ra);
    // And it is checkable from the verdict alone.
    let v: perci::verdict::Verdict = serde_json::from_value(a.clone()).unwrap();
    assert!(v.receipt_matches());
    let mut forged = v.clone();
    forged.receipt.input_sha256 = b["receipt"]["input_sha256"].as_str().unwrap().to_owned();
    assert!(!forged.receipt_matches());
}

#[test]
fn gate_status_posture_and_summary_agree() {
    let d = temp("consistent");
    let run = format!("consistent-{}", std::process::id());
    let pre = |cmd: &str| {
        tool(
            &d,
            "agent_gate_step",
            json!({"phase": "pre", "action": {"kind": "shell", "command": cmd}}),
            Some(&run),
        )
    };
    // Release before tests: the sequence rule refuses, and everything says so.
    let push = pre("git push origin main");
    assert_consistent(&push);
    assert_eq!(push["status"], "fail");
    assert_eq!(push["result"]["posture"], "refuse");
    assert_eq!(push["result"]["policy_posture"], "propose_and_verify");
    assert_eq!(
        push["result"]["summary"],
        "refuse (seq.deploy_requires_tests)"
    );
    assert_eq!(
        codes(&push)[0],
        "SEQUENCE_REQUIRES",
        "the first reason explains the status"
    );
    for cmd in [
        "ls",
        "cargo test",
        "rm -rf build",
        "git push --force origin main",
    ] {
        assert_consistent(&pre(cmd));
    }
}

#[test]
fn temporal_route_is_an_alias_of_next_step() {
    let d = temp("alias");
    let v = tool(
        &d,
        "temporal_route",
        json!({"history": ["shell:cargo build"]}),
        None,
    );
    assert_eq!(v["tool"], "perci.next_step");
    assert_eq!(v["status"], "abstain", "unsure without history: {v}");
}

#[test]
fn hard_refusal_is_not_double_reported() {
    let d = temp("dedupe");
    let run = format!("dedupe-{}", std::process::id());
    let v = tool(
        &d,
        "agent_gate_step",
        json!({"phase": "pre", "action": {"kind": "shell", "command": "git push --force origin main"}}),
        Some(&run),
    );
    assert_eq!(v["status"], "fail");
    assert_eq!(codes(&v), vec!["POLICY_REFUSE"]);
    assert_eq!(v["result"]["summary"], "refuse (git.force_push)");
    let rows = v["result"]["sequence"].as_array().unwrap();
    assert_eq!(rows[0]["rule"], "seq.deploy_requires_tests");
    assert_eq!(rows[0]["satisfied"], false);
    assert!(rows[0]["superseded"].is_string());
    // Without a ledger the same: no SEQUENCE_UNCHECKED on a refusal.
    let mut c = perci(&d);
    let out = c
        .env("PERCI_LEDGER", "off")
        .args(["tool", "agent_gate_step", "--json"])
        .arg(json!({"phase": "pre", "action": {"kind": "shell", "command": "git push --force origin main"}}).to_string())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["status"], "fail");
    assert!(!codes(&v).iter().any(|c| c.starts_with("SEQUENCE")), "{v}");
}

#[test]
fn deleting_a_path_that_does_not_exist_is_not_flagged() {
    let d = temp("absent");
    let ws = d.join("ws");
    std::fs::create_dir_all(ws.join("present")).unwrap();
    let ws_s = ws.to_string_lossy().into_owned();
    let run = format!("absent-{}", std::process::id());
    let pre = |action: Value| {
        tool(
            &d,
            "agent_gate_step",
            json!({"phase": "pre", "action": action}),
            Some(&run),
        )
    };

    let v = pre(json!({"kind": "shell", "command": "rm -rf no-such-dir", "cwd": ws_s}));
    assert_eq!(v["status"], "pass", "{v}");
    assert_consistent(&v);
    assert_eq!(codes(&v), vec!["DELETE_TARGETS_ABSENT"]);
    assert!(v["result"]["sequence"][0]["skipped"].is_string());
    let v = pre(json!({"kind": "delete", "paths": ["no-such-file.txt"], "cwd": ws_s}));
    assert_eq!(v["status"], "pass", "{v}");

    // Safety: everything that is not a verified-absent literal path keeps its flag.
    for action in [
        json!({"kind": "shell", "command": "rm -rf present", "cwd": ws_s}),
        json!({"kind": "shell", "command": "rm -rf no-such-dir present", "cwd": ws_s}),
        json!({"kind": "shell", "command": "rm -rf no-such-*", "cwd": ws_s}),
        json!({"kind": "shell", "command": "rm -rf $DIR", "cwd": ws_s}),
        json!({"kind": "shell", "command": "rm -rf no-such-dir"}),
        json!({"kind": "shell", "command": "rm -rf no-such-dir && rm -rf present", "cwd": ws_s}),
        json!({"kind": "shell", "command": "sudo rm -rf no-such-dir", "cwd": ws_s}),
        json!({"kind": "shell", "command": "Remove-Item -Filter x no-such-dir", "cwd": ws_s}),
    ] {
        let v = pre(action.clone());
        assert_ne!(v["status"], "pass", "{action} -> {v}");
        assert_consistent(&v);
    }
    // Hard refusals stay refusals even when nothing is there.
    for action in [
        json!({"kind": "shell", "command": "rm -rf /", "cwd": ws_s}),
        json!({"kind": "delete", "paths": ["models/nope.pwgt"], "cwd": ws_s}),
        json!({"kind": "delete", "paths": [".git"], "cwd": ws_s}),
    ] {
        let v = pre(action.clone());
        assert_eq!(v["status"], "fail", "{action} -> {v}");
    }
    // A shell delete of a protected path is never lifted, even when absent.
    let v = pre(json!({"kind": "shell", "command": "rm -rf .git/nope", "cwd": ws_s}));
    assert_eq!(v["status"], "abstain", "{v}");
    assert!(v["result"].get("delete_targets_absent").is_none());
}

#[test]
fn exact_accepts_a_numeric_claimed_value() {
    // MCP hosts (and the official inspector CLI) coerce "41" to the number 41.
    let d = temp("numclaim");
    let v = tool(
        &d,
        "exact",
        json!({"expr": "17% of 240", "claimed": 41}),
        None,
    );
    assert_eq!(v["status"], "fail", "{v}");
    assert!(codes(&v).contains(&"EXACT_MISMATCH".to_owned()));
    let v = tool(
        &d,
        "exact",
        json!({"expr": "17% of 240", "claimed": 40.8}),
        None,
    );
    assert_eq!(v["status"], "pass", "{v}");
    let v = tool(
        &d,
        "exact",
        json!({"expr": "17% of 240", "claimed": true}),
        None,
    );
    assert_eq!(v["status"], "error", "{v}");
}
