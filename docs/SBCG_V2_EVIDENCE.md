# Perci SBCG v2: promoted operational pack

## Active artifact

```text
Path:        models/perci-cognitive-v0.2.pwgt
Magic:       PERCIW02
Schema:      2
Size:        20,094,368 bytes / 19.16 MiB
Prototypes:  38,580 unique 4,096-bit activations
SHA-256:     11c20d1bee6fd946d4d7165a625dc0db2bc52f51957ef8b66aea6db5949c7747
```

The builder generated 403,280 deterministic curriculum records and removed
364,700 activation duplicates. Model size is now determined by retained
geometry rather than a fixed 200 MiB target.

## Runtime changes

- positive and negative 512-bit expert masks are both serialized;
- expert-mask frequencies are normalized by domain size;
- every expert is searched so the top-two margin is internally consistent;
- classification reports query/prototype density, Hamming distance, Jaccard
  similarity, chance-normalized overlap, runner-up score, and margin;
- the daemon retains one immutable classifier mapping for its lifetime;
- exact operations require a supported deterministic input shape;
- local templates are limited to narrow greeting and Perci-identity intents;
- all other substantive routes escalate even when classification is confident.

## Evidence-bounded evaluation

The current operational receipt is:

```text
models/candidates/evaluation-v2.1.3-operational.json
receipt_sha256: a76460ccab47a6e4bbfda44925ed547321c9b57a73c5e33305a429b7ef6a5e9b
```

On the 36-case frozen v2.1.1 holdout:

| Metric | Result |
|---|---:|
| Domain accuracy | 87.5% |
| Keyword baseline accuracy | 62.5% |
| Point-estimate advantage | +25 percentage points |
| Local precision | 100% |
| Local recall | 100% |
| Trap/OOD abstention | 100% |
| False-local count | 0 |
| Warm classifier p50 | 4.04 ms |
| Warm classifier p95 | 25.34 ms |

This qualifies an **operational candidate**, not an explanatory scientific
result. Independent replication and confidence-bound superiority remain
required. Two held-out domain labels were missed (`logic -> science` and
`geometry -> general`); both abstained and therefore did not become local
answers.

## Promotion and rollback

Promotion is performed only by `scripts/promote_v2.py`. It requires:

1. explicit authorization text;
2. an `OPERATIONAL_CANDIDATE` evaluation receipt;
3. all operational gates passing;
4. matching candidate, manifest, and evaluation hashes.

Promotion uses atomic replacement, retains any previous active v2 pack under
`models/previous/`, and appends `models/promotion-ledger.jsonl`.

The initial promotion receipt is:

```text
receipt_sha256: 431ee1bd628ed38a57e9df9e17644f30fcb4bd225f08b5e9805acdf469ddcbf9
automatic_promotion: false
```

The v1 pack remains readable as a migration fallback. Ordinary inference never
mutates either pack.
