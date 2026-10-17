//! Swappable Bitwork weight packs, verified at load.
//!
//! A pack is a folder with a `perci-pack.json` manifest:
//!
//! ```json
//! {"schema": "perci.weight-pack.v1", "name": "acme-router", "version": "1.0.0",
//!  "kind": "bitwork-router", "entry": "router.pwgt",
//!  "categories": ["greeting", "math", ...],
//!  "files": [{"path": "router.pwgt", "bytes": 20094368, "sha256": "..."}]}
//! ```
//!
//! Every listed file is hashed when the pack is loaded; the entry's labels
//! must equal `categories` (same order). A missing file, a wrong size or hash,
//! a path outside the folder or a pin mismatch is an **error**, never a
//! fallback to another pack.
//!
//! Where packs come from, in order (first name wins):
//! 1. built-in `v0.1`, `v0.2`, `v0.3` (`PERCI_HOME/models`, hashes from
//!    `models/PACKAGE_MANIFEST.json`, which is committed to git);
//! 2. `PERCI_WEIGHT_PACKS` (path list): a folder holding `perci-pack.json`,
//!    a folder or file holding a `perci.packs.v1` index
//!    (`packs.manifest.json`), or a folder whose subfolders hold manifests.
//!    (`PERCI_PACKS` is the knowledge-pack root, see `intel_packs`.)
//! 3. `PERCI_PROFILE/packs/` (same rules);
//! 4. `profile.json`: `packs` (a list of paths, or an object `name →
//!    {path, manifest}` where `manifest` is a `perci.pack-manifest.v1` file)
//!    and `packs_index` (a `perci.packs.v1` index).
//!
//! Index entries (`perci.packs.v1`) carry `pack_id`, `file`, `bytes`,
//! `sha256`, `labels` and optionally `sidecar` + `sidecar_sha256`,
//! `version`, `default`, `lexical_priors`, `calibration`. Their pin is the
//! sha256 of the entry's canonical JSON.
//!
//! Per-pack options: `lexical_priors` (`"builtin"`, `"off"` — the default
//! for non-built-in packs — or `{"points": 48, "labels": {label: [cues]}}`)
//! and `calibration` (`perci.route-calibration.v1`, from
//! `perci packs calibrate`).
//!
//! Trust: the manifest proves integrity (the files are the ones it lists).
//! Authenticity comes from a **pin**: the sha256 of the manifest file, set in
//! `profile.json` (`"pins": {"name": "<sha256>"}`). Signatures are not
//! supported (no crypto dependency); a manifest `signature` field is reported
//! as unverified. `"require_pins": true` in `profile.json` makes unpinned
//! external packs an error.

use crate::cognitive::CognitiveWeights;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Component, Path, PathBuf};

pub const SCHEMA: &str = "perci.weight-pack.v1";
pub const MANIFEST_FILE: &str = "perci-pack.json";
pub const BUILTIN: &[&str] = &["v0.1", "v0.2", "v0.3"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackFile {
    pub path: String,
    /// Expected size; `None` skips the size check (the hash still applies).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema: String,
    pub name: String,
    pub version: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    pub entry: String,
    pub categories: Vec<String>,
    pub files: Vec<PackFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<Value>,
    /// `"builtin"`, `"off"` or a custom table (see the module docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lexical_priors: Option<Value>,
    /// `perci.route-calibration.v1` (margin buckets → measured accuracy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<Value>,
}

fn default_kind() -> String {
    "bitwork-router".into()
}

/// A discovered pack (not yet verified).
#[derive(Clone, Debug)]
pub struct PackInfo {
    pub manifest: Manifest,
    /// Folder the manifest's relative paths resolve against.
    pub dir: PathBuf,
    /// sha256 of the manifest bytes (what a pin pins).
    pub manifest_sha256: String,
    pub source: String,
    pub pin: Option<String>,
    pub builtin: bool,
    /// Marked `"default": true` in an index (used when nothing else names
    /// the tool pack).
    pub default: bool,
}

impl PackInfo {
    pub fn entry_path(&self) -> PathBuf {
        self.dir.join(&self.manifest.entry)
    }
    /// `name@version manifest-sha256:<hex>`, recorded in receipts.
    pub fn identity(&self) -> String {
        format!(
            "{}@{} manifest-sha256:{}",
            self.manifest.name, self.manifest.version, self.manifest_sha256
        )
    }
}

/// Result of [`verify`].
#[derive(Clone, Debug, Serialize)]
pub struct VerifyReport {
    pub name: String,
    pub version: String,
    pub source: String,
    pub ok: bool,
    pub pinned: bool,
    pub manifest_sha256: String,
    pub files_checked: usize,
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
}

fn profile_config() -> Value {
    crate::profile::config()
}

fn pins(cfg: &Value) -> std::collections::BTreeMap<String, String> {
    cfg.get("pins")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.trim().to_lowercase())))
                .collect()
        })
        .unwrap_or_default()
}

