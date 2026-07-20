//! Governed injection for Perci evolve (HYDRA essentials, pure Rust).
//!
//! Extracted from HYDRA Injector theory for in-repo use only:
//!
//! ```text
//! anchor → inject → retract → seal → report
//! No anchor, no injection.
//! No boundary, no promotion.
//! No seal, no trust.
//! ```
//!
//! Two surfaces:
//! 1. **Codeweave** — marker-bound text injection with reviewable unified diffs.
//! 2. **Residual field** — small 2D f64 operator for BRPC-style stress telemetry.
//!
//! Never executes injected code. Never auto-promotes `.pwgt`.
//! Default apply path is dry-run (plan only).

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER_TAG: &str = "HYDRA-INJECT";
const DEFAULT_MAX_BYTES: usize = 16_000;

const FORBIDDEN: &[&str] = &[
    "eval(",
    "exec(",
    "os.system",
    "subprocess.",
    "Invoke-Expression",
    "std::process::Command",
    "rm -rf",
    "Remove-Item -Recurse",
];

const ALLOW_EXT: &[&str] = &[
    ".rs", ".py", ".md", ".json", ".toml", ".yml", ".yaml", ".txt", ".jsonl",
];

// ─── Codeweave ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeInjectSpec {
    pub target_file: String,
    pub marker: String,
    pub code: String,
    #[serde(default)]
    pub name: String,
    /// before | after | replace
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default)]
    pub rationale: String,
    #[serde(default = "default_profile")]
    pub profile: String,
}

