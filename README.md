# Perci

<p align="center">
  <strong>Compact governed local intelligence.</strong><br>
  A Rust-native neuro-symbolic assistant built around fast binary cognition, exact tools, append-only memory, and explicit capability boundaries.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/core-Rust-000000?style=flat-square&logo=rust">
  <img alt="Local first" src="https://img.shields.io/badge/runtime-local--first-2563eb?style=flat-square">
  <img alt="Cognitive weights" src="https://img.shields.io/badge/cognitive%20weights-200%20MiB-7c3aed?style=flat-square">
  <img alt="Inference" src="https://img.shields.io/badge/inference-integer--only-059669?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-d97706?style=flat-square">
</p>

---

## What Perci is

**Perci** is an experimental local assistant that combines five deliberately separate layers:

1. **Reflex routing** for immediate intent recognition.
2. **Packed associative cognition** for domain selection and nearest-prototype retrieval.
3. **Deterministic reasoning tools** for arithmetic and geometry.
4. **Append-only local memory** for governed persistence.
5. **A backend contract** for optionally attaching a stronger language model later.

The bundled cognitive model is real, operational, deterministic, and exactly **200 MiB**. It is not a transformer, not a compressed copy of a frontier model, and not presented as general intelligence.

Perci is designed around a simpler principle:

> Use learned binary cognition to choose the right path, then use inspectable tools to produce results that should not be guessed.

## Why Perci exists

Most assistants place routing, language generation, memory, reasoning, and tool use inside one opaque model boundary. Perci separates them.

That separation makes the system easier to inspect, benchmark, modify, govern, and extend. A compact associative layer can remain extremely fast while exact solvers handle calculation and an optional language backend handles fluid prose.

Perci is therefore best understood as a **cognitive systems architecture**, not merely a chatbot.

## Architecture

```text
                         ┌──────────────────────┐
User input ─────────────▶│  64-bit reflex pass │
                         └──────────┬───────────┘
                                    │
                                    ▼
                         ┌──────────────────────┐
                         │ Bitwork cognitive    │
                         │ weight network       │
                         │                      │
                         │ • expert routing     │
                         │ • prototype recall   │
                         │ • response selection │
                         └───────┬──────┬───────┘
                                 │      │
                    exact intent │      │ language intent
                                 ▼      ▼
                    ┌────────────────┐  ┌────────────────────┐
                    │ Deterministic  │  │ Built-in response  │
                    │ reasoning      │  │ or external model  │
                    │ tools          │  │ backend             │
                    └───────┬────────┘  └──────────┬─────────┘
                            │                      │
                            └──────────┬───────────┘
                                       ▼
                           ┌──────────────────────┐
                           │ Governance + local   │
                           │ append-only memory   │
                           └──────────┬───────────┘
                                      ▼
                                  Response
```

## The cognitive weight pack

The bundled file is:

```text
models/perci-cognitive-v0.1.pwgt
```

| Property | Value |
|---|---:|
| Exact size | `209,715,200` bytes |
| Human-readable size | `200 MiB` |
| Format magic | `PERCIW01` |
| Associative prototypes | `403,266` |
| Activation width | `4,096 bits` |
| Packed activation | `64 × u64` |
| Expert domains | `16` |
| Inference arithmetic | Integer-only |

Each prompt is encoded into a sparse distributed binary activation using normalized lexical features, adjacent word pairs, prefixes, suffixes, character trigrams, prompt length, and deterministic hashing.

At inference time Perci:

1. Encodes the prompt into 4,096 bits.
2. Scores learned expert masks using `AND` and `POPCOUNT`.
3. Selects the strongest expert partitions.
4. Finds the nearest packed prototype.
5. Chooses a response path.
6. Delegates exact work to deterministic tools where available.

No floating point is used in the associative weight-inference path.

See [`WEIGHTS.md`](WEIGHTS.md) for the file format, curriculum, inference process, evaluation scope, checksum record, limitations, and upgrade path.

## Cognitive domains

```text
greeting      identity       english        logic
math          geometry       memory         code
governance    planning       explanation    systems
science       creativity     comparison     general
```

The included curriculum covers grammar, structured reasoning, mathematics, geometry, memory intent, Rust, PowerShell, software architecture, planning, science, governance boundaries, and system-oriented prompts.

## Quick start

### Requirements

- Windows, macOS, or Linux
- Rust toolchain with Cargo
- Git LFS when cloning the bundled weight file
- Python 3 only for rebuilding or independently verifying the weights

### Clone

```powershell
git lfs install
git clone https://github.com/jacksonjp0311-gif/Perci.git
cd .\Perci
```

