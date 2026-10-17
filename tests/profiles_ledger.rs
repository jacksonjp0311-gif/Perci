//! Weight packs, profiles, the step ledger, next_step and the verdict cache,
//! end to end through the `perci` binary (CLI and MCP stdio).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("perci-prof-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn perci(profile: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_perci"));
    cmd.env("PERCI_HOME", env!("CARGO_MANIFEST_DIR"))
        .env("PERCI_DATA", profile.join("data"))
        .env("PERCI_PROFILE", profile)
        .env_remove("PERCI_POLICY")
        .env_remove("PERCI_TOOL_PACK")
        .env_remove("PERCI_WEIGHT_PACKS")
        .env_remove("PERCI_TOOLS")
        .env_remove("PERCI_RUN_ID")
        .env_remove("PERCI_LEDGER");
    cmd
}

fn tool(profile: &Path, name: &str, args: Value, extra: &[(&str, &str)]) -> Value {
    let mut c = perci(profile);
    for (k, v) in extra {
        c.env(k, v);
    }
    let out = c
        .args(["tool", name, "--json", &args.to_string()])
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

fn v02_usable() -> bool {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("models/perci-cognitive-v0.2.pwgt");
    let ok = p.is_file() && !perci::paths::is_lfs_pointer(&p);
    if !ok {
        assert!(
            !perci::cognitive::weights_required(),
            "weights required (PERCI_REQUIRE_WEIGHTS=1) but v0.2 is missing or an LFS pointer"
        );
        eprintln!("skipping: v0.2 weights not available");
    }
    ok
}

#[test]
fn external_pack_is_verified_and_tampering_is_an_error() {
    if !v02_usable() {
        return;
    }
    let prof = temp("pack");
    let packdir = prof.join("packs/acme-router");
    std::fs::create_dir_all(&packdir).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("models/perci-cognitive-v0.2.pwgt");
    std::fs::copy(&src, packdir.join("router.pwgt")).unwrap();
    let st = perci(&prof)
        .args(["packs", "manifest"])
        .arg(&packdir)
        .args(["--name", "acme-router", "--version", "1.0.0"])
        .status()
        .unwrap();
    assert!(st.success());

    let list = perci(&prof)
        .args(["packs", "list", "--json"])
        .output()
        .unwrap();
    let list: Value = serde_json::from_slice(&list.stdout).unwrap();
    let row = list["packs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "acme-router")
        .expect("discovered under PERCI_PROFILE/packs");
    assert_eq!(row["pinned"], false);
    let ok = perci(&prof)
        .args(["packs", "verify", "acme-router"])
        .status()
        .unwrap();
    assert!(ok.success());

    // The route tool uses it by name, reports its own labels and has no
    // built-in lexical priors (external pack).
    let v = tool(
        &prof,
        "route",
        json!({"text": "why is the sky blue", "pack": "acme-router"}),
        &[],
    );
    assert_ne!(v["status"], "error", "{v}");
    assert_eq!(v["result"]["bitwork"]["lexical_priors"], "off");
    assert!(v["result"]["bitwork"]["labels"].as_array().unwrap().len() >= 2);
    assert_eq!(v["result"]["bitwork"]["decision"], "uncalibrated");
    assert!(v["receipt"]["weights"]["bitwork:acme-router"]
        .as_str()
        .unwrap()
        .starts_with("acme-router@1.0.0"));

    // Pin it with a wrong hash: refused.
    std::fs::write(
        prof.join("profile.json"),
        json!({"schema": "perci.profile.v1", "pins": {"acme-router": "00".repeat(32)}}).to_string(),
    )
    .unwrap();
    let v = tool(
        &prof,
        "route",
        json!({"text": "hi", "pack": "acme-router"}),
        &[],
    );
    assert_eq!(v["status"], "error");
    assert_eq!(v["reasons"][0]["code"], "PACK_UNAVAILABLE");
    std::fs::remove_file(prof.join("profile.json")).unwrap();

    // Flip one byte: verify and route both fail; no fallback to v0.2.
    let mut bytes = std::fs::read(packdir.join("router.pwgt")).unwrap();
    let n = bytes.len();
    bytes[n - 7] ^= 0x40;
    std::fs::write(packdir.join("router.pwgt"), &bytes).unwrap();
    let bad = perci(&prof)
        .args(["packs", "verify", "acme-router"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stdout).contains("does not match"));
    let v = tool(
        &prof,
        "route",
        json!({"text": "hi", "pack": "acme-router"}),
        &[],
    );
    assert_eq!(v["status"], "error");
    assert!(
        v["reasons"][0]["detail"]
            .as_str()
            .unwrap()
            .contains("modified"),
        "{v}"
    );
    // Default tool pack named but missing: an error, not v0.2.
    let v = tool(
        &prof,
        "route",
        json!({"text": "hi"}),
        &[("PERCI_TOOL_PACK", "nope")],
    );
    assert_eq!(v["status"], "error");
    let _ = std::fs::remove_dir_all(&prof);
}

#[test]
fn index_and_pack_manifest_formats_load() {
    if !v02_usable() {
        return;
    }
    let prof = temp("index");
    let dir = prof.join("private-packs");
    std::fs::create_dir_all(&dir).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("models/perci-cognitive-v0.2.pwgt");
    std::fs::copy(&src, dir.join("r.pwgt")).unwrap();
    let sha = perci::doctor::sha256_file(&dir.join("r.pwgt")).unwrap();
    let bytes = std::fs::metadata(dir.join("r.pwgt")).unwrap().len();
    std::fs::write(
        dir.join("packs.manifest.json"),
        json!({"schema": "perci.packs.v1", "packs": [
            {"pack_id": "idx-router", "role": "route", "default": true, "file": "r.pwgt",
             "format": "PERCIW03", "bytes": bytes, "sha256": sha}
        ]})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        dir.join("r.pack.json"),
        json!({"schema": "perci.pack-manifest.v1", "pack_id": "x", "magic": "PERCIW03",
               "byte_length": bytes, "checksum_sha256": sha, "path": "r.pwgt",
               "capability_tags": ["route"]})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        prof.join("profile.json"),
        json!({"schema": "perci.profile.v1",
               "packs_index": "private-packs/packs.manifest.json",
               "packs": {"side-router": {"path": "private-packs/r.pwgt", "manifest": "private-packs/r.pack.json"}}})
        .to_string(),
    )
    .unwrap();
    let ok = perci(&prof)
        .args(["packs", "verify", "idx-router", "side-router"])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stdout)
    );
    // The index's default pack is the tool default.
    let v = tool(
        &prof,
        "route",
        json!({"text": "explain rust lifetimes"}),
        &[],
    );
    assert_eq!(v["result"]["bitwork"]["pack"], "idx-router", "{v}");
    // PERCI_WEIGHT_PACKS adds folders too; PERCI_PACKS (knowledge notes) does not.
    let other = temp("index-env");
    let listed = perci(&other)
        .env("PERCI_WEIGHT_PACKS", &dir)
        .args(["packs", "list", "--json"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&listed.stdout).contains("idx-router"));
    let listed = perci(&other)
        .env("PERCI_PACKS", &dir)
        .args(["packs", "list", "--json"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&listed.stdout).contains("idx-router"));
    let _ = std::fs::remove_dir_all(&prof);
    let _ = std::fs::remove_dir_all(&other);
}