fn default_mode() -> String {
    "after".into()
}
fn default_root() -> String {
    ".".into()
}
fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES
}
fn default_profile() -> String {
    "library".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeInjectResult {
    pub target_file: String,
    pub admissible: bool,
    pub applied: bool,
    pub dry_run: bool,
    pub diff: String,
    pub rollback_diff: String,
    pub warnings: Vec<String>,
    pub metrics: CodeInjectMetrics,
    pub session_id: String,
    pub rationale: String,
    pub risk_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeInjectMetrics {
    pub original_bytes: usize,
    pub injected_bytes: usize,
    pub diff_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkerHit {
    pub file: String,
    pub line: usize,
    pub marker: String,
    pub name: String,
    pub profile: String,
    pub is_slot: bool,
}

// ─── Residual field ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    #[serde(default = "default_target_volume")]
    pub target_volume: f64,
    #[serde(default = "default_retract")]
    pub retract_fraction: f64,
    #[serde(default = "default_pin")]
    pub pin_strength: f64,
    #[serde(default = "default_band")]
    pub boundary_band: usize,
    #[serde(default = "default_seal_steps")]
    pub seal_steps: usize,
    #[serde(default = "default_seal_alpha")]
    pub seal_alpha: f64,
}

fn default_target_volume() -> f64 {
    1.0
}
fn default_retract() -> f64 {
    0.25
}
fn default_pin() -> f64 {
    0.35
}
fn default_band() -> usize {
    1
}
fn default_seal_steps() -> usize {
    8
}
fn default_seal_alpha() -> f64 {
    0.35
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            target_volume: default_target_volume(),
            retract_fraction: default_retract(),
            pin_strength: default_pin(),
            boundary_band: default_band(),
            seal_steps: default_seal_steps(),
            seal_alpha: default_seal_alpha(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldResult {
    pub field: Vec<Vec<f64>>,
    pub metrics: FieldMetrics,
    pub admissible: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetrics {
    pub mean: f64,
    pub std: f64,
    pub mass: f64,
    pub curvature_rms: f64,
    pub omega: f64,
    pub nontriviality: f64,
    pub bounded: f64,
}

// ─── Public API ──────────────────────────────────────────────────────────────

pub fn session_id(seed: &str) -> String {
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut h: u64 = 0xcbf29ce484222325;
    for b in seed.bytes().chain(t.to_le_bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("perci-hydra-{:016x}", h)
}

pub fn discover_markers(root: &Path) -> io::Result<Vec<MarkerHit>> {
    let mut out = Vec::new();
    walk_markers(root, root, &mut out)?;
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    Ok(out)
}

fn walk_markers(root: &Path, dir: &Path, out: &mut Vec<MarkerHit>) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.')
            || name == "target"
            || name == "node_modules"
            || name == "__pycache__"
        {
            continue;
        }
        if path.is_dir() {
            walk_markers(root, &path, out)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        if !ALLOW_EXT.contains(&ext.as_str()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains(MARKER_TAG) {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        for (i, line) in text.lines().enumerate() {
            if !line.contains(MARKER_TAG) {
                continue;
            }
            let meta = parse_marker_metadata(line);
            let stripped = line.trim();
            let is_slot = (stripped.starts_with('#')
                || stripped.starts_with("//")
                || stripped.starts_with("/*")
                || stripped.starts_with("<!--"))
                && stripped.contains("HYDRA-INJECT:");
            out.push(MarkerHit {
                file: rel.clone(),
                line: i + 1,
                marker: stripped.to_string(),
                name: meta.0,
                profile: meta.1,
                is_slot,
            });
        }
    }
    Ok(())
}

fn parse_marker_metadata(line: &str) -> (String, String) {
    let mut name = String::new();
    let mut profile = String::new();
    for part in line.split_whitespace() {
        if let Some(v) = part.strip_prefix("name=") {
            name = v.to_string();
        }
        if let Some(v) = part.strip_prefix("profile=") {
            profile = v.to_string();
        }
    }
    (name, profile)
}

pub fn plan_code_injection(spec: &CodeInjectSpec) -> CodeInjectResult {
    let sid = session_id(&format!("{}:{}", spec.target_file, spec.marker));
    let root = PathBuf::from(&spec.root);
    let root = if root.is_absolute() {
        root
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    };
    let target = root.join(&spec.target_file);
    let mut warnings = validate_spec(spec, &root, &target);
    if !warnings.is_empty() {
        return CodeInjectResult {
            target_file: target.display().to_string(),
            admissible: false,
            applied: false,
            dry_run: true,
            diff: String::new(),
            rollback_diff: String::new(),
            warnings,
            metrics: CodeInjectMetrics::default(),
            session_id: sid,
            rationale: spec.rationale.clone(),
            risk_score: risk_score(spec),
        };
    }
    let original = match fs::read_to_string(&target) {
        Ok(t) => t,
        Err(e) => {
            warnings.push(format!("read failed: {e}"));
            return CodeInjectResult {
                target_file: target.display().to_string(),
                admissible: false,
                applied: false,
                dry_run: true,
                diff: String::new(),
                rollback_diff: String::new(),
                warnings,
                metrics: CodeInjectMetrics::default(),
                session_id: sid,
                rationale: spec.rationale.clone(),
                risk_score: risk_score(spec),
            };
        }
    };
    let updated = match inject_text(&original, &spec.marker, &spec.code, &spec.mode) {
        Ok(u) => u,
        Err(e) => {
            warnings.push(e);
            return CodeInjectResult {
                target_file: target.display().to_string(),
                admissible: false,
                applied: false,
                dry_run: true,
                diff: String::new(),
                rollback_diff: String::new(),
                warnings,
                metrics: CodeInjectMetrics::default(),
                session_id: sid,
                rationale: spec.rationale.clone(),
                risk_score: risk_score(spec),
            };
        }
    };
    let diff = unified_diff(&original, &updated, &spec.target_file);
    let rollback = unified_diff(&updated, &original, &spec.target_file);
    let metrics = CodeInjectMetrics {
        original_bytes: original.len(),
        injected_bytes: spec.code.len(),
        diff_lines: diff.lines().count(),
    };
    CodeInjectResult {
        target_file: target.display().to_string(),
        admissible: true,
        applied: false,
        dry_run: true,
        diff,
        rollback_diff: rollback,
        warnings,
        metrics,
        session_id: sid,
        rationale: spec.rationale.clone(),
        risk_score: risk_score(spec),
    }
}

/// Apply only when `dry_run` is false and the plan is admissible.
pub fn apply_code_injection(spec: &CodeInjectSpec, dry_run: bool) -> CodeInjectResult {
    let mut result = plan_code_injection(spec);
    result.dry_run = dry_run;
    if dry_run || !result.admissible || result.diff.is_empty() {
        return result;
    }
    let target = PathBuf::from(&result.target_file);
    // Rebuild after text for write.
    let original = match fs::read_to_string(&target) {
        Ok(t) => t,
        Err(e) => {
            result.warnings.push(format!("apply read failed: {e}"));
            result.admissible = false;
            return result;
        }
    };
    let updated = match inject_text(&original, &spec.marker, &spec.code, &spec.mode) {
        Ok(u) => u,
        Err(e) => {
            result.warnings.push(e);
            result.admissible = false;
            return result;
        }
    };
    match fs::write(&target, updated) {
        Ok(()) => {
            result.applied = true;
            result.dry_run = false;
        }
        Err(e) => {
            result.warnings.push(format!("write failed: {e}"));
            result.admissible = false;
        }
    }
    result
}

fn validate_spec(spec: &CodeInjectSpec, root: &Path, target: &Path) -> Vec<String> {
    let mut w = Vec::new();
    if let Ok(canon_root) = root.canonicalize() {
        match target.canonicalize() {
            Ok(canon_t) => {
                if !canon_t.starts_with(&canon_root) {
                    w.push("target file escapes configured root".into());
                }
            }
            Err(_) => {
                // may not exist yet
                if !target.exists() {
                    w.push("target file does not exist".into());
                }
            }
        }
    }
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    if !ALLOW_EXT.contains(&ext.as_str()) {
        w.push(format!("extension {ext:?} is not allowed"));
    }
    if spec.marker.is_empty() {
        w.push("marker is required".into());
    }
    if spec.code.len() > spec.max_bytes {
        w.push("code exceeds max_bytes".into());
    }
    if (spec.profile == "strict" || spec.profile == "library") && spec.rationale.trim().is_empty() {
        w.push(format!("profile {:?} requires rationale", spec.profile));
    }
    let low = spec.code.to_ascii_lowercase();
    for pat in FORBIDDEN {
        if low.contains(&pat.to_ascii_lowercase()) {
            w.push(format!("forbidden pattern matched: {pat}"));
        }
    }
    if target.is_file() {
        if let Ok(text) = fs::read_to_string(target) {
            if !text.contains(&spec.marker) {
                w.push("marker not found in target file".into());
            } else if let Some(line) = text.lines().find(|l| l.contains(&spec.marker)) {
                let (n, p) = parse_marker_metadata(line);
                if !spec.name.is_empty() && !n.is_empty() && n != spec.name {
                    w.push(format!(
                        "marker name mismatch: expected {:?}, found {:?}",
                        spec.name, n
                    ));
                }
                if !p.is_empty() && p != spec.profile {
                    w.push(format!(
                        "marker profile mismatch: expected {:?}, found {:?}",
                        spec.profile, p
                    ));
                }
            }
        }
    }
    w
}

pub fn inject_text(original: &str, marker: &str, code: &str, mode: &str) -> Result<String, String> {
    if !original.contains(marker) {
        return Err("marker not found in target file".into());
    }
    let insertion = format!("{}\n", code.trim_matches('\n'));
    match mode {
        "replace" => Ok(original.replacen(marker, code, 1)),
        "before" => Ok(original.replacen(marker, &format!("{insertion}{marker}"), 1)),
        "after" => {
            let with_nl = format!("{marker}\n");
            if original.contains(&with_nl) {
                Ok(original.replacen(&with_nl, &format!("{with_nl}{insertion}"), 1))
            } else {
                Ok(original.replacen(marker, &format!("{marker}\n{insertion}"), 1))
            }
        }
        other => Err(format!("mode must be before|after|replace, got {other}")),
    }
}

pub fn unified_diff(before: &str, after: &str, name: &str) -> String {
    if before == after {
        return String::new();
    }
    let b: Vec<&str> = before.lines().collect();
    let a: Vec<&str> = after.lines().collect();
    let mut out = String::new();
    out.push_str(&format!("--- {name}:before\n+++ {name}:after\n"));
    // Two-pointer line diff (handles pure insertions cleanly).
    let mut i = 0usize;
    let mut j = 0usize;
    while i < b.len() || j < a.len() {
        if i < b.len() && j < a.len() && b[i] == a[j] {
            i += 1;
            j += 1;
            continue;
        }
        // Find next common line
        let mut bi = i;
        let mut aj = j;
        let mut found = false;
        let mut best = (b.len(), a.len());
        let limit = 40.min(b.len().saturating_sub(i)).max(1);
        for di in 0..=limit {
            if i + di >= b.len() {
                break;
            }
            for dj in 0..=limit {
                if j + dj >= a.len() {
                    break;
                }
                if b[i + di] == a[j + dj] {
                    best = (i + di, j + dj);
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        if found {
            bi = best.0;
            aj = best.1;
        } else {
            bi = b.len();
            aj = a.len();
        }
        let del_n = bi.saturating_sub(i);
        let add_n = aj.saturating_sub(j);
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            i + 1,
            del_n.max(if add_n > 0 && del_n == 0 { 0 } else { 1 }),
            j + 1,
            add_n.max(if del_n > 0 && add_n == 0 { 0 } else { 1 })
        ));
        for k in i..bi {
            out.push_str(&format!("-{}\n", b[k]));
        }
        for k in j..aj {
            out.push_str(&format!("+{}\n", a[k]));
        }
        i = bi;
        j = aj;
    }
    out
}

fn risk_score(spec: &CodeInjectSpec) -> f64 {
    let profile_w = match spec.profile.as_str() {
        "strict" => 0.05,
        "library" => 0.12,
        "docs" => 0.08,
        "experimental" => 0.35,
        _ => 0.2,
    };
    let ext = Path::new(&spec.target_file)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let ext_w = if matches!(ext, "md" | "txt" | "json" | "toml" | "yml" | "yaml") {
        0.05
    } else if ext == "rs" {
        0.2
    } else {
        0.15
    };
    let size_w = (spec.code.len() as f64 / spec.max_bytes.max(1) as f64 * 0.35).min(0.35);
    let mode_w = if spec.mode == "replace" { 0.1 } else { 0.03 };
    (profile_w + ext_w + size_w + mode_w).min(1.0)
}

// ─── Residual field operator (pure f64) ──────────────────────────────────────

pub fn residual_field(
    mask: &[Vec<bool>],
    field: &[Vec<f64>],
    cfg: &FieldConfig,
) -> Result<FieldResult, String> {
    let rows = mask.len();
    if rows == 0 {
        return Err("mask empty".into());
    }
    let cols = mask[0].len();
    if field.len() != rows || field.iter().any(|r| r.len() != cols) {
        return Err("field/mask shape mismatch".into());
    }
    if !mask.iter().flatten().any(|&m| m) {
        return Err("mask must contain at least one admissible cell".into());
    }

    let mut injected = vec![vec![0.0; cols]; rows];
    let mut mass = 0.0;
    for r in 0..rows {
        for c in 0..cols {
            if mask[r][c] {
                mass += field[r][c].abs();
            }
        }
    }
    let n_mask = mask.iter().flatten().filter(|&&m| m).count() as f64;
    for r in 0..rows {
        for c in 0..cols {
            if mask[r][c] {
                injected[r][c] = if mass <= 1e-12 {
                    cfg.target_volume / n_mask
                } else {
                    field[r][c] * (cfg.target_volume / mass)
                };
            }
        }
    }

    // pin boundary band toward mean
    let mut pinned = injected.clone();
    let mean = mean_mask(&pinned, mask);
    let band = boundary_band(mask, cfg.boundary_band);
    let strength = cfg.pin_strength.clamp(0.0, 1.0);
    for r in 0..rows {
        for c in 0..cols {
            if band[r][c] {
                pinned[r][c] = (1.0 - strength) * pinned[r][c] + strength * mean;
            }
            if !mask[r][c] {
                pinned[r][c] = 0.0;
            }
        }
    }

    // retract
    let mut retracted = pinned.clone();
    let mean2 = mean_mask(&retracted, mask);
    let frac = cfg.retract_fraction.clamp(0.0, 1.0);
    for r in 0..rows {
        for c in 0..cols {
            if mask[r][c] {
                retracted[r][c] -= frac * mean2;
            } else {
                retracted[r][c] = 0.0;
            }
        }
    }

    // seal smooth
    let mut sealed = retracted;
    let alpha = cfg.seal_alpha.clamp(0.0, 1.0);
    for _ in 0..cfg.seal_steps {
        let avg = neighbor_average(&sealed, mask);
        for r in 0..rows {
            for c in 0..cols {
                if mask[r][c] {
                    sealed[r][c] = (1.0 - alpha) * sealed[r][c] + alpha * avg[r][c];
                } else {
                    sealed[r][c] = 0.0;
                }
            }
        }
    }

    let metrics = residual_metrics(&sealed, mask);
    let mut warnings = Vec::new();
    if metrics.bounded < 1.0 {
        warnings.push("residual contains non-finite values".into());
    }
    if metrics.mass <= 1e-9 {
        warnings.push("residual trivialized to near-zero mass".into());
    }
    if metrics.nontriviality < 0.02 {
        warnings.push("residual may be over-smoothed or structurally trivial".into());
    }
    if cfg.seal_alpha > 0.8 {
        warnings.push("seal_alpha is high; smoothness may be mistaken for validity".into());
    }
    let admissible = warnings.is_empty();
    Ok(FieldResult {
        field: sealed,
        metrics,
        admissible,
        warnings,
    })
}

/// Map BRPC-like factor values in [0,1] to residual stress field (1−v).
pub fn brpc_stress_field(values: &[f64]) -> (Vec<Vec<bool>>, Vec<Vec<f64>>) {
    let mut vals: Vec<f64> = values
        .iter()
        .map(|v| (1.0 - v.clamp(0.0, 1.0)).max(0.02))
        .collect();
    while vals.len() < 9 {
        let m = vals.iter().sum::<f64>() / vals.len().max(1) as f64;
        vals.push(m);
    }
    let field = vec![
        vec![vals[0], vals[1], vals[2]],
        vec![vals[3], vals[4], vals[5]],
        vec![vals[6], vals[7], vals[8]],
    ];
    let mask = vec![vec![true; 3]; 3];
    (mask, field)
}

fn mean_mask(field: &[Vec<f64>], mask: &[Vec<bool>]) -> f64 {
    let mut s = 0.0;
    let mut n = 0.0;
    for r in 0..field.len() {
        for c in 0..field[0].len() {
            if mask[r][c] {
                s += field[r][c];
                n += 1.0;
            }
        }
    }
    if n > 0.0 {
        s / n
    } else {
        0.0
    }
}

fn boundary_band(mask: &[Vec<bool>], width: usize) -> Vec<Vec<bool>> {
    let rows = mask.len();
    let cols = mask[0].len();
    if width == 0 {
        return vec![vec![false; cols]; rows];
    }
    let mut eroded = mask.to_vec();
    for _ in 0..width {
        let mut next = eroded.clone();
        for r in 0..rows {
            for c in 0..cols {
                if !eroded[r][c] {
                    next[r][c] = false;
                    continue;
                }
                let up = r > 0 && eroded[r - 1][c];
                let down = r + 1 < rows && eroded[r + 1][c];
                let left = c > 0 && eroded[r][c - 1];
                let right = c + 1 < cols && eroded[r][c + 1];
                next[r][c] = up && down && left && right;
                if r == 0 || r + 1 == rows || c == 0 || c + 1 == cols {
                    next[r][c] = false;
                }
            }
        }
        eroded = next;
    }
    let mut band = vec![vec![false; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            band[r][c] = mask[r][c] && !eroded[r][c];
        }
    }
    band
}

fn neighbor_average(field: &[Vec<f64>], mask: &[Vec<bool>]) -> Vec<Vec<f64>> {
    let rows = field.len();
    let cols = field[0].len();
    let mut out = field.to_vec();
    for r in 0..rows {
        for c in 0..cols {
            if !mask[r][c] {
                out[r][c] = 0.0;
                continue;
            }
            let mut s = 0.0;
            let mut n = 0.0;
            for (dr, dc) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let rr = r as i32 + dr;
                let cc = c as i32 + dc;
                if rr < 0 || cc < 0 || rr as usize >= rows || cc as usize >= cols {
                    continue;
                }
                let rr = rr as usize;
                let cc = cc as usize;
                if mask[rr][cc] {
                    s += field[rr][cc];
                    n += 1.0;
                }
            }
            out[r][c] = if n > 0.0 { s / n } else { field[r][c] };
        }
    }
    out
}

fn residual_metrics(field: &[Vec<f64>], mask: &[Vec<bool>]) -> FieldMetrics {
    let mut vals = Vec::new();
    for r in 0..field.len() {
        for c in 0..field[0].len() {
            if mask[r][c] {
                vals.push(field[r][c]);
            }
        }
    }
    let mean = if vals.is_empty() {
        0.0
    } else {
        vals.iter().sum::<f64>() / vals.len() as f64
    };
    let var = if vals.is_empty() {
        0.0
    } else {
        vals.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / vals.len() as f64
    };
    let std = var.sqrt();
    let mass = vals.iter().map(|v| v.abs()).sum::<f64>();
    let lap = laplacian(field);
    let mut cvals = Vec::new();
    for r in 0..field.len() {
        for c in 0..field[0].len() {
            if mask[r][c] {
                cvals.push(lap[r][c]);
            }
        }
    }
    let curvature_rms = if cvals.is_empty() {
        0.0
    } else {
        (cvals.iter().map(|v| v * v).sum::<f64>() / cvals.len() as f64).sqrt()
    };
    let omega = 1.0 / (1.0 + curvature_rms.abs());
    let nontriviality = if vals.is_empty() {
        0.0
    } else {
        (std / (mean.abs() + std + 1e-12)).min(1.0)
    };
    let bounded = if vals.iter().all(|v| v.is_finite()) {
        1.0
    } else {
        0.0
    };
    FieldMetrics {
        mean,
        std,
        mass,
        curvature_rms,
        omega,
        nontriviality,
        bounded,
    }
}

fn laplacian(field: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = field.len();
    let cols = field[0].len();
    let mut out = vec![vec![0.0; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            let up = if r > 0 { field[r - 1][c] } else { field[r][c] };
            let down = if r + 1 < rows {
                field[r + 1][c]
            } else {
                field[r][c]
            };
            let left = if c > 0 { field[r][c - 1] } else { field[r][c] };
            let right = if c + 1 < cols {
                field[r][c + 1]
            } else {
                field[r][c]
            };
            out[r][c] = up + down + left + right - 4.0 * field[r][c];
        }
    }
    out
}

/// Load BRPC receipt factors if present; return P,M,B,R,K,U,D values.
pub fn load_brpc_factor_values(path: &Path) -> Option<Vec<f64>> {
    let text = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let factors = v.get("factors")?;
    let order = ["P", "M", "B", "R", "K", "U", "D"];
    let mut out = Vec::new();
    for k in order {
        let val = factors
            .get(k)
            .and_then(|f| f.get("value"))
            .and_then(|x| x.as_f64())
            .unwrap_or(0.5);
        out.push(val);
    }
    Some(out)
}

pub fn write_json_pretty(path: &Path, value: &impl Serialize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let s = serde_json::to_string_pretty(value).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(path, s + "\n")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn inject_after_marker() {
        let original = "// HYDRA-INJECT:slot name=demo profile=library\nfn main() {}\n";
        let marker = "// HYDRA-INJECT:slot name=demo profile=library";
        let code = "// injected note";
        let out = inject_text(original, marker, code, "after").unwrap();
        assert!(out.contains("injected note"));
        assert!(out.find(marker).unwrap() < out.find("injected note").unwrap());
    }

    #[test]
    fn forbidden_blocks_eval() {
        let dir = std::env::temp_dir().join("perci-hydra-test");
        let _ = fs::create_dir_all(&dir);
        let target = dir.join("slot.py");
        let mut f = fs::File::create(&target).unwrap();
        writeln!(f, "# HYDRA-INJECT:slot name=demo profile=library").unwrap();
        let spec = CodeInjectSpec {
            target_file: target.file_name().unwrap().to_string_lossy().into(),
            marker: "# HYDRA-INJECT:slot name=demo profile=library".into(),
            code: "eval('x')\n".into(),
            name: "demo".into(),
            mode: "after".into(),
            root: dir.to_string_lossy().into(),
            max_bytes: 8000,
            rationale: "test".into(),
            profile: "library".into(),
        };
        let r = plan_code_injection(&spec);
        assert!(!r.admissible);
        assert!(r.warnings.iter().any(|w| w.contains("forbidden")));
    }

    #[test]
    fn residual_field_admissible_on_stress() {
        let vals = vec![0.78, 0.78, 0.84, 0.78, 1.0, 0.89, 1.0];
        let (mask, field) = brpc_stress_field(&vals);
        let r = residual_field(&mask, &field, &FieldConfig::default()).unwrap();
        assert!(r.metrics.bounded > 0.9);
        assert!(r.metrics.omega > 0.5);
    }

    #[test]
    fn plan_dry_run_does_not_write() {
        let dir = std::env::temp_dir().join("perci-hydra-plan");
        let _ = fs::create_dir_all(&dir);
        let target = dir.join("note.md");
        fs::write(
            &target,
            "# HYDRA-INJECT:slot name=demo profile=docs\n\nbody\n",
        )
        .unwrap();
        let before = fs::read_to_string(&target).unwrap();
        let spec = CodeInjectSpec {
            target_file: "note.md".into(),
            marker: "# HYDRA-INJECT:slot name=demo profile=docs".into(),
            code: "\n// candidate\n".into(),
            name: "demo".into(),
            mode: "after".into(),
            root: dir.to_string_lossy().into(),
            max_bytes: 8000,
            rationale: String::new(),
            profile: "docs".into(),
        };
        let r = apply_code_injection(&spec, true);
        assert!(r.admissible);
        assert!(!r.applied);
        assert_eq!(fs::read_to_string(&target).unwrap(), before);
    }
}

// HYDRA-INJECT:slot name=boundary_lock profile=library
// HYDRA-INJECT:slot name=geometry_note profile=library
// HYDRA-INJECT:slot name=hardness_seed profile=library
