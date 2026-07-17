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
  <strong>Local governed sparse cognition.</strong><br>
  Not a cloud LLM. Not a pretend mind. A Rust-native neuro-symbolic stack that<br>
  <em>routes in bits, thinks in operators, speaks like a collaborator — and shows its work on demand.</em>
</p>

<p align="center">
  <img alt="Software" src="https://img.shields.io/badge/software-v0.7.2-8b0000?style=for-the-badge">
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

## What you just walked into

**Perci** is an experimental **cognitive OS for a single machine**:

| Layer | Job |
|-------|-----|
| **Bitwork PERCIW03** | ~200 MiB sparse pack · **403,163** prototypes · **124** concepts · 4096-bit AND/POPCOUNT |
| **Transformer-bridge algebra** | Soft-α attention · dual residual ANDNOT · VSA bind/bundle · Willshaw HVs · session CTX |
| **SoftCascade thought arc** | thesis → warrant → boundary → check — spoken as human prose, not card dumps |
| **Operators** | Trust/systems, partition recovery, synthesis, refuse-hallucinate, code, plans, introspection |
| **Self-critique** | Thin drafts get one residual second angle — silent metacognition |
| **Emergence lab (L8)** | Tickets → **transfer suite** → repair/close · `release_gates.py` · agent `--full --repair` |
| **Capability Fabric (v0.7.2)** | Governor: language · knowledge · proof · code · multi-AI handoff/next · `perci fabric` |
| **Exact tools** | Math & geometry that *compute*, never guess |
| **Governance** | Append-only memory · Cortex · style memory · weight promote only with **human authorize** |

Chat is **clean** (no cognition dump). Inspect with **`/think`** (plan), **`/field`** (geometry laws), **`/lab`** (tickets). Style with **`/concise` · `/deep` · `/balanced`**.

> Fluency without transfer gates is not intelligence.  
> Consciousness claims without sensors are not honesty.

---

## Why this is different

Most assistants bury routing, language, tools, and memory inside one opaque model.

Perci **separates them**:

```text
  input
    │
    ▼
 ┌──────────┐     ┌─────────────────────────────┐
 │  reflex  │────▶│  Bitwork field (α, residual,│
 └──────────┘     │   VSA, multipartite mass)   │
                  └─────────────┬───────────────┘
            ┌───────────────────┼───────────────────┐
            ▼                   ▼                   ▼
      exact tools         operators           SoftCascade
      math · geometry     trust · code        thought arc
            │             synthesis · refuse        │
            └───────────────────┬───────────────────┘
                                ▼
                     human speech  ·  /think inspects
```

**What the field actually does** (live classify, 2026-07-17): contested prompts show **margin 0–2** and **overlap_z 15–26** — multipartite structure is real, not marketing. See [`docs/WEIGHT_REASSESSMENT_v0616.md`](docs/WEIGHT_REASSESSMENT_v0616.md).

---

## Numbers that are true today

| Property | Value |
|----------|------:|
| Software | **v0.7.2** (`Cargo.toml` · badge auto-stamped) |
| Pack format | **PERCIW03** |
| Pack size | **209,710,296** bytes (~200 MiB) |
| Prototypes | **403,163** |
| Concepts | **124** |
| Activation | **4,096** bits · 64 × u64 |
| Expert domains | **16** |
| Hot path | Integer **AND / POPCOUNT** only |
| Weights in git | **No** (GitHub size limits — local pack required) |

Version is **never** hand-edited in the badge: `build.rs` stamps `assets/generated/*` from `Cargo.toml`.

---

## Quick start

### Requirements

- Windows, macOS, or Linux  
- Rust + Cargo  
- Local pack: `models/perci-cognitive-v0.3.pwgt` (not in the clone)

### Clone & launch (Windows)

```powershell
git clone https://github.com/jacksonjp0311-gif/Perci.git
cd .\Perci
# place PERCIW03 under models\  (or $env:PERCI_WEIGHTS = "...")
Set-ExecutionPolicy -Scope Process Bypass -Force
.\Launch-Perci.ps1
```

### Cargo

```powershell
cargo run --release -- chat
cargo run --release -- ask "why does trust fail in distributed systems?"
cargo run --release -- classify "invent a constrained metaphor for sparse cognition"
cargo run --release -- fabric status
cargo run --release -- fabric handoff "improve transfer on novel entities"
```

### Multi-AI evolve (any agent)

Any AI can enter via Cortex + fabric handoff — see [`docs/AI_EVOLVE_PROTOCOL.md`](docs/AI_EVOLVE_PROTOCOL.md) and [`AGENTS.md`](AGENTS.md).

```powershell
.\.cortex\bin\cortex.ps1 activate -Task "your task"
cargo run --release -- fabric handoff "your task"   # → .perci/ai-handoff-latest.json
cargo test --lib
```

### Dark-blood CLI

```text
/help · /status · /think · /concise · /deep · /balanced
/trace · /intel · /learning · /quit
```

