//! Ledger step names through PowerShell / shell wrappers, ledger_append from
//! an action, pinned calibration files, and policy overlays that narrow the
//! base policy's write paths, end to end through the `perci` binary.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("perci-pso-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tool(profile: &Path, name: &str, args: Value, run: Option<&str>) -> Value {
    let mut c = Command::new(env!("CARGO_BIN_EXE_perci"));
    c.env("PERCI_HOME", env!("CARGO_MANIFEST_DIR"))
        .env("PERCI_DATA", profile.join("data"))
        .env("PERCI_PROFILE", profile)
        .env("PERCI_AGENTSEARCH", "off")
        .env_remove("PERCI_POLICY")
        .env_remove("PERCI_TOOL_PACK")
        .env_remove("PERCI_WEIGHT_PACKS")
        .env_remove("PERCI_TOOLS")
        .env_remove("PERCI_LEDGER");
    match run {
        Some(r) => c.env("PERCI_RUN_ID", r),
        None => c.env_remove("PERCI_RUN_ID"),
    };
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

fn codes(v: &Value) -> Vec<String> {
    v["reasons"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| r["code"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn wrapped_scripts_are_recorded_as_what_they_run_and_hit_sequence_rules() {
    let prof = temp("steps");
    std::fs::write(
        prof.join("sequence-rules.json"),
        json!({"schema": "perci.sequence.v1",
               "classes": {"deploy": ["shell:stage_release.ps1"], "tests": ["shell:run-tests.ps1"]}})
        .to_string(),
    )
    .unwrap();
    let run = Some("wrapped");
    let gate = |cmd: &str| {
        tool(
            &prof,
            "agent_gate_step",
            json!({"phase": "pre", "action": {"kind": "shell", "command": cmd}}),
            run,
        )
    };
    // `& .\scripts\stage_release.ps1` used to be recorded as `shell:&`.
    let v = gate(r"& .\scripts\stage_release.ps1 -Tag v1");
    assert!(codes(&v).contains(&"SEQUENCE_REQUIRES".to_owned()), "{v}");
    let show = tool(&prof, "ledger_show", json!({"last": 1}), run);
    assert_eq!(
        show["result"]["shown"][0]["step"], "shell:stage_release.ps1",
        "{show}"
    );

    // The same script through the PowerShell host is the same step.
    let v =
        gate(r"powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\stage_release.ps1");
    assert!(codes(&v).contains(&"SEQUENCE_REQUIRES".to_owned()), "{v}");

    // ledger_append derives the step from the action (no `step` given).
    let v = tool(
        &prof,
        "ledger_append",
        json!({"action": {"kind": "shell", "command": r". .\run-tests.ps1"}, "status": "pass", "phase": "post"}),
        run,
    );
    assert_eq!(v["status"], "pass", "{v}");
    assert_eq!(v["result"]["step"], "shell:run-tests.ps1", "{v}");
    let v = tool(&prof, "ledger_append", json!({"status": "pass"}), run);
    assert_eq!(v["status"], "error", "{v}");

    // Tests passed after the last change: the deploy rule is now satisfied.
    let v = gate(r"pwsh -f scripts/stage_release.ps1");
    assert!(!codes(&v).contains(&"SEQUENCE_REQUIRES".to_owned()), "{v}");
    let ok = tool(&prof, "ledger_verify", json!({}), run);
    assert_eq!(ok["status"], "pass", "{ok}");
}

#[test]
fn overlay_narrows_the_base_write_paths() {
    let prof = temp("narrow");
    std::fs::write(
        prof.join("policy.json"),
        json!({"extends": "base",
               "write": {"allow_within": ["docs/**", "README.md"],
                         "allow_globs": {"$remove": ["*.md"]}}})
        .to_string(),
    )
    .unwrap();
    let write = |p: &str| {
        tool(
            &prof,
            "policy_check",
            json!({"action": {"kind": "write", "paths": [p]}}),
            None,
        )
    };
    // Allowed by the base (src/**) but outside allow_within: back to the default.
    let v = write("src/main.rs");
    assert_ne!(v["status"], "pass", "{v}");
    assert!(v.to_string().contains("write.outside_allow_within"), "{v}");
    assert_eq!(write("docs/guide.md")["status"], "pass");
    // `$remove` dropped `*.md`, so README.md is no longer allowed by any glob.
    let v = write("README.md");
    assert_ne!(v["status"], "pass", "{v}");
    // Protected paths stay refused.
    assert_eq!(write("docs/.env")["status"], "fail");
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
fn pinned_calibration_must_match() {
    if !v02_usable() {
        return;
    }
    let prof = temp("calpin");
    std::fs::create_dir_all(prof.join("calibration")).unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/route-calibration/v0.2.json");
    let bytes = std::fs::read(&src).unwrap();
    std::fs::write(prof.join("calibration/v0.2.json"), &bytes).unwrap();
    let sha = perci::doctor::sha256_hex(&bytes);
    let profile = |pin: &str| {
        std::fs::write(
            prof.join("profile.json"),
            json!({"schema": "perci.profile.v1",
                   "calibration": {"v0.2": {"path": "calibration/v0.2.json", "sha256": pin}}})
            .to_string(),
        )
        .unwrap();
    };
    let route = || {
        tool(
            &prof,
            "route",
            json!({"text": "why is my rust build slow", "pack": "v0.2"}),
            None,
        )
    };
    profile(&sha);
    let v = route();
    assert_ne!(v["status"], "error", "{v}");
    assert_eq!(
        v["result"]["bitwork"]["confidence"]["calibrated"], true,
        "{v}"
    );

    // Edited after pinning: the pack refuses to load instead of using it.
    let mut edited = bytes.clone();
    edited.extend_from_slice(b"\n");
    std::fs::write(prof.join("calibration/v0.2.json"), &edited).unwrap();
    let v = route();
    assert_eq!(v["status"], "error", "{v}");
    assert!(v.to_string().contains("was modified"), "{v}");

    // A wrong pin is the same failure; an unpinned path still loads leniently.
    std::fs::write(prof.join("calibration/v0.2.json"), &bytes).unwrap();
    profile(&"0".repeat(64));
    assert_eq!(route()["status"], "error");
}
