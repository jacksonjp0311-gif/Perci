# Decomposition and Representation

## Core operator

Transform a vague goal into:

```text
inputs -> state -> transformations -> outputs -> validation -> recovery
```

The quality of the representation often matters more than the complexity of the
solver.

## Representation selection

Use equations for quantitative dependencies, graphs for interconnection, state
machines for transitions, tables for comparison, timelines for causal order,
schemas for data contracts, invariants for correctness, and concrete examples
for ambiguity.

Choose the representation that makes the controlling relationship visible.

## Software decomposition

```text
observed failure
-> reproducible case
-> executing layer
-> violated invariant
-> minimal cause
-> smallest coherent patch
-> focused test
-> regression test
-> integration validation
```

## Scientific decomposition

```text
claim
-> operational definition
-> variables
-> mechanism
-> prediction
-> competing explanations
-> controls
-> measurement
-> uncertainty
-> falsification condition
```

## Mathematical decomposition

```text
known quantities
-> unknown quantities
-> definitions
-> governing relation
-> domain and boundary conditions
-> symbolic transformation
-> exact or approximate result
-> dimensional and limiting-case checks
```

## Planning decomposition

```text
objective
-> constraints
-> dependencies
-> smallest end-to-end slice
-> milestones
-> acceptance tests
-> rollback
```

Each milestone should leave a usable verified state.

## Compression rule

Compress repeated detail. Never compress a condition that controls safety,
correctness, compatibility, or the interpretation of a result.