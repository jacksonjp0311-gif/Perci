# Perci documentation

## Using Perci (tool layer)

| doc | what's in it |
|---|---|
| [QUICKSTART.md](QUICKSTART.md) | install, connect Claude Code / Cursor / Codex / Claude Desktop / any MCP client, host test status, verifying downloads, 60-second demo, the Temporal Router |
| [TOOLS.md](TOOLS.md) | every tool, verdict and receipt format, ledger and sequence rules, the Temporal Router, gate posture, latency |
| [PROFILES.md](PROFILES.md) | your own policy overlays, sequence rules, weight packs and calibration |
| [BENCHMARKS.md](BENCHMARKS.md) | the public benchmark script and results on Linux, macOS and Windows |
| [BITWORK_WHITEPAPER.md](BITWORK_WHITEPAPER.md) | **Bitwork white paper**: the binary router's encoding, scoring, retrieval and calibration, a reproducible baseline comparison ([evals/bitwork_baseline](../evals/bitwork_baseline/)), limitations, and where Bitwork sits in Perci |
| [DECISIONS.md](DECISIONS.md) | why the tool layer works the way it does (numbered decisions) |
| [DECISIONS-evals-memory.md](DECISIONS-evals-memory.md) | decisions for CI, bug2eval cases and cited memory |
| [../CHANGELOG.md](../CHANGELOG.md) | changes per release |

## Research engine (experimental, historical)

Perci also contains an experimental local chat engine built on its Bitwork
weights. These notes record how it was built and measured. They are kept for
reference; numbers in them are snapshots tied to the receipts they cite, and
some describe designs that were later replaced. Start with
[RESEARCH_LOG.md](RESEARCH_LOG.md) (the former README).

- Bitwork and weights (current reference: [BITWORK_WHITEPAPER.md](BITWORK_WHITEPAPER.md)): [BITWORK_COGNITION_v0622.md](BITWORK_COGNITION_v0622.md),
  [BITWORK_EMERGENCE.md](BITWORK_EMERGENCE.md), [BITWORK_MATH_PATH.md](BITWORK_MATH_PATH.md),
  [BITWORK_V3_EVIDENCE.md](BITWORK_V3_EVIDENCE.md), [LOWBIT_LAYER.md](LOWBIT_LAYER.md),
  [WEIGHT_REASSESSMENT_v0616.md](WEIGHT_REASSESSMENT_v0616.md), [SBCG_V2_EVIDENCE.md](SBCG_V2_EVIDENCE.md),
  [MODULAR_COGNITION_v1.md](MODULAR_COGNITION_v1.md), [NATIVE_DEPTH_v086.md](NATIVE_DEPTH_v086.md),
  [INTELLIGENCE_PACKS.md](INTELLIGENCE_PACKS.md), [../WEIGHTS.md](../WEIGHTS.md)
- Engine design: [COGNITIVE_WORKSPACE.md](COGNITIVE_WORKSPACE.md), [CAPABILITY_FABRIC_v070.md](CAPABILITY_FABRIC_v070.md),
  [GOVERNED_WILL.md](GOVERNED_WILL.md), [PERCICTX1_CONTEXT_CARD.md](PERCICTX1_CONTEXT_CARD.md),
  [OPEN_LANGUAGE_BRIDGE.md](OPEN_LANGUAGE_BRIDGE.md), [TRANSFORMER_BRIDGE.md](TRANSFORMER_BRIDGE.md),
  [CORTEX_INTEGRATION.md](CORTEX_INTEGRATION.md), [INTERACTION_LEARNING.md](INTERACTION_LEARNING.md),
  [INTELLIGENCE_CHANNELS_v0624.md](INTELLIGENCE_CHANNELS_v0624.md), [PATTERN_INTEL_v0623.md](PATTERN_INTEL_v0623.md),
  [DARK_BLOOD_CLI.md](DARK_BLOOD_CLI.md)
- Evaluation and evolution: [CAPABILITY_SCORECARD.md](CAPABILITY_SCORECARD.md), [EVOLUTION.md](EVOLUTION.md),
  [EMERGENCE_LEDGER.md](EMERGENCE_LEDGER.md), [CROSS_DOMAIN_EVOLUTION.md](CROSS_DOMAIN_EVOLUTION.md),
  [BREAKTHROUGH_FOUR_v0625.md](BREAKTHROUGH_FOUR_v0625.md), [LIVE_TEST_TEN.md](LIVE_TEST_TEN.md),
  [AI_EVOLVE_PROTOCOL.md](AI_EVOLVE_PROTOCOL.md), [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) (engine gates),
  [../VALIDATION.md](../VALIDATION.md), [SECURITY_ASSESSMENT_20260722.md](SECURITY_ASSESSMENT_20260722.md)
- Goals and roadmaps (aspirational): [GOAL.md](GOAL.md), [LOCAL_AGI_ROADMAP.md](LOCAL_AGI_ROADMAP.md),
  [SUPERINTELLIGENCE_PATH.md](SUPERINTELLIGENCE_PATH.md), [EIC_V1_6_ALIGNMENT.md](EIC_V1_6_ALIGNMENT.md),
  [SACRED_GEOMETRY_CURRICULUM.md](SACRED_GEOMETRY_CURRICULUM.md)
