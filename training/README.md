# Perci training

Perci v0.1 includes a fully generated 200 MiB binary associative weight pack.

## Reproducible build

```powershell
python .\scripts\build_weights.py --output .\models\perci-cognitive-v0.1.pwgt
```

The builder uses a fixed seed and produces:

- 403,266 binary prototypes
- 4,096-bit activations
- 16 expert partitions
- learned positive expert masks
- an exact 200 MiB output file
- a JSON manifest containing checksums and limitations

## Evaluation

```powershell
python -m pip install numpy
python .\scripts\test_weights.py
```

The included `heldout-evaluation.txt` records the build-time probe results. This test measures domain routing and prototype retrieval only.

## Native language track

Perci v0.8.11 can rebuild native binary language fields without an external model:

    cargo run --release -- language train --repo
    cargo run --release -- language sample "explain a boundary in plain language"

The field is a multi-order byte sequence learner. Its weights are binary
threshold planes, while the tokenizer-free runtime keeps the hot path
inspectable and has no external model dependency. It improves local phrasing and continuation,
but it is not a web-scale language model and should not be evaluated as one.

The same command also builds `PERCPHR1`, a capped word/phrase transition field
with numeric token IDs and binary threshold-coded edges. It supplies a more
compositional generation path while retaining the same review, hold-out, and
explicit-promotion rules.

It also builds `PERCREL1` and `PERCIWM1` side fields. `PERCIWM1` is a typed,
bounded subject/relation/object memory with domain, polarity, confidence, and
evidence bins. It is a reranking signal, not an authority layer; keep candidate
world fields isolated until adversarial held-out checks show a reproducible
improvement.

Generate the adversarial gate with:

    python scripts/adversarial_curriculum.py models/candidates/adversarial-v0.8.4-heldout.jsonl --count 120 --offset 300

The native backend also maintains a fixed 256-bit recurrent dialogue state.
It is replayable from recent turns, affects primer/sampling selection, and is
never promoted as factual knowledge.

Raw private conversation exports should not be trained directly. Review,
redact, deduplicate, separate canonical from speculative content, and hold out
evaluation cases before rebuilding. Deliberate training is a candidate artifact;
it does not silently promote facts or rewrite the cognitive pack.

For noisy human input, review the paired routing curriculum in
`training/dialogue-typo-v1.jsonl`. It tests explicit aliases, one-edit
transpositions/substitutions, exact-tool preservation, cross-domain routing, and
an invented-name control. The normalizer is intentionally bounded: it repairs
known language terms but never treats a fuzzy match as new knowledge.

### Reviewed continuity candidate

The next native-language phase uses a small reviewed corpus and a separate
held-out set:

```powershell
python scripts/train_dialogue_candidate.py
python scripts/evaluate_dialogue_candidate.py `
  --phrase-weights models/candidates/perci-dialogue-continuity-v1.bphr
```

`dialogue-continuity-v1.jsonl` contains 24 train examples across disagreement,
revision, clarification, typo repair, depth, evidence, and creative turns.
`dialogue-continuity-v1-heldout.jsonl` contains 12 paraphrased checks. The
builder trains response continuations behind the same `<topic>` primer
contexts used at runtime and writes an isolated PERCPHR1 candidate plus a
hash receipt. It never replaces the active field.

The first candidate matched the active chat path on this small held-out set:
`5/12` required checks, `0.4167` required rate, `0.5` topic binding, and zero
duplicate responses in both arms. The result is therefore `HOLD`, not a failed
build: the candidate loads and changes the direct phrase sampler, but the
voice/operator layer still dominates full dialogue. Scale the corpus and add
fresh paraphrases before considering promotion.

### Prompt-conditioned paired-turn candidate

The next context experiment keeps the previous user turn, a bounded prior
answer, and the current turn in hidden transition history. Build and evaluate it
without touching the active field:

```powershell
python scripts/train_dialogue_candidate.py `
  --corpus training/dialogue-continuity-v2.jsonl `
  --prompt-conditioned `
  --output-base models/candidates/perci-dialogue-continuity-v2.blng
python scripts/evaluate_dialogue_candidate.py `
  --questions training/dialogue-continuity-v2-heldout.jsonl `
  --phrase-weights models/candidates/perci-dialogue-continuity-v2.bphr `
  --output models/candidates/evaluation-dialogue-continuity-v2.json
```

The first paired-turn candidate tied its baseline at `3/12` required checks
(`0.25`) and `0.2917` topic binding. It is a valid isolated artifact, not a
promotion: increase the reviewed conversation diversity before changing the
active field.

### Relational controller v3

`dialogue-relational-v3.jsonl` adds state-conditioned examples for referent
binding, prior-claim recovery, revision, repetition, out-of-distribution
abstention, and transfer. Its six-pair holdout is intentionally small and
unseen. The isolated candidate tied the active arm at `1/6` required checks and
remains `HOLD`; the runtime controller and critic are the current experiment,
not the candidate phrase weights.

### Cross-domain lattice v1

`dialogue-cross-domain-v1.jsonl` teaches the reviewed shape of a cross-domain
answer: name every requested frame, state the shared axis, preserve each
mechanism, and attach a domain-specific test. The held-out file contains warm
seed/follow-up turns, natural “analyze across domains” wording, unknown-domain
coverage, evidence requests, and novel-domain transfer.

Run the live evaluation with:

```powershell
python scripts/evaluate_cross_domain.py `
  --perci-bin target\release\perci.exe `
  --output models\candidates\evaluation-cross-domain-v1.json
```

The current v0.9.0 receipt is `12/12 PASS`. This measures controller and
evidence-map behavior; it does not authorize phrase-weight promotion.

### Governed-core charter v1

The charter is a runtime policy surface, not a weight-training shortcut. Keep
fixtures for evidence-before-claim, explicit uncertainty, destructive-action
refusal, reversible repair, and human authorization separate from language
phrases. Unit tests and fresh-process dialogue checks must show that the policy
is present in traces and Fabric plans while the active Bitwork pack remains
unchanged.

## Layered low-bit training (PERCLBW1)

The native Rust path also packs reviewed row-major matrices into a layered
low-bit sidecar. Ternary sign/zero planes carry structure, block scales carry
magnitude, residual planes carry approximation error, and a bounded correction
lane protects repeated directions. INT4 activations use a sparse higher-
precision escape lane for outliers. This is the numerical substrate for a
future Perci-native matrix layer; it is not a claim that the current sparse
associative pack is a Transformer or that low reconstruction error implies
language-model intelligence.

```powershell
cargo run --release -- lowbit train `
  training\lowbit\example-train.json `
  models\candidates\perci-lowbit-example.blw `
  --block-size 64 --residual-planes 2 --rank 8
```

The command writes a `.blw` candidate and a JSON receipt with checksums,
baseline/candidate weight error, held-out matrix-vector error, and an explicit
`promote_recommended: false`. Candidate promotion remains a separate,
human-authorized evaluation decision.

Re-open the bytes for an independent assessment:

```powershell
cargo run --release -- lowbit assess `
  training\lowbit\example-train.json `
  models\candidates\perci-lowbit-example.blw
```

This checks the serialized field and returns `PASS` only when the candidate
still beats the baseline on the reviewed probes. A hold is a useful result: it
prevents a better-looking in-memory reconstruction from becoming a false
weight claim.
