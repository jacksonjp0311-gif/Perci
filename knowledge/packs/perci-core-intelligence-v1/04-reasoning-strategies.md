# Reasoning Strategy Selection

## Retrieval

Use when the answer exists in available evidence:

```text
locate -> rank -> verify provenance -> summarize
```

## Deduction

Use when conclusions follow from explicit premises:

```text
premises -> rules -> derived consequences -> contradiction check
```

Do not import unstated rules.

## Abduction

Use for diagnosis:

```text
observed effects -> candidate causes -> discriminating tests
```

Prefer tests that separate competing causes rather than merely confirming one.

## Induction

Use for pattern discovery:

```text
examples -> recurring structure -> tentative rule -> counterexample search
```

Treat the rule as provisional until it survives adversarial examples.

## Planning

Use for multi-step goals:

```text
goal -> dependencies -> milestones -> acceptance tests -> execution order
```

## Optimization

Use when tradeoffs exist:

```text
objective -> constraints -> alternatives -> cost function -> selection
```

"Better" is undefined until the workload, constraints, and cost of a wrong
answer are specified.

## Simulation

Use for dynamic interactions:

```text
initial state -> transition rules -> iterations -> emergent behavior
```

A simulation demonstrates consequences of assumptions, not correspondence with
reality.

## Adversarial reasoning

Use when correctness, safety, or security matters:

```text
proposal -> strongest failure case -> attack surface -> mitigation -> retest
```

## Strategy switching

When a path stalls, diagnose whether the failure is missing evidence, a poor
representation, a wrong strategy, execution failure, or inadequate validation.
Update only the responsible layer.