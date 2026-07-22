//! Sparse hierarchical capability router (Phase 1).
//!
//! ```text
//! prompt → domain cues → capability tags → select ≤K packs → telemetry
//! ```
//!
//! Installed pack count must not force global scan. Only selected packs are
//! "active" for a turn. Mapping is advisory until Phase 3+ pack loaders exist.

use crate::pack_manifest::{discover_manifests, PackFamily, PackManifest, PromotionStatus};
use crate::thought_plan::Intent;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Per-turn routing telemetry (no secrets).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterTelemetry {
    pub total_installed_bytes: u64,
    pub mapped_bytes: u64,
    pub accessed_bytes: u64,
    pub installed_pack_count: usize,
    pub active_pack_count: usize,
    pub routing_latency_us: u64,
    pub domains: Vec<String>,
    pub capability_tags: Vec<String>,
    pub active_pack_ids: Vec<String>,
    pub skipped_candidate_packs: usize,
}

/// Result of sparse pack selection for one turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub intent: Intent,
    pub domains: Vec<String>,
    pub capability_tags: Vec<String>,
    pub active_packs: Vec<String>,
    pub telemetry: RouterTelemetry,
    /// Human-readable one-liner for receipts.
    pub summary: String,
}

/// Capability router over on-disk manifests + always-on PERCIW03.
#[derive(Debug, Clone)]
pub struct CapabilityRouter {
    pub pack_root: PathBuf,
    pub max_active: usize,
    /// When true, only Active/Authorized packs may activate (candidates ignored).
    pub production_only: bool,
}

impl Default for CapabilityRouter {
    fn default() -> Self {
        Self {
            pack_root: PathBuf::from("models/candidates/packs"),
            max_active: 3,
            production_only: false,
        }
    }
}

impl CapabilityRouter {
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            pack_root: root.into(),
            ..Self::default()
        }
    }

    /// Route a prompt to a small active pack set. Never scans weight payloads.
    pub fn route(&self, user: &str) -> RouteDecision {
        let t0 = Instant::now();
        let intent = Intent::infer_from_prompt(user);
        let domains = infer_domains(user);
        let tags = infer_capability_tags(user, intent);

        let manifests = if self.pack_root.is_dir() {
            discover_manifests(&self.pack_root)
        } else {
            Vec::new()
        };

        let mut installed_bytes = 0u64;
        let mut skipped = 0usize;
        for m in &manifests {
            installed_bytes = installed_bytes.saturating_add(m.byte_length.max(1));
        }

        // PERCIW03 always active as reflex field (logical; bytes reported separately).
        let mut active: Vec<String> = vec!["PERCIW03".into()];
        let mut mapped = 0u64;

        for m in &manifests {
            if self.production_only && !m.is_production_eligible() {
                skipped += 1;
                continue;
            }
            // Candidates may be *selected for experiment* but never auto-promoted.
            if !self.production_only
                && !matches!(
                    m.promotion_status,
                    PromotionStatus::Candidate
                        | PromotionStatus::Evaluated
                        | PromotionStatus::Authorized
                        | PromotionStatus::Active
                )
            {
                skipped += 1;
                continue;
            }
            let score = pack_relevance(m, &tags, &domains);
            if score <= 0 {
                continue;
            }
            if active.len() >= self.max_active {
                break;
            }
            if !active.iter().any(|id| id == &m.pack_id || id == &m.magic) {
                active.push(if m.pack_id.is_empty() {
                    m.magic.clone()
                } else {
                    m.pack_id.clone()
                });
                // Mapped only selected: report declared byte_length (0 for scaffold).
                mapped = mapped.saturating_add(m.byte_length);
            }
        }

        // Ensure family defaults appear as logical selections when tags match
        // even without on-disk manifests (Phase 1 scaffolding).
        for (tag, family) in [
            ("semantic", "PERCISEM1"),
            ("reasoning", "PERCIRSN1"),
            ("discourse", "PERCIDSC1"),
            ("language", "PERCILM1"),
            ("fold", "PERCIFLD1"),
        ] {
            if tags.iter().any(|t| t == tag)
                && active.len() < self.max_active
                && !active.iter().any(|a| a.contains(family))
            {
                active.push(format!("{family}:logical"));
            }
        }

        let latency = t0.elapsed().as_micros() as u64;
        // Accessed bytes: only router index work in Phase 1 (manifest JSON size proxy).
        let accessed = (active.len() as u64).saturating_mul(256);

        let telemetry = RouterTelemetry {
            total_installed_bytes: installed_bytes,
            mapped_bytes: mapped,
            accessed_bytes: accessed,
            installed_pack_count: manifests.len() + 1, // + PERCIW03 logical
            active_pack_count: active.len(),
            routing_latency_us: latency,
            domains: domains.clone(),
            capability_tags: tags.clone(),
            active_pack_ids: active.clone(),
            skipped_candidate_packs: skipped,
        };

        let summary = format!(
            "intent={} domains={} packs={} mapped={}B accessed={}B route_us={}",
            intent.as_str(),
            domains.join("+"),
            active.join(","),
            mapped,
            accessed,
            latency
        );

        RouteDecision {
            intent,
            domains,
            capability_tags: tags,
            active_packs: active,
            telemetry,
            summary,
        }
    }
}

