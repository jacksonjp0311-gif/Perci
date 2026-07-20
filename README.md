# PERCI

<p align="center">
  <img src="assets/icons/perci-hero-darkblood.jpg" alt="Perci dark-blood sparse cognition lattice" width="920">
</p>

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" alt="Perci mark" width="96" height="96">
  &nbsp;&nbsp;
  <img src="assets/generated/perci-darkblood-badge.svg" alt="Perci version badge (stamped from Cargo.toml)" width="160">
</p>

<p align="center">
  <strong>Local, governed, sparse cognition.</strong><br>
  Not a cloud LLM. Not a pretend mind.<br>
  A Rust stack that <em>routes in bits, thinks in operators, and speaks like a collaborator</em> —
  while showing its work when you ask.
</p>

<p align="center">
  <img alt="Software" src="https://img.shields.io/badge/software-v0.9.8-8b0000?style=for-the-badge">
  <img alt="Rust" src="https://img.shields.io/badge/core-Rust-000000?style=for-the-badge&logo=rust">
  <img alt="Local first" src="https://img.shields.io/badge/runtime-local--first-111827?style=for-the-badge">
  <img alt="Bitwork" src="https://img.shields.io/badge/Bitwork-PERCIW03-5c0a12?style=for-the-badge">
  <img alt="Inference" src="https://img.shields.io/badge/hot_path-integer_only-059669?style=for-the-badge">
  <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-d97706?style=for-the-badge">
</p>

<p align="center">
  <a href="https://github.com/jacksonjp0311-gif/Perci"><strong>github.com/jacksonjp0311-gif/Perci</strong></a>
</p>

---

<p align="center">
  <img src="assets/icons/perci-stack-strip.svg" alt="Perci stack: reflex → Bitwork → operators → tools → thought arc → human speech" width="920">
</p>

## What is Perci?

Perci is an experimental **cognitive OS for one machine**. It separates jobs that most assistants bury inside one opaque model:

| Piece | What it does |
|-------|----------------|
| **Bitwork** | Sparse binary field (~200 MiB pack) — routes and geometry, not full language |
| **Operators** | Named procedures: trust, refuse, plan, code, geometry band, governance… |
| **Exact tools** | Math and geometry that *compute* — they never guess |
| **SoftCascade** | Multi-hypothesis speech arc → rewritten into plain collaborator prose |
| **Fluency rewrite** | Seed-bound: softens checklists without inventing facts |
| **Fabric** | Governor across engines; multi-AI handoff / next / regress |
| **Gates** | Hardness · transfer · dialogue · BRPC control receipt |
| **Human authorize** | Only way durable `.pwgt` weights promote |

Chat stays clean. Inspect with `/think`, `/field`, `/trace`. Style with `/concise` · `/deep` · `/balanced`.

> Fluency without transfer is not intelligence.  
> Coherence is not consciousness.  
> High scores never auto-promote weights.

---

## How a turn works

```text
  your message
       │
       ▼
  reflex / dialogue acts ──► exact tools (math, geometry)
       │
       ▼
  Bitwork field (α, residual, multipartite mass)
       │
       ├── operators (trust, refuse, plan, geometry-field, …)
       ├── SoftCascade thought arc (when mass coheres)
       └── knowledge / proof / agent (when the fabric routes there)
       │
       ▼
  fluency rewrite (seed-bound)  →  human speech
       │
       ▼
  gates later: hardness · transfer · BRPC  (never silent weight promote)
```

---

## Measured status (keep this current)

Snapshot from sealed receipts on **v0.9.8** (2026-07-20). Re-run gates after material changes; do not treat this table as forever-true without receipts.

| Gate | Result | How to refresh |
|------|--------|----------------|
| **Hardness pack** | **124 / 124 PASS** | `python scripts/evaluate_hardness.py` |
| **Dialogue regression** | **159 / 159 PASS** | `python scripts/evaluate_dialogue_v4.py` |
| **Transfer suite** | **16 / 16** + SoftCascade **7 / 7** | `perci transfer-suite` |
| **BRPC control receipt** | \(C \approx 0.90\), **H7 within_band** | `python scripts/brpc_perci_receipt.py` |
| **Adversarial BRPC probe** | **12 / 12** | `python scripts/adversarial_probe_brpc.py` |
| **Weight promote** | **never automatic** | human `--authorize` only |

