//! `perci fetch-weights`: download the runtime weights into an installed Perci.
//!
//! Release archives carry the binary, the weight *manifest* and the small
//! config files, but not the weights themselves (about 240 MB for the default
//! set). The weights are published once per release as separate assets, named
//! after their file (`perci-cognitive-v0.3.pwgt`, ...), and this command
//! downloads the ones `models/PACKAGE_MANIFEST.json` lists, checks every file's
//! size and SHA-256 against that manifest, and only then moves it into place.
//! A file that already matches is left alone, so the command is safe to re-run.
//!
//! Sets: `tools` (the v0.2 pack the `route` tool uses by default, ~20 MB),
//! `default` (everything except the legacy v0.1 pack, ~240 MB) and `all`.
//!
//! Source: `--from URL|DIR`, else `PERCI_WEIGHTS_URL`, else this version's
//! GitHub release. Downloads use the system `curl` (shipped with Windows 10+,
//! macOS and practically every Linux), so Perci keeps no HTTP dependency.

use crate::{doctor, paths};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Public repository the release assets live in.
pub const RELEASE_REPO: &str = "jacksonjp0311-gif/Perci";

/// Release download base for this build (`.../releases/download/v<version>`).
pub fn default_base_url() -> String {
    format!(
        "https://github.com/{RELEASE_REPO}/releases/download/v{}",
        env!("CARGO_PKG_VERSION")
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightSet {
    Tools,
    Default,
    All,
}

impl WeightSet {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "tools" | "tool" | "minimal" => Some(Self::Tools),
            "default" | "chat" => Some(Self::Default),
            "all" | "full" => Some(Self::All),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Tools => "tools",
            Self::Default => "default",
            Self::All => "all",
        }
    }

    /// Does an artifact of manifest `kind` belong to this set?
    pub fn includes(self, kind: &str) -> bool {
        match self {
            Self::All => true,
            Self::Default => !kind.starts_with("legacy"),
            Self::Tools => kind.starts_with("fallback"),
        }
    }
}

/// One manifest entry selected for download.
#[derive(Clone, Debug)]
pub struct Wanted {
    pub path: String,
    pub kind: String,
    pub bytes: Option<u64>,
    pub sha256: Option<String>,
}

impl Wanted {
    /// Release asset name: the file's base name.
    pub fn asset(&self) -> String {
        Path::new(&self.path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.clone())
    }
}

