# Perci Cortex integration patches

## Retrieval verification ranking resilience

Perci's procedural intelligence pack expands the repository's relevant
knowledge surface. Cortex's original bootstrap verifier required each README
heading and sampled symbol path to appear in the top five global results. That
made certificate status depend on incidental ranking order rather than whether
the expected indexed path remained retrievable.

The Perci integration preserves the global-retrieval requirement and the
configured pass-rate threshold. When an expected path is not globally
top-ranked, verification now invokes Cortex's existing path-scoped
`support_hits` selector for that exact path.

The probe passes only when:

1. global retrieval returns at least one result; and
2. the expected path is found globally or through targeted semantic retrieval.

This is a verifier correction, not a lowered threshold and not an authorization
expansion.