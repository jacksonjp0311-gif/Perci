# Perci

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" alt="Perci Dark-Blood mark" width="160" height="160">
  <br>
  <img src="assets/generated/perci-darkblood-badge.svg" alt="Perci version badge (auto-stamped from Cargo.toml)" width="180">
</p>

**Version** is always `Cargo.toml`, stamped into the badge by `build.rs` (never hand-edit the badge). See [`assets/README.md`](assets/README.md).

Open-domain synthesis, multi-hop plans, operator programs, agent lab, and governed Bitwork cognition. Software **v0.6.1** adds transformer-bridge algebra (soft attention, dual residual, VSA, Willshaw concept HVs, session CTX) plus **SoftCascade** multi-hypothesis compose without token decoding. The promoted local pack is `PERCIW03` (~200 MiB, 403,163 prototypes, 124 concepts).

<p align="center">
  <strong>Compact governed local intelligence.</strong><br>
  A Rust-native neuro-symbolic assistant built around fast binary cognition, exact tools, append-only memory, and explicit capability boundaries.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/core-Rust-000000?style=flat-square&logo=rust">
  <img alt="Local first" src="https://img.shields.io/badge/runtime-local--first-2563eb?style=flat-square">
  <img alt="Software" src="https://img.shields.io/badge/software-v0.6.1-8b0000?style=flat-square">
  <img alt="Bitwork pack" src="https://img.shields.io/badge/Bitwork%20PERCIW03-~200%20MiB%20local-111827?style=flat-square">
  <img alt="Inference" src="https://img.shields.io/badge/inference-integer--only-059669?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-d97706?style=flat-square">
</p>

---

## What Perci is

**Perci** is an experimental local assistant with deliberately separate layers:

