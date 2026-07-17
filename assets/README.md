# Perci Dark-Blood branding

## Single source of truth

**Version lives only in `Cargo.toml` → `[package].version`.**

On every `cargo build` / `cargo test`, `build.rs`:

1. Writes `assets/generated/VERSION`
2. Regenerates `assets/generated/perci-darkblood-badge.svg` with the current `vX.Y.Z`
3. Writes `assets/generated/brand-manifest.json`
4. Exports `PERCI_BRAND_VERSION` for compile-time checks

## Files

| Path | Role | Edit? |
|------|------|-------|
| `assets/icons/perci-darkblood-mark.svg` | Timeless diamond/blood sigil | yes (art) |
| `assets/icons/perci-darkblood-mark.jpg` | Raster mark for README / shortcuts | replace art only |
| `assets/generated/perci-darkblood-badge.svg` | Mark + **version** | **never** (auto) |
| `assets/generated/VERSION` | Plain version stamp | **never** (auto) |
| `assets/generated/brand-manifest.json` | Paths + policy | **never** (auto) |

## Guarantees

- UI banner, `/status`, and `perci::branding::version_label()` use `env!("CARGO_PKG_VERSION")`.
- Unit tests fail if `PERCI_BRAND_VERSION` ≠ crate version, or if the badge SVG lacks `v{version}` after a build.
- Bumping the crate version and rebuilding is enough — no manual icon retouch for the version string.

## After a version bump

```powershell
# 1) Edit Cargo.toml version
# 2) Rebuild (stamps badge + VERSION)
cargo build --release
# 3) Tests enforce brand sync
cargo test --lib branding
```
