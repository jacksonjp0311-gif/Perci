//! Common pack manifest for modular binary cognition (Phase 1).
//!
//! Sidecar JSON (and optional binary header magic) describe installable packs.
//! Promotion status is explicit: candidate → evaluated → authorized → active.
//! Never auto-promote.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Known pack family identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PackFamily {
    /// Existing Bitwork reflex / routing field (do not replace).
    Perciw03,
    /// Semantic field (entities, roles, relations, intent).
    Percisem1,
    /// Reasoning-transition transitions (not full answers).
    Percirsn1,
    /// Discourse planning acts.
    Percidsc1,
    /// Constrained language realization.
    Percilm1,
    /// Operator-dependent fold experiment.
    Percifld1,
    /// Unknown / third-party extension.
    Other,
}

impl PackFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Perciw03 => "PERCIW03",
            Self::Percisem1 => "PERCISEM1",
            Self::Percirsn1 => "PERCIRSN1",
            Self::Percidsc1 => "PERCIDSC1",
            Self::Percilm1 => "PERCILM1",
            Self::Percifld1 => "PERCIFLD1",
            Self::Other => "OTHER",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "PERCIW03" | "PERCIW02" | "PERCIW01" => Self::Perciw03,
            "PERCISEM1" => Self::Percisem1,
            "PERCIRSN1" => Self::Percirsn1,
            "PERCIDSC1" => Self::Percidsc1,
            "PERCILM1" => Self::Percilm1,
            "PERCIFLD1" => Self::Percifld1,
            _ => Self::Other,
        }
    }
}

/// Durable promotion gate — human only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStatus {
    #[default]
    Candidate,
    Evaluated,
    Authorized,
    Active,
    Retired,
}

impl PromotionStatus {
    pub fn may_influence_production(self) -> bool {
        matches!(self, Self::Active | Self::Authorized)
    }
}

/// Common pack manifest (sidecar JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackManifest {
    pub schema: String,
    pub magic: String,
    pub format_version: u32,
    pub pack_id: String,
    pub family: String,
    pub capability_tags: Vec<String>,
    #[serde(default)]
    pub dimensions: u32,
    #[serde(default)]
    pub record_count: u64,
    #[serde(default)]
    pub byte_length: u64,
    #[serde(default)]
    pub checksum_sha256: String,
    #[serde(default)]
    pub corpus_hash: String,
    #[serde(default)]
    pub builder_version: String,
    #[serde(default)]
    pub evaluation_receipt: String,
    pub promotion_status: PromotionStatus,
    #[serde(default)]
    pub authorization_record: String,
    #[serde(default)]
    pub dependency_packs: Vec<String>,
    #[serde(default)]
    pub compatible_decoders: Vec<String>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub notes: String,
}

impl PackManifest {
    pub const SCHEMA: &'static str = "perci.pack-manifest.v1";

    pub fn family_enum(&self) -> PackFamily {
        PackFamily::parse(&self.family)
    }

    pub fn validate_basic(&self) -> Result<(), String> {
        if self.schema != Self::SCHEMA {
            return Err(format!("unexpected schema {}", self.schema));
        }
        if self.pack_id.is_empty() {
            return Err("pack_id empty".into());
        }
        if self.magic.is_empty() {
            return Err("magic empty".into());
        }
        Ok(())
    }

    pub fn is_production_eligible(&self) -> bool {
        self.promotion_status.may_influence_production()
    }
}

/// Discover pack manifests under a directory (non-recursive by default depth 2).
pub fn discover_manifests(root: &Path) -> Vec<PackManifest> {
    let mut out = Vec::new();
    walk_manifests(root, 0, 2, &mut out);
    out
}

fn walk_manifests(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PackManifest>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_manifests(&path, depth + 1, max_depth, out);
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !(name.ends_with(".pack.json")
            || name.ends_with(".manifest.json")
            || (name.contains("perci") && name.ends_with(".json") && name.contains("pack")))
        {
            // Accept *pack*.json under models/candidates/packs/
            if !(name.ends_with(".json")
                && dir
                    .components()
                    .any(|c| c.as_os_str().to_string_lossy().contains("packs")))
            {
                continue;
            }
        }
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(mut m) = serde_json::from_str::<PackManifest>(&text) {
                if m.path.is_empty() {
                    m.path = path.display().to_string();
                }
                if m.validate_basic().is_ok() {
                    out.push(m);
                }
            }
        }
    }
}

/// Default candidate pack root.
pub fn default_pack_root() -> PathBuf {
    PathBuf::from("models/candidates/packs")
}

/// Write a candidate manifest (never marks active).
pub fn write_candidate_manifest(path: &Path, mut manifest: PackManifest) -> Result<(), String> {
    manifest.schema = PackManifest::SCHEMA.into();
    if !matches!(
        manifest.promotion_status,
        PromotionStatus::Candidate | PromotionStatus::Evaluated
    ) {
        manifest.promotion_status = PromotionStatus::Candidate;
    }
    manifest.validate_basic()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(path, text + "\n").map_err(|e| e.to_string())
}

/// Stub builder for empty experimental pack sidecars (no weight bytes).
pub fn scaffold_family_manifest(
    family: PackFamily,
    pack_id: &str,
    tags: &[&str],
    byte_budget: u64,
) -> PackManifest {
    PackManifest {
        schema: PackManifest::SCHEMA.into(),
        magic: family.as_str().into(),
        format_version: 1,
        pack_id: pack_id.into(),
        family: family.as_str().into(),
        capability_tags: tags.iter().map(|s| (*s).to_owned()).collect(),
        dimensions: 4096,
        record_count: 0,
        byte_length: 0,
        checksum_sha256: String::new(),
        corpus_hash: String::new(),
        builder_version: env!("CARGO_PKG_VERSION").into(),
        evaluation_receipt: String::new(),
        promotion_status: PromotionStatus::Candidate,
        authorization_record: "pending human authorize".into(),
        dependency_packs: vec!["PERCIW03".into()],
        compatible_decoders: vec![family.as_str().into()],
        path: String::new(),
        notes: format!(
            "Candidate scaffold only. Engineering budget ~{byte_budget} bytes; not padded. Never auto-promote."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn family_parse_and_scaffold() {
        assert_eq!(PackFamily::parse("PERCISEM1"), PackFamily::Percisem1);
        let m = scaffold_family_manifest(
            PackFamily::Percisem1,
            "percisem1-v0.1-candidate",
            &["semantic", "binding", "intent"],
            64 * 1024 * 1024,
        );
        assert_eq!(m.schema, PackManifest::SCHEMA);
        assert!(!m.is_production_eligible());
        assert!(m.validate_basic().is_ok());
    }

    #[test]
    fn write_and_discover_roundtrip() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("perci-pack-test-{stamp}"));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("percisem1.pack.json");
        let m = scaffold_family_manifest(
            PackFamily::Percisem1,
            "test-sem1",
            &["semantic"],
            1024,
        );
        write_candidate_manifest(&path, m).expect("write");
        let found = discover_manifests(&dir);
        assert!(
            found.iter().any(|p| p.pack_id == "test-sem1"),
            "found={found:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn promotion_gate_blocks_candidate() {
        let mut m = scaffold_family_manifest(PackFamily::Percirsn1, "rsn", &["reason"], 1);
        assert!(!m.is_production_eligible());
        m.promotion_status = PromotionStatus::Active;
        assert!(m.is_production_eligible());
    }
}
