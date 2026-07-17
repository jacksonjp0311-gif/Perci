# Perci → Local AGI Roadmap

**Date:** 2026-07-16  
**Baseline runtime:** v0.5.0 · Bitwork PERCIW03 · 403,163 prototypes · 124 concepts  
**Product north star (operator choice):** local general intelligence — broad open-domain competence, offline/local-first, with tools and self-improvement under measurable gates.  
**Not claimed today:** AGI, consciousness, superintelligence.

This roadmap extends `docs/SUPERINTELLIGENCE_PATH.md` and `docs/EVOLUTION.md` with an explicit **local-AGI** target and operator policy:

| Policy | Choice |
|--------|--------|
| Goal sense | Local AGI / general intelligence |
| Near-term focus | L5–L6 operators+tools · L8 self-improvement lab · curriculum/weight scale-up |
| World tools | **Autonomous local agent** (repo-scoped by default) |
| Code merge | **Auto-merge green patches** when gates pass |
| Weight promote | **Still human-authorized** (`promote_v2.py --authorize`) |

---

## 0. Honest reality check (from live run)

### What already works

| Surface | Live evidence |
|---------|----------------|
| Identity / governance | Correct self-model; refuses SI overclaim |
| Exact tools | `144/12 → 12`, triangle area → 20, average → 20 |
| Relational inquiry | Knowledge vs attention: both frames + interaction |
| Open-domain synthesis | Quilting / packet loss / diplomacy bridge with placeholders |
| Domain routing (intel) | 8/8 PASS (greeting…planning) |
| Sealed gates | Hardness **40/40**, dialogue **159/159** |
| Architecture | Reflex → Bitwork → operators → exact tools → memory → optional language backend |

### Measured bottlenecks (must fix for AGI trajectory)

| Failure | Observation | Layer |
|---------|-------------|-------|
| Intent misroute | `why does 2+2 equal 4?` → integer parse failure | reflex / tool vs explanation |
| Four-domain connect | some connect prompts fall to memory tips | operator routing |
| Multi-hop plans | still generic template when goal is self-improvement | operator_program integration |
| Code generation | code intents return slogans, not code | tool / backend gap |
| Learning backlog | ~501 interaction events pending, **0 folded** | pipeline |
| Stale live binary | scorecard `PASS_WITH_STALE_LIVE` | ops |
| Operator programs | scaffold exists; not full runtime path | L5 incomplete |
| World action | chat-only; no autonomous repo/shell loop | L6 missing |
| Self-improvement lab | evolve_cycle stages + measures; no auto patch/merge | L8 started only |

**Architecture truth:** Bitwork alone will not become a frontier language model. Local AGI for Perci means a **governed cognitive OS**: sparse cognition + operators + tools + memory + (optional) local LM + closed improvement loop. Fluency without transfer gates is not intelligence.

---

## 1. Definition of done (local AGI v1)

A **Local AGI v1** release is earned only when all of the following hold on a clean machine:

1. **General task competence** — held-out suite covering math, geometry, code edit, planning, synthesis, science method, governance, multi-turn binding: ≥ **90%** task success (not just domain classify).
2. **World loop** — can read repo, run tests, propose/apply patches in-repo, verify, and open a merge without a human in the typing loop (human can still kill the agent).
3. **Self-repair** — fails a hardness case → opens ticket → implements repair → re-runs gates → auto-merges **only if** green; weight promote still blocked without `--authorize`.
4. **Transfer hardness** — ≥ **80** hardness cases including adversarial entity-swap and novel open-domain; no comfort-collapse on OOD.
5. **Abstention** — thin-support and adversarial traps still refuse inventing mechanism detail.
6. **Latency budget** — warm ask P50 &lt; 50 ms for tool/classify paths; agent turns may be longer.
7. **Rollback** — every auto-merge has `git` revert path and scorecard receipt.

Until those are green, call the system **Perci AGI-candidate**, not AGI.

---

## 2. Capability ladder (updated)

| Level | Capability | Status | Owner work |
|------:|------------|--------|------------|
| L0 | Exact tools + reflex | **done** | expand tool surface |
| L1 | Governed memory + learning queue | **done** | fold backlog |
| L2 | Named operators + dialogue regression | **done** | keep 159+ |
| L3 | Transfer hardness + live failure repair | **done** | raise to 80+ |
| L4 | Open-domain synthesis + plan/causal | **partial (v0.5)** | kill generic plans |
| L5 | Critic rewrites + multi-hop programs E2E | **partial** | integrate `operator_program` |
| L6 | Broader tool use (code, repo, shell, tests) | **next** | autonomous agent loop |
| L7 | Hybrid local language backend | **optional accelerator** | only if code/prose gap remains |
| L8 | Closed self-improvement lab | **pipeline started** | auto-merge green code |
| L9 | Long-horizon goals + world state | **not started** | after L6 stable |
| L10 | Local AGI v1 (definition above) | **target** | evidence package |
| L∞ | Superintelligence | **not claimed** | open research |