/// Synthesized manifests for the packs shipped in `PERCI_HOME/models`.
fn builtin(name: &str) -> Result<PackInfo, String> {
    let models = crate::paths::asset("models");
    let pkg_path = models.join("PACKAGE_MANIFEST.json");
    let pkg_bytes = std::fs::read(&pkg_path).map_err(|e| format!("{}: {e}", pkg_path.display()))?;
    let pkg: Value =
        serde_json::from_slice(&pkg_bytes).map_err(|e| format!("{}: {e}", pkg_path.display()))?;
    let file = format!("perci-cognitive-{name}.pwgt");
    let art = pkg["artifacts"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|x| x["path"].as_str() == Some(&format!("models/{file}")))
        })
        .ok_or_else(|| format!("built-in pack {name} is not listed in PACKAGE_MANIFEST.json"))?;
    let side = models.join(format!("{file}.json"));
    let side_v: Value = std::fs::read(&side)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or_else(|| format!("{}: missing or invalid", side.display()))?;
    let categories: Vec<String> = side_v["labels"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let manifest = Manifest {
        schema: SCHEMA.into(),
        name: name.into(),
        version: side_v["revision"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}", side_v["version"])),
        kind: default_kind(),
        entry: file.clone(),
        categories,
        files: vec![PackFile {
            path: file,
            bytes: Some(art["bytes"].as_u64().unwrap_or(0)),
            sha256: art["sha256"].as_str().unwrap_or("").to_lowercase(),
        }],
        description: side_v["name"].as_str().map(str::to_owned),
        signature: None,
        lexical_priors: Some(json!("builtin")),
        calibration: None,
    };
    let canonical = serde_json::to_vec(&manifest).unwrap_or_default();
    let sha = crate::doctor::sha256_hex(&canonical);
    Ok(PackInfo {
        manifest,
        dir: models,
        // Built-ins are pinned by the committed PACKAGE_MANIFEST.json.
        pin: Some(sha.clone()),
        manifest_sha256: sha,
        source: "builtin (models/PACKAGE_MANIFEST.json)".into(),
        builtin: true,
        default: false,
    })
}

/// Read `dir/perci-pack.json`.
pub fn read_pack_dir(dir: &Path, source: &str) -> Result<PackInfo, String> {
    let path = dir.join(MANIFEST_FILE);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{}: not a weight-pack manifest ({e})", path.display()))?;
    if manifest.schema != SCHEMA {
        return Err(format!("{}: schema must be {SCHEMA}", path.display()));
    }
    if manifest.name.is_empty() || !valid_name(&manifest.name) {
        return Err(format!(
            "{}: name must be 1-64 chars of [A-Za-z0-9._-]",
            path.display()
        ));
    }
    if BUILTIN.contains(&manifest.name.as_str()) {
        return Err(format!(
            "{}: name {} is reserved for a built-in pack",
            path.display(),
            manifest.name
        ));
    }
    Ok(PackInfo {
        manifest,
        dir: dir.to_path_buf(),
        manifest_sha256: crate::doctor::sha256_hex(&bytes),
        source: source.to_owned(),
        pin: None,
        builtin: false,
        default: false,
    })
}

pub const INDEX_SCHEMA: &str = "perci.packs.v1";
pub const INDEX_FILE: &str = "packs.manifest.json";
pub const PACK_MANIFEST_SCHEMA: &str = "perci.pack-manifest.v1";

fn str_list(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn kind_of(entry: &Value) -> String {
    let format = entry
        .get("format")
        .or_else(|| entry.get("magic"))
        .and_then(Value::as_str)
        .unwrap_or("PERCIW03");
    let role = entry.get("role").and_then(Value::as_str).unwrap_or("route");
    if role == "route" && format.starts_with("PERCIW0") {
        default_kind()
    } else {
        format!("{role}/{format}")
    }
}

fn check_reserved(name: &str, where_: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err(format!(
            "{where_}: pack name {name:?} must be 1-64 chars of [A-Za-z0-9._-]"
        ));
    }
    if BUILTIN.contains(&name) {
        return Err(format!(
            "{where_}: name {name} is reserved for a built-in pack"
        ));
    }
    Ok(())
}

