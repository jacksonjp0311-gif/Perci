# Neural Interlink

## Purpose

The interlink reduces repeated whole-repository scanning by propagating task activation through existing repository relationships.

## Node model

Each indexed file becomes a node. Thresholds vary slightly by evidence class:

- authoritative files activate more readily;
- source and tests use moderate thresholds;
- documentation and runtime evidence require slightly stronger excitation.

Node tags include file kind, language, path segments, suffix, and authoritative status.

## Synapse model

Synapses are compiled from current Cortex graph edges. Relation priors weight different forms of evidence:

- resolved imports and references;
- source-to-test relationships;
- co-change history;
- documentation links;
- imports, references, and calls.

Selected symmetric evidence classes receive bounded reverse associations.

## Activation

1. Hybrid retrieval produces task-relevant evidence chunks.
2. Chunk scores are normalized into file-level seed excitation.
3. Nodes integrate incoming excitation with a bounded nonlinearity.
4. Fired nodes propagate through the highest-weight existing synapses.
5. Depth and node budgets limit propagation.
6. Newly fired paths become support candidates.
7. The most relevant chunk from each support path competes for the context budget.

## Determinism

The activation state hash includes the repository, task hash, graph hash, seed strengths, and ordered activation records. Operational timestamps are excluded.

## Plasticity

Bounded Hebbian strengthening uses co-activation only on traversed existing synapses. Weight updates move toward a fixed upper bound and are capped.

```text
delta = eta * pre * post * (maximum - current)
```

Plasticity is disabled in `read_only` mode and cannot create new topology.

## Efficiency metrics

Activation packets expose:

- total nodes;
- nodes considered;
- nodes fired;
- support nodes;
- propagation steps;
- sparse activation ratio;
- considered fraction;
- maximum depth.

These are operational efficiency measurements, not biological claims.