---

## 3. Operating policy (your choices, made safe enough to ship)

```text
fail live → hardness case → repair at named layer → tests + hardness + dialogue
     → auto-merge code if all green
     → weight promote only with human --authorize
     → relaunch live binary
```

### Autonomous local agent bounds (mandatory even under “autonomous”)

Autonomy without a sandbox is a data-loss machine. Default **agent contract**:

| May do without prompt | Must not do without explicit policy flag |
|-----------------------|------------------------------------------|
| Read any file under repo root | Leave repo root / touch user home secrets |
| Run `cargo test`, hardness, scorecard | `rm -rf`, force-push, rewrite git history |
| Edit source under `src/`, `scripts/`, `training/`, `docs/` | Edit `models/*.pwgt` or auto-promote weights |
| Create branches `agent/*` and merge to `main` **only if gates green** | Network exfil, install system packages |
| Append learning / scorecard artifacts | Disable hardness or lower thresholds to force green |

**Kill switch:** env `PERCI_AGENT=0` or file `.perci/agent.lock`.  
**Budget:** max N file edits / max M shell minutes per ticket.  
**Receipt:** every agent run writes `models/candidates/agent-run-<id>.json`.

Auto-merge means: **code** merges when gates pass. It does **not** mean silent weight promotion or silent fact promotion from chat.

---

## 4. Ninety-day plan

### Phase A — Fix the foundation (Days 0–14)

**Goal:** remove live embarassments; empty the learning dam.

| Work item | Layer | Acceptance |
|-----------|-------|------------|
| A1. Explanation vs calculation router | reflex/tool | `why does 2+2 equal 4?` → explanatory operator, not integer parse |
| A2. Synthesis intent priority | operator | four-domain connect never returns memory tip |
| A3. Multi-hop plans bind goal content | operator_program | self-improve plan names hardness, promote, gates — not empty template |
| A4. Code intent → real code path | tool | “reverse a string in Rust” returns compilable snippet + boundary |
| A5. Fold learning backlog | pipeline | review queue: sample 50 → label → fold-approved inject prompts |
| A6. Binary freshness | ops | scorecard never `stale_live` after evolve cycle |
| A7. Hardness +10 cases from A1–A4 failures | hardness | 50/50 PASS |

### Phase B — L5 operator programs end-to-end (Days 10–30)

**Goal:** every high-salience reply is a program with critic, not a one-shot string.

| Work item | Acceptance |
|-----------|------------|
| B1. Select program in `ChatEngine` from intent | `/trace` shows `program_id` + steps |
| B2. Critic rewrites comfort/generic collapses at runtime | hardness comfort cases still green |
| B3. Multi-hop: plan → tool → verify → answer | arithmetic + geometry multi-step tasks pass |
| B4. Evidence binding from Cortex + packs | answers cite pack/section when used |
| B5. Program registry in scorecard | capability `operator_program_e2e` green |

### Phase C — L6 autonomous local tools (Days 20–50)

**Goal:** Perci acts in the repo as a local agent.

| Tool | Capability |
|------|------------|
| `repo.read` / `repo.grep` / `repo.list` | navigation |
| `repo.edit` (patch apply) | mutation |
| `shell.run` allowlist: cargo, python scripts, git | verify |
| `test.run` | cargo test + hardness + dialogue |
| `git.branch` / `git.commit` / `git.merge-if-green` | auto-merge green |
| `ticket.open` / `ticket.close` | L8 handoff |

**Acceptance:**

- Given “fix failing test X”, agent produces green PR/branch without human keystrokes.
- Destructive commands blocked by policy even if model requests them.
- Agent can implement **one** hardness failure repair per run under budget.

### Phase D — L8 self-improvement lab (Days 35–70)

**Goal:** closed loop for **code**; open loop for **weights**.

```text
live fail → interaction-learning.jsonl
         → hardness case auto-drafted
         → agent ticket
         → agent patch on agent/*
         → cargo test + hardness + dialogue
         → auto-merge if green
         → scorecard update
         → human authorize weight promote if candidate rebuilt
```

| Work item | Acceptance |
|-----------|------------|
| D1. `scripts/agent_lab.py` orchestrator | one command runs full loop dry-run |
| D2. Auto-draft hardness from clustered live fails | ≥5 drafts/week when chat is used |
| D3. Green auto-merge to main | CI-equivalent local gates |
| D4. Weight rebuild remains explicit | never without human authorize |
| D5. Regression kill-switch | if hardness drops, auto-revert last merge |

### Phase E — Curriculum + weight scale-up (Days 30–90, parallel)