/// Packs of a `perci.packs.v1` index file.
pub fn read_index(path: &Path, source: &str) -> Vec<Result<PackInfo, String>> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => return vec![Err(format!("{} ({source}): {e}", path.display()))],
    };
    let v: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => return vec![Err(format!("{}: not valid JSON ({e})", path.display()))],
    };
    if v.get("schema").and_then(Value::as_str) != Some(INDEX_SCHEMA) {
        return vec![Err(format!(
            "{}: schema must be {INDEX_SCHEMA}",
            path.display()
        ))];
    }
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let Some(list) = v.get("packs").and_then(Value::as_array) else {
        return vec![Err(format!("{}: packs must be a list", path.display()))];
    };
    list.iter()
        .map(|e| {
            let where_ = format!("{} ({source})", path.display());
            let name = e
                .get("pack_id")
                .or_else(|| e.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            check_reserved(name, &where_)?;
            let file = e
                .get("file")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{where_}: {name}: file is required"))?;
            let sha = e
                .get("sha256")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{where_}: {name}: sha256 is required"))?;
            let mut files = vec![PackFile {
                path: file.to_owned(),
                bytes: e.get("bytes").and_then(Value::as_u64),
                sha256: sha.to_lowercase(),
            }];
            if let (Some(sc), Some(sc_sha)) = (
                e.get("sidecar").and_then(Value::as_str),
                e.get("sidecar_sha256").and_then(Value::as_str),
            ) {
                files.push(PackFile {
                    path: sc.to_owned(),
                    bytes: None,
                    sha256: sc_sha.to_lowercase(),
                });
            }
            let manifest = Manifest {
                schema: SCHEMA.into(),
                name: name.to_owned(),
                version: e
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or("unversioned")
                    .to_owned(),
                kind: kind_of(e),
                entry: file.to_owned(),
                categories: str_list(&e["labels"]),
                files,
                description: e
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                signature: e.get("signature").cloned(),
                lexical_priors: e.get("lexical_priors").cloned(),
                calibration: e.get("calibration").cloned(),
            };
            Ok(PackInfo {
                manifest,
                dir: dir.clone(),
                manifest_sha256: crate::doctor::sha256_hex(canonical(e).as_bytes()),
                source: format!("index {}", path.display()),
                pin: None,
                builtin: false,
                default: e.get("default").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn canonical(v: &Value) -> String {
    crate::verdict::canonical_json(v)
}

/// One pack described by a `perci.pack-manifest.v1` file (`checksum_sha256`,
/// `byte_length`), named `name`, with weights at `weights` (else the
/// manifest's own `path`, relative to it). Labels come from the weights'
/// `.json` sidecar when present (hash-checked through the weights).
pub fn read_pack_manifest(
    name: &str,
    manifest_path: &Path,
    weights: Option<PathBuf>,
    source: &str,
) -> Result<PackInfo, String> {
    let where_ = format!("{} ({source})", manifest_path.display());
    check_reserved(name, &where_)?;
    let bytes = std::fs::read(manifest_path).map_err(|e| format!("{where_}: {e}"))?;
    let v: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("{where_}: not valid JSON ({e})"))?;
    if v.get("schema").and_then(Value::as_str) != Some(PACK_MANIFEST_SCHEMA) {
        return Err(format!("{where_}: schema must be {PACK_MANIFEST_SCHEMA}"));
    }
    let base = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let weights = match weights {
        Some(w) => w,
        None => base.join(
            v.get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{where_}: path is required"))?,
        ),
    };
    let dir = weights.parent().map(Path::to_path_buf).unwrap_or_default();
    let file = weights
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .ok_or_else(|| format!("{where_}: weights path has no file name"))?;
    let sha = v
        .get("checksum_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{where_}: checksum_sha256 is required"))?;
    let sidecar: Option<Value> = std::fs::read(dir.join(format!("{file}.json")))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let categories = str_list(
        v.get("labels")
            .or_else(|| sidecar.as_ref().and_then(|s| s.get("labels")))
            .unwrap_or(&Value::Null),
    );
    let mut kind_src = v.clone();
    if kind_src.get("role").is_none() {
        let tags = str_list(&v["capability_tags"]);
        if !tags.is_empty() && !tags.iter().any(|t| t == "route") {
            kind_src["role"] = json!(tags[0]);
        }
    }
    let manifest = Manifest {
        schema: SCHEMA.into(),
        name: name.to_owned(),
        version: v
            .get("version")
            .or_else(|| v.get("builder_version"))
            .and_then(Value::as_str)
            .unwrap_or("unversioned")
            .to_owned(),
        kind: kind_of(&kind_src),
        entry: file.clone(),
        categories,
        files: vec![PackFile {
            path: file,
            bytes: v.get("byte_length").and_then(Value::as_u64),
            sha256: sha.to_lowercase(),
        }],
        description: v.get("notes").and_then(Value::as_str).map(str::to_owned),
        signature: v.get("signature").cloned(),
        lexical_priors: v.get("lexical_priors").cloned(),
        calibration: v.get("calibration").cloned(),
    };
    Ok(PackInfo {
        manifest,
        dir,
        manifest_sha256: crate::doctor::sha256_hex(&bytes),
        source: source.to_owned(),
        pin: None,
        builtin: false,
        default: false,
    })
}

pub fn valid_name(n: &str) -> bool {
    (1..=64).contains(&n.len())
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn expand(dir: &Path, source: &str, out: &mut Vec<Result<PackInfo, String>>) {
    if dir.is_file() {
        if dir.file_name().is_some_and(|n| n == MANIFEST_FILE) {
            if let Some(parent) = dir.parent() {
                out.push(read_pack_dir(parent, source));
            }
        } else {
            out.extend(read_index(dir, source));
        }
        return;
    }
    if dir.join(MANIFEST_FILE).is_file() {
        out.push(read_pack_dir(dir, source));
        return;
    }
    if dir.join(INDEX_FILE).is_file() {
        out.extend(read_index(&dir.join(INDEX_FILE), source));
        return;
    }
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            let mut subs: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.join(MANIFEST_FILE).is_file())
                .collect();
            subs.sort();
            for s in subs {
                out.push(read_pack_dir(&s, source));
            }
        }
        Err(e) if source != "profile packs/" => {
            out.push(Err(format!("{} ({source}): {e}", dir.display())))
        }
        Err(_) => {}
    }
}

/// Every pack Perci can see, built-ins first. Entries that failed to parse
/// are returned as errors so `perci packs list` can show them.
pub fn discover() -> Vec<Result<PackInfo, String>> {
    let mut out: Vec<Result<PackInfo, String>> = BUILTIN
        .iter()
        .filter(|n| crate::paths::asset(format!("models/perci-cognitive-{n}.pwgt")).exists())
        .map(|n| builtin(n))
        .collect();
    if let Some(list) = std::env::var_os("PERCI_WEIGHT_PACKS") {
        for dir in std::env::split_paths(&list) {
            if !dir.as_os_str().is_empty() {
                expand(&dir, "PERCI_WEIGHT_PACKS", &mut out);
            }
        }
    }
    let cfg = match crate::profile::config_checked() {
        Ok(c) => c,
        Err(e) => {
            out.push(Err(e));
            Value::Null
        }
    };
    let profile_packs = crate::paths::profile_file("packs");
    let listed_index = cfg
        .get("packs_index")
        .and_then(Value::as_str)
        .map(crate::profile::resolve);
    // The profile's own packs/ folder, unless profile.json names its index
    // (then that index is read once, below).
    if listed_index.as_deref() != Some(profile_packs.join(INDEX_FILE).as_path()) {
        expand(&profile_packs, "profile packs/", &mut out);
    }
    match cfg.get("packs") {
        Some(Value::Array(list)) => {
            for p in list {
                if let Some(path) = p.get("path").and_then(Value::as_str).or_else(|| p.as_str()) {
                    expand(
                        &crate::profile::resolve(path),
                        "profile.json packs",
                        &mut out,
                    );
                }
            }
        }
        Some(Value::Object(map)) => {
            for (name, spec) in map {
                let path = spec
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| spec.as_str())
                    .map(crate::profile::resolve);
                let manifest = spec
                    .get("manifest")
                    .and_then(Value::as_str)
                    .map(crate::profile::resolve);
                match (manifest, path) {
                    (Some(m), w) => out.push(read_pack_manifest(name, &m, w, "profile.json packs")),
                    (None, Some(p)) if p.is_dir() || p.extension().is_some_and(|e| e == "json") => {
                        expand(&p, "profile.json packs", &mut out)
                    }
                    (None, Some(p)) => out.push(Err(format!(
                        "profile.json packs.{name}: {} has no manifest; add \"manifest\" or run perci packs manifest",
                        p.display()
                    ))),
                    (None, None) => out.push(Err(format!("profile.json packs.{name}: path is required"))),
                }
            }
        }
        Some(Value::Null) | None => {}
        Some(_) => out.push(Err("profile.json packs must be a list or an object".into())),
    }
    if let Some(index) = listed_index {
        out.extend(read_index(&index, "profile.json packs_index"));
    }
    let pins = pins(&cfg);
    let mut seen = std::collections::BTreeSet::new();
    out.into_iter()
        .map(|r| match r {
            Ok(mut info) => {
                if !seen.insert(info.manifest.name.clone()) {
                    return Err(format!(
                        "duplicate pack name {} at {} (first one wins)",
                        info.manifest.name,
                        info.dir.display()
                    ));
                }
                if !info.builtin {
                    info.pin = pins.get(&info.manifest.name).cloned();
                }
                Ok(info)
            }
            Err(e) => Err(e),
        })
        .collect()
}

