# Perci v0.8.6 — native depth tracks

**Claim boundary:** engineering measurements only. Not AGI, consciousness, or unrestricted autonomy. Binary fields (PERCIWM1 / PERCPHR1 / …) promote only with **human authorize** when held-out beats active.

## Tracks shipped

| # | Track | Module / command | Promote? |
|---|--------|------------------|----------|
| 1 | Adversarial re-probe + `slot_pair_binding_rate` | `scripts/adversarial_reprobe_v086.py` | n/a (measure) |
| 2 | Compositional multi-hop world | `src/compositional_world.rs` · `perci fabric compose` | never auto |
| 3 | Native generative decoder | `src/native_decoder.rs` · `perci fabric decode` | n/a |
| 4 | Reason / search / verify receipts | `src/reason_loop.rs` · `perci fabric reason` | n/a |
| 5 | Replay baselines | `src/replay_learn.rs` · `perci fabric replay` | **always false** in code |

## Entity-slot law (from v0.8.5)

Surface names are role-fillers. Relation transfer requires both motif slots in speech. Creative-constraint must not steal adversarial entity-swap prompts.

## Promote gate (human)

```text
held-out candidate metrics > active on same curriculum
AND release_gates.py PASS
AND human --authorize for binary artifacts
```

No path in `replay_learn` sets `promote_recommended = true` automatically.
