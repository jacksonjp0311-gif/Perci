# Perci

<p align="center">
  <img src="assets/icons/perci-darkblood-mark.jpg" alt="Perci Dark-Blood mark" width="160" height="160">
  <br>
  <img src="assets/generated/perci-darkblood-badge.svg" alt="Perci version badge (auto-stamped from Cargo.toml)" width="180">
</p>

**Version** is always `Cargo.toml` → stamped into the badge by `build.rs` (never hand-edit the badge). See `assets/README.md`.

Open-domain synthesis, multi-hop plans, operator programs, agent lab, and governed Bitwork cognition. The promoted `PERCIW03` artifact is a ~200 MiB Bitwork pack with 403,163 prototypes and 124 weight-resident concepts.

<p align="center">
  <strong>Compact governed local intelligence.</strong><br>
  A Rust-native neuro-symbolic assistant built around fast binary cognition, exact tools, append-only memory, and explicit capability boundaries.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/core-Rust-000000?style=flat-square&logo=rust">
  <img alt="Local first" src="https://img.shields.io/badge/runtime-local--first-2563eb?style=flat-square">
  <img alt="Software" src="https://img.shields.io/badge/software-v0.6.0-8b0000?style=flat-square">
  <img alt="Bitwork pack" src="https://img.shields.io/badge/Bitwork%20PERCIW03-~200%20MiB%20local-111827?style=flat-square">
  <img alt="Inference" src="https://img.shields.io/badge/inference-integer--only-059669?style=flat-square">
  <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-d97706?style=flat-square">
</p>

---

## What Perci is

**Perci** is an experimental local assistant that combines deliberately separate layers:

1. **Reflex routing** for immediate intent recognition.
2. **Packed associative cognition** (Bitwork) for domain selection and nearest-prototype retrieval.
3. **Transformer-bridge algebra** — soft-attention mixture weights, dual residual hops, VSA bind/bundle, Willshaw concept HVs, session CTX bind (see [`docs/TRANSFORMER_BRIDGE.md`](docs/TRANSFORMER_BRIDGE.md)).
4. **Deterministic reasoning tools** for arithmetic and geometry.
5. **Stateful cognitive operators** for synthesis, critique, trust/systems, and multi-domain connect.
6. **Append-only local memory** + Cortex for governed persistence.
7. **A backend contract** for optional language generation without granting it exact-tool authority.

The promoted Bitwork pack (`PERCIW03`, ~200 MiB) is real, operational, and
**local-only** (not in git — GitHub size limits). Software **v0.6.0** adds
compositional and multi-hop readout on top of that pack without a decoder stack.
It is not a transformer, does not call an LLM, and is not general intelligence.

Perci is designed around a simpler principle:

> Use learned binary cognition to choose the right path, then use inspectable tools to produce results that should not be guessed.

## Why Perci exists

Most assistants place routing, language generation, memory, reasoning, and tool use inside one opaque model boundary. Perci separates them.

That separation makes the system easier to inspect, benchmark, modify, govern, and extend. A compact associative layer can remain extremely fast while exact solvers handle calculation and an optional language backend handles fluid prose.

Perci is therefore best understood as a **cognitive systems architecture**, not merely a chatbot.

## Architecture

```text
                         Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
User input Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€“Â¶Ã¢â€â€š  64-bit reflex pass Ã¢â€â€š
                         Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
                                    Ã¢â€â€š
                                    Ã¢â€“Â¼
                         Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
                         Ã¢â€â€š Bitwork cognitive    Ã¢â€â€š
                         Ã¢â€â€š weight network       Ã¢â€â€š
                         Ã¢â€â€š                      Ã¢â€â€š
                         Ã¢â€â€š Ã¢â‚¬Â¢ expert routing     Ã¢â€â€š
                         Ã¢â€â€š Ã¢â‚¬Â¢ prototype recall   Ã¢â€â€š
                         Ã¢â€â€š Ã¢â‚¬Â¢ response selection Ã¢â€â€š
                         Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
                                 Ã¢â€â€š      Ã¢â€â€š
                    exact intent Ã¢â€â€š      Ã¢â€â€š language intent
                                 Ã¢â€“Â¼      Ã¢â€“Â¼
                    Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â  Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
                    Ã¢â€â€š Deterministic  Ã¢â€â€š  Ã¢â€â€š Built-in response  Ã¢â€â€š
                    Ã¢â€â€š reasoning      Ã¢â€â€š  Ã¢â€â€š or external model  Ã¢â€â€š
                    Ã¢â€â€š tools          Ã¢â€â€š  Ã¢â€â€š backend             Ã¢â€â€š
                    Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ  Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
                            Ã¢â€â€š                      Ã¢â€â€š
                            Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
                                       Ã¢â€“Â¼
                           Ã¢â€Å’Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â
                           Ã¢â€â€š Governance + local   Ã¢â€â€š
                           Ã¢â€â€š append-only memory   Ã¢â€â€š
                           Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Â¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€Ëœ
                                      Ã¢â€“Â¼
                                  Response
```