#[test]
fn profile_policy_overlay_and_sequence_rules() {
    let prof = temp("policy");
    // Overlay that extends the base with one refuse rule.
    std::fs::write(
        prof.join("policy.json"),
        json!({"schema": "perci.policy.v1", "extends": "base",
               "shell": {"refuse": [{"id": "site.no_make_release", "program": ["make"],
                                      "subcommand": "release", "reason": "releases go through CI"}]}})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        prof.join("sequence-rules.json"),
        json!({"schema": "perci.sequence.v1",
               "classes": {"lint": ["shell:cargo clippy"], "release": ["shell:make publish"]},
               "rules": [{"id": "site.release_requires_lint", "before": "release", "requires": "lint",
                          "status": "pass", "posture": "refuse", "reason": "lint first"}]})
        .to_string(),
    )
    .unwrap();
    let pc = |cmd: &str| {
        tool(
            &prof,
            "policy_check",
            json!({"action": {"kind": "shell", "command": cmd}}),
            &[],
        )
    };
    assert_eq!(pc("make release")["status"], "fail");
    assert_eq!(pc("git status")["status"], "pass", "base rules still apply");
    assert_eq!(pc("rm -rf /")["status"], "fail");

    let gate = |cmd: &str, run: &str| {
        tool(
            &prof,
            "agent_gate_step",
            json!({"phase": "pre", "action": {"kind": "shell", "command": cmd}}),
            &[("PERCI_RUN_ID", run)],
        )
    };
    let v = gate("make publish", "r-seq");
    assert_eq!(v["status"], "fail", "{v}");
    assert!(v["reasons"]
        .to_string()
        .contains("site.release_requires_lint"));
    // Record a lint pass, then the same step clears the rule and cites it.
    let rec = tool(
        &prof,
        "ledger_append",
        json!({"step": "shell:cargo clippy", "status": "pass", "phase": "post"}),
        &[("PERCI_RUN_ID", "r-seq")],
    );
    assert_eq!(rec["status"], "pass", "{rec}");
    let v = gate("make publish", "r-seq");
    assert_ne!(v["status"], "fail", "{v}");
    assert!(v["evidence"].to_string().contains("ledger:r-seq#"), "{v}");

    // A sequence file in another format is not ignored.
    std::fs::write(
        prof.join("sequence-rules.json"),
        json!({"schema": "something.else.v1", "rules": []}).to_string(),
    )
    .unwrap();
    let v = gate("git status", "r-bad");
    assert_eq!(v["status"], "abstain");
    assert!(
        v["reasons"].to_string().contains("SEQUENCE_RULES_INVALID"),
        "{v}"
    );
    let _ = std::fs::remove_dir_all(&prof);
}

