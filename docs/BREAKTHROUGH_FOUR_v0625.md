# Four breakthrough paths · v0.6.25

## 1. Agent repair (hardness fail → green without hand-written operator)

```powershell
perci agent lab --repair-hardness
```

Writes `models/candidates/auto-repairs.jsonl` (runtime catalog).  
Loaded by `auto_repairs::try_auto_repair` in deliberation. **No weight promote.**

## 2. SoftCascade-only trust/lag alignment

`softcascade_trust_alignment_body` supplies structural trust speech when primary is off-topic.  
Wired into SoftCascade `domain_body` + thesis. Verified by:

```powershell
perci transfer-suite   # includes SoftCascade-only trust block
```

## 3. Hardness 100+

Pack targets **H100+** with entity-swap trust variants. Gate: `evaluate_hardness.py` PASS.

## 4. Held-out AGI-candidate ≥90%

```powershell
cargo build --release
python scripts/evaluate_heldout_agi_candidate.py
```

Threshold **0.90** on 25 clean-machine prompts via `perci ask`.

## Claim boundary

These paths strengthen **governed local cognitive OS** capability.  
They do **not** claim AGI, consciousness, or auto weight learning.
