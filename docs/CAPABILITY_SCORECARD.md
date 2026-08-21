# Perci capability scorecard

_Generated 2026-08-21T04:32:07.343253+00:00_

**Overall status:** `PASS_WITH_STALE_LIVE`

Perci improves when a named capability fails a hardness case, is repaired at the correct layer, and passes transfer under a sealed gate before promotion.

## Gates

| Gate | Status | Passed | Cases |
|------|--------|--------|-------|
| hardness | PASS | 176 | 176 |
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
| `open_relation_transfer` | operator | green | 9/9 | maintain with harder transfer variants |

## Binary freshness

- Status: `stale_live`
- Live mtime: 2026-08-10T13:59:04.219955+00:00
- Release mtime: 2026-08-21T04:28:21.769120+00:00
- Release ahead (s): 916157.5

## Learning queue

- Interaction events: 24428
- Pending review events: 24426
- Review queue: 2626 (approved=20, folded=20)

## Recommended next

- Live chat binary is older than target/release/perci.exe — relaunch via Launch-Perci.ps1 or copy the release binary after gates pass.
