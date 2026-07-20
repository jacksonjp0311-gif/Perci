# Typed dialogue workspace

Phase 8 adds a small, inspectable working-memory record for each human-facing
turn. The v0.9 relational loop now uses that record to choose a bounded plan,
critique the rendered answer, and apply only safe continuity repairs. It is a
routing and composition layer, not hidden chain-of-thought and not a claim that
the binary phrase field has become a frontier language model.

`DialogueWorkspace::derive` records:

| Field | Purpose |
| --- | --- |
| `act` | answer, follow-up, challenge, revision, test, synthesis, plan, social, or learning intent |
| `goal` | inform, explain, repair, evaluate, create, plan, learn, relate, or social outcome |
| `topic` | a short typo-repaired salient topic key |
| `prior_claim` | a bounded first-sentence view of the latest assistant claim |
| `referent` | the previous topic when a turn uses terms such as “that”, “it”, or “again” |
| `evidence` | none, seeking, supplied, or exact evidence posture |
| `uncertainty` | unmarked, explicit, referential, or out-of-distribution signal |
| `continuity` | new thread, threaded, or referential turn |
| `response_budget` | brief, balanced, or deep answer budget |

The record is derived from the raw turn, bounded recent history, and the
existing voice classifier. It is added as a hidden hint to native,
deterministic, and optional adapter paths so they share one view of the
conversation. The user-facing response still comes from governed operators,
exact tools, the native phrase field, and the voice layer.

## Why this is the next leverage point

The Phase 7 candidate experiments showed that adding phrase transitions did not
by itself fix full-dialogue continuity: the active and candidate fields tied on
held-out follow-up checks. That result points to a state-representation boundary.
A turn that says “why does that matter?” needs an explicit referent, goal, and
depth budget before additional language weights can use it reliably.

The workspace makes that boundary testable without silently promoting weights.
Its stable `hint()` form is suitable for `/trace`, candidate training context,
and future held-out evaluation. Unknown tokens remain marked as out of
distribution rather than being forced into a familiar topic.

## v0.9 relational loop

The workspace selects one of a few inspectable plans: `relational_followup`,
`relational_revision`, `claim_challenge`, `bounded_synthesis`, or
`direct_answer`. A plan names operations such as binding the prior claim,
testing a counterexample, preserving a mechanism boundary, or answering at a
requested depth. It does not expose or generate a private chain of thought.

Before a human-facing answer is committed, the workspace critic checks for
missing referents, accidental repetition, a missed deep-answer budget, generic
fallback text, and confident output for out-of-distribution tokens. A missing
referent may receive a reversible topic-binding repair. Other failures remain
visible as audit flags rather than being patched with invented content. An
empty renderer result gets a non-empty, honest fallback instead of a blank turn.

## Cross-domain analysis and local knowledge

The workspace now shares a bounded cross-domain summary with the operator and
Capability Fabric. It canonicalizes common aliases such as `biology → life`,
selects a shared axis only when the local frame catalog supports one, and
keeps a mechanism plus test for each known domain. Unknown requested domains
remain in a missing-coverage list; they are not filled with fluent specialist
claims.

For prompts that ask for evidence, the Fabric performs per-domain pack probes
and a mixed-query probe. Retrieved cards are source-bearing context only. A
cross-domain follow-up therefore answers with separate tests and controls for
each domain, while the shared relation remains a hypothesis until those tests
support it. The warm held-out suite is
`training/dialogue-cross-domain-v1-heldout.jsonl`; its current receipt is
`12/12 PASS`.

## Acceptance criteria

Future workspace changes should preserve:

1. exact-tool routing and arithmetic correctness;
2. abstention for invented or ungrounded terms;
3. follow-up referent binding across at least three turns;
4. response depth that follows the user's request rather than a fixed template;
5. no automatic fact or weight promotion.

The current unit tests cover referential follow-up binding, evidence/depth
extraction, and out-of-distribution abstention. Broader paired-turn holdouts
remain the gate for any future native weight candidate.

The governed-core hypothesis ledger extends this workspace with claim type,
evidence posture, and a falsifiable next check. It also preserves specialized
operator ownership when the language sidecar is available, preventing a generic
continuation from replacing an evidence-bound answer.