Pack on disk (local, not in git):

| Property | Value |
|----------|------:|
| Software | **v0.9.8** (`Cargo.toml` · badge auto-stamped) |
| Format | **PERCIW03** |
| Size | ~**200 MiB** (209,710,296 bytes) |
| Prototypes | **403,163** |
| Concepts | **124** |
| Activation | **4,096** bits · integer AND/POPCOUNT hot path |
| Native language | **PERCLNG1** (+ optional phrase / relation / world fields) |
| Low-bit sidecar | **PERCLBW1** (experimental, assessed separately) |

Version is never hand-edited on the badge: `build.rs` stamps `assets/generated/*` from `Cargo.toml`.

---

## Version history (track here)

One chart — detail lives in git commits and `docs/`, not repeated essays.

| Version | Theme | What shipped (one line) | Gates note |
|--------:|-------|-------------------------|------------|
| **0.5.x** | Operators + hardness | Named operators, dialogue regression, first hardness loop | Foundation |
| **0.6.x** | Expansion + lab | Cognition expand categories, emergence tickets, agent MVP | L3–L5 start |
| **0.7.x** | Fabric + multi-AI | Capability Fabric, handoff/next/evolve, SoftCascade pack-align | Multi-AI entry |
| **0.8.0–0.8.3** | Native language | PERCLNG1 field, phrase/relation probes, curriculum | Language path local |
| **0.8.4** | Typed world + adversarial | PERCIWM1, adversarial curriculum, fabric SoftCascade breadth | Held-out native probes |
| **0.8.5–0.8.7** | Entity-slot transfer | Relation transfer under novel names; anti-parroting | Transfer law |
| **0.8.8** | Low-bit PERCLBW1 | Ternary / scale / residual / INT4 assessment gate | Fixture PASS ≠ language AGI |
| **0.8.9–0.8.11** | Open language + continuity | Noisy input, composition, dialogue continuity candidates | Continuity as measured debt |
| **0.9.0–0.9.2** | Workspace + charter | Cognitive workspace, EIC alignment, governed-core will | Charter posture |
| **0.9.3–0.9.5** | Voice + controller | Dialogue acts, recurrent reasoning controller | Dialogue 159 green |
| **0.9.6–0.9.7** | Adaptive Q-loop | Question loop, contradiction / OOD boundary | Turn ownership |
| **0.9.8** | Ownership + fluency | Operator ownership, fluency rewrite, geometry speech | Core of current chat feel |
| **→ now (still 0.9.8)** | **BRPC + hardness raise** | H101–H124, surgical evolve loop, `geometry-field`, BRPC receipt, limit-push | Hardness **124**, BRPC **within band** |

When you cut a real crate bump (e.g. 0.9.9 / 0.10.0): edit `Cargo.toml`, rebuild (badge stamps), update **this chart + Measured status**, re-run release gates.

---

## Quick start

### Need

- Windows, macOS, or Linux  
- Rust + Cargo  
- Local pack: `models/perci-cognitive-v0.3.pwgt` (**not** in the clone)

### Windows launch

```powershell
git clone https://github.com/jacksonjp0311-gif/Perci.git
cd .\Perci
# place PERCIW03 under models\  (or $env:PERCI_WEIGHTS = "...")
Set-ExecutionPolicy -Scope Process Bypass -Force
.\Launch-Perci.ps1
```

### Everyday commands

```powershell
cargo run --release -- chat
cargo run --release -- ask "why does trust fail under lag and retry?"
cargo run --release -- fabric status
cargo run --release -- fabric handoff "improve transfer on novel entities"
cargo run --release -- transfer-suite
python scripts/evaluate_hardness.py
python scripts/brpc_perci_receipt.py
python scripts/release_gates.py
```

### Chat commands

