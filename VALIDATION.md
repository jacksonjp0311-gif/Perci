# Validation record

## Completed in the build environment

- Generated `models/perci-cognitive-v0.1.pwgt` from the deterministic curriculum.
- Verified exact size: `209,715,200` bytes.
- Verified format magic: `PERCIW01`.
- Verified 16 expert entries and 403,266 prototype records.
- Verified SHA-256 against the generated manifest.
- Loaded and queried the weight file through the Python reference implementation.
- Ran 16 held-out domain probes; all 16 routed to the expected expert in the recorded run.
- Performed a structural delimiter pass over the Rust source.

## Not completed in the build environment

Rust and Cargo were not installed, so the Rust crate was not compiled or benchmarked here. Run:

```powershell
.\Launch-Perci.ps1 -Mode test
```

before treating the Rust executable as verified on your machine.

## Interpretation

The held-out probes validate the binary associative routing mechanism and prototype retrieval. They do not demonstrate parity with a pretrained transformer or establish general intelligence.
## Perci v0.1.1 Windows integration validation

- Date (UTC): 2026-07-15
- Rust compiler: $RustVersion
- Cargo: $CargoVersion
- Python: $PythonVersion
- Release tests: executed by the integration script
- Cortex engine: vendored and bootstrapped as repository Perci
- Memory boundary: append-only JSONL plus explicit Cortex episodic events
- Authority boundary: Cortex evidence is recommendation/context only
