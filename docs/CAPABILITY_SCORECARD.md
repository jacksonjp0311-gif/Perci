# Perci capability scorecard

_Generated 2026-07-20T12:40:09.672411+00:00_

**Overall status:** `PASS_WITH_STALE_LIVE`

Perci improves when a named capability fails a hardness case, is repaired at the correct layer, and passes transfer under a sealed gate before promotion.

## Gates

| Gate | Status | Passed | Cases |
|------|--------|--------|-------|
| hardness | PASS | 100 | 100 |
| dialogue | PASS | 159 | 159 |

## Capabilities

| ID | Layer | State | Pass rate | Next |
|----|-------|-------|-----------|------|
| `cross_domain_synthesis` | operator | green | 20/20 | maintain with harder transfer variants |
| `relational_inquiry` | operator | green | 11/11 | maintain with harder transfer variants |
| `transfer_vs_template` | operator | green | 30/30 | maintain with harder transfer variants |
| `honest_abstention` | critic | green | 12/12 | maintain with harder transfer variants |
| `followup_binding` | operator | green | 3/3 | maintain with harder transfer variants |
| `exact_tool_authority` | tool | green | 13/13 | maintain with harder transfer variants |
| `governed_learning_loop` | pipeline | green | 11/11 | maintain with harder transfer variants |
| `binary_freshness` | pipeline | unmeasured | - | run evaluate_hardness.py |

## Binary freshness

- Status: `stale_live`
- Live mtime: 2026-07-20T12:28:59.933708+00:00
- Release mtime: 2026-07-20T12:37:02.187252+00:00
- Release ahead (s): 482.3

## Learning queue

- Interaction events: 19205
- Pending review events: 19203
- Review queue: 338 (approved=20, folded=20)

## Recommended next

- Live chat binary is older than target/release/perci.exe — relaunch via Launch-Perci.ps1 or copy the release binary after gates pass.