fn infer_domains(user: &str) -> Vec<String> {
    let t = user.to_ascii_lowercase();
    let mut d = Vec::new();
    let table = [
        ("trust", "systems"),
        ("lag", "systems"),
        ("timeout", "systems"),
        ("geometry", "geometry"),
        ("boundary", "geometry"),
        ("code", "code"),
        ("rust", "code"),
        ("cargo", "code"),
        ("memory", "memory"),
        ("learn", "memory"),
        ("plan", "planning"),
        ("proof", "logic"),
        ("conscious", "identity"),
        ("who are you", "identity"),
        ("connect ", "comparison"),
        ("entropy", "science"),
    ];
    for (k, dom) in table {
        if t.contains(k) && !d.iter().any(|x| x == dom) {
            d.push((*dom).into());
        }
    }
    if d.is_empty() {
        d.push("general".into());
    }
    d.truncate(4);
    d
}

fn infer_capability_tags(user: &str, intent: Intent) -> Vec<String> {
    let t = user.to_ascii_lowercase();
    let mut tags = vec!["routing".into()];
    match intent {
        Intent::CausalExplanation | Intent::Trust | Intent::Verification => {
            tags.push("reasoning".into());
            tags.push("semantic".into());
            tags.push("discourse".into());
        }
        Intent::Synthesis | Intent::Comparison => {
            tags.push("semantic".into());
            tags.push("reasoning".into());
            tags.push("discourse".into());
        }
        Intent::Plan | Intent::Teaching => {
            tags.push("reasoning".into());
            tags.push("discourse".into());
        }
        Intent::Identity | Intent::Refuse => {
            tags.push("semantic".into());
            tags.push("discourse".into());
        }
        Intent::Social | Intent::Exact => {}
        Intent::Unknown => {
            tags.push("semantic".into());
        }
    }
    if t.contains("fold") || t.contains("compress") || t.contains("hypervector") {
        tags.push("fold".into());
    }
    if t.contains("fluent") || t.contains("wording") || t.contains("rewrite") {
        tags.push("language".into());
    }
    tags
}

fn pack_relevance(m: &PackManifest, tags: &[String], domains: &[String]) -> i32 {
    let mut score = 0i32;
    for tag in tags {
        if m.capability_tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tag))
        {
            score += 2;
        }
    }
    for dom in domains {
        if m.capability_tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(dom))
        {
            score += 1;
        }
    }
    // Family boosts from magic.
    match m.family_enum() {
        PackFamily::Percisem1 if tags.iter().any(|t| t == "semantic") => score += 3,
        PackFamily::Percirsn1 if tags.iter().any(|t| t == "reasoning") => score += 3,
        PackFamily::Percidsc1 if tags.iter().any(|t| t == "discourse") => score += 3,
        PackFamily::Percilm1 if tags.iter().any(|t| t == "language") => score += 3,
        PackFamily::Percifld1 if tags.iter().any(|t| t == "fold") => score += 3,
        PackFamily::Perciw03 => score += 1,
        _ => {}
    }
    score
}

/// Convenience: route using default pack root if present.
pub fn route_prompt(user: &str) -> RouteDecision {
    let root = Path::new("models/candidates/packs");
    if root.is_dir() {
        CapabilityRouter::with_root(root).route(user)
    } else {
        CapabilityRouter::default().route(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack_manifest::{scaffold_family_manifest, write_candidate_manifest, PackFamily};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn always_includes_perciw03() {
        let d = CapabilityRouter::default().route("hello");
        assert!(d.active_packs.iter().any(|p| p.contains("PERCIW03")));
        assert!(d.telemetry.active_pack_count >= 1);
    }

    #[test]
    fn trust_selects_reasoning_and_semantic_logical() {
        let d = CapabilityRouter::default()
            .route("how should interfaces earn trust under lag and retry?");
        assert_eq!(d.intent, Intent::Trust);
        assert!(d.capability_tags.iter().any(|t| t == "reasoning"));
        assert!(
            d.active_packs.iter().any(|p| {
                p.contains("PERCIRSN1")
                    || p.contains("PERCISEM1")
                    || p.contains("percirsn")
                    || p.contains("percisem")
            }),
            "packs={:?}",
            d.active_packs
        );
        assert!(d.telemetry.routing_latency_us < 50_000);
    }

    #[test]
    fn sparse_select_from_manifests() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("perci-router-{stamp}"));
        let _ = std::fs::create_dir_all(&dir);
        let m = scaffold_family_manifest(
            PackFamily::Percisem1,
            "sem1-test",
            &["semantic", "binding"],
            1024,
        );
        write_candidate_manifest(&dir.join("sem1.pack.json"), m).unwrap();
        let router = CapabilityRouter {
            pack_root: dir.clone(),
            max_active: 3,
            production_only: false,
        };
        let d = router.route("bind the subject and condition for delayed trust");
        assert!(d
            .active_packs
            .iter()
            .any(|p| p == "sem1-test" || p.contains("PERCISEM1")));
        assert!(d.telemetry.installed_pack_count >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn production_only_skips_candidates() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("perci-router-prod-{stamp}"));
        let _ = std::fs::create_dir_all(&dir);
        let m = scaffold_family_manifest(PackFamily::Percirsn1, "rsn-cand", &["reasoning"], 1);
        write_candidate_manifest(&dir.join("rsn.pack.json"), m).unwrap();
        let router = CapabilityRouter {
            pack_root: dir.clone(),
            max_active: 3,
            production_only: true,
        };
        let d = router.route("explain a causal mechanism with counterexamples");
        assert!(!d.active_packs.iter().any(|p| p == "rsn-cand"));
        assert!(d.telemetry.skipped_candidate_packs >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
