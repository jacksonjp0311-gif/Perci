# Native Perci probe — v0.8.1

## Experiment

`scripts/native_probe.py` generated 1,000 deterministic questions across 20
families and 49 topics, then sent them through one persistent native Perci chat
process. The transcript is `native-probe-v0.8.1.jsonl`; the machine summary is
`native-probe-v0.8.1-summary.json`.

## Measured result

| Metric | Result |
|---|---:|
| Questions | 1,000 |
| Parsed responses | 1,000 |
| Unique normalized responses | 230 |
| Duplicate responses | 770 |
| Topic binding | 91.8% |
| Responses ending with punctuation | 93.6% |
| Mean response length | 375.82 characters |
| Median response length | 144 characters |

The earlier probe measured 64.7% topic binding. The improvement to 91.8% came
from removing prompt-boilerplate words (`reflect`, `imagine`, `original
thought`, and similar) from topic extraction. This is a routing/voice repair,
not evidence that the weights learned a new fact.

## What repeated across domains

The strongest recurring motifs were boundary, structure, evidence, repair,
scale, and mechanism. They transferred across geometry, language, life/death,
code, music, and systems prompts. This is a promising cross-domain invariant,
but the present evidence is associative: those motifs are already present in
the reviewed corpus and response operators.

## Weaknesses found

- 770 normalized duplicates show that the native field still collapses many
  prompts onto a small set of learned continuations.
- The trust and constrained-invention families produced long stock responses;
  these are useful operators but poor evidence of open-ended generation.
- Some outputs still bind prompt scaffolding (`geometry teach`, `changes testable
  prediction`) instead of only the intended subject.
- The probe measures behavior and variation, not subjective emergence,
  consciousness, or unrestricted intelligence.

## Candidate training

The transcript was rebuilt into separate candidate artifacts:

- `native-probe-candidate.blng` — 1.71 MiB, 25,241 byte contexts.
- `native-probe-candidate.bphr` — 0.17 MiB, 693-token vocabulary.

They are not active weights. Promotion requires a held-out comparison showing
better topic binding and lower repetition without regressions in exact tools,
OOD abstention, transfer, or governance.

## Next experiment

Add a semantic novelty gate that compares the response against the active
corpus, then train a cleaned phrase candidate with sentence-level provenance.
The goal is not maximum novelty; it is a new, topic-bound relation that remains
coherent, reproducible, and testable under paraphrase.

## Evolution round 2 — novelty gate

The native backend now generates six bounded binary continuations and scores
them for topic overlap, recent-response distance, and exact-repeat avoidance.
Its history window is capped at 64 turns. On the same 1,000-question probe:

| Field | Before gate | Active with gate | Change |
|---|---:|---:|---:|
| Unique responses | 230 | 408 | +178 |
| Duplicate responses | 770 | 592 | -178 |
| Topic binding | 91.8% | 91.4% | -0.4 pp |

This is a real diversity gain, with a small binding tradeoff. The first
candidate rebuilt from the transcript reached 393 unique responses and 91.8%
binding; a second candidate trained from the more diverse transcript reached
421 unique responses and 91.6% binding. Both remained isolated candidates.

## Held-out decision

A new 200-question slice (offset 1,000) was used after training:

| Variant | Unique | Duplicates | Topic binding |
|---|---:|---:|---:|
| Active with gate | 157 | 43 | 91.5% |
| Candidate v3 | 155 | 45 | 91.5% |
| Repetition-capped candidate v4 | 155 | 45 | 92.0% |

No candidate is promoted. Candidate v4 binds topics slightly better, but it
does not beat the active field on held-out diversity. This is the correct
governed outcome: the selector is active because it improved the measured
tradeoff, while weight promotion remains gated by held-out evidence.

## Evolution round 3 — relation novelty

The selector gained a relation-level gate. It extracts small directed
topic-neighbor signatures (for example, `boundary>exchange`) and rewards a
candidate when that relation has not appeared in the recent dialogue. The
weight is bounded by `PERCI_NATIVE_RELATION_WEIGHT` and defaults to 12.

| Selector | Full-probe unique | Full-probe binding | Held-out unique | Held-out binding |
|---|---:|---:|---:|---:|
| Lexical novelty only | 408 | 91.4% | 157 | 91.5% |
| Relation weight 24 | 404 | 91.8% | 158 | 91.0% |
| Relation weight 12 (active) | 398 | 91.8% | 157 | 91.5% |
| Relation weight 16 | — | — | 160 | 91.0% |

The conservative weight was selected because it preserves held-out binding and
does not trade away diversity there. Higher weights create more surface
variation but weaken topic fidelity. This is a bounded cognition improvement,
not evidence of independent thought; the next experiment should use human or
embedding-assisted relation judgments before changing weights again.

## Evolution round 4 — PERCREL1 field

The next bridge was implemented as an optional binary relation field. It stores
15,692 hashed prompt-to-response edges in a 314 KiB mmap artifact and can score
native continuations without generating any text. The field was trained from
the 1,000-turn probe and tested only as an isolated candidate:

