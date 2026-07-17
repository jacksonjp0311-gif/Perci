# Perci evolution loop

Goal: improve **measurable capability**, not merely fluent replies.
See also `docs/SUPERINTELLIGENCE_PATH.md` for the honest capability ladder
(superintelligence is **not** a claimed endpoint of operator patches alone).

```text
live chat failures
      │
      ▼
interaction-learning.jsonl
      │  stage
      ▼
review queue (human approve + label)
      │  fold-approved
      ▼
inject prompts / operator repair / tool work
      │  rebuild candidate (optional)
      ▼
hardness pack + operational/dialogue gates
      │  explicit --authorize
      ▼
promote + relaunch live binary
```

## Quick start (weekly)

```powershell
# 1) From repo root — stage evidence, run hardness, write scorecard
python .\scripts\evolve_cycle.py

# 2) Read the scorecard
Get-Content .\docs\CAPABILITY_SCORECARD.md

# 3) Repair one red/yellow capability at the named layer
#    (operator / tool / voice / pipeline) — one capability per cycle

# 4) Re-measure
python .\scripts\evaluate_hardness.py
python .\scripts\capability_scorecard.py

# 5) Only after gates pass, promote weights (never automatic)
python .\scripts\promote_v2.py `
  --candidate .\models\candidates\<candidate>.pwgt `
  --evaluation .\models\candidates\<operational>.json `
  --supplemental-evaluation .\models\candidates\evaluation-hardness-v1.json `
  --authorize "human: <why this promote is justified>"

# 6) Relaunch so chat uses the fresh binary
.\Launch-Perci.ps1
```

## Artifacts

| Path | Role |
|------|------|
| `training/hardness/capabilities.json` | Named capabilities, layers, failure modes |
| `training/hardness/hardness-pack-v1.jsonl` | Hard transfer / live-failure cases |
| `scripts/evaluate_hardness.py` | Sealed hardness gate |
| `scripts/capability_scorecard.py` | Aggregated status + recommendations |
| `scripts/evolve_cycle.py` | Orchestrates stage → hardness → scorecard |
| `scripts/stage_interaction_learning.py` | Queue + fold interaction evidence |
| `scripts/promote_v2.py` | Explicit authorized promotion only |
| `src/operator_program.rs` | Inspectable multi-step program + critic scaffold |
| `docs/CAPABILITY_SCORECARD.md` | Human-readable latest scorecard |

## Rules

1. **One capability per cycle** when possible.
2. Capture a **failing example** before changing code or weights.
3. Prefer **transfer variants** (entity swap, paraphrase, distractors) over more surface Q&A.
4. **Never** auto-promote facts or weights from conversation.
5. After promote or composition repair, **relaunch** so `target/live` is not stale.
6. Raise hardness when the pack is saturated (all green).

## Hardness levels

| Level | Meaning |
|------:|---------|
| 1–2 | Regression / exact tools / governed basics |
| 3 | Known live failure modes |
| 4 | Multi-turn binding / four-domain synthesis |
| 5 | Adversarial transfer (banned crutch words, novel frames) |

## Operator programs

`src/operator_program.rs` defines inspectable programs:

```text
intent → program steps → answer → critic checks → confidence penalty if flags fire
```

Deliberation remains the primary high-salience path. Programs annotate audit
traces and score answers against capability-specific checks (comfort collapse,
missing domains, generic checklists, illegal promotion claims).

## What not to do

- Chase “superintelligence” slogans without a harder gate
- Patch only dialogue cosmetics when hardness is red
- Dump unlabelled interaction logs into weights
- Leave a stale live binary answering while release builds advance