/// Find one pack by name.
pub fn find(name: &str) -> Result<PackInfo, String> {
    let mut problems = Vec::new();
    for r in discover() {
        match r {
            Ok(info) if info.manifest.name == name => return Ok(info),
            Ok(_) => {}
            Err(e) => problems.push(e),
        }
    }
    let mut msg = format!("no weight pack named {name:?}");
    if !problems.is_empty() {
        msg.push_str(&format!(" (unreadable packs: {})", problems.join("; ")));
    }
    Err(msg)
}

fn safe_rel(p: &str) -> bool {
    let path = Path::new(p);
    !p.is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

fn require_pins() -> bool {
    profile_config()
        .get("require_pins")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Static checks shared by [`verify`] and [`load`].
fn check_manifest(info: &PackInfo, problems: &mut Vec<String>, warnings: &mut Vec<String>) {
    let m = &info.manifest;
    if m.kind != "bitwork-router" {
        problems.push(format!(
            "kind {:?} is not supported (bitwork-router)",
            m.kind
        ));
    }
    if m.categories.is_empty() {
        warnings.push("no categories listed: the labels are taken from the weights".into());
    }
    if let Err(e) = priors_of(m) {
        problems.push(e);
    }
    if !m.files.iter().any(|f| f.path == m.entry) {
        problems.push(format!("entry {} is not listed in files", m.entry));
    }
    for f in &m.files {
        if !safe_rel(&f.path) {
            problems.push(format!(
                "{}: paths must stay inside the pack folder",
                f.path
            ));
        }
        if f.sha256.len() != 64 || !f.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            problems.push(format!("{}: sha256 must be 64 hex characters", f.path));
        }
    }
    match &info.pin {
        Some(pin) if !pin.eq_ignore_ascii_case(&info.manifest_sha256) => problems.push(format!(
            "manifest sha256 {} does not match the pin {pin}",
            info.manifest_sha256
        )),
        Some(_) => {}
        None if require_pins() => {
            problems.push("unpinned pack and profile.json sets require_pins".into())
        }
        None => warnings.push(format!(
            "unpinned: integrity checked, authenticity not; pin it with \"pins\": {{\"{}\": \"{}\"}} in profile.json",
            m.name, info.manifest_sha256
        )),
    }
    if m.signature.is_some() {
        warnings.push("signature present but not verified (unsupported); rely on the pin".into());
    }
}

/// Hash every file and check the manifest. Does not load the weights.
pub fn verify(info: &PackInfo) -> VerifyReport {
    let t = std::time::Instant::now();
    let mut problems = Vec::new();
    let mut warnings = Vec::new();
    check_manifest(info, &mut problems, &mut warnings);
    let mut checked = 0;
    for f in &info.manifest.files {
        if !safe_rel(&f.path) {
            continue;
        }
        let path = info.dir.join(&f.path);
        if crate::paths::is_lfs_pointer(&path) {
            problems.push(format!("{}: Git LFS pointer, run git lfs pull", f.path));
            continue;
        }
        match std::fs::metadata(&path) {
            Err(e) => {
                problems.push(format!("{}: {e}", f.path));
                continue;
            }
            Ok(md) if f.bytes.is_some_and(|b| b != md.len()) => {
                problems.push(format!(
                    "{}: {} bytes, manifest says {}",
                    f.path,
                    md.len(),
                    f.bytes.unwrap_or(0)
                ));
                continue;
            }
            Ok(_) => {}
        }
        match crate::doctor::sha256_file(&path) {
            Ok(h) if h.eq_ignore_ascii_case(&f.sha256) => checked += 1,
            Ok(h) => problems.push(format!(
                "{}: sha256 {h} does not match the manifest",
                f.path
            )),
            Err(e) => problems.push(format!("{}: {e}", f.path)),
        }
    }
    VerifyReport {
        name: info.manifest.name.clone(),
        version: info.manifest.version.clone(),
        source: info.source.clone(),
        ok: problems.is_empty(),
        pinned: info.pin.is_some(),
        manifest_sha256: info.manifest_sha256.clone(),
        files_checked: checked,
        problems,
        warnings,
        elapsed_ms: t.elapsed().as_millis() as u64,
    }
}

/// A verified, loaded router pack.
pub struct Loaded {
    pub info: PackInfo,
    pub weights: CognitiveWeights,
    pub warnings: Vec<String>,
}

/// Load a pack by name: manifest checks, hashes of every file (the entry is
/// hashed from the mapped bytes that classification will read), and the
/// entry's labels must equal `categories`.
pub fn load(name: &str) -> Result<Loaded, String> {
    let info = find(name)?;
    let mut problems = Vec::new();
    let mut warnings = Vec::new();
    check_manifest(&info, &mut problems, &mut warnings);
    if !problems.is_empty() {
        return Err(format!("pack {name}: {}", problems.join("; ")));
    }
    for f in info
        .manifest
        .files
        .iter()
        .filter(|f| f.path != info.manifest.entry)
    {
        let path = info.dir.join(&f.path);
        match crate::doctor::sha256_file(&path) {
            Ok(h) if h.eq_ignore_ascii_case(&f.sha256) => {}
            Ok(_) => {
                return Err(format!(
                    "pack {name}: {} was modified (sha256 mismatch)",
                    f.path
                ))
            }
            Err(e) => return Err(format!("pack {name}: {}: {e}", f.path)),
        }
    }
    let entry = info.entry_path();
    let mut weights = CognitiveWeights::load(&entry)
        .map_err(|e| format!("pack {name}: {}: {e}", entry.display()))?;
    let spec = info
        .manifest
        .files
        .iter()
        .find(|f| f.path == info.manifest.entry)
        .cloned()
        .unwrap_or(PackFile {
            path: String::new(),
            bytes: None,
            sha256: String::new(),
        });
    if spec.bytes.is_some_and(|b| b != weights.size_bytes() as u64) {
        return Err(format!(
            "pack {name}: {} is {} bytes, manifest says {}",
            spec.path,
            weights.size_bytes(),
            spec.bytes.unwrap_or(0)
        ));
    }
    let actual = weights.sha256();
    if !actual.eq_ignore_ascii_case(&spec.sha256) {
        return Err(format!(
            "pack {name}: {} was modified (sha256 {actual}, manifest {})",
            spec.path, spec.sha256
        ));
    }
    let labels = weights.label_names();
    if !info.manifest.categories.is_empty() && labels != info.manifest.categories {
        return Err(format!(
            "pack {name}: categories {:?} do not match the weights' labels {labels:?}",
            info.manifest.categories
        ));
    }
    let priors = priors_of(&info.manifest).map_err(|e| format!("pack {name}: {e}"))?;
    if let crate::cognitive::LexicalPriors::Custom(groups) = &priors {
        if let Some((l, _, _)) = groups.iter().find(|(l, _, _)| !labels.contains(l)) {
            return Err(format!(
                "pack {name}: lexical_priors names unknown label {l:?}"
            ));
        }
    }
    weights.set_lexical_priors(priors);
    let mut info = info;
    if info.manifest.calibration.is_none() {
        info.manifest.calibration = external_calibration(&info);
    }
    Ok(Loaded {
        info,
        weights,
        warnings,
    })
}

/// The pack's keyword priors: built-in packs use the built-in table; other
/// packs have none unless their manifest supplies a table.
pub fn priors_of(m: &Manifest) -> Result<crate::cognitive::LexicalPriors, String> {
    use crate::cognitive::LexicalPriors;
    let Some(v) = &m.lexical_priors else {
        return Ok(if BUILTIN.contains(&m.name.as_str()) {
            LexicalPriors::Builtin
        } else {
            LexicalPriors::Off
        });
    };
    match v {
        Value::String(s) if s == "builtin" => Ok(LexicalPriors::Builtin),
        Value::String(s) if s == "off" || s == "none" => Ok(LexicalPriors::Off),
        Value::Bool(false) | Value::Null => Ok(LexicalPriors::Off),
        Value::Object(o) => {
            let points = o.get("points").and_then(Value::as_i64).unwrap_or(48);
            if !(1..=1000).contains(&points) {
                return Err("lexical_priors.points must be 1..1000".into());
            }
            let labels = o
                .get("labels")
                .and_then(Value::as_object)
                .ok_or("lexical_priors.labels must be an object of label → [cue phrases]")?;
            Ok(LexicalPriors::Custom(
                labels
                    .iter()
                    .map(|(l, cues)| (l.clone(), str_list(cues), points as i32))
                    .collect(),
            ))
        }
        _ => Err("lexical_priors must be \"builtin\", \"off\" or {points, labels}".into()),
    }
}

/// Calibration outside the manifest: `config/route-calibration/<name>.json`
/// for built-ins (committed), `calibration.<name>` in `profile.json` for any
/// pack (a path, profile-relative).
fn external_calibration(info: &PackInfo) -> Option<Value> {
    let read = |p: PathBuf| -> Option<Value> {
        std::fs::read(&p)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .filter(|v| v.get("schema").and_then(Value::as_str) == Some(CALIBRATION_SCHEMA))
    };
    let cfg = profile_config();
    if let Some(p) = cfg
        .pointer(&format!("/calibration/{}", info.manifest.name))
        .and_then(Value::as_str)
    {
        return read(crate::profile::resolve(p));
    }
    if info.builtin {
        return read(crate::paths::asset(format!(
            "config/route-calibration/{}.json",
            info.manifest.name
        )));
    }
    None
}

pub const CALIBRATION_SCHEMA: &str = "perci.route-calibration.v1";

/// Margin bucket lower edges used by `perci packs calibrate`.
pub const MARGIN_BUCKETS: &[i32] = &[0, 1, 2, 4, 8, 16, 32, 64];

/// Wilson 95% lower bound of `correct / n`, in permille.
pub fn wilson_lower_permille(correct: u64, n: u64) -> u32 {
    if n == 0 {
        return 0;
    }
    let (c, n) = (correct as f64, n as f64);
    let z = 1.96f64;
    let p = c / n;
    let denom = 1.0 + z * z / n;
    let centre = p + z * z / (2.0 * n);
    let half = z * ((p * (1.0 - p) + z * z / (4.0 * n)) / n).sqrt();
    (((centre - half) / denom).max(0.0) * 1000.0).floor() as u32
}

/// What a calibration table says about one margin: the bucket holding it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibratedConfidence {
    pub bucket_min_margin: i32,
    pub n: u64,
    pub correct: u64,
    pub accuracy_permille: u32,
    /// Wilson 95% lower bound: the number gates use.
    pub lower_permille: u32,
}

