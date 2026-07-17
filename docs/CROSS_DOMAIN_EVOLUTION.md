# Cross-domain evolution assessment (post v0.5.1)

**Date:** 2026-07-16  
**Baseline:** Perci v0.5.1 · hardness **43/43** · lib tests **90/90** · T1–T3 landed  
**Method:** internal state (roadmap, scorecard, live probes) + external literature across neuro-symbolic AI, HD/VSA, classical cognitive architectures, self-improving coding agents, and context-graph memory.

This is an **assessment and insight map**, not a promotion receipt.  
Superintelligence / AGI is **not** claimed.

---

## 1. Where Perci stands now

### Strengths (real, measured)

| Asset | Why it matters externally |
|-------|---------------------------|
| Separated layers (reflex → Bitwork → operators → tools → memory → governance) | Matches 2025–26 neuro-symbolic consensus: neural *perception/routing* + symbolic *tools/rules* + integration engine |
| Integer-only 4,096-bit associative core + prototypes | Kin to sparse distributed memory / binary codes — fast, local, inspectable |
| Exact tool authority | Industry push: deterministic kernels wrapped around nondeterministic language |
| Hardness + dialogue gates + no auto-promote weights | Rare honesty boundary vs “self-improving” LLM agents that rewrite themselves without sealed gates |
| Operator programs + `/trace` (v0.5.1) | Aligns with “transparent reasoning paths” demand in neuro-symbolic product literature |
| Agent MVP (repo allowlist, kill switch) | Seed of L6/L8 without surrendering weight authority |

### Debt (still red/yellow)

| Debt | Evidence |
|------|----------|
| Learning queue dammed | ~501 pending, **0 folded** (scorecard) |
| Agent is heuristic, not general | Goal templates; not full tool-loop planner |
| Multi-hop programs partial | Steps audited; not always executed as tool chains |
| Multi-word domain synthesis critic | Live: “sparse distributed memory” → critic rewrite + generic filler |
| No world state / long-horizon goals | L9 not started |
| Optional LM still unattached | Code breadth beyond snippets remains thin without L7 or much more code curriculum |
| Scorecard stale vs 43-case hardness | Re-run evolve_cycle after relaunch |

---

## 2. Cross-domain map → Perci translation

### A. Neuro-symbolic systems (2025–26)

**External pattern:** neural perception · symbolic knowledge · integration engine · human-in-the-loop compliance.  
**Perci already is this shape.**  

**Insight:** Do **not** dissolve into “just attach a bigger LLM.” The field is moving *toward* Perci’s separation of concerns, not away from it.  
**Evolve by:** strengthening the integration engine (operator programs that *call* tools), not replacing Bitwork with chat fluency.

### B. Sparse distributed memory & HD/VSA

**External pattern:** high-dimensional binary/real vectors; **bind** (role–filler), **bundle** (superposition), **permute** (order); holistic analogy (“dollar of Mexico”); Kanerva SDM / Binary Spatter Codes.  

**Perci today:** Bitwork encodes prompts into 4,096-bit activations and does prototype/expert scoring — **association + routing**, not full VSA algebra.  

**Insight — highest technical upside for “smarter without a transformer”:**

| VSA idea | Perci evolution |
|----------|-----------------|
| Binding (role ⊙ filler) | Encode “domain:mechanism” pairs as bound binary codes; synthesis becomes unbind/compare, not only string templates |
| Bundling | Multi-domain connect = bundle of domain vectors + shared-axis vector |
| Holistic transform | Learn transfer operators from hardness pairs (entity swap) as transform HVs |
| Clean-up memory | Nearest prototype is already cleanup — extend to multi-hop cleanup after operator steps |

**Caution:** Full HDC rewrite of Bitwork is a multi-month research track. Near-term: **prototype-level binding for semantic frames** (encode frame pairs, not only lexical features).

### C. Classical cognitive architectures (ACT-R, Soar, LIDA)

**External pattern:**

| Architecture | Core idea | Perci analogue |
|--------------|-----------|----------------|
| ACT-R | Procedural + declarative memory, activation | Operators + packs/memory; missing: utility learning on productions |
| Soar | Problem space, **impasse → subgoal** | Hardness fail should spawn agent ticket (impasse); partial |
| LIDA | Cognitive cycle, attention bottleneck, SDM-like memory | Bitwork + attention-like routing; missing: explicit cycle with attention selection before act |

**Insight:** Next agent lab should model **Soar impasse**:  
`gate fail → subgoal ticket → repair layer → re-test → return`.  
That is more “cognitive architecture” than more chat operators.

### D. Self-improving coding agents (SICA, SWE-bench loops)

**External pattern:** agent edits *itself*, re-benches, keeps gains (17%→53% on SWE subsets). Almost always **LLM-centered**.  

**Perci difference (strategic):** improve **code + operators** under gates; **weights never self-promote**.  

**Insight:** Steal the *loop*, not the *opaque self-edit*:

```text
evaluate hardness → cluster fails → agent patch → cargo test + hardness
  → auto-merge green code → human authorize weight promote only if needed
```

Agent v0.5.1 is the embryo; needs: fail clustering, patch generation beyond hardness append, merge-to-main policy with revert.

### E. Context graphs & three memories

**External pattern:** long-term knowledge · short-term conversation · **reasoning/decision traces**.  

**Perci today:** packs + JSONL memory + session + `/trace` deliberation — but traces are **not** a queryable graph linking decisions to entities.  

**Insight:** Promote `/trace` + interaction-learning into a **decision ledger** (JSONL or Cortex-linked):

