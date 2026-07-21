# Modular Binary Cognition (v1)

**Status:** Phase 1–6 interfaces + SEM/RSN/DSC/LM candidate packs · **chat wire** (substantive intents, quality-gated) · PERCIW03 retained  
**Claim boundary:** engineering architecture + measured telemetry — not consciousness, not AGI, not quantum cognition, not auto-promote.

## Law

```text
PERCIW03     → fast reflex, domain routing, concept activation, governance cues
operators    → inspectable reasoning (ThoughtPlan preferred for substantive turns)
exact tools  → arithmetic / geometry truth
memory       → deliberate store + session context
new packs    → optional mmap modules selected sparsely by capability router
human        → durable weight promote only after evaluation receipt
```

Never auto-promote `.pwgt` or pack binaries. Total installed size must not determine per-turn cost.

## Stack (target)

```text
User input
  → dialogue workspace + semantic parse
  → PERCIW03 reflex / capability routing
  → PERCISEM1 semantic field        (Phase 3)
  → PERCIRSN1 reasoning transitions (Phase 4)
  → exact tools, evidence, memory
  → PERCIDSC1 discourse plan        (Phase 5)
  → PERCILM1 language realization   (Phase 6)
  → governor / critic
  → answer + operational receipt
```

## Delivered modules

| Module | Path | Role | Phase |
|--------|------|------|-------|
| Pack manifest | `src/pack_manifest.rs` | Common JSON+header contract, discovery, promotion status | 1 |
| Capability router | `src/capability_router.rs` | Hierarchical domain→capability→pack select; telemetry | 1 |
| ThoughtPlan | `src/thought_plan.rs` | Structured cognitive product (not private CoT) | 2 |
| Field fold harness | `src/field_fold.rs` | Operator-dependent fold experiment (PERCIFLD1) | 1/7 |
| **PERCISEM1** | `src/semantic_field.rs` | Semantic frame extract, HV encode, mmap pack, retrieve | **3** |
| **PERCIRSN1** | `src/reason_transition.rs` | Bounded reasoning transitions → ThoughtPlan | **4** |
| **PERCIDSC1** | `src/discourse_plan.rs` | Intent×depth×variant discourse acts + materialize slots | **5** |
| **PERCILM1** | `src/language_realize.rs` | Constrained wording from plan slots (1/2/4-bit style) | **6** |

### Phase 3 — PERCISEM1

Binary magic `PERCISEM1`. Each record stores 8 role-bound 256-bit HVs + bundled query + label.

```text
prompt → extract_frame → encode (role-permute ⊕ filler) → bundle query
       → nearest stored frames by Hamming agreement
```

Fixtures: `training/modular/semantic-frames-v1.jsonl`  
Eval: `training/modular/semantic-eval-v1.jsonl` · `perci modular eval-sem`  
Build: `perci modular build-sem` → `models/candidates/packs/percisem1-v0.1.bsem`

### Phase 4 — PERCIRSN1

Binary magic `PERCIRSN1`. Policy table + **in-process bounded executor** (not private CoT):

```text
bind_request → decompose → assumptions → mechanism/principle
→ alternatives → counterexample → analogy bound → uncertainty
→ compress → halt
```

Build: `perci modular build-rsn` → `models/candidates/packs/percirsn1-v0.1.brsn`  
Run: `perci modular reason "<prompt>"`

### Phase 5 — PERCIDSC1

Binary magic `PERCDSC1`. Selects discourse skeletons by intent, style depth, and
history-sensitive variant so multi-turn answers do not always use the same act order.

```text
ThoughtPlan → plan_discourse → acts + connectives → materialize_slots
```

Build: `perci modular build-dsc` → `models/candidates/packs/percidsc1-v0.1.bdsc`  
Run: `perci modular discourse "<prompt>"` · `perci modular eval-dsc`

### Phase 6 — PERCILM1

Binary magic `PERCLM1`. **Wording only**: composes continuous prose from discourse
slots and ThoughtPlan claims. Compares 1-bit / 2-bit / 4-bit connective density.
Never invents tools results or overrides refusals.

```text
SEM frame → RSN transitions → DSC plan → LM realize → governor checks
```

Build: `perci modular build-lm` → `models/candidates/packs/percilm1-v0.1.blm1`  
Run: `perci modular realize [1|2|4] "<prompt>"`

### Chat integration (v0.9.8 evolve)

Live chat calls `language_realize::try_chat_realize` **after** social / exact / operator /
dialogue-act paths and **before** SoftCascade fluid generation:

```text
try_chat_realize(user, recent)
  → eligible intent or frontier turn or deictic followup
  → realize_from_prompt (SEM→RSN→DSC→LM)
  → modular_quality_ok gate (reject thin "structure under constraint" shells)
  → envelope as operator modular-realize
  → else SoftCascade
```

Social, exact tools, and thin unknown turns stay off this path.

### What stays finished-string

- Social / greeting reflexes  
- Exact math / geometry tools  
- Explicit refusals (consciousness, auto-promote, OOD invent)

### What migrates toward ThoughtPlan

Substantive operators may still emit answer text for compatibility, but should also populate a `ThoughtPlan` for receipts and future discourse/language packs.

## Pack identifiers

| Magic / id | Role | Phase | Budget (engineering, not padding) |
|------------|------|-------|-------------------------------------|
| `PERCIW03` | Reflex / routing field | active | ~200 MiB |
| `PERCISEM1` | Semantic field | 3 | 32–64 MiB |
| `PERCIRSN1` | Reasoning transitions | 4 | 96–256 MiB |
| `PERCIDSC1` | Discourse planning | 5 | 16–48 MiB |
| `PERCILM1` | Language realization | 6 | 128–512 MiB |
| `PERCIFLD1` | Fold experiment | 7 | experiment |

## Sparse activation contract

```text
prompt
→ compact index / PERCIW03 domain score
→ capability tags
→ select ≤K packs (default K=3)
→ mmap only selected shards
→ bounded cycles
→ telemetry: installed / mapped / accessed bytes
```

A future multi-GB install must never global-scan all packs.

## Folding hypothesis (operational)

```text
input state
→ semantic field
→ operator-selected fold
→ compact binary representation
→ operator-compatible retrieval
→ reconstructed or transformed state
```

Folding is **not** assumed infinite or lossless. Measure collapse, transfer, and operator-matched vs mismatched decode. “Observer” means the decode/score operator, not phenomenology.

## Promotion status

```text
candidate → evaluated → authorized → active
```

Only `active` packs may influence production speech after human authorize. Builders write `candidate` only.

## Related docs

- `WEIGHTS.md` — PERCIW03 / PERCLNG1 / PERCIWM1 formats  
- `docs/CAPABILITY_FABRIC_v070.md` — multi-engine governor  
- `docs/LOWBIT_LAYER.md` — PERCLBW1 representation  
- `AGENTS.md` — authority law  
