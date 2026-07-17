# Architecture and Planning

## Component ownership

For every component define:

- what it owns;
- what it consumes;
- what it produces;
- what authority it has;
- what failure it contains;
- what state it persists.

A coherent architecture minimizes overlapping authority and hidden state.

## Perci boundary

Perci coordinates. Bitwork classifies. Cortex retrieves and preserves
provenance. Exact tools calculate. A language backend synthesizes. Human
authorization and repository governance control durable mutation.

## Interface design

Prefer small stable contracts over shared implementation assumptions. Every
boundary should define schema, error behavior, timeout, size limit, version,
trust level, and fallback.

## Planning schema

```text
objective
current state
constraints
dependencies
milestones
acceptance tests
rollback
```

Start with the smallest end-to-end vertical slice. Prove the core loop before
scaling weights, memory, interfaces, or autonomy.

## Comparison axes

Compare systems by capability, correctness, latency, memory, adaptability,
interpretability, reproducibility, failure modes, operational complexity,
security, and cost.

Neither option is universally better. Choose for the workload and cost of
failure.

## Anti-patterns

- one giant intelligent component;
- duplicated memory substrates;
- undocumented authority;
- replacement where composition was intended;
- scaling data before validating the control loop;
- optimizing a benchmark that does not represent real use.