# Bitwork v0.3 evidence

## v0.4.12 context-aware variation repair - 2026-07-16

The runtime now connects Bitwork concepts to explicit language operators for
cross-domain inquiry, coherent-thought synthesis, conceptual imagery,
context-bound self-revision, relational questions, and context-aware answer
variation that preserves both sides of a pair such as knowledge and attention.
This is a composition-layer change
over the promoted v0.4.9 artifact; the weight file and its SHA-256 remain
unchanged. The 159-case warm dialogue regression passed, including geometry/life
teaching inquiries, knowledge/attention relational questions, and repeated-turn
variation checks.

Current dialogue receipt:
`29325e543bca9b4064530d2074a93604af3a963b0f73d9db2ff799f4ec74d9d3`.
Concept-transfer receipt:
`a9f38e1c7fd089b4871b7b33317fe8d46cca04667d7fc04b4e9514525a79393a`.
Operational receipt:
`705bee73bfe4c3fb42b02f8c978334d52a7de6bef34236574a99401e4f1e5fd8`.

## Promoted artifact

```text
Path:       models/perci-cognitive-v0.3.pwgt
Magic:      PERCIW03
Size:       209,710,296 bytes (199.995 MiB)
Prototypes: 403,163 unique 4,096-bit states
Concepts:   124
Domains:    16
SHA-256:    13c9fa081324c94355aa4e482cd8674482046cc5d97ddeeaef3ef1d05f374588
```

The v0.4.9 promotion adds operation-oriented reasoning facets and 24 concepts
while remaining under the 200 MiB ceiling. The prior active artifact is
retained under `models/previous/` for rollback.

This is native Bitwork: deterministic sparse binary encoding, signed expert
masks, integer prototype search, and explicit concept identifiers stored in the
weight records. It contains no transformer and calls no language model.

## Why expansion helped

The old 200 MiB v1 pack held 403,266 records, but many were duplicate
activations. The deduplicated v2 pack proved that 38,580 unique states were
more useful than padding. v3 therefore spends the larger budget on new semantic
frames and concepts rather than duplicate mass. It reaches almost the v1 record
count while preserving uniqueness and adds a concept table that can return the
matched idea, not only a domain label.

## Promotion gates

| Suite | Result |
|---|---:|
| Original held-out domain routing | 1.0 |
| Original local precision / recall | 1.0 / 1.0 |
| Original trap abstention | 1.0 |
| Hard 76-case transfer routing | 1.0 |
| Hard transfer local precision / recall | 1.0 / 1.0 |
| Hard transfer trap abstention | 1.0 |
| Concept transfer | 16 / 16 |

Receipts:

- `models/candidates/evaluation-v3-original-promote.json`
- `models/candidates/evaluation-v3-transfer-promote.json`
- `models/candidates/evaluation-v3-concepts-promote.json`
- `models/promotion-ledger.jsonl`

The promotion script requires explicit human authorization, validates the
candidate and receipt hashes, and now binds supplemental transfer and concept
receipts into the promotion ledger.

## Claim boundary

These results establish improvement on the recorded suites. They do not prove
100× general intelligence, consciousness, phenomenology, or unrestricted
language competence. The storage and unique prototype counts grew by about
10× because the permitted 200 MiB budget is about 10× the v2 pack. Further
claims require wider independent evaluation.

## v0.3.1 response-consumption correction

The weight artifact did not change. Runtime v0.3.1 adds a semantic gate between
prototype retrieval and prose emission: a concept identifier may remain useful
telemetry, but its insight is emitted only when the prompt contains supporting
alias or meaning overlap. It also resolves explicit references to the previous
turn and blocks identical outputs across different questions.

Evidence:

- `models/candidates/evaluation-v3.1-dialogue.json`: 6/6 transcript replay,
  all outputs unique.
- `models/candidates/evaluation-v3.1-original.json`: routing 1.0.
- `models/candidates/evaluation-v3.1-transfer.json`: transfer 1.0.
- `models/candidates/evaluation-v3.1-concepts.json`: 16/16 concept transfer.
## v0.4.9 reasoning-response expansion - 2026-07-16

The promoted `PERCIW03` artifact now contains 403,163 unique 4,096-bit
prototypes, 124 weight-resident concepts, and 209,710,296 bytes (199.995 MiB).
The new curriculum is organized around operations rather than only topics:
falsification, calibration, model update, experiment design, observation versus
inference, mechanism versus metaphor versus evidence, transfer testing,
self-critique, routing, composition, ablation, regression, and selection.

Evidence receipts:

- operational held-out gate: `e8d7fec83e610d354a2c8c2d0e808d31fe1b632ac47a6bcd2bb841d305ba19fe`
- concept-transfer gate: `c7eb5a042057f0420eef3568a7ed03415130ce781f203b88a5f7363ba9cb0140`
- dialogue gate: `34424553cae69784448e9a93cb0bbba760b5628a5ecaacb382b200d609df0654`
- active model SHA-256: `13c9fa081324c94355aa4e482cd8674482046cc5d97ddeeaef3ef1d05f374588`

The artifact is still an associative binary routing/selection system with
deterministic operators; increasing its size does not turn it into a general
language model or establish consciousness. The improvement claim is limited to
the measured prompt families and held-out gates above.
## v0.4.12 context-aware variation repair - 2026-07-16

The runtime now connects Bitwork concepts to explicit language operators for
cross-domain inquiry, coherent-thought synthesis, conceptual imagery,
context-bound self-revision, relational questions, and context-aware answer
variation that preserves both sides of a pair such as knowledge and attention.
This is a composition-layer change over the promoted v0.4.9 artifact; the
weight file and its SHA-256 remain unchanged. The 159-case warm dialogue
regression passed, including geometry/life teaching inquiries,
knowledge/attention relational questions, and repeated-turn variation checks.

Current dialogue receipt: `29325e543bca9b4064530d2074a93604af3a963b0f73d9db2ff799f4ec74d9d3`.
