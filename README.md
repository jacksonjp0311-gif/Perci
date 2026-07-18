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
  <img alt="Software" src="https://img.shields.io/badge/software-v0.8.4-8b0000?style=for-the-badge">
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
| **Capability Fabric (v0.8.4)** | Governor: native language · typed world model · knowledge · proof · code · multi-AI handoff/next/regress · SoftCascade pack-align |
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

## Emergent discoveries from the v0.7.3 loop

The strongest new result is not a claim that the pack became conscious. It is a measurable engineering loop:

```text
weak route → typed field event → ticket/curriculum candidate
          → operator or tool repair → paraphrase/entity transfer
          → close only when the gate holds
```

The current ledger shows a **dual-authority split**: Bitwork probes the geometry while operators own most successful speech. This is useful, but it also exposes the next bottleneck: routing alignment, not simply more prototypes. In the latest 500-event window, the field recorded 443 matches, 103 curriculum `primary_off` events, and 82 `geometry_blind` events; 18 speech outcomes were recorded, all successful. Transfer is therefore stronger evidence than smooth wording, while speech coverage still needs to grow.

The system now treats three memories as distinct: the Bitwork pack, append-only ledgers, and session/Cortex state. Folding them into one mutable blob would make curriculum provenance ambiguous. The active `.pwgt` remains human-authorized; this release changes operators, orchestration, evaluation, and governance rather than silently changing weights.

Conversational repairs are regression targets too. “What are you sensing?” must reach operational introspection, while cross-domain prompts such as geometry plus life must preserve a concrete relation and its boundary instead of reusing a stock concept. The local language sidecar keeps the operator’s answer in the foreground; provenance and governance remain inspectable without forcing the same header/footer into every response.

## v0.8.4 native binary language + typed world model (external adapters now opt-in)

The native PERCLNG1 field is the default language surface. The compatibility
path below is disabled unless PERCI_ENABLE_EXTERNAL_LM=1 is explicitly set.

The open-language bottleneck now has a bounded escape hatch instead of a
silent preset: `PERCI_MODEL_URL` connects a local OpenAI-compatible endpoint
directly to the warm CompositeBackend. The adapter supports LM Studio,
llama.cpp/vLLM-style `/v1/chat/completions`, and Ollama `/api/chat` payloads.
It adds a 4-second default timeout, short output budget, Bitwork routing hints,
recent dialogue, and a critic gate. A failed model call or rejected answer
falls back to the existing deterministic path, so enabling a model cannot
remove exact tools, abstention, or weight governance. This is a language
quality path, not evidence of unrestricted intelligence.

---

## Numbers that are true today

| Property | Value |
|----------|------:|
| Software | **v0.8.4** (`Cargo.toml` · badge auto-stamped) |
| Pack format | **PERCIW03** |
| Pack size | **209,710,296** bytes (~200 MiB) |
| Prototypes | **403,163** |
| Concepts | **124** |
| Activation | **4,096** bits · 64 × u64 |
| Expert domains | **16** |
| Hot path | Integer **AND / POPCOUNT** only |
| Native language field | **PERCLNG1** · mmap · binary threshold planes |
| Weights in git | **Cognitive pack local** · native language rebuilt locally |

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
python scripts/release_gates.py
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

## Native binary language training

The default Perci language path is now a Perci-owned PERCLNG1 binary field.
It learns multi-order context transitions with four binary threshold planes
(at least 1, 2, 4, or 8 observations), mmap-loads them in Rust, and generates
bounded continuations with integer-only back-off.

    cargo run --release -- language train --repo
    cargo run --release -- language status
    cargo run --release -- language sample "what is geometry teaching us about life"

The native field is a compact sequence learner, not a claim of frontier-model
breadth. Exact arithmetic and geometry remain deterministic tools. The older
HTTP/command adapters are compatibility paths only and require
PERCI_ENABLE_EXTERNAL_LM=1.

The same rebuild also creates `PERCPHR1`, a bounded word/phrase transition
field. It uses a capped binary vocabulary, order-4 numeric token contexts, and
threshold-coded next-token edges. Perci selects a state-conditioned learned
primer, then composes a continuation through numeric back-off; no response
card is treated as truth. Both artifacts remain local generated weights and
must be rebuilt deliberately from a reviewed corpus.

The rebuild can also create `PERCREL1`, an optional mmap relation field. It
stores hashed prompt-to-response edges and scores native continuations as an
inspectable tie-breaker. Held-out tests currently keep it isolated because the
field does not yet beat the active selector on generalization.

The same rebuild creates `PERCIWM1`, an optional typed world-model field. It
stores bounded subject/relation/object edges plus a coarse domain, polarity,
confidence, and evidence bin. At inference it rewards a candidate that
preserves a learned typed relation from the current prompt; it cannot synthesize
new prose or promote a claim as truth. The field is mmap-loaded and remains
isolated until an adversarial held-out pack shows a real gain.