/// Entries of `manifest` that belong to `set`.
pub fn select(manifest: &Value, set: WeightSet) -> Vec<Wanted> {
    manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|e| {
                    let path = e.get("path")?.as_str()?.to_owned();
                    let kind = e
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("artifact")
                        .to_owned();
                    set.includes(&kind).then(|| Wanted {
                        path,
                        kind,
                        bytes: e.get("bytes").and_then(Value::as_u64),
                        sha256: e.get("sha256").and_then(Value::as_str).map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Does `file` already hold exactly what the manifest entry describes?
pub fn matches(file: &Path, want: &Wanted) -> Result<(), String> {
    let meta = fs::metadata(file).map_err(|_| "missing".to_owned())?;
    if paths::is_lfs_pointer(file) {
        return Err("lfs-pointer".into());
    }
    if let Some(bytes) = want.bytes {
        if meta.len() != bytes {
            return Err(format!("size {} (expected {bytes})", meta.len()));
        }
    }
    match &want.sha256 {
        Some(expected) => {
            let got = doctor::sha256_file(file).map_err(|e| e.to_string())?;
            if got.eq_ignore_ascii_case(expected) {
                Ok(())
            } else {
                Err(format!("sha256 {got} (expected {expected})"))
            }
        }
        None => Err("manifest has no sha256 for this file".into()),
    }
}

fn join_url(base: &str, asset: &str) -> String {
    format!("{}/{asset}", base.trim_end_matches('/'))
}

/// Fetch `asset` from `base` (URL or local directory) into `tmp`.
fn fetch(base: &str, asset: &str, tmp: &Path, quiet: bool) -> Result<(), String> {
    let local = Path::new(base);
    if !base.contains("://") && local.is_dir() {
        fs::copy(local.join(asset), tmp)
            .map(|_| ())
            .map_err(|e| format!("copy {}: {e}", local.join(asset).display()))
    } else {
        let url = join_url(base, asset);
        let mut cmd = Command::new("curl");
        cmd.args(["-fL", "--retry", "3", "--retry-delay", "2"]);
        cmd.arg(if quiet { "-sS" } else { "-#" });
        cmd.arg("-o").arg(tmp).arg(&url);
        let status = cmd.status().map_err(|e| {
            format!("could not run curl ({e}); download {url} by hand into the models folder")
        })?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("curl failed for {url} ({status})"))
        }
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub path: String,
    pub action: &'static str,
    pub detail: String,
}

/// Download (or confirm) every selected file under `home`.
pub fn fetch_all(
    home: &Path,
    wanted: &[Wanted],
    base: &str,
    force: bool,
    quiet: bool,
) -> Vec<Outcome> {
    let mut out = Vec::new();
    for w in wanted {
        let dest = home.join(&w.path);
        if !force && matches(&dest, w).is_ok() {
            out.push(Outcome {
                path: w.path.clone(),
                action: "present",
                detail: "already verified".into(),
            });
            continue;
        }
        if let Some(dir) = dest.parent() {
            if let Err(e) = fs::create_dir_all(dir) {
                out.push(Outcome {
                    path: w.path.clone(),
                    action: "failed",
                    detail: format!("cannot create {}: {e}", dir.display()),
                });
                continue;
            }
        }
        let mut tmp = dest.clone().into_os_string();
        tmp.push(".part");
        let tmp = PathBuf::from(tmp);
        if !quiet {
            eprintln!("fetching {} ...", w.asset());
        }
        let result = fetch(base, &w.asset(), &tmp, quiet)
            .and_then(|()| matches(&tmp, w).map_err(|e| format!("verification failed: {e}")))
            .and_then(|()| {
                // A read-only mapped copy on Windows cannot be replaced in
                // place; remove the old file first (it was wrong anyway).
                let _ = fs::remove_file(&dest);
                fs::rename(&tmp, &dest).map_err(|e| format!("rename: {e}"))
            });
        match result {
            Ok(()) => out.push(Outcome {
                path: w.path.clone(),
                action: "downloaded",
                detail: "size + sha256 verified".into(),
            }),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                out.push(Outcome {
                    path: w.path.clone(),
                    action: "failed",
                    detail: e,
                });
            }
        }
    }
    out
}

/// `perci fetch-weights [--set tools|default|all] [--from URL|DIR] [--force] [--list] [--json] [--quiet]`.
/// Exit 0 when every selected file is present and verified, 1 otherwise, 2 on usage/config errors.
pub fn run_cli(args: &[String]) -> i32 {
    let usage = "usage: perci fetch-weights [--set tools|default|all] [--from URL|DIR] [--force] [--list] [--json] [--quiet]";
    let mut set = WeightSet::Default;
    let mut from: Option<String> = None;
    let (mut force, mut list, mut as_json, mut quiet) = (false, false, false, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--set" => match it.next().and_then(|v| WeightSet::parse(v)) {
                Some(s) => set = s,
                None => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "--tools" => set = WeightSet::Tools,
            "--all" => set = WeightSet::All,
            "--from" | "--url" => match it.next() {
                Some(v) => from = Some(v.clone()),
                None => {
                    eprintln!("{usage}");
                    return 2;
                }
            },
            "--force" => force = true,
            "--list" | "--dry-run" => list = true,
            "--json" => as_json = true,
            "--quiet" | "-q" => quiet = true,
            "-h" | "--help" => {
                println!("{usage}");
                return 0;
            }
            other => {
                eprintln!("perci fetch-weights: unknown argument {other:?}\n{usage}");
                return 2;
            }
        }
    }
    let base = from
        .or_else(|| {
            std::env::var("PERCI_WEIGHTS_URL")
                .ok()
                .filter(|v| !v.is_empty())
        })
        .unwrap_or_else(default_base_url);
    let home = paths::home();
    let manifest_path = home.join(paths::HOME_MARKER);
    let manifest: Value = match fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
    {
        Some(m) => m,
        None => {
            eprintln!(
                "perci fetch-weights: no weight manifest at {} (Perci home: {}, {}). Set PERCI_HOME to the install folder.",
                manifest_path.display(),
                home.display(),
                paths::home_source().label()
            );
            return 2;
        }
    };
    let wanted = select(&manifest, set);
    if list {
        for w in &wanted {
            let mb = w.bytes.map(|b| b as f64 / 1e6).unwrap_or(0.0);
            println!(
                "{:<40} {:>8.1} MB  {}",
                w.path,
                mb,
                join_url(&base, &w.asset())
            );
        }
        return 0;
    }
    let outcomes = fetch_all(&home, &wanted, &base, force, quiet || as_json);
    let ok = outcomes.iter().all(|o| o.action != "failed");
    if as_json {
        println!(
            "{}",
            json!({
                "ok": ok,
                "set": set.label(),
                "home": home.display().to_string(),
                "source": base,
                "files": outcomes.iter().map(|o| json!({"path": o.path, "action": o.action, "detail": o.detail})).collect::<Vec<_>>(),
            })
        );
    } else {
        for o in &outcomes {
            println!("  {:<10} {}  {}", o.action, o.path, o.detail);
        }
        println!(
            "\n{} ({} set, home {})",
            if ok {
                "Weights ready."
            } else {
                "Some weights could not be fetched."
            },
            set.label(),
            home.display()
        );
    }
    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_for(dir: &Path, files: &[(&str, &str, &[u8])]) -> Value {
        json!({"artifacts": files.iter().map(|(name, kind, data)| {
            let _ = dir;
            json!({"path": format!("models/{name}"), "kind": kind, "bytes": data.len(), "sha256": doctor::sha256_hex(data)})
        }).collect::<Vec<_>>()})
    }

    #[test]
    fn sets_select_by_kind() {
        let m = manifest_for(
            Path::new("."),
            &[
                ("a.pwgt", "active_associative_weights", b"a"),
                ("b.pwgt", "fallback_associative_weights", b"b"),
                ("c.pwgt", "legacy_associative_weights", b"c"),
            ],
        );
        let names = |s| {
            select(&m, s)
                .into_iter()
                .map(|w| w.asset())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(WeightSet::Tools), vec!["b.pwgt"]);
        assert_eq!(names(WeightSet::Default), vec!["a.pwgt", "b.pwgt"]);
        assert_eq!(names(WeightSet::All).len(), 3);
    }

    #[test]
    fn fetch_from_dir_verifies_and_rejects_tampered_files() {
        let root = std::env::temp_dir().join(format!("perci-fetch-{}", std::process::id()));
        let src = root.join("release");
        let home = root.join("home");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("good.bin"), b"good weights").unwrap();
        fs::write(src.join("bad.bin"), b"tampered!!").unwrap();
        let m = json!({"artifacts": [
            {"path": "models/good.bin", "kind": "active", "bytes": 12, "sha256": doctor::sha256_hex(b"good weights")},
            {"path": "models/bad.bin", "kind": "active", "bytes": 10, "sha256": doctor::sha256_hex(b"original!!")},
        ]});
        let wanted = select(&m, WeightSet::All);
        let out = fetch_all(&home, &wanted, src.to_str().unwrap(), false, true);
        assert_eq!(out[0].action, "downloaded", "{out:?}");
        assert_eq!(out[1].action, "failed", "{out:?}");
        assert!(out[1].detail.contains("sha256"), "{out:?}");
        assert!(!home.join("models/bad.bin").exists());
        assert!(!home.join("models/bad.bin.part").exists());
        // Re-running leaves a verified file alone.
        let again = fetch_all(&home, &wanted[..1], src.to_str().unwrap(), false, true);
        assert_eq!(again[0].action, "present");
        let _ = fs::remove_dir_all(&root);
    }
}