- entity nodes (capability, layer, hardness id)  
- decision edges (chose operator X, critic flagged Y)  
- reuse in agent planning (“last time H41 failed at reasoning layer”)

This is cheaper than Neo4j and fits local-first governance.

### F. Sparse interpretability (SAEs, sparse circuits)

**External pattern:** sparse features as purer concepts than dense neurons.  

**Perci alignment:** already sparse binary.  

**Insight:** Treat each **weight-resident concept** and **prototype cluster** as an interpretability unit — hardness cases should name which concept/expert fired (`/intel` already partially does). Extend agent receipts to record expert domain + margin.

---

## 3. What “emerged” from the cross-search

### Emergence 1 — Perci is early, not obsolete

The industry narrative for 2026 is **neuro-symbolic agents + deterministic tools + memory graphs**. Perci’s core bet (Bitwork route + exact tools + governance) is directionally mainstream. The gap is **depth of the integration engine and world loop**, not the philosophical choice.

### Emergence 2 — Composition is the missing math

Chat operators scale poorly. HD/VSA literature says compositionality needs **algebra** (bind/bundle), not more `if text.contains`.  
**Highest-leverage research bet:** binary bind for multi-domain synthesis before growing to 200 concepts.

### Emergence 3 — Impasse is the self-improvement primitive

Soar + SICA agree: intelligence grows when failure creates a structured subgoal.  
Perci’s hardness pack is the right sensor; the agent must **automatically open impasses** from red cases.

### Emergence 4 — Three memories, not one log

Folding the 501-event queue without structure will poison curriculum.  
Split:

1. **Style/prefs** (already safe)  
2. **Knowledge candidates** (pending teach)  
3. **Decision traces** (operator/critic/agent) → context graph lite  

### Emergence 5 — Live probe confirmation

Cross-domain connect of *SDM + VSA + Bitwork* triggered **critic rewrite with generic filler** because multi-word domains failed `names_all_requested_domains`.  
That is a concrete, implementable fix and a hardness case waiting to be written.

---

## 4. Ranked next work (do these, in order)

### P0 — Ops hygiene (hours)

1. Relaunch live binary (`Launch-Perci.ps1`) so scorecard is not `stale_live`.  
2. Re-run `python scripts/evolve_cycle.py` → refresh scorecard to hardness 43.  
3. Sample fold of learning queue: approve **20** high-signal events only (not all 501).

### P1 — Impasse lab (L8) — 1–2 weeks

4. `scripts/agent_lab.py` or `perci agent lab`:  
   read last hardness JSON → open ticket → optional patch template → re-eval.  
5. Auto-draft hardness from clustered live fails (already in roadmap Phase D).  
6. On green: commit on `agent/*`; optional merge with revert hook.  
   **Still never auto-promote `.pwgt`.**

### P2 — Composition repair (L4/L5) — 1 week

7. Multi-word domain terms in synthesis critic (space/`_`/hyphen aware).  
8. Specialist frames pack for: SDM, binding, impasse, hardness-gate, self-repair (teach via packs, not slogans).  
9. Program runtime: at least one program that **executes** a tool step (not only audits).

### P3 — Context graph lite (memory) — 1–2 weeks

10. Append decision traces: `{turn, operator, program_id, critic_ok, hardness_ids}`.  
11. Agent reads last N decision traces when planning a repair.  
12. Query CLI: `perci trace history` / Cortex remember of durable decisions only.

### P4 — VSA / Bitwork research track (parallel, longer)

13. Prototype **binary bind** for frame pairs in a side module (does not replace PERCIW03 until sealed eval).  
14. Measure: open-domain synthesis hardness + entity-swap transfer before/after.  
15. Only then consider PERCIW04 format.

### P5 — L7 hybrid LM (only if needed)

16. Attach local LM via `PERCI_MODEL_CMD` **under** Bitwork+tool authority after P1–P2 still leave code/prose &lt;70% task success.  
17. Never give LM exact-tool or promote authority.

### Explicitly deprioritize

- More personality/voice polish without hardness gain  
- Unlabelled dump of 501 interactions into weights  
- Claiming AGI from green 43/43  
- Unbounded shell / leave-repo autonomy  

---

## 5. Suggested next implementation ticket (single)

**T4 — Multi-word synthesis + impasse stub**

1. Fix critic domain matching for multi-word terms.  
2. Hardness H44: connect “sparse distributed memory, vector symbolic binding, and Bitwork”.  
3. `perci agent lab --from-hardness` dry-run that prints impasse ticket from any red case.  
4. Decision-trace line in `models/candidates/decision-trace.jsonl` for each high-salience deliberation.

Acceptance: H44 green; agent lab dry-run OK; 43 suite still green; no weight touch.

---

## 6. One-line strategy after cross-search

> **Deepen the neuro-symbolic integration engine (programs, impasse, bind/composition, decision memory) and the gated self-repair loop — do not race the transformer on fluency.**

---

## 7. Sources (external, for orientation)

- Neuro-symbolic layered systems / agents (2025–26 industry + surveys)  
- Kanerva SDM; HD/VSA surveys (Kleyko et al.); Binary Spatter Codes  
- ACT-R / Soar / LIDA cognitive architecture summaries  
- Self-Improving Coding Agent (arXiv:2504.15228) and SWE-bench agent loops  
- Context graphs: knowledge + conversation + decision-trace memory  

Internal: `docs/LOCAL_AGI_ROADMAP.md`, `docs/SUPERINTELLIGENCE_PATH.md`, `docs/EVOLUTION.md`, `VALIDATION.md` v0.5.1.
