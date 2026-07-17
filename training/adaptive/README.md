# Adaptive Perci training

Local-only curriculum evolution for the deduplicated `PERCIW02` Bitwork pack.

## Pipeline

```text
interaction evidence → review queue → approved inject prompts → v2 candidate build → sealed evaluation → authorized promotion
```

## Commands

```powershell
# From perci/
python scripts/adaptive_train.py
python scripts/adaptive_train.py --morph

# From Lumen
perci adapt
perci adapt morph
```

## What morph means

- Rebuilds associative prototypes (`PERCIW02`), not gradient fine-tuning.
- Injects real surface forms from wins + Lumen cortex/curriculum (~1/5 of prototypes per label).
- Seed is xor'd with inject hash so the pack **actually changes** when curriculum changes.
- Builds candidates first; active-pack replacement requires the promotion gate.

## Files

| Path | Role |
|------|------|
| `traces.jsonl` | Every interact classify/ask outcome |
| `inject_prompts.json` | Labeled prompts mixed into rebuild |
| `last_run.json` | Win rate summary |