pub fn calibrated_confidence(cal: &Value, margin: i32) -> Option<CalibratedConfidence> {
    let buckets = cal.get("buckets")?.as_array()?;
    let b = buckets
        .iter()
        .filter(|b| {
            b["min_margin"]
                .as_i64()
                .is_some_and(|m| m <= i64::from(margin))
        })
        .max_by_key(|b| b["min_margin"].as_i64().unwrap_or(i64::MIN))?;
    let n = b["n"].as_u64()?;
    let correct = b["correct"].as_u64()?;
    Some(CalibratedConfidence {
        bucket_min_margin: b["min_margin"].as_i64()? as i32,
        n,
        correct,
        accuracy_permille: (correct * 1000).checked_div(n).unwrap_or(0) as u32,
        lower_permille: wilson_lower_permille(correct, n),
    })
}

/// Labelled cases from JSONL: text in `text` or `prompt`, gold label in
/// `label`, `expected_label` or `expected_domain`.
fn read_cases(paths: &[String]) -> Result<(Vec<(String, String)>, String), String> {
    let mut out = Vec::new();
    let mut all = Vec::new();
    for p in paths {
        let bytes = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
        all.extend_from_slice(&bytes);
        for line in String::from_utf8_lossy(&bytes).lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let text = v
                .get("text")
                .or_else(|| v.get("prompt"))
                .and_then(Value::as_str);
            let label = v
                .get("label")
                .or_else(|| v.get("expected_label"))
                .or_else(|| v.get("expected_domain"))
                .and_then(Value::as_str);
            if let (Some(t), Some(l)) = (text, label) {
                out.push((t.to_owned(), l.to_owned()));
            }
        }
    }
    Ok((out, crate::doctor::sha256_hex(&all)))
}