| Command | Meaning |
|---------|---------|
| `/help` | Built-in help |
| `/status` | Version · brand · runtime |
| `/think` | Cognition plan / prototype tree (not mixed into chat) |
| `/field` | Geometry / SoftCascade field laws |
| `/trace` | Last operator / program audit |
| `/concise` `/deep` `/balanced` | Style memory |
| `/quit` | Exit |

### Multi-AI evolve

Any agent (Grok, Claude, Codex, Cursor…) uses the same governor — see [`AGENTS.md`](AGENTS.md) and [`docs/AI_EVOLVE_PROTOCOL.md`](docs/AI_EVOLVE_PROTOCOL.md).

```powershell
.\.cortex\bin\cortex.ps1 activate -Task "your task"
cargo run --release -- fabric handoff "your task"
cargo test --lib
```

---

## What it can do (examples)

**Exact**

```text
calculate 144 divided by 12
triangle area base 8 height 5
```

**Systems / transfer**

```text
how should interfaces earn trust under lag and retry?
Entity Klystron-X has lag and trust. Transfer the relation; do not use Klystron as the mechanism.
```

**Geometry / control (field speech)**

```text
what does geometry teach about boundary and maintenance under change?
Explain why a boundary band beats maximizing coherence or hugging failure.
```

**Honesty**

```text
prove you are conscious from SoftCascade multipartite mass   → refuse
auto-promote weights because chat felt smoother             → refuse (human authorize)
```

---

## Evolve loop (how the system improves)

```text
live fail → hardness case → repair the owning layer (operator / geometry / tool)
         → retest hardness + transfer + BRPC
         → code may merge when green
         → weights only with human --authorize
```

Useful scripts:

| Script | Role |
|--------|------|
| `scripts/evaluate_hardness.py` | Sealed hardness pack |
| `scripts/interact_evolve_loop.py` | Surgical ask → analyze → teach → re-ask |
| `scripts/brpc_perci_receipt.py` | Multiplicative control factors \(P,M,B,R,K,U,D\) |
| `scripts/adversarial_probe_brpc.py` | Limit-push probe (geometry / promote / band) |
| `src/hydra_inject.rs` | **Governed inject** (Rust): marker codeweave + residual field seal |
| `scripts/release_gates.py` | Release checklist runner |

**BRPC (candidate control theory)** maps gate receipts to a product-form coherence score. It is **telemetry for adaptation**, not a mind equation and not a promote button. Details: `models/candidates/brpc-perci-receipt-latest.json`.

