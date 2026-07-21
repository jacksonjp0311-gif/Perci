//! Dark-Blood brand identity — version always tracks Cargo.toml.
//!
//! Policy: never hardcode a release version in UI strings or assets by hand.
//! Use `version()` / `env!("CARGO_PKG_VERSION")` only. `build.rs` regenerates
//! `assets/generated/*` so badges cannot lag the crate version.

/// Crate version from Cargo.toml (compile-time).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Same version stamped by build.rs (must match `version()`).
pub fn branded_version() -> &'static str {
    env!("PERCI_BRAND_VERSION")
}

/// Build identity: `0.9.8+9086af8` (semver + git short rev). Empty-rev → semver only.
///
/// Baked at compile time so the banner proves which source this binary was built from.
/// Launch-Perci always rebuilds before chat so this stays current without manual checks.
pub fn build_id() -> &'static str {
    option_env!("PERCI_BUILD_ID").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Human label: `Perci v0.9.8+9086af8 · dark-blood`
pub fn version_label() -> String {
    format!("Perci v{} · dark-blood", build_id())
}

/// Relative path to the version-agnostic mark (diamond/blood sigil).
pub fn mark_svg_path() -> &'static str {
    "assets/icons/perci-darkblood-mark.svg"
}

/// Relative path to the versioned badge (regenerated each build).
pub fn badge_svg_path() -> &'static str {
    "assets/generated/perci-darkblood-badge.svg"
}

/// Relative path to raster mark for README / previews.
pub fn mark_raster_path() -> &'static str {
    "assets/icons/perci-darkblood-mark.jpg"
}

/// Relative path to multi-size Windows icon for shortcuts.
pub fn mark_ico_path() -> &'static str {
    "assets/icons/perci-darkblood.ico"
}

/// Assert generated VERSION file matches crate (when present on disk).
pub fn generated_version_file() -> Option<String> {
    let path = std::path::Path::new("assets/generated/VERSION");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_env_matches_crate_version() {
        assert_eq!(
            branded_version(),
            version(),
            "build.rs PERCI_BRAND_VERSION must equal CARGO_PKG_VERSION"
        );
    }

    #[test]
    fn generated_version_file_matches_when_present() {
        if let Some(file_v) = generated_version_file() {
            // VERSION is build_id (semver or semver+gitrev); must start with crate version.
            assert!(
                file_v == version() || file_v.starts_with(&format!("{}+", version())),
                "assets/generated/VERSION={file_v} must start with crate version {}",
                version()
            );
        }
    }

    #[test]
    fn version_label_contains_crate_version() {
        let label = version_label();
        assert!(label.contains(version()));
        assert!(label.contains("dark-blood"));
        assert!(label.contains(build_id()) || build_id() == version());
    }

    #[test]
    fn build_id_starts_with_crate_version() {
        let id = build_id();
        assert!(
            id == version() || id.starts_with(&format!("{}+", version())),
            "build_id={id}"
        );
    }

    #[test]
    fn badge_svg_contains_current_version_when_present() {
        let path = std::path::Path::new(badge_svg_path());
        if path.is_file() {
            let svg = std::fs::read_to_string(path).expect("read badge");
            assert!(
                svg.contains(&format!("v{}", version())),
                "badge SVG must embed current version"
            );
            assert!(svg.contains("AUTO-GENERATED"));
        }
    }
}
