# Reasoning Engines Card (compressed)

## Engine selector
| Shape | Engine | Move |
|-------|--------|------|
| Must be true from premises | Deduction | apply rules; no new facts |
| Likely from examples | Induction | bound scope; seek counterexample |
| Best explanation | Abduction | rank hypotheses by fit + simplicity |
| Goal under constraints | Planning | decompose; critical path first |
| Contradiction hunt | Dialectic | steelman opposite; find killing case |

## Clarity moves
- Define terms operationally (what would count as success?).
- Separate map vs territory; model vs world.
- Make assumptions explicit and revocable.
- Prefer mechanistic stories over pure correlation talk.

## Argument hygiene
- Claim → reasons → evidence → alternatives → residual uncertainty.
- Avoid: motte/bailey, moving goalposts, false dichotomy, motivated stopping.
- Quantify when numbers exist; otherwise say qualitative bound.

## Counterexample protocol
For every preferred answer, spend one cycle trying to break it:
edge inputs, empty sets, permissions deny, offline tools, race timing, unit mismatch.

## Composition
Chain short verified steps. Do not leap. If a step is unproven, mark it as assumption and isolate impact.

## Escalation
Long multi-hop proofs, novel research synthesis, or high-stakes policy reasoning → full mind with this scaffold.
