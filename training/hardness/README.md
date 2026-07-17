# Hardness pack

Transfer- and failure-oriented cases for Perci capability evolution.

| File | Role |
|------|------|
| `capabilities.json` | Named capabilities, layers, success criteria |
| `hardness-pack-v1.jsonl` | Scored prompts (required / forbidden substrings) |

```powershell
python ..\..\scripts\evaluate_hardness.py
python ..\..\scripts\capability_scorecard.py
python ..\..\scripts\evolve_cycle.py
```

Raise hardness when everything is green: add entity swaps, paraphrases,
distractors, and banned-crutch variants. See `docs/EVOLUTION.md`.
