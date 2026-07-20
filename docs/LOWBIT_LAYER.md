# Layered low-bit field (PERCLBW1)

Perci v0.8.8 adds an independent assessment gate over the measured low-bit
representation introduced as a **sidecar** to the
active `PERCIW03` associative pack. It is not a claim that the current pack is
a dense Transformer, and it does not silently replace or promote the active
weights.

## Why this shape

One bit cannot represent direction, magnitude, zero, residual error, and
outliers at the same time. The sidecar separates those jobs:

| Signal | Representation |
|---|---|
| topology and direction | ternary `{-1, 0, +1}` sign masks |
| magnitude | Q8.8 scale per 64-weight block (configurable 2–128) |
| approximation error | up to three additional ternary residual planes |
| ordinary activations | INT4 values with a Q8.8 block scale |
| exceptional activations | sparse Q8.8 outlier lane |
| activation conditioning | reversible orthonormal Walsh–Hadamard rotation |
| repeated weight error | bounded low-rank `U·Vᵀ` correction path |
| working memory | remains multi-bit; the existing `BinaryDialogueState` is unchanged |

The code lives in [`src/low_bit.rs`](../src/low_bit.rs). Its binary field magic
is `PERCLBW1`; `LayeredMatrix::to_bytes` and `from_bytes` provide deterministic
round-tripping without an external runtime dependency.

## Commands

```powershell
cargo run -- lowbit status
cargo run -- lowbit probe
```

## Native candidate training

The representation now has a reproducible native packing loop. It accepts a
reviewed matrix dataset, builds an isolated `PERCLBW1` candidate, evaluates
matrix-vector probes, and writes a JSON receipt beside the binary artifact:

```powershell
cargo run -- lowbit train `
  training\lowbit\example-train.json `
  models\candidates\perci-lowbit-example.blw `
  --block-size 64 --residual-planes 2 --rank 8
```

The dataset schema is `perci.lowbit.train.v1`:

```json
{
  "schema": "perci.lowbit.train.v1",
  "rows": 2,
  "cols": 4,
  "weights": [0.1, -0.2, 0.3, -0.4, 0.2, 0.1, -0.5, 0.7],
  "heldout": [
    {"input": [1, 0, 0, 0], "target": [0.1, 0.2]}
  ]
}
```

The report compares a ternary-plus-residual baseline against the candidate
with the low-rank correction lane. It records a deterministic FNV-1a checksum,
weight MSE, held-out MSE, byte size, and whether the candidate beat the
baseline. `promote_recommended` is always `false`; the result is evidence for
review, never authority.

Re-open and assess the serialized candidate independently:

```powershell
cargo run -- lowbit assess `
  training\lowbit\example-train.json `
  models\candidates\perci-lowbit-example.blw
```

`lowbit assess` validates the on-disk magic/version, dimensions, checksum,
reconstruction, and held-out matrix-vector probes without rebuilding the
candidate. It returns `perci.lowbit.assessment.v1` with `PASS` or `HOLD`. A
`PASS` is only a representation receipt; it is not language-model training or
permission to promote weights.

The example run produced a 280-byte candidate, reduced weight MSE from
`0.00090606674` to `0.0`, and reduced held-out MSE from `0.0024127217` to
`8.88e-16`. Those are tiny synthetic matrix numbers that validate the pipeline,
not a claim about language quality or general intelligence.

The probe exercises the entire representation without writing a promoted
weight. It reports baseline and corrected reconstruction error, INT4 error,
outlier count, Hadamard round-trip error, and binary serialization error.

The measured v0.8.8 probe is:

```text
baseline weight MSE:   0.00335556
corrected weight MSE:  0.00050891
INT4 activation MSE:   0.00413758
sparse outliers:       1
Hadamard max error:    0.00000095
PERCLBW1 bytes:        816
binary roundtrip:      0.00000000
```

Those numbers are a deterministic engineering fixture, not a language-model
quality claim. They establish that the correction path can recover measurable
quantization error while the outlier and rotation paths remain bounded.

## Integration boundary

The existing Perci field is a sparse 4,096-bit associative network. It stores
prototype geometry and selects operators; it is not a dense matrix trained by
back-propagation. Therefore this change adds the representation and its tests
without pretending that a `.pwgt` file can be converted into a full language
model by changing its header.

The next legitimate evolution is a human-authorized training/packing pipeline
that supplies dense row-major matrices (or a new native Perci layer) to this
sidecar, evaluates it on held-out workloads, and only then considers a runtime
adapter. The promotion gate must compare baseline versus candidate hashes,
transfer, exact-tool regressions, abstention, latency, and memory size.

## Governance

- `PERCIW03` remains the active cognitive pack.
- The sidecar is diagnostic until a candidate is evaluated and explicitly
  promoted.
- No command in this module mutates `.pwgt`, `.bwm`, or `.bphr` artifacts.
- The low-bit path does not establish consciousness, general language fluency,
  or frontier-model parity; those require separate held-out evidence.