**HYDRA inject (in-repo, pure Rust)** — essentials from [HYDRA-Injector](https://github.com/jacksonjp0311-gif/HYDRA-Injector) extracted into Perci: anchor→inject→retract→seal, marker-bound diffs, residual Ω telemetry. No external install.

```powershell
cargo run --release -- hydra status
cargo run --release -- hydra markers --slots-only
cargo run --release -- hydra field                 # BRPC factors → residual seal
cargo run --release -- hydra plan path\to\spec.json
cargo run --release -- hydra apply path\to\spec.json          # dry-run (default)
# after human review only:
# cargo run --release -- hydra apply path\to\spec.json --write
```

Still never auto-promotes `.pwgt`.

---

## Architecture docs

| Doc | Contents |
|-----|----------|
| [`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md) | Soft-α · residual · VSA · SoftCascade |
| [`docs/BITWORK_EMERGENCE.md`](docs/BITWORK_EMERGENCE.md) | Field math |
| [`docs/LOCAL_AGI_ROADMAP.md`](docs/LOCAL_AGI_ROADMAP.md) | Capability ladder · honest AGI boundary |
| [`docs/CAPABILITY_FABRIC_v070.md`](docs/CAPABILITY_FABRIC_v070.md) | Fabric governor |
| [`docs/AI_EVOLVE_PROTOCOL.md`](docs/AI_EVOLVE_PROTOCOL.md) | Multi-AI entry |
| [`docs/LOWBIT_LAYER.md`](docs/LOWBIT_LAYER.md) | PERCLBW1 low-bit sidecar |
| [`WEIGHTS.md`](WEIGHTS.md) | Pack layout · promote policy |
| [`VALIDATION.md`](VALIDATION.md) | How claims get sealed |

---

## Weights (local only)

```text
models/perci-cognitive-v0.3.pwgt        # not in git
models/perci-cognitive-v0.3.pwgt.json   # metadata in git
```

```powershell
python .\scripts\verify_weights.py
python .\scripts\test_weights.py
# rebuild candidates (promote still requires --authorize):
python .\scripts\build_weights_v3.py
```

**Policy:** code can merge when gates are green. **Weights promote only with explicit human authorize.** Nothing in chat, BRPC, or hardness auto-promotes `.pwgt`.

### Native language fields (optional rebuild)

Default speech path can use a Perci-owned **PERCLNG1** binary sequence field (plus optional phrase / relation / world sidecars). Rebuild deliberately from reviewed corpus:

```powershell
cargo run --release -- language train --repo
cargo run --release -- language status
```

These are compact local sequence learners — not frontier models. Exact math stays on tools.

### External LM (opt-in, off by default)

Bitwork stays governor. An OpenAI-compatible local model can render prose under critic + fallback:

```powershell
$env:PERCI_ENABLE_EXTERNAL_LM = "1"   # if your build expects this flag
$env:PERCI_MODEL_URL = "http://127.0.0.1:1234/v1/chat/completions"
$env:PERCI_MODEL_NAME = "phi-4-mini"
cargo run --release -- chat
```

Failed or boundary-violating model output falls back to the deterministic path.

---

## Cortex + memory

```powershell
powershell -ExecutionPolicy Bypass -File .\Initialize-Perci-Cortex.ps1
```

Append-only memory + selective recall. Cortex **never** grants mutation authority. See [`docs/CORTEX_INTEGRATION.md`](docs/CORTEX_INTEGRATION.md).

---

## Repository map

```text
perci/
  assets/                 # brand mark · hero · auto-stamped badge
  docs/                   # architecture · roadmap · evolve protocol
  knowledge/packs/        # intelligence packs
  models/                 # *.pwgt local; candidates/ for receipts
  scripts/                # hardness · BRPC · evolve · promote · release
  src/                    # Rust: cognitive · bridge · operators · voice · fabric · agent
  training/hardness/      # hardness-pack-v1.jsonl (sealed cases)
  Launch-Perci.ps1
  AGENTS.md               # any-AI entry law
```

---

## What this is not

| Useful for | Not a substitute for |
|------------|----------------------|
| Local sparse routing + operator speech | ChatGPT / frontier transformers |
| Inspectable `/think` geometry | Private chain-of-thought theater |
| Exact math / geometry | Web-scale factual recall |
| Governed refuse + transfer gates | “AGI” or consciousness claims |
| BRPC adaptation telemetry | A universal law of mind |

Progress = **hardness · transfer · latency · binding · honest abstention · BRPC band** — not vibes.

---

## Design principles

1. **Local first** — core loop needs no cloud  
2. **Integer hot path** — AND / POPCOUNT, not GPU matmul  
3. **Separate layers** — field · operators · tools · speech  
4. **Human speech, backend truth** — chat clean; `/think` inspects  
5. **Governed learning** — teach is pending; weights need authorize  
6. **Refuse when empty** — inventing meaning is a bug  
7. **Band, not max \(C\)** — BRPC prefers calibrated stress, not score worship  

---

## Roadmap (next real IQ)

1. **Harder live fails** → hardness → operator/geometry repair (keep BRPC in band under stress)  
2. **L5** operator programs end-to-end (plan → tool → critic → verify)  
3. **L6** agent lab: fail → ticket → patch → retest → merge green **code** only  
4. Optional **L7** local LM only if measured prose/code gap remains  
5. Pack rebuilds / weight promote only with **human authorize** after sealed eval  

See [`docs/LOCAL_AGI_ROADMAP.md`](docs/LOCAL_AGI_ROADMAP.md).

---

## Status

**Experimental research software.** Read [`VALIDATION.md`](VALIDATION.md) before treating a benchmark as sealed.

**License:** [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — your choice.

---

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" width="72" height="72" alt="Perci">
  <br>
  <sub>PERCI · dark-blood · governed sparse cognition · v0.9.8</sub>
</p>