struct Mcp {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
    id: u64,
}

impl Mcp {
    fn start(profile: &Path, run: &str) -> Mcp {
        let mut child = perci(profile)
            .env("PERCI_RUN_ID", run)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut m = Mcp { child, out, id: 0 };
        m.rpc("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}));
        m.notify("notifications/initialized");
        m
    }
    fn notify(&mut self, method: &str) {
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(stdin, "{}", json!({"jsonrpc": "2.0", "method": method})).unwrap();
    }
    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        let stdin = self.child.stdin.as_mut().unwrap();
        writeln!(
            stdin,
            "{}",
            json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params})
        )
        .unwrap();
        stdin.flush().unwrap();
        loop {
            let mut line = String::new();
            assert!(self.out.read_line(&mut line).unwrap() > 0, "mcp closed");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v["id"] == json!(self.id) {
                return v;
            }
        }
    }
    fn call(&mut self, name: &str, args: Value) -> Value {
        let r = self.rpc("tools/call", json!({"name": name, "arguments": args}));
        r["result"]["structuredContent"].clone()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_records_a_verifiable_ledger_and_suggests_next_steps() {
    let prof = temp("mcp");
    let mut m = Mcp::start(&prof, "run-a");
    let names: Vec<String> = m.rpc("tools/list", json!({}))["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    for t in ["ledger_append", "ledger_verify", "ledger_show", "next_step"] {
        assert!(names.iter().any(|n| n == t), "{t} missing from {names:?}");
    }
    let push =
        json!({"phase": "pre", "action": {"kind": "shell", "command": "git push origin main"}});
    let v = m.call("agent_gate_step", push.clone());
    assert_eq!(v["status"], "fail", "deploy before tests: {v}");
    assert!(v["reasons"]
        .to_string()
        .contains("seq.deploy_requires_tests"));
    assert_eq!(v["receipt"]["ledger"], "ledger:run-a#1");
    let t = m.call(
        "agent_gate_step",
        json!({"phase": "post", "action": {"kind": "shell", "command": "cargo test"}, "exit_code": 0}),
    );
    assert_eq!(t["status"], "pass", "{t}");
    let v = m.call("agent_gate_step", push.clone());
    assert_ne!(v["status"], "fail", "{v}");
    assert!(
        v["evidence"].to_string().contains("ledger:run-a#2"),
        "cites the test run: {v}"
    );

    // Cache: the second identical exact call is served from the cache.
    let a = m.call("exact", json!({"expr": "2^64"}));
    let b = m.call("exact", json!({"expr": "2^64"}));
    assert!(a["receipt"].get("cache_hit").is_none());
    assert_eq!(b["receipt"]["cache_hit"], true);
    assert_eq!(a["receipt"]["output_sha256"], b["receipt"]["output_sha256"]);

    let ver = m.call("ledger_verify", json!({}));
    assert_eq!(ver["status"], "pass", "{ver}");
    let show = m.call("ledger_show", json!({"last": 2}));
    assert_eq!(show["result"]["shown"].as_array().unwrap().len(), 2);

    // next_step after `cargo test` has seen `git push` follow it.
    let n = m.call(
        "next_step",
        json!({"history": ["agent_gate_step:post:shell:cargo test"]}),
    );
    assert_ne!(n["status"], "error", "{n}");
    assert!(
        n["result"]["next"].to_string().contains("shell:git push"),
        "{n}"
    );
    drop(m);

    // Tamper with the file: verify fails.
    let path = prof.join("ledger/run-a.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replacen("\"status\":\"fail\"", "\"status\":\"pass\"", 1),
    )
    .unwrap();
    let out = perci(&prof)
        .args(["ledger", "verify", "run-a"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let _ = std::fs::remove_dir_all(&prof);
}

#[test]
fn tool_schema_prints_the_input_schema() {
    let prof = temp("schema");
    let out = perci(&prof)
        .args(["tool", "schema", "check_math"])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["name"], "check_math");
    assert!(v["inputSchema"]["properties"]["text"].is_object(), "{v}");
    let out = perci(&prof)
        .args(["tool", "schema", "verdict"])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.to_string().contains("perci.verdict.v1"));
    let _ = std::fs::remove_dir_all(&prof);
}
