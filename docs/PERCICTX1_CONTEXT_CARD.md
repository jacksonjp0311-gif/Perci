# PERCICTX1 — Context Cards, Geometry Lines, and Observer-Aware Speech

PERCICTX1 is the bounded interface between Perci's routing substrate and its
human-facing language. It does not claim consciousness and it is not private
chain-of-thought. It records the distinctions a response must preserve so an
observer can reconstruct the active context.

## The mathematical hypothesis

Let:

- (C) be the intended context;
- (O) be the rendered answer;
- (hat C) be the context reconstructed by an observer from (O).

Smooth speech is only one variable:

\[
S = \text{fluency of the output channel}
\]

The two variables that keep fluency honest are:

\[
F = P(\hat C = C)
\]

for context fidelity, and:

\[
V = P(\text{useful next action}\mid O,C)
\]

for viability. A fluent generic answer can have high (S) while (F) and
(V) remain low.

Perci therefore records the bounded proxy:

\[
Q_{observer}
  = H(S,F,V,G)\,(1-P_{over})
\]

where (G) is geometry-line alignment, (P_{over}) is an over-smoothing
penalty, and (H) is the harmonic mean:

\[
H(x_1,\ldots,x_n)
  = \frac{n}{\sum_i 1/x_i}.
\]

The harmonic mean is deliberate: a smooth answer cannot hide a missing
referent, a missing mechanism, or a broken cross-domain relation. This scalar is
an engineering proxy, not proof of understanding.

## Context-card envelope

Every card has a stable envelope:

```text
schema, intent, act, goal, topic, entities, referent,
evidence, uncertainty, continuity, response_budget, prior_turns
```

The payload is modular. Geometry, dialogue, governance, memory, and language do
not need identical internal fields; they need a shared interface that preserves
intent, scope, evidence, uncertainty, and the requested response shape.

Examples:

```text
music + code + geometry --shares-axis--> structure
current-topic --continues--> prior-referent
explain --targets--> memory
```

These are geometry lines: subject, relation, object, and support. They give the
renderer a load-bearing relation instead of a bag of nearby words. A cross-
domain line must preserve the mechanism boundary: sharing an axis does not
make the domains share a physical cause.

## Speech contract

The card supplies a compact directive to the local backend:

```text
lead with the answer;
preserve the geometry relation;
match the requested depth;
name uncertainty when it matters;
keep evidence separate from metaphor.
```

The final response remains ordinary prose. The card and metrics appear in the
inspectable trace, not in the user's conversation unless they ask for them.

## Current implementation

- `src/context_card.rs` implements `PERCICTX1`, geometry lines, metrics, and
  harmonic scoring.
- `src/chat.rs` derives the card before routing, passes its speech directive to
  the backend, and records the card/observer metrics in the deliberation trace.
- Existing `DialogueWorkspace`, semantic frames, `ThoughtPlan`, `PERCPHR1`,
  and Frontier Arc speech remain intact; the card is an additive envelope over
  them.

## Held-out observer gate

`scripts/evaluate_context_observer.py` runs the sealed cases in
`models/candidates/evaluation-context-observer-v1.json` against one fresh local
process. The current receipt (`evaluation-context-observer-latest.json`) is
**12/12 PASS**, with mean observer score **0.919** and mean geometry alignment
**1.000**. The gate is intentionally external to the response trace: it checks
what an observer can recover from rendered speech rather than accepting a
private reasoning claim. Its token checks are a first bounded proxy; future
rounds should add semantic graders, entity swaps, and human pairwise review.

## Required tests for future evolution

An improvement is valid only if it preserves or improves:

1. referent recovery after a follow-up;
2. intent recovery after a paraphrase;
3. relation preservation after an entity swap;
4. counterexample-driven revision;
5. observer viability on the requested next action;
6. exact-tool and abstention authority;
7. latency and output-length budgets.

Fluency alone is not a promotion gate. A candidate that sounds smoother while
losing context fidelity or geometry alignment remains a hold.
