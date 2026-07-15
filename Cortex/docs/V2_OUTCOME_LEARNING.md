# Cortex v2: Outcome-Grounded Learning

Cortex v2 closes the learning loop without granting learned state authority over the repository.

```text
task → deterministic activation → evidence packet → verification outcome
     → bounded credit proposal → replay gate → immutable ledger event
```

`cortex outcome` is the only v2 path that can adapt a synapse. Activation records a reproducible trace but does not update weights. An outcome is always recorded, including under `read_only`; only a verified ledger, bounded proposals, a nonzero reward, and `normal` or `constrained` Governor mode may promote updates.

```powershell
python -m cortex --home <home> outcome --repo Cortex --activation-id act_... `
  --status verified --verification pytest --json
```

Outcome statuses have conservative default rewards: `verified` +1.0, `diagnosed` +0.70, `helpful` +0.40, `unknown` 0.0, `irrelevant` -0.35, `failed` -0.75, and `unsafe` -1.0. A supplied reward is clamped to [-1, 1].

The initial shadow gate verifies ledger integrity, deterministic proposal construction, and synapse bounds. It is intentionally a narrow foundation for future repository-native replay corpora; it does not claim answer-quality improvements without those evaluations.

## Context protocol

`cortex protocol` emits `cortex-context/1.0`, an agent-neutral packet with evidence, governance, environment, structural paths, unknowns, and explicit prohibited actions. It is a context contract, not a mutation authority.