## The cognitive weight pack

The bundled file is:

```text
models/perci-cognitive-v0.3.pwgt
```

| Property | Value |
|---|---:|
| Exact size | `200,425,696` bytes |
| Human-readable size | `191.14 MiB` |
| Format magic | `PERCIW03` |
| Unique associative prototypes | `385,308` |
| Weight-resident concepts | `100` |
| Activation width | `4,096 bits` |
| Packed activation | `64 Ãƒâ€” u64` |
| Expert domains | `16` |
| Inference arithmetic | Integer-only |

Each prompt is encoded into a sparse distributed binary activation using normalized lexical features, adjacent word pairs, prefixes, suffixes, character trigrams, prompt length, and deterministic hashing.

At inference time Perci:

1. Encodes the prompt into 4,096 bits.
2. Scores positive and negative expert masks using `AND` and `POPCOUNT`.
3. Measures top-two margin, query density, Jaccard similarity, and chance-normalized overlap.
4. Finds the nearest packed prototype.
5. Chooses a response path.
6. Delegates exact work to deterministic tools where available.

Prototype search remains integer-only; floating point is used only for final normalized confidence telemetry and abstention.

See [`docs/BITWORK_V3_EVIDENCE.md`](docs/BITWORK_V3_EVIDENCE.md) for the
promoted v3 format, evaluation receipts, limitations, and promotion contract.
The v2 and v1 packs remain readable fallbacks.

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

The interactive shell uses Perci's dark-blood terminal theme. Color follows TTY
detection; set `PERCI_COLOR=always` or `PERCI_COLOR=never` to override it.

Run the transparent live intelligence probe without entering chat:

```powershell
.\Launch-Perci.ps1 -Mode intel
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
/learning   inspect adaptive dialogue state and pending evidence
/intel      show live labels, margins, z-scores, and Jaccard similarity
/prompt     show the active personality prompt
/quit       exit Perci
```

Perci learns safe conversational preferences immediately and records every
bounded interaction as pending evidence. It does not silently promote facts or
rewrite weights. See [`docs/INTERACTION_LEARNING.md`](docs/INTERACTION_LEARNING.md).

### Evolve measurable capability

```powershell
python .\scripts\evolve_cycle.py
Get-Content .\docs\CAPABILITY_SCORECARD.md
```

This stages interaction evidence, runs the hardness pack, and writes a capability
scorecard. Promotion remains explicit and human-authorized. See
[`docs/EVOLUTION.md`](docs/EVOLUTION.md).

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
          Ã¢â€ â€œ
Perci personality + retrieved memory
          Ã¢â€ â€œ
Bitwork routing and governance
          Ã¢â€ â€œ
exact reasoning and tools
          Ã¢â€ â€œ
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

The recorded evaluation routed all **16 of 16 domain probes** to their expected expert. This is a focused routing and associative-retrieval testÃ¢â‚¬â€not a benchmark of broad factual knowledge, general reasoning, or language-model parity.

See [`VALIDATION.md`](VALIDATION.md) for the exact validation record and its interpretation.

## Rebuild the weights

The weight build is deterministic:

```powershell
python .\scripts\build_weights.py `
  --output .\models\candidates\perci-cognitive-v0.2.pwgt
