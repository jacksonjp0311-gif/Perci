# Perci Dark-Blood branding

## Single source of truth

**Version lives only in `Cargo.toml` → `[package].version`.**

A normal `cargo build` / `cargo test` writes the build identity only to
Cargo's `OUT_DIR` (exported as `PERCI_BUILD_ID` / `PERCI_BRAND_VERSION`), so
building never dirties the working tree.

When you bump the version, refresh the checked-in badge once:

```bash
PERCI_STAMP_ASSETS=1 cargo build --release   # PowerShell: $env:PERCI_STAMP_ASSETS=1; cargo build --release
```

That rewrites `assets/generated/perci-darkblood-badge.svg` (committed) and the
local, git-ignored `VERSION`, `BUILD_ID` and `brand-manifest.json`.

## Files

| Path | Role | Edit? |
|------|------|-------|
| `assets/icons/perci-darkblood-mark.svg` | Timeless diamond/blood sigil | yes (art) |
| `assets/icons/perci-darkblood-mark.jpg` | Raster mark for README / previews (letter **P**) | replace art only |
| `assets/icons/perci-darkblood.ico` | Multi-size Windows shortcut icon | rebuild via script |
| `assets/icons/perci-hero-darkblood.jpg` | README hero lattice banner | replace art only |
| `assets/icons/perci-stack-strip.svg` | Cognitive stack strip (exact labels) | yes (art) |
| `assets/generated/perci-darkblood-badge.svg` | Mark + **version** | **never** (`PERCI_STAMP_ASSETS=1`) |
| `assets/generated/VERSION` | Plain version stamp (git-ignored) | **never** (auto) |
| `assets/generated/brand-manifest.json` | Paths + policy (git-ignored) | **never** (auto) |

## Windows desktop shortcut icon

If the Desktop `Perci.lnk` shows a blank/generic icon, the `.ico` path is missing.

```powershell
python scripts/build_icon_ico.py --desktop
# Launcher also repairs Desktop\Perci.lnk to assets/icons/perci-darkblood.ico
.\Launch-Perci.ps1 -Mode status
```

## Always-current binary (never ask "am I on this version?")

`Launch-Perci.ps1` **always**:

1. Kills leftover `perci.exe` processes (stale daemons)
2. `cargo build --release` into `target/live`
3. Mirrors the live binary to `target/release/perci.exe`
4. Writes `.perci/runtime-stamp.json`
5. Starts **that** binary for chat

Banner / `perci version` show `v0.9.8+<gitrev>` (from `PERCI_BUILD_ID`).  
`perci ask` refuses a warm daemon whose `build_id` does not match this binary.

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
