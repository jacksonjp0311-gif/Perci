# Perci capability scorecard

_Generated 2026-07-22T14:34:12.698656+00:00_

**Overall status:** `OPERATIONAL_CANDIDATE`

Perci improves when a named capability fails a hardness case, is repaired at the correct layer, and passes transfer under a sealed gate before promotion.

## Gates

| Gate | Status | Passed | Cases |
|------|--------|--------|-------|
| hardness | PASS | 136 | 136 |
| dialogue | PASS | 159 | 159 |
| observer_context | PASS | 12 | 12 |

## Capabilities

| ID | Layer | State | Pass rate | Next |
|----|-------|-------|-----------|------|
| `cross_domain_synthesis` | operator | green | 23/23 | maintain with harder transfer variants |
| `relational_inquiry` | operator | green | 14/14 | maintain with harder transfer variants |
| `transfer_vs_template` | operator | green | 34/34 | maintain with harder transfer variants |
| `honest_abstention` | critic | green | 15/15 | maintain with harder transfer variants |
| `followup_binding` | operator | green | 8/8 | maintain with harder transfer variants |
| `exact_tool_authority` | tool | green | 14/14 | maintain with harder transfer variants |
| `governed_learning_loop` | pipeline | green | 15/15 | maintain with harder transfer variants |
| `binary_freshness` | pipeline | unmeasured | - | run evaluate_hardness.py |
| `geometry_speech` | operator | green | 3/3 | maintain with harder transfer variants |

## Binary freshness

- Status: `live_current_or_newer`
- Live mtime: 2026-07-22T14:34:04.563062+00:00
- Release mtime: 2026-07-22T14:27:01.064626+00:00
- Release ahead (s): -423.5

## Learning queue

- Interaction events: 21140
- Pending review events: 21138
- Review queue: 2420 (approved=20, folded=20)

## Recommended next

- Raise hardness: add entity-swapped / paraphrased cases to training/hardness/hardness-pack-v1.jsonl