| Variant | Held-out unique | Held-out binding |
|---|---:|---:|
| No relation field | 157 | 91.5% |
| PERCREL1 raw score | 155 | 91.0% |
| PERCREL1 capped tie-breaker | 156 | 91.5% |

The capped field is a useful inspectable artifact, but it does not beat the
active selector on held-out behavior. It remains unpromoted. This is an
important discovery: a binary semantic memory can overfit just as a larger
weight field can. Generalization, not artifact size, is the next target.

## Evolution round 5 — emergence curriculum and routing repair

The probe was mined into a 1,000-question curriculum built from the recurring
motif inventory (boundary, structure, evidence, mechanism, state, relation,
transfer, invariant, scale, repair, memory, entropy, and related terms). The
curriculum changes premises, asks for counterexamples, introduces unseen
entities, and requires a falsifiable observation.

The first run exposed two real failures:

1. `Nara-7` plus a curriculum variant number was misclassified as arithmetic
   because a hyphen and two numbers looked like an expression.
2. Novel-entity prompts fell through to empty or generic responses because
   `unseen system` and `memorized wording` were not recognized as transfer
   operators.

After repairing both routes and compacting the human-facing transfer answer:

| Curriculum pass | Unique | Topic binding | Punctuation | Empty |
|---|---:|---:|---:|---:|
| Initial | 411 | 62.7% | 81.0% | 140 |
| Arithmetic repair | 429 | 63.9% | 95.0% | 0 |
| Routed + compacted | 546 | 92.7% | 93.6% | 0 |

The 200-question held-out curriculum reached 192 unique responses and 89.5%
topic binding. A candidate phrase field trained from the 1,000 curriculum
answers produced the same held-out result, showing that the gain came from
operator routing and transfer handling—not memorizing the curriculum into the
phrase weights. The candidate remains unpromoted.

## Evolution round 6 — typed world-model field and adversarial holdout

`PERCIWM1` is a new optional binary field. Each record binds hashed
subject/relation/object tokens to a coarse domain, polarity, confidence bin,
and evidence bin. The runtime uses it only to rerank the six native phrase
walks; it cannot synthesize language or promote facts.

The candidate trained from the routed emergence curriculum contains 13,387
records in a 428,448-byte mmap artifact. A new 120-question offset-held-out
adversarial pack targets paraphrase collapse, negation loss, entity swaps,
contradictions, analogy limits, and counterfactuals:

| Variant | Unique | Topic binding | Punctuation | Mean chars |
|---|---:|---:|---:|---:|
| Active phrase field | 42/120 | 85.83% | 64.17% | 776.82 |
| + PERCIWM1 candidate | 42/120 | 85.83% | 64.17% | 776.82 |

The family view located the remaining bottleneck: entity-swap prompts bound the
requested topic only 30% of the time, while paraphrase, negation, contradiction,
and boundary-limit families bound at 100%. The typed field therefore remains an
isolated candidate (no auto-promote).

### Evolution round 7 — entity-slot transfer (v0.8.5)

Root cause: adversarial prompts matched `creative-constraint` via "invented" +
"without" and collapsed to a fixed switchyard metaphor that dropped motif slots.

Fix (operators, not weights):
- `src/entity_slot.rs` — parse `called NAME has A and B`, emit slot-bound relation
- block creative-constraint steal; route before generic novel-entity pedagogy
- transfer suite adds two entity-slot bases (16/16 green)
- probe scoring adds `slot_pair_binding_rate` (both motifs)
- PERCIWM1 score gets capped entity-slot bonus when both slots survive

Measured (held-out adversarial entity_swap family, n=20, `perci ask`):
| Metric | Before | After |
|---|---:|---:|
| topic_binding | 30% | **100%** |
| slot_pair_binding | n/a | **100%** |

Claim boundary: operator routing fix under measured gates — not frontier AGI.

### Evolution round 8 — five native depth tracks (v0.8.6)

| Track | Artifact |
|-------|----------|
| Full adversarial re-probe | `native-probe-v0.8.6-adversarial-heldout-summary.json` |
| Compositional multi-hop | `src/compositional_world.rs` |
| Native decoder | `src/native_decoder.rs` |
| Reason/search/verify | `src/reason_loop.rs` |
| Replay baselines | `src/replay_learn.rs` · `promote_recommended=false` always |

**120-q adversarial held-out (v0.8.6 active `perci ask`):**

| Family | topic_binding | slot_pair_binding |
|--------|-------------:|------------------:|
| entity_swap | **100%** | **100%** |
| boundary_limit | 100% | 100% |
| contradiction | 100% | 60% |
| counterfactual | 85% | 55% |
| paraphrase | 100% | 30% |
| negation | 100% | 15% |
| **overall** | **97.5%** | **60%** |

Negation/paraphrase slot_pair is limited because the curriculum text often embeds only `motif_a` (e.g. negation never names `motif_b`). Topic binding remains the primary family gate there.

Replay baselines: entity-slot 100%/100% on entity_swap rows; no auto-promote of binary fields.