Native dialogue also carries a fixed 256-bit recurrent state. User and
assistant turns are absorbed with integer rotation/XOR updates, so turn order
changes the next primer and sampling path without creating an unbounded memory
blob or introducing a neural runtime.

The phrase backend now samples six bounded binary continuations and chooses the
one with the best topic binding, recent-response novelty, and topic-neighbor
relation score. The conservative relation weight is tunable with
`PERCI_NATIVE_RELATION_WEIGHT` (default `12`); held-out comparison still
controls weight promotion.

Run the broad native probe and review its evidence:

    python scripts/native_probe.py

It asks 1,000 questions in one persistent process and writes a JSONL transcript
plus summary under `models/candidates/`. The probe is measurement data; it does
not auto-promote its candidate weights.

To compare an isolated phrase candidate against the active field, use a new
tag and pass the candidate path. This leaves the active weights untouched:

    python scripts/native_probe.py --tag v0.8.1-novelty --phrase-weights models/candidates/native-probe-candidate.bphr

To cap repeated training examples before a candidate rebuild:

    python scripts/clean_probe.py models/candidates/native-probe-v0.8.1-novelty-active.jsonl models/candidates/native-probe-v0.8.1-clean.jsonl --limit-per-response 2

To build and run the next emergence curriculum, mine the prior transcript into
counterexamples, perturbations, and unseen-entity transfer questions:

    python scripts/emergence_curriculum.py models/candidates/native-probe-v0.8.1-relation12-active.jsonl models/candidates/emergence-curriculum-v0.8.3.jsonl --count 1000
    python scripts/native_probe.py --tag v0.8.3-emergence-curriculum --questions-file models/candidates/emergence-curriculum-v0.8.3.jsonl

For the next gate, generate adversarial questions that target paraphrase
collapse, negation loss, contradiction handling, entity substitution, and
analogy boundaries. Keep the offset-separated file held out from training:

    python scripts/adversarial_curriculum.py models/candidates/adversarial-v0.8.4.jsonl --count 300
    python scripts/adversarial_curriculum.py models/candidates/adversarial-v0.8.4-heldout.jsonl --count 120 --offset 300
    python scripts/native_probe.py --tag v0.8.4-adversarial --questions-file models/candidates/adversarial-v0.8.4.jsonl

To evaluate a world-model candidate without replacing the active artifact:

    cargo run --release -- language train models/candidates/native-probe-v0.8.3-emergence-curriculum-final.jsonl models/candidates/world-candidate-v0.8.4.blng 6
    python scripts/native_probe.py --tag v0.8.4-world-candidate --questions-file models/candidates/adversarial-v0.8.4-heldout.jsonl --world-weights models/candidates/world-candidate-v0.8.4.bwm

The command emits four native files next to the requested output; only the
`.bwm` path is used by the isolated world-field comparison. Promotion remains
human-authorized and requires no regression in exact tools, abstention, or
topic binding.

## External model compatibility (disabled by default)

Bitwork stays the governor. Perci can now use a local OpenAI-compatible model
as a fast language surface while keeping routing, tools, evidence, memory, and
weight promotion under Perci's control. LM Studio, llama.cpp servers, Ollama,
and Phi-family local endpoints are supported. The model is a renderer, not the
authority layer; failed, empty, overlong, or boundary-violating output falls
back to the governed local path.

```powershell
# LM Studio / llama.cpp / vLLM style endpoint
$env:PERCI_MODEL_URL = "http://127.0.0.1:1234/v1/chat/completions"
$env:PERCI_MODEL_NAME = "phi-4-mini"

# Ollama style endpoint (use the model name you installed)
# $env:PERCI_MODEL_URL = "http://127.0.0.1:11434/api/chat"
# $env:PERCI_MODEL_NAME = "phi4-mini"

$env:PERCI_MODEL_TIMEOUT_MS = "4000"
$env:PERCI_MODEL_MAX_TOKENS = "320"
cargo run --release -- chat
```

The optional command adapter remains available through `PERCI_MODEL_CMD`.
The typed language sidecar is still useful when a process must return
`perci.language-response.v1` explicitly:

```powershell
$env:PERCI_LANGUAGE_SIDECAR = "python scripts/perci_language_sidecar.py"
cargo run --release -- chat
```

The HTTP path is zero-cost when `PERCI_MODEL_URL` is unset. It uses a bounded
local request timeout, sends Bitwork hints as untrusted routing notes, and
tries the deterministic/operator response whenever the model is unavailable.

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
  <sub>PERCI · dark-blood · governed sparse cognition · v0.8.4</sub>
</p>
