# Perci capability scorecard

_Generated 2026-07-17T01:47:47.565486+00:00_

**Overall status:** `OPERATIONAL_CANDIDATE`

Perci improves when a named capability fails a hardness case, is repaired at the correct layer, and passes transfer under a sealed gate before promotion.

## Gates

| Gate | Status | Passed | Cases |
|------|--------|--------|-------|
| hardness | PASS | 44 | 44 |
| dialogue | PASS | 159 | 159 |

## Capabilities

| ID | Layer | State | Pass rate | Next |
|----|-------|-------|-----------|------|
| `cross_domain_synthesis` | operator | green | 13/13 | maintain with harder transfer variants |
| `relational_inquiry` | operator | green | 8/8 | maintain with harder transfer variants |
| `transfer_vs_template` | operator | green | 3/3 | maintain with harder transfer variants |
| `honest_abstention` | critic | green | 7/7 | maintain with harder transfer variants |
| `followup_binding` | operator | green | 3/3 | maintain with harder transfer variants |
| `exact_tool_authority` | tool | green | 7/7 | maintain with harder transfer variants |
| `governed_learning_loop` | pipeline | green | 3/3 | maintain with harder transfer variants |
| `binary_freshness` | pipeline | unmeasured | - | run evaluate_hardness.py |

## Binary freshness

- Status: `synced`
- Live mtime: 2026-07-17T01:47:28.950312+00:00
- Release mtime: 2026-07-17T01:47:28.950312+00:00
- Release ahead (s): 0.0

## Learning queue

- Interaction events: 533
- Pending review events: 533
- Review queue: 338 (approved=20, folded=20)

## Recommended next

- Raise hardness: add entity-swapped / paraphrased cases to training/hardness/hardness-pack-v1.jsonl
