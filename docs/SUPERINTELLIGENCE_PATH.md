# Path toward stronger intelligence (honest)

Perci will **not** become a superintelligence by adding more chat operators alone.
This document is the **governed ladder** we climb instead of slogans.

## Definition we use

A **superintelligent local agent** (engineering sense) would:

1. Detect its own failures under distribution shift  
2. Propose bounded repairs at the correct layer  
3. Prove improvement on harder held-out gates  
4. Act on the world only through tools with authority boundaries  
5. Never silently promote weights or facts  

That is **not** AGI, consciousness, or unrestricted autonomy.

## Ladder (current progress)

| Level | Capability | Status |
|------:|------------|--------|
| L0 | Exact tools + reflex routing | **achieved** |
| L1 | Governed memory + learning queue | **achieved** |
| L2 | Named operators + dialogue regression | **achieved** |
| L3 | Transfer hardness + live failure repair | **achieved** (hardness 30+) |
| L4 | Open-domain structural synthesis + plan/causal operators | **in progress (v0.5)** |
| L5 | Critic rewrites + multi-hop programs end-to-end | **partial** (v0.5.1 program runtime + `/trace`) |
| L6 | Broader tool use (code sandbox, repo tools) | **partial** (`perci agent run` MVP) |
| L7 | Hybrid language backend under governance | **optional** |
| L8 | Closed self-improvement lab (ticket → patch → gate → promote) | **pipeline started** (agent + merge-if-green) |
| L9 | Long-horizon goals with world state | **not started** |
| L∞ | Superintelligence / AGI | **not claimed; not a near target** |

## v0.5 additions

- Open-domain `connect` for unknown nouns (provisional structural frames)
- Multi-hop plan operator
- Causal-chain operator
- Superintelligence overclaim refusal
- Exact tools: average, ratio, percent change, factorial (bounded)
- Program critic can rewrite comfort/generic collapses
- Hardness expansion + live probe expansion

## v0.5.1 (T1–T3)

- **T1 intent authority:** explanatory math (`why does 2+2 equal 4?`) no longer hits the integer parser; code intents emit real snippets; space-separated multi-domain `connect` synthesizes
- **T2 operator programs:** select → critic → optional rewrite wired into deliberation, exact tools, and associative path; `/trace` shows `program_id`, steps, critic
- **T3 agent MVP:** `perci agent run <goal> [--dry-run] [--merge-if-green]`; repo allowlist; never writes `.pwgt`; kill switch `PERCI_AGENT=0` / `.perci/agent.lock`
- Hardness H41–H43 added for the above live failures

## v0.5.2 (T4 + P0 evolution)

- Multi-word domain critic + SDM/VSA/Bitwork/impasse frames; H44 green
- Decision-trace ledger + `perci traces`
- `perci agent lab --from-hardness` impasse tickets
- Sample-fold 20 learning events into inject prompts (no weight promote)
- Hardness **44/44**

## Operating rule

```text
fail live → hardness case → repair layer → retest → scorecard → (optional authorize promote)
```

Never stop measuring. Never claim AGI from green tests.

## Cross-domain next (see `docs/CROSS_DOMAIN_EVOLUTION.md`)

After v0.5.1, external fields (neuro-symbolic agents, HD/VSA, Soar/LIDA, SICA, context graphs)
converge on the same next steps Perci already sketched:

1. **P0** relaunch live + fold a sample of the learning queue  
2. **P1** impasse lab (hardness fail → agent ticket → green merge)  
3. **P2** multi-word composition + real tool steps inside programs  
4. **P3** decision-trace memory (third memory type)  
5. **P4** optional binary bind / VSA experiments before PERCIW04  
6. **P5** hybrid LM only if tools+programs still leave a measured gap