/// Measure accuracy by margin bucket on held-out labelled cases.
pub fn calibrate(name: &str, case_files: &[String]) -> Result<Value, String> {
    let loaded = load(name)?;
    let (cases, cases_sha) = read_cases(case_files)?;
    if cases.is_empty() {
        return Err("no labelled cases (need text|prompt and label|expected_domain)".into());
    }
    let labels = loaded.weights.label_names();
    let mut n = vec![0u64; MARGIN_BUCKETS.len()];
    let mut ok = vec![0u64; MARGIN_BUCKETS.len()];
    let mut unknown_gold = 0u64;
    let mut wrong_margins: Vec<i32> = Vec::new();
    for (text, gold) in &cases {
        if !labels.contains(gold) {
            unknown_gold += 1;
            continue;
        }
        let m = loaded
            .weights
            .classify(text)
            .map_err(|e| format!("classify: {e}"))?;
        let i = MARGIN_BUCKETS
            .iter()
            .rposition(|b| *b <= m.margin.max(0))
            .unwrap_or(0);
        n[i] += 1;
        if &m.label == gold {
            ok[i] += 1;
        } else {
            wrong_margins.push(m.margin);
        }
    }
    let total: u64 = n.iter().sum();
    let correct: u64 = ok.iter().sum();
    wrong_margins.sort_unstable();
    Ok(json!({
        "schema": CALIBRATION_SCHEMA,
        "pack": loaded.info.identity(),
        "cases": total,
        "cases_skipped_unknown_label": unknown_gold,
        "cases_sha256": cases_sha,
        "accuracy_permille": (correct * 1000).checked_div(total).unwrap_or(0),
        "max_wrong_margin": wrong_margins.last(),
        "buckets": MARGIN_BUCKETS.iter().enumerate().map(|(i, b)| json!({
            "min_margin": b, "n": n[i], "correct": ok[i],
            "lower_permille": wilson_lower_permille(ok[i], n[i]),
        })).collect::<Vec<_>>(),
        "note": "Accuracy of the top label by margin bucket on these held-out cases. Gates use the Wilson 95% lower bound; small buckets are deliberately pessimistic.",
    }))
}

