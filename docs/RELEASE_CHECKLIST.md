# Release checklist (v0.6.21+)

Run before every version bump that claims intelligence growth.

## Hard gates (must pass)

```powershell
cargo test --lib
python .\scripts\evaluate_hardness.py
.\target\release\perci.exe transfer-suite
.\target\release\perci.exe lab unified
```

Or one shot:

```powershell
python .\scripts\release_gates.py
```

## Soft checks

```powershell
.\target\release\perci.exe status   # version matches Cargo.toml
# LIVE_TEST_TEN — see docs/LIVE_TEST_TEN.md
.\target\release\perci.exe lab queue
```

## Rules

1. **Transfer suite FAIL** → do not ship the claimed fix.  
2. **Hardness red** → open impasse (`perci agent lab --from-hardness`) or repair.  
3. **Open primary-fix tickets** → `perci agent lab --from-emergence [--repair]` or justify.  
4. **Weights** → never auto-promote; `promote_v2.py --authorize` only after sealed eval.  
5. **Brand** → `assets/generated/VERSION` == `Cargo.toml` version after `cargo build --release`.

## Product law

Mixture thesis is a **crutch**. Transfer + operators are the truth path for speech. Pack rebuild is optional and human-gated.
