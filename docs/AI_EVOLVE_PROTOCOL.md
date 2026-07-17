# AI Evolve Protocol — multi-agent interconnection (v0.7.1)

**Purpose:** Any AI can enter this repository and evolve Perci without violating the Capability Fabric.

## 1. Identity of the system

Perci is a **governor**, not a single-model chatbot:

```text
user / AI agent
    → Bitwork route + operators
    → exact tools / proof / retrieval / language / code engines
    → critic + capability tokens
    → human authorize for weights & high-risk merges
```

## 2. Entry checklist (every AI session)

| Step | Action |
|------|--------|
| 1 | `cortex activate -Task "..."` |
| 2 | Read `docs/CAPABILITY_FABRIC_v070.md` + this file |
| 3 | `perci fabric plan "<task>"` |
| 4 | Edit only the engine that owns the gap |
| 5 | `cargo test --lib` |
| 6 | Relevant gates (hardness / transfer / semantic / heldout) |
| 7 | `cortex remember` + `consolidate` |
| 8 | Commit with complete-sentence message |

## 3. Gap → engine map

| Gap | Engine to extend | Do not |
|-----|------------------|--------|
| Fluency | `language_sidecar` + external process | Stuff pack with prose |
| Fresh facts | `knowledge_fabric` + evidence JSONL | Auto-promote weights |
| Formal math | `proof_engine` + PERCI_PROOF_ENGINE | Accept “sounds proven” |
| Code change | `agent` + worktree + tests | Edit outside allowlist |
| Routing/geometry | Bitwork curriculum (human authorize) | Silent pack swap |
| Measurement | hardness / semantic / transfer | Lower the bar |

## 4. Language sidecar protocol

Request schema: `perci.language-request.v1` (`LanguageRequest` in `fabric.rs`).  
Response schema: `perci.language-response.v1`.

```powershell
$env:PERCI_LANGUAGE_SIDECAR = "python scripts/perci_language_sidecar.py"
```

Default: in-process local governed synthesizer (deterministic).

## 5. Knowledge fabric

- Pack retrieval: `intel_packs::retrieve`
- Typed evidence: `EvidenceRecord`
- Stage evidence: `knowledge_fabric::stage_evidence`
- Ledger: `models/candidates/knowledge-evidence.jsonl`

## 6. Code agent isolation

```powershell
$env:PERCI_AGENT_WORKTREE = "1"
perci agent run "..." --dry-run
```

Budgets: max edits, wall time, network off by default. Kill switch: `PERCI_AGENT=0` or `.perci/agent.lock`.

## 7. Definition of done for an AI PR

- [ ] Fabric plan matches the gap  
- [ ] Tests green  
- [ ] No `.pwgt` mutation  
- [ ] Cortex decision recorded  
- [ ] Claim boundary intact (no AGI/consciousness/auto-promote)  

## 8. Optimized multi-AI process

```text
AI_A discovers fail → hardness/ticket
AI_B patches engine → cargo test
AI_C expands transfer/semantic gates
AI_D runs release_gates → human authorize promote only if needed
```

Shared artifacts: JSONL ledgers, tickets, auto-repairs, Cortex cards, fabric plans,
`.perci/ai-handoff-latest.json`.

## 9. Machine-readable handoff (preferred entry)

```powershell
perci fabric handoff "your task here"
# → stdout: perci.ai-handoff.v1 JSON
# → disk:   .perci/ai-handoff-latest.json
perci fabric evolve   # human-readable multi-AI loop
```

Packet fields: `plan`, `entry_checklist`, `gap_engine_map`, `surfaces`, `gates`,
`env_hooks`, `next_commands`, `lab_hint`, `claim_boundary`.

Load the JSON at session start; do not re-derive authority law from memory alone.

## 10. Interconnection contract

| Channel | Direction | Artifact |
|---------|-----------|----------|
| Cortex | AI ↔ repo memory | activate / remember / consolidate |
| Fabric plan | task → engines | `FabricPlan` |
| Handoff | AI_A → AI_B | `.perci/ai-handoff-latest.json` |
| Lab feed | emergence → work queue | tickets / patterns |
| Evidence | retrieval → claims | `knowledge-evidence.jsonl` |
| Auto-repairs | runtime → curriculum | `auto-repairs.jsonl` |
| Gates | CI truth | hardness / transfer / semantic / heldout |

Parallel AIs must not race weight promote. Prefer disjoint engines or sequential
tickets. Smallest reversible patch wins.