/// Default router pack for tools: `PERCI_TOOL_PACK`, else `tool_pack` in
/// `profile.json`, else an index entry marked `"default": true` with role
/// route, else `v0.2`. Never falls back silently: a named pack that does not
/// load is an error.
pub fn default_name() -> String {
    if let Ok(p) = std::env::var("PERCI_TOOL_PACK") {
        if !p.trim().is_empty() {
            return p.trim().to_owned();
        }
    }
    if let Some(p) = profile_config().get("tool_pack").and_then(Value::as_str) {
        return p.to_owned();
    }
    if let Some(info) = discover()
        .into_iter()
        .flatten()
        .find(|i| i.default && i.manifest.kind == "bitwork-router")
    {
        return info.manifest.name;
    }
    "v0.2".into()
}

/// Write `dir/perci-pack.json` for the files in `dir` (hashes computed now).
pub fn write_manifest(
    dir: &Path,
    name: &str,
    version: &str,
    entry: &str,
) -> Result<PathBuf, String> {
    if !valid_name(name) || BUILTIN.contains(&name) {
        return Err(format!("invalid or reserved pack name {name:?}"));
    }
    let entry_path = dir.join(entry);
    let weights = CognitiveWeights::load(&entry_path)
        .map_err(|e| format!("{}: {e}", entry_path.display()))?;
    let mut files = Vec::new();
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.file_name().is_some_and(|n| n != MANIFEST_FILE))
        .collect();
    names.sort();
    for p in names {
        let rel = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let bytes = std::fs::metadata(&p).map_err(|e| e.to_string())?.len();
        let sha256 = crate::doctor::sha256_file(&p).map_err(|e| e.to_string())?;
        files.push(PackFile {
            path: rel,
            bytes: Some(bytes),
            sha256,
        });
    }
    let manifest = Manifest {
        schema: SCHEMA.into(),
        name: name.into(),
        version: version.into(),
        kind: default_kind(),
        entry: entry.into(),
        categories: weights.label_names(),
        files,
        description: None,
        signature: None,
        lexical_priors: Some(json!("off")),
        calibration: None,
    };
    let out = dir.join(MANIFEST_FILE);
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(out)
}

/// JSON row for `perci packs list`.
pub fn list_row(info: &PackInfo) -> Value {
    json!({
        "name": info.manifest.name,
        "version": info.manifest.version,
        "source": info.source,
        "dir": info.dir.display().to_string(),
        "entry": info.manifest.entry,
        "categories": info.manifest.categories.len(),
        "files": info.manifest.files.len(),
        "manifest_sha256": info.manifest_sha256,
        "pinned": info.pin.is_some(),
    })
}

