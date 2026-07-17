# Emergence ledger — geometry speaks; lab answers

**Software:** v0.6.19+  
**Log:** `models/candidates/emergence-geometry.jsonl` (override `PERCI_EMERGENCE_LOG`)  
**Tickets:** `models/candidates/emergence-tickets/`  
**Curriculum:** `models/candidates/emergence-curriculum.jsonl`  
**Claim boundary:** Engineering telemetry + governed candidates. Not consciousness. Never auto-promotes weights.

## Loop

```text
classify / probe
    → typed MatchEvent (serde JSONL)
         authority: softcascade | probe | <operator>
    → analyze + lessons(ledger)
    → SoftCascade:
         primary off + multipartite + mix hits → mixture thesis (CRUTCH)
         chronic / geometry_blind → multipartite + critique + topic bind
    → if softcascade|probe + (mixture_crutch | chronic primary_off)
         → lab ticket primary-fix-{label}.md  (idempotent)
         → curriculum candidate JSONL sample
    → SpeechEvent (speech_hit / miss, used_mix_thesis)
    → TransferProbe (optional / tests)
         base + paraphrase + novel nouns → pass only if topic binds across transforms
    → /field  laws + tickets
    → /lab    ticket files + curriculum path
```

## Fixes vs thin v0.6.18

| Thin point | v0.6.19 law |
|------------|-------------|
| Hints not tickets | Chronic/crutch → **primary-fix ticket** + curriculum candidate |
| Stringy JSONL | **Typed** `LedgerEvent` via serde; corrupt lines skipped |
| Double-record bias | Curriculum ranking uses **only** `softcascade` + `probe` authority |
| Forever mixture crutch | Tag `mixture_crutch` + ticket: fix primary/operator; mixture temporary |
| No transfer gate | `evaluate_transfer` / `default_transfer_set` + Transfer events |

## Event kinds

| `kind` | When |
|--------|------|
| `match` | classify / probe |
| `speech` | SoftCascade sealed body |
| `ticket` | lab primary-fix staged |
| `transfer` | transfer gate result |

## Policy fields

| Field | Meaning |
|-------|---------|
| `prefer_mixture_thesis` | Replace thesis with on-topic mixture insight |
| `mixture_crutch` | Primary wrong; mixture is temporary UX only |
| `open_primary_fix_ticket` | Stage lab ticket (curriculum authority only) |
| `force_multipartite_arc` | Contested / multipartite / blind / chronic |
| `geometry_blind` | Primary and mixture both miss user tokens |

## Transfer gate

Honest emergence bar (aligned with `emergence-vs-memorization` operator):

1. **base** prompt speech hits user content tokens  
2. **paraphrase** still hits  
3. **novel nouns** (entity swap) still hits structural constraints  

**Pass** = base OK **and** (paraphrase OK **or** novel OK).  
Do not claim emergence from a single template hit.

## Commands

```text
/field      # events + laws + open tickets
/lab        # tickets dir + curriculum + transfer counts
/emergence  # alias of /field
/geometry   # alias of /field
/think      # cognition plan (chat stays clean)
```

## Never

- Auto-promote `.pwgt` from ledger or tickets  
- Count operator-authority matches toward pack curriculum ranking  

## Related

- [`docs/WEIGHT_REASSESSMENT_v0616.md`](WEIGHT_REASSESSMENT_v0616.md)  
- [`docs/BITWORK_EMERGENCE.md`](BITWORK_EMERGENCE.md)  
- [`docs/TRANSFORMER_BRIDGE.md`](TRANSFORMER_BRIDGE.md)  
- [`docs/LIVE_TEST_TEN.md`](LIVE_TEST_TEN.md)
