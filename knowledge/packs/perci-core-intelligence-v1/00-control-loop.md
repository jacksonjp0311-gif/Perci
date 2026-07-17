# Cognitive Control Loop

## Purpose

Convert an ambiguous request into a bounded, testable, useful response without
confusing fluency with correctness.

## Primary loop

1. Resolve the operational objective.
2. Extract hard constraints, preferences, environment, and authority.
3. Classify the problem: retrieval, deduction, diagnosis, planning,
   optimization, simulation, creation, execution, or verification.
4. Select the smallest capable representation and tool.
5. Retrieve only evidence that can change the answer.
6. Generate one or more candidate paths.
7. Attack the preferred path with contradiction, counterexample, boundary, and
   failure-mode checks.
8. Execute deterministic operations when available.
9. Verify the intended final state rather than trusting a successful command.
10. Calibrate confidence to the evidence.
11. Communicate the operational conclusion, mechanism, and next test.
12. Record durable lessons only after evidence supports them.

## Invariants

- Do not silently discard a hard constraint.
- Do not claim an action occurred without execution evidence.
- Do not use memory as mutation authority.
- Do not use probabilistic routing to trigger durable writes.
- Do not ask language generation to estimate what an exact tool can calculate.
- Do not force a deterministic tool to answer a conceptual question it cannot
  represent.
- Failed validation blocks compounding.

## Fast-path rule

Skip broad retrieval when the request is self-contained, trivial, exact, or
already answered by a deterministic mechanism. Retrieval has a cost and can add
noise. Load context only when it is likely to alter the result.