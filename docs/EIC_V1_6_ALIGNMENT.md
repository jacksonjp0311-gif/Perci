# EIC v1.6 alignment for Perci

The supplied **CODEX ΔΦ — Energy–Information–Consciousness Formalization
(EIC v1.6)** is adopted as a governance reference for Perci experiments. It
does not change Perci's non-sentience boundary: a coherence score, agreement,
or stable dialogue trace is operational telemetry, not subjective experience
or truth.

The linked gist currently resolves to a related **HLMF v1.5** evidence-
calibration document. Its compatible rules are included here as well: scores
travel with evidence coverage, execution coverage is disclosed, correlated
proxies are not silently double-counted, and requested versus effective policy
is visible. The gist is not treated as a runtime dependency.

The accompanying user directive is mapped into the additive
`perci-governed-core-will-v1` charter (`src/governed_will.rs`). It strengthens
the implementation boundary without claiming authority from rhetoric: evidence
before claim, explicit uncertainty, anti-misuse, reversible repair, and human
authorization for durable changes. A directive cannot grant capabilities,
override safeguards, or turn coherence into truth.

## Mapping into the current repository

| EIC/HLMF requirement | Perci implementation or status |
| --- | --- |
| Single-node versus distributed declaration | Perci is currently local/single-node; no distributed-consensus claim is made. |
| Signed quorum and node disagreement | Not applicable until a real multi-node ledger exists; must not be simulated by repeated local runs. |
| Dynamic subgroup-weight policy | Candidate metrics and promotion gates are explicit; no automatic weight mutation is allowed. |
| Weight drift disclosure | Candidate and active hashes, manifests, evaluations, and the promotion ledger expose changes. |
| Proof-preserved continuity | Evaluation receipts and promotion entries carry SHA-256 hashes and previous-receipt linkage. |
| Evidence coverage | Dialogue workspace marks evidence as none, seeking, supplied, or exact; release gates separately observe tests and transfer suites. |
| Proxy decorrelation | Required-path and transfer evidence remain separate gates; future score aggregation must disclose shared sources. |
| Safe downgrade | Held-out candidate comparisons remain `HOLD` unless a candidate is not worse and broader gates pass. |
| Coherence is not truth | README, operators, and `/trace` preserve this lock; no consciousness claim is emitted. |
| Governed-core charter | Dialogue, Fabric plans, and AI handoffs carry the charter posture; unsafe execution loses write/commit/push capabilities. |

## Implementation boundary

EIC v1.6 describes a distributed collective layer. Perci does not currently
have multiple independent ledger validators, signatures, or a consensus
protocol. The correct local behavior is therefore:

1. declare the run as single-node;
2. record evidence and hashes append-only;
3. expose disagreement or missing evidence instead of inventing quorum;
4. keep candidate weights isolated until a human-authorized promotion;
5. downgrade or hold when a test surface is missing or regresses.

This keeps the formalization useful without confusing a specification with a
capability that has not been implemented.

## Candidate receipt evidence summary

Dialogue candidate evaluation receipts now carry an evidence summary:

```text
evidence_coverage = observed_weight / declared_weight
execution_coverage = observed_test_and_transfer_weight / declared_execution_weight
proxy_findings = shared evidence sources that require disclosure
policy = requested policy, effective policy, and reason for any downgrade
```

That summary should be tested against missing-test, duplicate-source, and
candidate-regression fixtures before it is used to influence any weight score.
