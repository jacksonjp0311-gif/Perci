# Bitwork cognition expansion · v0.6.22

**Claim boundary:** Operators, frames, hardness, transfer, inject prompts.  
**Never:** automatic `.pwgt` promotion. Curriculum/staging only for human `--authorize` rebuilds.

## Goal of this expansion

Strengthen **governed, inspectable, compositional** cognition across six high-value categories while keeping sparsity and the L8 lab loop.

## Categories → operators

| # | Category | Operator | Primary surface |
|---|----------|----------|-----------------|
| 1 | Multi-step planning & agent loops | `agent-loop-plan` | measure → ticket → transfer → close (+ lag recovery) |
| 2 | Cross-domain composition | `cross-domain-compose` | geometry × systems × math × logic × creativity |
| 3 | Uncertainty calibration | `uncertainty-calibration` | confidence tiers + refuse / `/intel` metrics |
| 4 | Memory & ledger | `ledger-memory-integrate` | Cortex + emergence + tickets + aging |
| 5 | Self-critique & meta | `meta-critique-queue` | `/think` · `/trace` · queue suggestions |
| 6 | Novel entity generalization | `novel-entity-generalize` | structure transfer, anti-overfit |

**Code:** `src/cognition_expand.rs` · wired early in `deliberation::try_deliberate`.  
**Frames:** `EXPAND_FRAMES` merged into SoftCascade `activate_semantic_frames`.  
**Hardness:** H61–H72.  
**Transfer suite:** includes agent-loop, compose, uncertainty, novel-entity bases.

## Training without densifying the pack

1. **Operators** own speech for these regions (fast, inspectable).  
2. **Semantic frames** enrich SoftCascade multipartite mass.  
3. **Inject prompts** (`training/adaptive/inject_prompts.json`) + curriculum JSONL stage evidence.  
4. **Hardness + transfer** lock generalization.  
5. **Weights** only if a future authorized rebuild folds reviewed curriculum.

## Verify

```powershell
cargo test --lib cognition_expand
.\target\release\perci.exe transfer-suite
python .\scripts\evaluate_hardness.py
python .\scripts\release_gates.py
```

## Related

- `docs/GOAL.md`  
- `docs/EMERGENCE_LEDGER.md`  
- `docs/RELEASE_CHECKLIST.md`  
