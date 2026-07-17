# Governed Interaction Learning

Perci learns from interaction in two bounded layers.

1. Safe dialogue preferences apply immediately. Feedback such as “too
   procedural,” “not smooth,” or “be concise” updates the local dialogue
   profile used by later turns.
2. Every bounded, non-sensitive exchange is appended as a pending learning
   record. Facts, procedures, corrections, and weight curriculum do not promote
   automatically.

Runtime files:

```text
memory/dialogue-profile.json
memory/interaction-learning.jsonl
```

**Counters:** `/status` reports both `interactions` (profile) and `event_log`
(JSONL lines). Teaching candidates and other non-turn rows make the log longer
than the profile. On startup Perci reconciles by lifting profile counts when
the log is ahead — it never invents weight knowledge from the gap.

Inspect the state with `perci learning` or `/learning`.

Humans can explicitly stage a knowledge claim in ordinary conversation without
promoting it:

```text
I want you to learn that a capability claim needs provenance and a falsifiable test
```

The equivalent explicit CLI shortcut is optional:

```text
/teach A capability claim should include provenance and a falsifiable test
```

This creates a `pending_review` candidate. Use `remember that ...` for a
deliberate durable note. Neither path silently changes the active weight file.

To stage interaction evidence for curriculum review:

```powershell
python .\scripts\stage_interaction_learning.py
```

Review `training/adaptive/interaction-review.json`, set `approved` to `true`
and provide a valid `label` only for accepted prompts, then fold them:

```powershell
python .\scripts\stage_interaction_learning.py --fold-approved
```

Folding still does not change active weights. A candidate rebuild, sealed
evaluation, explicit authorization, and promotion receipt remain required.

For the full weekly loop (stage → hardness pack → capability scorecard):

```powershell
python .\scripts\evolve_cycle.py
```

See [`docs/EVOLUTION.md`](EVOLUTION.md).

Sensitive-looking interactions are stored only as redacted evidence. The
learning log explicitly records that automatic fact promotion and automatic
weight mutation are false.