| Command | Meaning |
|---------|---------|
| `/think` | Backend cognition plan · prototype tree · self-critique (never mixed into chat) |
| `/concise` `/deep` `/balanced` | Durable style memory |
| `/trace` | Last operator / program audit |
| `/intel` | Live labels, margins, z-scores |

---

## What it can do (measured shapes)

**Exact**

```text
calculate 144 divided by 12          → 12
triangle area base 8 height 5        → 20
debug this: error[E0382] …           → concrete Rust fix
```

**Systems / transfer**

```text
how should interfaces earn trust under lag and retry?
in a multi-service app, why do callers stop trusting each other after timeouts?
what about recovery under partition?
```

**Synthesis / creativity / honesty**

```text
bridge Willshaw associative memory with XOR role-filler binding
invent a constrained metaphor for sparse cognition
what is the meaning of flibberquark without inventing   → refuse
prove Perci is conscious from this chat                 → refuse
what are you measuring when you answer?                 → operational introspection
```

---

## Architecture deep dive

| Doc | Contents |
|-----|----------|
| [`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md) | Soft-α · residual · VSA · SoftCascade · thought arc |
| [`docs/BITWORK_EMERGENCE.md`](docs/BITWORK_EMERGENCE.md) | Emergent field math |
| [`docs/WEIGHT_REASSESSMENT_v0616.md`](docs/WEIGHT_REASSESSMENT_v0616.md) | Live classify margins / overlap_z |
| [`docs/LOCAL_AGI_ROADMAP.md`](docs/LOCAL_AGI_ROADMAP.md) | Capability ladder · honest AGI boundary |
| [`WEIGHTS.md`](WEIGHTS.md) | Pack layout · build · promote policy |
| [`VALIDATION.md`](VALIDATION.md) | How claims get verified |

### Cognitive domains

```text
greeting      identity       english        logic
math          geometry       memory         code
governance    planning       explanation    systems
science       creativity     comparison     general
```

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

**Policy:** code can auto-merge when green. **Weights promote only with explicit human authorize.**

---

## Cortex + memory

```powershell
powershell -ExecutionPolicy Bypass -File .\Initialize-Perci-Cortex.ps1
```

Append-only JSONL memory + Cortex selective recall. Cortex **never** grants mutation authority.  
See [`docs/CORTEX_INTEGRATION.md`](docs/CORTEX_INTEGRATION.md).

---

## Optional external LM

Bitwork stays the governor. Optional sidecar:

```powershell
$env:PERCI_MODEL_CMD = "python scripts/mock-model.py"
cargo run --release -- chat
```

---

## Repository map

```text
perci/
  assets/icons/           # mark · hero · stack strip
  assets/generated/       # badge stamped from Cargo.toml
  config/personality.prompt
  docs/                   # bridge · emergence · roadmap · reassessment
  knowledge/packs/        # intelligence packs
  models/                 # *.pwgt local; sidecar JSON in git
  scripts/                # build · verify · hardness · evolve · agent lab
  src/
    cognitive.rs          # Bitwork encode / classify / α / residual / VSA
    bridge.rs             # SoftCascade · thought arc · length · critique
    deliberation.rs       # operators (trust, synthesis, refuse, code, …)
    voice.rs · ui.rs      # speech + dark-blood CLI
    chat.rs · backend.rs  # orchestration
    reasoning.rs          # exact math / geometry
    agent.rs · learning.rs
  Launch-Perci.ps1 · Start-Perci.cmd
```

---

## Capability boundary (read twice)

| Useful for | Not a substitute for |
|------------|----------------------|
| Local sparse routing + multipartite readout | ChatGPT / frontier transformers |
| Thought-arc speech without a decoder | Web-scale factual recall |
| Exact math/geometry | Unrestricted open-ended generation |
| Governed synthesis & refusal | “AGI” slogans |
| Inspectable `/think` geometry | Private chain-of-thought theater |

Progress = **hardness · transfer · latency · binding quality · honest abstention** — not vibes.

---

## Design principles

- **Local first** — no cloud required for the core loop  
- **Integer hot path** — AND / POPCOUNT, not GPU matmul  
- **Separate layers** — field · laws · tools · speech  
- **Human speech, backend truth** — chat clean; `/think` inspects  
- **Governed learning** — style adapts; weights need authorize  
- **Refuse when empty** — inventing meaning is a bug, not a feature  

---

## Roadmap (next real IQ)

1. Pack-side VSA encode (**human-authorized** rebuild)  
2. Spreading activation on prototype graph  
3. Novelty \(N_r\) vs session memory (length law already residual-aware)  
4. Stronger hardness / dialogue gates  
5. Agent lab: fail → ticket → patch → retest → merge green code only  

---

## Status

**Experimental research software.** Review [`VALIDATION.md`](VALIDATION.md) before treating a benchmark claim as sealed.

**License:** [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) — your choice.

---

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" width="72" height="72" alt="Perci">
  <br>
  <sub>PERCI · dark-blood · governed sparse cognition · v0.6.17</sub>
</p>