### Launch on Windows

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
.\Launch-Perci.ps1
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
cargo run --release -- ask "how should Lumen Cortex and Bitwork interconnect"
```

### Runtime status

```powershell
cargo run --release -- status
```

## Chat commands

```text
/help       show available commands
/status     inspect runtime and backend state
/prompt     show the active personality prompt
/quit       exit Perci
```

Natural-language tool examples:

```text
calculate 10 divided by 4
triangle area base 8 height 5
pythagorean sides 3 and 4
remember that Perci uses governed local memory
recall governed local memory
```

## Exact reasoning instead of guessed arithmetic

Perci's cognitive weights recognize when a request belongs to a mathematical or geometric domain. The final calculation is then performed by deterministic code rather than retrieved as an approximate language pattern.

This provides:

- repeatable outputs
- inspectable computation
- a clean boundary between classification and calculation
- less pressure to memorize arithmetic inside the model
- easier failure analysis and testing

The current exact-tool layer includes foundational arithmetic and geometry operations and is intended to expand over time.

## Governed local memory

Perci includes append-only local memory for explicit remember-and-recall operations. Memory is separated from the cognitive weight file so that runtime experience does not silently mutate the base model.

This creates a clear distinction between:

- immutable packaged cognition
- user-authorized persistent memory
- transient session context
- future retraining data

That boundary is central to Perci's local-first governance model.

## Personality

Perci's behavioral specification lives at:

```text
config/personality.prompt
```

The default personality is curious, candid, calm, technically exact, local-first, and governed.

The prompt directly informs external language backends and documents the intended behavior of the built-in system. It does not magically grant capabilities that the underlying backend does not possess.

## External language backend

Perci can delegate language generation to an external process through `PERCI_MODEL_CMD` while retaining its local orchestration, memory, tools, and governance layers.

```powershell
$env:PERCI_MODEL_CMD = "python scripts/mock-model.py"
cargo run --release -- chat
```

The external process receives structured `SYSTEM`, `MEMORY`, and `USER` sections through standard input and prints one response to standard output.

This contract creates a path toward architectures such as:

```text
quantized language model
          ↓
Perci personality + retrieved memory
          ↓
Bitwork routing and governance
          ↓
exact reasoning and tools
          ↓
validated response
```

The Bitwork layer can remain useful as a fast reflex, domain classifier, memory selector, and governance gate even when a stronger language core is attached.

## Verify the bundled weights

Verify size, structure, and checksum:

```powershell
python .\scripts\verify_weights.py
```

Run the optional held-out routing probes:

```powershell
python -m pip install numpy
python .\scripts\test_weights.py
```

The recorded evaluation routed all **16 of 16 domain probes** to their expected expert. This is a focused routing and associative-retrieval test—not a benchmark of broad factual knowledge, general reasoning, or language-model parity.

See [`VALIDATION.md`](VALIDATION.md) for the exact validation record and its interpretation.

## Rebuild the weights

The weight build is deterministic:

```powershell
python .\scripts\build_weights.py `
  --output .\models\perci-cognitive-v0.1.pwgt
```

The generated manifest records the architecture, deterministic seed, SHA-256 checksum, exact size, and declared limitations.

For lower-memory build environments, use:

```powershell
python .\scripts\build_weights_chunked.py
```

## Repository map

```text
perci/
├── models/
│   ├── perci-cognitive-v0.1.pwgt
│   └── perci-cognitive-v0.1.pwgt.json
├── src/
│   ├── cognitive.rs     # packed weight loading and inference
│   ├── backend.rs       # built-in and external backend contract
│   ├── chat.rs          # interactive orchestration
│   ├── reflex.rs        # fast first-pass intent routing
│   ├── reasoning.rs     # deterministic arithmetic and geometry
│   ├── memory.rs        # append-only local persistence
│   ├── personality.rs   # behavioral prompt loading
│   ├── lib.rs
│   └── main.rs
├── config/
│   └── personality.prompt
├── scripts/
│   ├── build_weights.py
│   ├── build_weights_chunked.py
│   ├── mock-model.py
│   ├── test_weights.py
│   └── verify_weights.py
├── training/
│   ├── from-lumen/
│   ├── heldout-evaluation.txt
│   ├── model-plan.toml
│   └── README.md
├── Launch-Perci.ps1
├── VALIDATION.md
├── WEIGHTS.md
└── Cargo.toml
```

## Capability boundary

Perci v0.1 is useful for:

- extremely compact local cognitive routing
- domain and intent recognition
- nearest-prototype associative retrieval
- structured response scaffolds
- exact foundational arithmetic
- basic geometry
- governed local memory
- system architecture prompts
- experimentation with hybrid cognitive systems

Perci v0.1 is **not** equivalent to ChatGPT, Qwen, Llama, Phi, or another pretrained transformer. It does not contain web-scale factual knowledge, unrestricted fluent generation, or deep general reasoning merely because its weight file is 200 MiB.

The weight pack is meaningful because it contains hundreds of thousands of packed cognitive states. Its intelligence is still bounded by its curriculum, retrieval architecture, response templates, tools, and evaluation coverage.

Progress should be measured through held-out tasks, latency, memory use, user outcomes, failure analysis, and comparison against simpler baselines—not file size alone.

## Design principles

- **Local first** — core operation does not require a cloud service.
- **Governed persistence** — memory writes are explicit and inspectable.
- **Honest architecture** — capabilities and limitations are stated directly.
- **Deterministic where possible** — exact tools handle exact work.
- **Composable cognition** — routing, memory, language, and tools remain separable.
- **Reproducible weights** — the cognitive pack can be rebuilt and verified.
- **Fast reflexes** — packed binary operations keep the cognitive path lightweight.

## Roadmap

- expand exact reasoning and symbolic tool coverage
- add stronger held-out evaluations and latency benchmarks
- improve associative response composition
- add richer governed memory retrieval
- formalize the external GGUF/backend adapter
- integrate Perci with Lumen and Cortex through explicit contracts
- preserve the Bitwork layer as a fast cognitive reflex and governance boundary

## Status

Perci is experimental research software. The packaged cognitive weights, manifest, Python verifier, held-out routing probes, Rust source, and external backend contract are included for inspection and reproduction.

Review [`VALIDATION.md`](VALIDATION.md) before treating a specific runtime or benchmark claim as verified.

## License

Perci source code and the generated Perci cognitive weights are available under either:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

You may use either license at your option.
