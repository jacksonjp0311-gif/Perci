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