1. **Reflex routing** for immediate intent recognition.
2. **Packed associative cognition** (Bitwork) for domain selection and nearest-prototype retrieval.
3. **Transformer-bridge algebra** — soft-attention mixture weights, dual residual hops, VSA bind/bundle, Willshaw concept HVs, session CTX bind ([`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md)).
4. **Deterministic reasoning tools** for arithmetic and geometry.
5. **Stateful cognitive operators** for synthesis, critique, trust/systems, and multi-domain connect.
6. **Append-only local memory** + Cortex for governed persistence.
7. **A backend contract** for optional language generation without granting it exact-tool authority.

The Bitwork pack (`PERCIW03`, ~200 MiB) is real and operational, but **local-only** (not in git — GitHub size limits). Software v0.6.0 adds compositional and multi-hop readout on top of that pack without a decoder stack. Perci is not a transformer, does not call an LLM by default, and is not general intelligence.

> Use learned binary cognition to choose the right path, then use inspectable tools to produce results that should not be guessed.

## Why Perci exists

Most assistants place routing, language generation, memory, reasoning, and tool use inside one opaque model boundary. Perci separates them so the system is easier to inspect, benchmark, modify, govern, and extend.

Perci is a **cognitive systems architecture**, not merely a chatbot.

## Architecture

```text
User input
    |
    v
[ 64-bit reflex pass ]
    |
    v
[ Bitwork cognitive weight network ]
    |  expert routing, prototype recall,
    |  soft-attn mixture, residual hops, VSA, CTX
    |
    +-- exact intent -----> [ deterministic tools ]
    |                              |
    +-- language intent --> [ fluid voice / optional external model ]
    |                              |
    +------------------------------+
                   |
                   v
        [ governance + append-only memory + Cortex ]
                   |
                   v
               Response
```

## The cognitive weight pack

**Not shipped in git.** Place or build locally:

```text
models/perci-cognitive-v0.3.pwgt
```

| Property | Value |
|---|---:|
| Exact size (active pack) | `209,710,296` bytes |
| Human-readable size | ~`199.995` MiB |
| Format magic | `PERCIW03` |
| Unique associative prototypes | `403,163` |
| Weight-resident concepts | `124` |
| Activation width | `4,096` bits |
| Packed activation | `64 x u64` |
| Expert domains | `16` |
| Inference arithmetic | Integer-only hot path |

Sidecar metadata: `models/perci-cognitive-v0.3.pwgt.json`. See [`WEIGHTS.md`](WEIGHTS.md).

At inference time Perci:

1. Encodes the prompt into 4,096 bits (bag + structure + VSA + optional CTX).
2. Scores expert masks with `AND` / `POPCOUNT` (top domains only for latency).
3. Finds top-k prototypes; builds soft-attention mixture weights.
4. Runs residual ANDNOT hops (up to 2) and Willshaw concept HV scoring.
5. Chooses operators / tools / fluid speech.
6. Delegates exact work to deterministic tools where available.

See [`docs/BITWORK_V3_EVIDENCE.md`](docs/BITWORK_V3_EVIDENCE.md) and [`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md).

## Cognitive domains

```text
greeting      identity       english        logic
math          geometry       memory         code
governance    planning       explanation    systems
science       creativity     comparison     general
```

## Quick start

### Requirements

- Windows, macOS, or Linux
- Rust toolchain with Cargo
- Python 3 for rebuilding or verifying weights (optional)
- A local `PERCIW03` pack under `models/` (not cloned from GitHub)

### Clone

```powershell
git clone https://github.com/jacksonjp0311-gif/Perci.git
cd .\Perci
```

### Obtain weights (required for full Bitwork)

```powershell
# Build a candidate pack, or copy a promoted pack into models/
python .\scripts\build_weights_v3.py
# Prefer: place models\perci-cognitive-v0.3.pwgt from your authorized build
```

Or set:

```powershell
$env:PERCI_WEIGHTS = "C:\path\to\perci-cognitive-v0.3.pwgt"
```

### Launch on Windows

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
.\Launch-Perci.ps1
```

Or:

```text
Start-Perci.cmd
```

Dark-blood terminal theme. Color follows TTY; override with `PERCI_COLOR=always` or `PERCI_COLOR=never`.

```powershell
.\Launch-Perci.ps1 -Mode intel
.\Launch-Perci.ps1 -Mode status
```

### Launch with Cargo

```powershell
cargo run --release -- chat
```

### One-shot prompts

```powershell
cargo run --release -- ask "who are you"
cargo run --release -- ask "calculate 144 divided by 12"
cargo run --release -- ask "triangle area base 8 height 5"
cargo run --release -- ask "why does trust fail in distributed systems?"
cargo run --release -- ask "connect sparse memory and vector symbolic architectures"
```

### Runtime status

```powershell
cargo run --release -- status
```

## Chat commands

```text
/help       show available commands
/status     inspect runtime and backend state
/learning   inspect adaptive dialogue state and pending evidence
/intel      show live labels, margins, z-scores, Jaccard similarity
/trace      last deliberation / operator program audit
/prompt     show the active personality prompt
/quit       exit Perci
```

Perci learns safe conversational preferences immediately and records bounded interactions as pending evidence. It does **not** silently promote facts or rewrite weights. See [`docs/INTERACTION_LEARNING.md`](docs/INTERACTION_LEARNING.md).

### Evolve measurable capability

```powershell
python .\scripts\evolve_cycle.py
Get-Content .\docs\CAPABILITY_SCORECARD.md
```

Promotion remains explicit and human-authorized. See [`docs/EVOLUTION.md`](docs/EVOLUTION.md).

Natural-language tool examples:

```text
calculate 10 divided by 4
triangle area base 8 height 5
pythagorean sides 3 and 4
remember that Perci uses governed local memory
recall governed local memory
```

## Exact reasoning

When a request is arithmetic or geometric, Perci routes with Bitwork then computes with deterministic code — not approximate language patterns.

- repeatable outputs
- inspectable computation
- clear boundary between classification and calculation

## Governed local memory + Cortex

Append-only JSONL memory is separate from the weight pack. Cortex provides repository assimilation, selective retrieval, sparse activation, and provenance. Cortex **never** grants mutation authority.

Initialize once per checkout:

```powershell
powershell -ExecutionPolicy Bypass -File .\Initialize-Perci-Cortex.ps1
```

Ordinary chat receives bounded Cortex evidence when attached. `remember that ...` is explicit-only. See [`docs/CORTEX_INTEGRATION.md`](docs/CORTEX_INTEGRATION.md) and [`docs/INTELLIGENCE_PACKS.md`](docs/INTELLIGENCE_PACKS.md).

## Personality

```text
config/personality.prompt
```

Default personality: curious, candid, calm, technically exact, local-first, and governed.

## External language backend

Optional external generation via `PERCI_MODEL_CMD` while Perci keeps orchestration, memory, tools, and governance:

```powershell
$env:PERCI_MODEL_CMD = "python scripts/mock-model.py"
cargo run --release -- chat
```

The process receives structured `SYSTEM` / `MEMORY` / `USER` on stdin and prints one response on stdout.

```text
optional quantized LM
        |
        v
Perci personality + retrieved memory
        |
        v
Bitwork routing + governance
        |
        v
exact tools + fluid voice
        |
        v
validated response
```

## Verify and rebuild weights

```powershell
python .\scripts\verify_weights.py
python -m pip install numpy
python .\scripts\test_weights.py
```

```powershell
python .\scripts\build_weights_v3.py
# lower-memory path:
python .\scripts\build_weights_chunked.py
```

Routing probes are focused associative tests — not claims of ChatGPT parity. See [`VALIDATION.md`](VALIDATION.md).

## Repository map

```text
perci/
  assets/                 # dark-blood mark + generated badge
  config/personality.prompt
  docs/                   # math path, transformer bridge, evolution, evidence
  knowledge/packs/        # intelligence packs for Cortex
  models/                 # *.pwgt local only; *.pwgt.json metadata in git
  scripts/                # build, verify, hardness, evolve
  src/
    cognitive.rs          # Bitwork encode / classify / mixture / residual / VSA
    deliberation.rs       # high-salience operators
    operator_program.rs   # program + critic runtime
    voice.rs / ui.rs      # fluid speech + dark-blood CLI
    backend.rs / chat.rs  # orchestration
    reasoning.rs          # exact math / geometry
    cortex.rs / memory.rs # Cortex + JSONL memory
    agent.rs / learning.rs
    main.rs / lib.rs
  training/               # curriculum, hardness, held-out
  Launch-Perci.ps1
  Start-Perci.cmd
  Cargo.toml
  WEIGHTS.md
  VALIDATION.md
```

## Capability boundary

**Useful for:**

- compact local cognitive routing
- multi-hypothesis mixture + residual second thoughts
- compositional role structure (VSA) without a decoder
- exact foundational arithmetic and geometry
- multi-domain connect / trust-systems operators
- governed local memory and Cortex retrieval
- hybrid experiments (Bitwork + optional external LM)

**Not equivalent to** ChatGPT, Qwen, Llama, Phi, or other pretrained transformers. No web-scale factual knowledge, unrestricted fluent generation, or deep general reasoning merely because the pack is ~200 MiB.

Progress is measured by held-out tasks, hardness gates, latency, binding quality, and baselines — not file size alone.

## Design principles

- **Local first** — core operation does not require a cloud service.
- **Governed persistence** — memory writes are explicit and inspectable.
- **Honest architecture** — capabilities and limits are stated directly.
- **Deterministic where possible** — exact tools handle exact work.
- **Composable cognition** — routing, memory, language, and tools remain separable.
- **Reproducible weights** — the pack can be rebuilt and verified (locally).
- **Fast reflexes** — packed binary ops keep the cognitive path lightweight.

## Roadmap

- pack-side VSA encode under human-authorized rebuild
- prototype graph / spreading activation
- stronger hardness and dialogue gates
- richer governed memory retrieval
- formal external GGUF/backend adapter
- keep Bitwork as the fast, inspectable governance boundary

## Status

Experimental research software. Review [`VALIDATION.md`](VALIDATION.md) before treating a runtime or benchmark claim as verified.

## License

Source and generated cognitive weights (when you build them) are available under either:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

You may use either license at your option.