/// `perci packs list|verify [NAME…]|manifest DIR --name N --version V [--entry F]` `[--json]`.
pub fn run_cli(args: &[String]) -> i32 {
    let json_out = args.iter().any(|a| a == "--json");
    let rest: Vec<&String> = args.iter().filter(|a| a.as_str() != "--json").collect();
    let sub = rest.first().map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" => {
            let all = discover();
            if json_out {
                let rows: Vec<Value> = all
                    .iter()
                    .map(|r| match r {
                        Ok(i) => list_row(i),
                        Err(e) => json!({"error": e}),
                    })
                    .collect();
                println!(
                    "{}",
                    json!({"schema": "perci.packs.v1", "default": default_name(), "packs": rows})
                );
            } else {
                println!("default tool pack: {}", default_name());
                for r in &all {
                    match r {
                        Ok(i) => println!(
                            "{:<16} {:<28} {:<10} {} files  {}",
                            i.manifest.name,
                            i.manifest.version,
                            if i.pin.is_some() {
                                "pinned"
                            } else {
                                "unpinned"
                            },
                            i.manifest.files.len(),
                            i.source
                        ),
                        Err(e) => println!("ERROR {e}"),
                    }
                }
            }
            i32::from(all.iter().any(|r| r.is_err()))
        }
        "verify" => {
            let names: Vec<String> = rest.iter().skip(1).map(|s| s.to_string()).collect();
            let mut reports = Vec::new();
            let mut errors = Vec::new();
            if names.is_empty() {
                for r in discover() {
                    match r {
                        Ok(i) => reports.push(verify(&i)),
                        Err(e) => errors.push(e),
                    }
                }
            } else {
                for n in &names {
                    match find(n) {
                        Ok(i) => reports.push(verify(&i)),
                        Err(e) => errors.push(e),
                    }
                }
            }
            let ok = errors.is_empty() && reports.iter().all(|r| r.ok);
            if json_out {
                println!(
                    "{}",
                    json!({"schema": "perci.packs-verify.v1", "ok": ok, "packs": reports, "errors": errors})
                );
            } else {
                for r in &reports {
                    println!(
                        "{} {}@{}  {} file(s) hashed in {} ms  {}",
                        if r.ok { "OK  " } else { "FAIL" },
                        r.name,
                        r.version,
                        r.files_checked,
                        r.elapsed_ms,
                        if r.pinned { "pinned" } else { "unpinned" }
                    );
                    for p in &r.problems {
                        println!("     problem: {p}");
                    }
                    for w in &r.warnings {
                        println!("     warning: {w}");
                    }
                }
                for e in &errors {
                    println!("FAIL {e}");
                }
            }
            i32::from(!ok)
        }
        "calibrate" => {
            let name = rest.get(1).map(|s| s.to_string());
            let out = rest
                .iter()
                .position(|a| a.as_str() == "--out")
                .and_then(|i| rest.get(i + 1))
                .map(|s| s.to_string());
            let files: Vec<String> = rest
                .iter()
                .skip(2)
                .filter(|a| a.as_str() != "--out" && Some(a.as_str()) != out.as_deref())
                .map(|s| s.to_string())
                .collect();
            let (Some(name), false) = (name, files.is_empty()) else {
                eprintln!("usage: perci packs calibrate NAME CASES.jsonl… [--out FILE]");
                return 2;
            };
            match calibrate(&name, &files) {
                Ok(v) => {
                    let text = serde_json::to_string_pretty(&v).unwrap_or_default() + "\n";
                    match out {
                        Some(o) => match std::fs::write(&o, text) {
                            Ok(()) => println!("wrote {o}"),
                            Err(e) => {
                                eprintln!("perci packs calibrate: {o}: {e}");
                                return 1;
                            }
                        },
                        None => print!("{text}"),
                    }
                    0
                }
                Err(e) => {
                    eprintln!("perci packs calibrate: {e}");
                    1
                }
            }
        }
        "manifest" => {
            let dir = rest.get(1).map(PathBuf::from);
            let get = |k: &str| {
                rest.iter()
                    .position(|a| a.as_str() == k)
                    .and_then(|i| rest.get(i + 1))
                    .map(|s| s.to_string())
            };
            let (Some(dir), Some(name), Some(version)) = (dir, get("--name"), get("--version"))
            else {
                eprintln!("usage: perci packs manifest DIR --name NAME --version VERSION [--entry FILE.pwgt]");
                return 2;
            };
            let entry = get("--entry").or_else(|| {
                std::fs::read_dir(&dir).ok().and_then(|d| {
                    let mut p: Vec<String> = d
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .filter(|n| n.ends_with(".pwgt"))
                        .collect();
                    p.sort();
                    (p.len() == 1).then(|| p.remove(0))
                })
            });
            let Some(entry) = entry else {
                eprintln!("perci packs manifest: pass --entry (need exactly one .pwgt or an explicit entry)");
                return 2;
            };
            match write_manifest(&dir, &name, &version, &entry) {
                Ok(p) => {
                    let sha = std::fs::read(&p)
                        .map(|b| crate::doctor::sha256_hex(&b))
                        .unwrap_or_default();
                    println!(
                        "wrote {}\nmanifest sha256 {sha} (pin this in profile.json)",
                        p.display()
                    );
                    0
                }
                Err(e) => {
                    eprintln!("perci packs manifest: {e}");
                    1
                }
            }
        }
        _ => {
            eprintln!("usage: perci packs list|verify [NAME…]|manifest DIR --name N --version V [--entry F]|calibrate NAME CASES.jsonl… [--out F] [--json]");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_static_checks() {
        let info = PackInfo {
            manifest: Manifest {
                schema: SCHEMA.into(),
                name: "t".into(),
                version: "1".into(),
                kind: "bitwork-router".into(),
                entry: "w.pwgt".into(),
                categories: vec!["a".into()],
                files: vec![PackFile {
                    path: "../w.pwgt".into(),
                    bytes: Some(1),
                    sha256: "00".into(),
                }],
                description: None,
                signature: None,
                lexical_priors: None,
                calibration: None,
            },
            dir: PathBuf::from("/nonexistent"),
            manifest_sha256: "ab".repeat(32),
            source: "test".into(),
            pin: Some("cd".repeat(32)),
            builtin: false,
            default: false,
        };
        let r = verify(&info);
        assert!(!r.ok);
        let all = r.problems.join("\n");
        assert!(all.contains("entry w.pwgt is not listed"), "{all}");
        assert!(all.contains("inside the pack folder"), "{all}");
        assert!(all.contains("64 hex"), "{all}");
        assert!(all.contains("does not match the pin"), "{all}");
        assert!(safe_rel("a/b.pwgt") && !safe_rel("/etc/x") && !safe_rel("a/../../x"));
        assert!(valid_name("acme-router_1.0") && !valid_name("bad name") && !valid_name(""));
    }
}