```

The generated manifest records the architecture, deterministic seed, SHA-256 checksum, exact size, and declared limitations.

For lower-memory build environments, use:

```powershell
python .\scripts\build_weights_chunked.py
```

## Repository map

```text
perci/
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ models/
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ perci-cognitive-v0.1.pwgt
Ã¢â€â€š   Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬ perci-cognitive-v0.1.pwgt.json
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ src/
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ cognitive.rs     # packed weight loading and inference
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ backend.rs       # built-in and external backend contract
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ chat.rs          # interactive orchestration
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ reflex.rs        # fast first-pass intent routing
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ reasoning.rs     # deterministic arithmetic and geometry
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ memory.rs        # append-only local persistence
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ personality.rs   # behavioral prompt loading
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ lib.rs
Ã¢â€â€š   Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬ main.rs
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ config/
Ã¢â€â€š   Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬ personality.prompt
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ scripts/
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ build_weights.py
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ build_weights_chunked.py
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ mock-model.py
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ test_weights.py
Ã¢â€â€š   Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬ verify_weights.py
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ training/
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ from-lumen/
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ heldout-evaluation.txt
Ã¢â€â€š   Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ model-plan.toml
Ã¢â€â€š   Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬ README.md
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ Launch-Perci.ps1
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ VALIDATION.md
Ã¢â€Å“Ã¢â€â‚¬Ã¢â€â‚¬ WEIGHTS.md
Ã¢â€â€Ã¢â€â‚¬Ã¢â€â‚¬ Cargo.toml
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

Progress should be measured through held-out tasks, latency, memory use, user outcomes, failure analysis, and comparison against simpler baselinesÃ¢â‚¬â€not file size alone.

## Design principles

- **Local first** Ã¢â‚¬â€ core operation does not require a cloud service.
- **Governed persistence** Ã¢â‚¬â€ memory writes are explicit and inspectable.
- **Honest architecture** Ã¢â‚¬â€ capabilities and limitations are stated directly.
- **Deterministic where possible** Ã¢â‚¬â€ exact tools handle exact work.
- **Composable cognition** Ã¢â‚¬â€ routing, memory, language, and tools remain separable.
- **Reproducible weights** Ã¢â‚¬â€ the cognitive pack can be rebuilt and verified.
- **Fast reflexes** Ã¢â‚¬â€ packed binary operations keep the cognitive path lightweight.

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
## Cortex memory integration

Perci v0.1.1 includes a vendored Cortex engine as its governed long-term
repository memory. Perci remains the cognitive coordinator; Cortex performs
repository assimilation, selective retrieval, sparse activation, provenance,
and Governor trust reduction.

Initialize it once per checkout:

```powershell
powershell -ExecutionPolicy Bypass -File .\Initialize-Perci-Cortex.ps1
```

Then launch without PowerShell execution-policy friction:

```text
Start-Perci.cmd
```

Ordinary chat receives bounded Cortex evidence packets. `remember that ...`
remains explicit-only and records append-only JSONL memory plus a Cortex
episodic event when Cortex is ready. Cortex context never grants mutation
authority. See [`docs/CORTEX_INTEGRATION.md`](docs/CORTEX_INTEGRATION.md).
## Perci v0.1.2: Intelligence Packs and Fast Cortex

Perci now indexes a curated procedural intelligence pack through Cortex. The
pack encodes observable reasoning behavior: intent resolution, decomposition,
strategy selection, evidence hierarchy, uncertainty calibration,
counterexamples, debugging, verification, architecture, mathematics, science,
memory consolidation, communication, and governance.

Performance changes:

- the 200 MiB Bitwork file is memory-mapped instead of copied into a Rust vector;
- greetings and trivial prompts bypass Cortex;
- the first substantive request lazily starts a persistent Cortex Python daemon;
- later requests reuse the warm SQLite/Cortex process;
- repeated retrievals use a bounded TTL cache;
- `Start-Perci.cmd` executes the compiled release binary directly;
- the console output is ASCII-safe;
- `/bench` measures fast, cold-Cortex, and cached-Cortex paths.

See [`docs/INTELLIGENCE_PACKS.md`](docs/INTELLIGENCE_PACKS.md).
