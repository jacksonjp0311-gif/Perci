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

/// Human label: `Perci v0.5.2 · dark-blood`
pub fn version_label() -> String {
    format!("Perci v{} · dark-blood", version())
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
            assert_eq!(
                file_v,
                version(),
                "assets/generated/VERSION must match crate version after build"
            );
        }
    }

    #[test]
    fn version_label_contains_crate_version() {
        let label = version_label();
        assert!(label.contains(version()));
        assert!(label.contains("dark-blood"));
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
