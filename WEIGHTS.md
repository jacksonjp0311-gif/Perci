# Perci Cognitive Weights

## Native language field (PERCLNG1)

Perci also owns a separate binary language artifact:

    models/perci-language-v0.1.blng

Build it without an external model:

    cargo run --release -- language train --repo
    cargo run --release -- language status

The field stores context records with four binary threshold planes. A set bit
means that a next ASCII symbol was observed at least 1, 2, 4, or 8 times for
that context. Context depths 1..=6 provide bounded back-off; Rust inference
uses mmap, integer scoring, and a deterministic PRNG. The artifact is a
trainable sequence memory, not a frontier transformer or a source of exact
facts. Review the corpus before rebuilding; active promotion remains explicit.

Each record is 71 bytes (`depth + 6-byte context + 4 × 128-bit planes`). The
decoder scores a candidate byte with `depth² × sum(threshold bits)` across
matching back-off records, so generation is integer-only and bounded. At the
current 132,525 records that is about 9 MiB; a larger reviewed corpus can grow
the field without changing the on-disk format.

The companion `models/perci-language-v0.2.bphr` field moves one level up:
tokens are assigned bounded numeric IDs, each context stores sparse next-token
edges with four binary count planes, and the index is sorted by numeric token
IDs. This is why the field can compose readable phrases without pretending to
be a transformer. The current candidate is roughly 0.5 MiB and is intentionally
small enough to retrain during an experiment.

The optional `models/perci-world-v0.1.bwm` (`PERCIWM1`) field adds typed edges:
hashed subject, relation, and object, plus domain, polarity, confidence, and
evidence bins. It is loaded with mmap and contributes only a bounded reranking
score to native phrase candidates. It is not a fact database and is never
auto-promoted. The v0.8.4 candidate was 13,387 records / 0.41 MiB and tied the
active phrase field on the adversarial held-out pack, so it remains isolated.

Dialogue continuity is separate from the phrase weights: four 64-bit lanes
form a bounded recurrent state, replayed from recent turns and folded into the
sampler seed. It is an order-sensitive context fingerprint, not a claim of
subjective memory or consciousness.

## GitHub note

`.pwgt` packs are **not committed** (~200 MiB; above GitHub file limits).
Sidecar metadata (`*.pwgt.json`) stays in-repo. Build locally:

```text
python scripts/build_weights_v3.py
# or restore a previously authorized pack under models/
```

Set `PERCI_WEIGHTS` or place `models/perci-cognitive-v0.3.pwgt` for launch.

## Active pack (local)

The active pack is `models/perci-cognitive-v0.3.pwgt` (`PERCIW03`): 209,710,296
bytes (199.995 MiB), 403,163 unique 4,096-bit prototypes, 16 domains, and 124
weight-resident concepts. Runtime software is **v0.6.0** (soft attention,
dual residual stream, VSA composition, Willshaw concept HVs, session CTX bind).
See [`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md),
[`docs/BITWORK_MATH_PATH.md`](docs/BITWORK_MATH_PATH.md), and
[`docs/BITWORK_V3_EVIDENCE.md`](docs/BITWORK_V3_EVIDENCE.md).

The `PERCIW02` 19.16 MiB pack and the following `PERCIW01` 200 MiB pack remain
readable fallbacks and comparison baselines.

## v0.1 legacy migration pack

## Identity

```text
File:          models/perci-cognitive-v0.1.pwgt
Format magic:  PERCIW01
Version:       1
Size:          209,715,200 bytes
Size:          200 MiB exactly
Architecture:  sparse binary associative Bitwork network
Prototypes:    403,266
Activation:    4,096 bits / 64 packed u64 words
Record size:   520 bytes
```

The canonical checksum is recorded in:

```text
models/perci-cognitive-v0.1.pwgt.json
```

## What these weights are

Each training prompt is converted into a sparse distributed binary activation. Features include normalized words, prefixes, suffixes, adjacent word pairs, character trigrams, prompt length, and a bias feature. Every lexical feature sets four positions in a 4,096-bit vector using deterministic FNV-1a hashing.

The build groups examples into 16 expert domains. Bit frequencies within each domain are contrasted against the other domains to create a learned positive expert mask. At inference time:

1. Encode the user prompt into 4,096 bits.
2. Score expert masks with `AND` and `POPCOUNT`.
3. Add narrow lexical priors for high-value terms such as `Rust`, `triangle`, `permission`, or `grammar`.
4. Scan the three best expert partitions.
5. Select the nearest stored prototype using packed intersection counts.
6. Use the prototype's response variant to choose a domain-appropriate Perci response.
7. Delegate exact arithmetic and geometry to deterministic tools.

No floating point is used in the weight inference path.

## Cognitive domains

```text
greeting      identity       english        logic
math          geometry       memory         code
governance    planning       explanation    systems
science       creativity     comparison     general
```

## Training curriculum

The deterministic builder generates paraphrased prompts covering:

- greetings and identity questions
- English grammar, meaning, definitions, and rewriting intent
- premises, assumptions, inference, contradiction, and conclusions
- arithmetic, algebraic wording, fractions, percentages, and ratios
- triangles, circles, coordinate language, and the Pythagorean theorem
- memory write and retrieval intent
- Rust, PowerShell, CLI, repository, debugging, and testing language
- authorization, sandboxing, ledgers, validation, and origin alignment
- plans, milestones, dependencies, risks, and acceptance criteria
- explanatory structure
- Perci, Lumen, Cortex, Bitwork, NEMO, and RHP architecture
- scientific testing and measurement language
- creative ideation
- explicit comparison criteria
- general analysis requests

The complete build can be reproduced with:

```powershell
python .\scripts\build_weights.py --output .\models\perci-cognitive-v0.1.pwgt
```

## Evaluation

The included held-out probe set uses phrasings and parameter values not copied directly from the generated prototype records. The recorded run classified all 16 domain probes correctly.

This is a small **routing and associative retrieval evaluation**. It is not an accepted benchmark of language understanding, mathematical reasoning, factual knowledge, or general intelligence.

## Why exact tools remain outside the weights

A compact binary network can recognize that a request is mathematical without reliably calculating every answer. Perci therefore uses the weights to select the correct cognitive path and uses deterministic integer/rational code for the result. This separation provides:

- inspectable computation
- repeatable results
- no floating-point dependency in the associative model
- less pressure to memorize arithmetic facts
- a clear distinction between neural classification and verified calculation

## Limitations

The model is not a transformer, not a distilled copy of ChatGPT, and not pretrained on a web-scale corpus. It cannot acquire broad world knowledge from file size alone. It uses trained associative prototypes and templated language around exact tools.

The 200 MiB target is meaningful because it contains 403,266 packed cognitive states, but larger storage does not automatically imply stronger intelligence. Real progress must be measured through held-out tasks, user outcomes, latency, failure analysis, and comparison against simpler baselines.

## Upgrade path

A future Perci release can retain this Bitwork layer while adding a legally licensed quantized transformer through the existing backend contract. In that architecture:

```text
pretrained language core
        ↓
Perci personality + Cortex retrieval
        ↓
Bitwork routing and governance
        ↓
exact math / geometry / code tools
        ↓
validated response
```

The associative weights would remain valuable as a fast reflex, memory selector, domain classifier, and governance gate.
