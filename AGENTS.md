# AGENTS.md

<!-- CORTEX:MANAGED:BEGIN -->
## Cortex Repository Memory Protocol

This repository uses Cortex for verified repository assimilation, selective recall, and sparse neural interlinking.
Every activation is first routed through the local deterministic Thalamus planner, which allocates memory lanes and inhibits irrelevant evidence.

### Mandatory startup sequence

Before broad repository reading, planning, editing, or code generation:

1. Run `.\.cortex\bin\cortex.ps1 activate -Task "<current task>"` on Windows PowerShell, or `./.cortex/bin/cortex.sh activate --task "<current task>"` on Bash.
2. Inspect the returned bootstrap status, governor mode, learned environment, evidence references, neural support paths, and structural neighborhood.
3. If the bootstrap certificate is missing, degraded, or stale, run the wrapper's `bootstrap` command before relying on memory.
4. Read only the cited files and line ranges first. Expand context only when the packet is insufficient.
5. Treat repository source, tests, compiler output, and current runtime evidence as more authoritative than summaries.
6. Record decisions, discoveries, invariants, failures, fixes, and outcomes with the wrapper's `remember` command.
7. Run `consolidate` at task completion to create a provenance-bearing Discovery Card.

### Authority boundary

Cortex provides memory, relationships, telemetry, sparse activation, and evidence references. Neural plasticity changes only bounded internal association weights; it never authorizes durable source mutation. The host repository's rules and explicit human authorization remain controlling.

### Required commands

```powershell
.\.cortex\bin\cortex.ps1 activate -Task "<task>"
.\.cortex\bin\cortex.ps1 query -Query "<narrow question>"
.\.cortex\bin\cortex.ps1 remember -Kind decision -Text "<decision>"
.\.cortex\bin\cortex.ps1 consolidate
```

```bash
./.cortex/bin/cortex.sh activate --task "<task>"
./.cortex/bin/cortex.sh query --query "<narrow question>"
./.cortex/bin/cortex.sh remember --kind decision --text "<decision>"
./.cortex/bin/cortex.sh consolidate
```
<!-- CORTEX:MANAGED:END -->

## Perci Capability Fabric — any AI entry protocol (v0.7.1+)

Any AI agent (Grok, Claude, Codex, Cursor, etc.) may evolve this repo under the same governor.

### Authority law

```text
Bitwork       → routing / geometry only
operators     → explicit reasoning
native language → PERCLNG1 binary sequence field; external LM adapters opt-in only
retrieval     → current facts + provenance (never bake into .pwgt silently)
exact tools   → arithmetic/geometry truth
proof engine  → formal/unresolved receipts (PERCI_PROOF_ENGINE optional)
code agent    → bounded edits + tests (PERCI_AGENT_WORKTREE=1 for isolation)
Perci         → orchestration, criticism, memory
human         → durable weight promote and high-risk merge
```

### Mandatory evolve loop (optimize for multi-AI)

1. **Activate** Cortex with the task string (see above).
2. **Plan** via fabric: `perci fabric plan "<task>"` or read `docs/CAPABILITY_FABRIC_v070.md` + `docs/AI_EVOLVE_PROTOCOL.md`.
3. **Change only the right engine** (do not densify Bitwork to fake fluency/facts/proof).
4. **Test**: `cargo test --lib` then `python scripts/release_gates.py` when claiming a release.
5. **Remember** decisions with Cortex; **consolidate** at end.
6. **Never** auto-promote `.pwgt`; never claim consciousness.

### High-value surfaces for AI edits

| Surface | Path |
|---------|------|
| Fabric governor | `src/fabric.rs`, `src/orchestrate.rs` |
| Language sidecar | `src/language_sidecar.rs`, `scripts/perci_language_sidecar.py` |
| Knowledge | `src/knowledge_fabric.rs`, `knowledge/packs/**` |
| Proof | `src/proof_engine.rs` |
| Agent / worktree | `src/agent.rs` (`PERCI_AGENT_WORKTREE=1`) |
| Semantic eval | `src/semantic_eval.rs`, `scripts/evaluate_semantic_v1.py` |
| Hardness | `training/hardness/hardness-pack-v1.jsonl` |
| Emergence lab | `src/emergence.rs` |
| Entity-slot transfer | `src/entity_slot.rs` (adversarial entity-swap) |
| Native binary fields | `src/binary_language.rs`, `binary_phrase.rs`, `binary_relation.rs`, `binary_world.rs` |
| Compositional multi-hop | `src/compositional_world.rs` · `perci fabric compose` |
| Native decoder | `src/native_decoder.rs` · `perci fabric decode` |
| Reason/search/verify | `src/reason_loop.rs` · `perci fabric reason` |
| Replay baselines | `src/replay_learn.rs` · `perci fabric replay` (never auto-promote) |
| HYDRA inject bridge | `scripts/hydra_perci_bridge.py` · [HYDRA-Injector](https://github.com/jacksonjp0311-gif/HYDRA-Injector) (anchor→plan→seal; dry-run default) |

### Interconnection commands

```powershell
perci fabric status
perci fabric handoff "your task"    # preferred: machine-readable entry packet
perci fabric next                  # open lab tickets → recommended engines
perci fabric regress               # transfer + SoftCascade pack-align snapshot
perci fabric evolve                # multi-AI loop summary
perci fabric plan "your task"
perci fabric knowledge "query"
perci fabric orchestrate "explain X"
perci lab feed
perci lab patterns
python scripts/release_gates.py
```

Handoff writes `.perci/ai-handoff-latest.json` (`perci.ai-handoff.v1`) so the next
AI can resume without re-deriving the governor map.

### Fail-closed rules for AI agents

- Prefer smallest reversible patch.
- Prefer operators/frames over weight edits.
- Prefer transfer + hardness green over prose claims.
- Record failures in tickets / auto-repairs / Cortex — do not hide them.
