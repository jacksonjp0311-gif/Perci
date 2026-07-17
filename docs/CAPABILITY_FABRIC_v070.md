# Perci v0.7.1 — Capability Fabric

**Design law:** Do not stretch Bitwork until it impersonates every missing capability.

```text
Perci Core (governor)
├── Bitwork routing and cognitive geometry
├── deterministic operators and exact tools
├── language-generation sidecar (local + optional process)
├── evidence and world-knowledge fabric (packs + ledger)
├── theorem/proof engine (exact tools + optional prover)
├── repository engineering agent (budgets + worktrees + tokens)
└── verification, sandboxing, and multi-AI handoff
```

Perci remains the governor. Specialized engines perform the work they are structurally better suited to perform. Human authorization remains required for weight promote and high-risk merges.

## Protocol surfaces

| Engine | Module / path | Status |
|--------|---------------|--------|
| Fabric plan + capability tokens | `src/fabric.rs` | **shipped** |
| AI handoff packet | `perci fabric handoff` · `perci.ai-handoff.v1` | **shipped** |
| Language request schema | `LanguageRequest` JSON | **shipped** |
| Language sidecar | `src/language_sidecar.rs` + `scripts/perci_language_sidecar.py` | **shipped** |
| Evidence records | `EvidenceRecord` · `knowledge_fabric` | **shipped** |
| Semantic evaluation | `src/semantic_eval.rs` | **shipped (L1–L5 proxy)** |
| Daemon security | loopback + optional token + payload limits | **shipped** |
| Agent fail-closed + budgets + worktrees | `ExecutionBudget` · `PERCI_AGENT_WORKTREE=1` | **shipped** |
| Proof receipts | `src/proof_engine.rs` · `PERCI_PROOF_ENGINE` | **shipped (stub + exact)** |
| Orchestrator | `src/orchestrate.rs` | **shipped** |
| Multi-AI evolve protocol | `docs/AI_EVOLVE_PROTOCOL.md` · `AGENTS.md` | **shipped** |
| Hybrid embeddings / full CAS / AST graph | future | **next depth** |

## CLI

```powershell
perci fabric status
perci fabric plan "explain retries under lag and prove idempotence"
perci fabric knowledge "trust under lag"
perci fabric orchestrate "explain SoftCascade"
perci fabric handoff "implement hardness H101 for novel transfer"
perci fabric evolve
```

## Phase map

### Phase 1 — Trustworthy foundation — **done (v0.7.0+)**

1. Agent fail-closed (failed steps fail report)  
2. Daemon authentication + loopback + timeouts + payload caps  
3. Stronger release gates  
4. Structured semantic evaluation  
5. Execution budgets (edits, wall time, network default off)

### Phase 2 — Language and knowledge — **done (v0.7.1)**

Local governed synthesizer + optional `PERCI_LANGUAGE_SIDECAR` process; pack retrieval + evidence ledger + contradiction notes; chat/orchestrate enrichment under critic.

### Phase 3 — Software engineering autonomy — **done (v0.7.1 baseline)**

Capability tokens, budgets, optional git worktrees (`PERCI_AGENT_WORKTREE=1`), allowlisted edits, hardness append path. Full AST graph remains depth work.

### Phase 4 — Mathematical reasoning — **done (v0.7.1 baseline)**

Exact tools as mechanical authority; formal requests yield kernel receipts when `PERCI_PROOF_ENGINE` is set, else honest `UnresolvedArgument`.

## Multi-AI interconnection

Any AI (Grok, Claude, Codex, Cursor, …) enters via:

1. Cortex activate  
2. `perci fabric handoff "<task>"` → writes `.perci/ai-handoff-latest.json`  
3. Edit only the owning engine  
4. Test + remember + consolidate  

See `docs/AI_EVOLVE_PROTOCOL.md`.

## Security notes

- Default bind `127.0.0.1`. Non-loopback requires `PERCI_DAEMON_ALLOW_NON_LOOPBACK=1`.  
- Set `PERCI_DAEMON_TOKEN` for production-ish local use; shutdown requires token.  
- Capability tokens default `network=false`, `git_push=false`, `read_secrets=false`.

## Claim boundary

Capability Fabric is an **engineering orchestration law**. It is not AGI, consciousness, or unrestricted autonomy.
