# Perci v0.7.0 — Capability Fabric

**Design law:** Do not stretch Bitwork until it impersonates every missing capability.

```text
Perci Core (governor)
├── Bitwork routing and cognitive geometry
├── deterministic operators and exact tools
├── language-generation sidecar (protocol)
├── evidence and world-knowledge services (protocol)
├── theorem/proof engine (adapter)
├── repository engineering agent (budgets + capability tokens)
└── verification, sandboxing, and governance
```

Perci remains the governor. Specialized engines perform the work they are structurally better suited to perform. Human authorization remains required for weight promote and high-risk merges.

## Protocol surfaces

| Engine | Module / path | Status |
|--------|---------------|--------|
| Fabric plan + capability tokens | `src/fabric.rs` | **shipped** |
| Language request schema | `LanguageRequest` JSON | **shipped** |
| Evidence records | `EvidenceRecord` | **shipped** |
| Semantic evaluation | `src/semantic_eval.rs` | **shipped (L1–L5 proxy)** |
| Daemon security | loopback + optional token + payload limits | **shipped** |
| Agent fail-closed + budgets | `ExecutionBudget` | **shipped** |
| Local LM sidecar process | external | **protocol ready, process optional** |
| Hybrid retrieval | intelligence packs + future embeddings | **lexical live; hybrid next** |
| CAS / theorem prover | adapters | **next** |
| Full AST agent | code agent | **budgets live; graph next** |

## CLI

```powershell
perci fabric status
perci fabric plan "explain retries under lag and prove idempotence"
```

## Phase map

### Phase 1 — Trustworthy foundation (this release)

1. Agent fail-closed (failed steps fail report)  
2. Daemon authentication + loopback + timeouts + payload caps  
3. Stronger release gates  
4. Structured semantic evaluation  
5. Execution budgets (edits, wall time, network default off)

### Phase 2 — Language and knowledge

Formal language sidecar process + hybrid retrieval + claim/contradiction.

### Phase 3 — Software engineering autonomy

Repo graph, AST edits, repair loop under capability tokens.

### Phase 4 — Mathematical reasoning

CAS + prover adapters with kernel-checked receipts.

## Security notes

- Default bind `127.0.0.1`. Non-loopback requires `PERCI_DAEMON_ALLOW_NON_LOOPBACK=1`.  
- Set `PERCI_DAEMON_TOKEN` for production-ish local use; shutdown requires token.  
- Capability tokens default `network=false`, `git_push=false`, `read_secrets=false`.

## Claim boundary

Capability Fabric is an **engineering orchestration law**. It is not AGI, consciousness, or unrestricted autonomy.
