# Memory, Learning, and Consolidation

## Memory classes

Fact, decision, preference, procedure, failure, correction, hypothesis,
evidence, outcome, constraint, and project state are different objects. Do not
store them as an undifferentiated pile.

## Consolidation

```text
raw event
-> classify
-> deduplicate
-> attach provenance
-> assign confidence
-> determine canonical status
-> connect related memories
-> mark superseded items
```

Current source and tests remain authoritative over generated summaries.

## Retrieval

```text
query
-> classify intent
-> lexical and semantic search
-> authority and recency weighting
-> bounded relationship expansion
-> deduplication
-> budget enforcement
-> provenance-bearing packet
```

Load only context that can change the answer.

## Learning from feedback

1. Record predicted and observed outcomes.
2. Classify failure as data, representation, strategy, execution, or validation.
3. Update only the responsible layer.
4. Add a regression example.
5. Strengthen a pattern only after repeated verified outcomes.

One correction must not rewrite unrelated knowledge.

## Staleness

A memory may be accurate historically and wrong currently. Store timestamps,
source hashes, supersession, and confidence. When origin state changes, reverify
before compounding.