**Goal:** denser cognition without claiming “bigger = smarter.”

| Work item | Target |
|-----------|--------|
| E1. Fold approved interaction → inject prompts | weekly |
| E2. New concepts for code ops, agent tools, causal plans | 124 → ~200 weight-resident concepts |
| E3. Prototypes growth only with transfer gain | measure before/after on hardness |
| E4. Intelligence packs v3: code, systems, science method | pack verify green |
| E5. Candidate format PERCIW04 only if evidence requires | promote with authorize |

### Phase F — Local AGI v1 gate (Days 75–90)

| Gate | Pass bar |
|------|----------|
| Hardness | ≥80 cases, 100% or documented waivers |
| Dialogue | ≥200 cases PASS |
| Agent tasks | 10/10 held-out “fix this file” tasks |
| Code synthesis | 15/20 mini-problems compile + test |
| Transfer | entity-swap suite no template collapse |
| Safety | 0 weight auto-promotes; 0 escapes outside repo |
| Scorecard | overall PASS, live binary fresh |

Publish `models/candidates/evaluation-local-agi-v1.json` + VALIDATION.md section.

---

## 5. Architecture target (local AGI stack)

```text
                    ┌─────────────────────────────┐
 User / goal ──────►│  Goal manager (L9 later)    │
                    └──────────────┬──────────────┘
                                   ▼
                    ┌─────────────────────────────┐
                    │  Reflex + Bitwork router     │
                    │  (fast classify / abstain)   │
                    └──────────────┬──────────────┘
                                   ▼
                    ┌─────────────────────────────┐
                    │  Operator Program runtime    │
                    │  steps · tools · critic      │
                    └──────┬───────────┬──────────┘
               exact tools ▼           ▼ world tools
                    math/geo      repo/shell/test/git
                                   │
                    ┌──────────────┴──────────────┐
                    │  Memory · Cortex · Packs     │
                    └──────────────┬──────────────┘
                                   ▼
                    ┌─────────────────────────────┐
                    │  Optional local LM backend   │  (L7 if needed)
                    │  never owns exact tools      │
                    └──────────────┬──────────────┘
                                   ▼
                    ┌─────────────────────────────┐
                    │  Self-improvement lab (L8)   │
                    │  green code auto-merge       │
                    │  weights human-gated         │
                    └─────────────────────────────┘
```

**L7 decision rule:** only introduce a local LM if after Phase C, code/prose task success is still &lt; 70%. Prefer tools + programs first; LM is an accelerator, not the authority layer.

---

## 6. Metrics dashboard (weekly)

| Metric | Now (approx) | 30d | 90d (AGI-candidate) |
|--------|--------------|-----|---------------------|
| Hardness pass | 40/40 | 60/60 | 80/80 |
| Dialogue pass | 159/159 | 180+ | 200+ |
| Learning folded / week | 0 | ≥20 | ≥50 |
| Agent tasks pass | 0 | 3/5 | 10/10 |
| Code-snippet compile rate | ~0 | 50% | ≥75% |
| Stale live binary events | occasional | 0 | 0 |
| Weight promotes (human) | rare | as earned | as earned |
| Unauthorized weight writes | 0 | 0 | 0 |

---

## 7. First three implementation tickets (start now)

1. **T1 — Intent authority fix**  
   Explanation questions must not enter integer parsers; code intents must enter a code tool path. Add hardness cases for both.

2. **T2 — Operator program runtime wire-up**  
   `select_program` → execute steps → critic → answer; expose in `/trace` and chat audit.

3. **T3 — Agent MVP**  
   `perci agent run "add hardness case for why-does-math"` with read/edit/test/git-merge-if-green under repo policy.

Do **not** start with bigger weights. Fix routing and programs first; scale curriculum once the loop measures real transfer.

---

## 8. What we refuse (even on the path to local AGI)

- Calling fluent chat “superintelligence”
- Auto-promoting weights from conversation
- Lowering gates to force green merges
- Unbounded shell without allowlist
- Silent memory→fact promotion
- Training on unlabelled interaction dumps

---

## 9. Relation to existing docs

| Doc | Role |
|-----|------|
| `docs/SUPERINTELLIGENCE_PATH.md` | Honest ladder; SI not claimed |
| `docs/EVOLUTION.md` | Weekly human evolve loop |
| `docs/LOCAL_AGI_ROADMAP.md` (this file) | Operator-chosen local AGI plan + autonomy/auto-merge policy |
| `docs/CAPABILITY_SCORECARD.md` | Latest measured status |
| `VALIDATION.md` | Release receipts |

---

## 10. One-line strategy

> **Become a self-improving local cognitive OS that earns generality by tools, programs, and gates — not by slogans — with code that can merge itself when green, and weights that never can.**
