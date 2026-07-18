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

Perci v0.8.4 can rebuild native binary language fields without an external model:

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
