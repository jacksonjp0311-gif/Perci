# Perci

**Perci** is a compact, local, governed neuro-symbolic assistant with a Rust CLI, an interactive chat shell, exact reasoning tools, append-only memory, and a real **200 MiB packed cognitive weight file**.

Perci is deliberately honest about its architecture. The bundled model is not a transformer and does not claim large-language-model capability. It is a binary associative Bitwork network that uses sparse 4,096-bit activations, trained expert masks, nearest-prototype retrieval, `AND`, and `POPCOUNT`. Arithmetic and geometry are handled by deterministic solvers rather than guessed by the model.

## What is included

```text
perci/
├── models/
│   ├── perci-cognitive-v0.1.pwgt       # exactly 200 MiB
│   └── perci-cognitive-v0.1.pwgt.json  # manifest + checksums
├── src/
│   ├── cognitive.rs    # weight loader and bit-level inference
│   ├── backend.rs      # built-in model + external model contract
│   ├── chat.rs         # chat orchestration
│   ├── reflex.rs       # 64-bit first-pass router
│   ├── reasoning.rs    # exact arithmetic and geometry
│   ├── memory.rs       # append-only local memory
│   ├── personality.rs  # personality prompt loader
│   ├── lib.rs
│   └── main.rs
├── config/personality.prompt
├── scripts/
│   ├── build_weights.py
│   ├── build_weights_chunked.py
│   ├── test_weights.py
│   └── verify_weights.py
├── training/
│   ├── model-plan.toml
│   ├── heldout-evaluation.txt
│   └── README.md
├── Launch-Perci.ps1
├── WEIGHTS.md
└── Cargo.toml
```

## Run Perci

From PowerShell:

```powershell
cd .\perci
.\Launch-Perci.ps1
```

Or directly with Cargo:

```powershell
cargo run --release -- chat
```

One-shot prompts:

```powershell
cargo run --release -- ask "who are you"
cargo run --release -- ask "calculate 144 divided by 12"
cargo run --release -- ask "triangle area base 8 height 5"
cargo run --release -- ask "how should Lumen Cortex and Bitwork interconnect"
```

Runtime status:

```powershell
cargo run --release -- status
```

## Chat commands

```text
/help       show commands
/status     show runtime status
/prompt     show the active personality prompt
/quit       exit
```

Natural-language tool examples:

```text
calculate 10 divided by 4
triangle area base 8 height 5
pythagorean sides 3 and 4
remember that Perci uses governed local memory
recall governed local memory
```

## The built-in weights

The file `models/perci-cognitive-v0.1.pwgt` contains:

- **403,266** trained associative prototypes
- **4,096 bits** per prompt activation
- **64 `u64` words** per activation
- **16 cognitive domains**
- learned expert masks
- integer-only nearest-prototype inference
- exact extracted size: **209,715,200 bytes (200 MiB)**

The model routes and responds across:

- English and grammar
- logic and structured reasoning
- mathematics
- geometry
- memory operations
- Rust, PowerShell, and software work
- governance and permission boundaries
- planning
- system architecture
- science
- creativity
- comparison
- general conversation

See [WEIGHTS.md](WEIGHTS.md) for the file format, training process, evaluation, checksum, and limitations.

## Verify the weights

```powershell
python .\scripts\verify_weights.py
```

Optional held-out routing test, requiring NumPy:

```powershell
python -m pip install numpy
python .\scripts\test_weights.py
```

## Rebuild the weights

The build is deterministic:

```powershell
python .\scripts\build_weights.py --output .\models\perci-cognitive-v0.1.pwgt
```

The output is exactly 200 MiB. The generated manifest records the SHA-256 checksum, deterministic seed, architecture, and limitations.

## Personality

Edit:

```text
config/personality.prompt
```

The prompt defines Perci as curious, candid, calm, technically exact, local-first, and governed. The prompt influences external language backends and documents the intended behavior of the built-in cognitive model.

## External language model override

A future GGUF or other pretrained model can override the built-in associative backend through `PERCI_MODEL_CMD`:

```powershell
$env:PERCI_MODEL_CMD = "python scripts/mock-model.py"
cargo run --release -- chat
```

The external process receives `SYSTEM`, `MEMORY`, and `USER` sections through standard input and must print one response to standard output.

## Current capability boundary

Perci v0.1 is useful for compact local routing, domain recognition, structured response patterns, exact arithmetic, basic geometry, governed memory, and system-oriented reasoning scaffolds.

It is **not** equivalent to ChatGPT, Qwen, Llama, or another pretrained transformer. It will not possess broad world knowledge, fluid unrestricted prose, or deep novel reasoning merely because the file is 200 MiB. The weight pack is real and operational, but its intelligence is bounded by the synthetic curriculum and the deterministic tools around it.

## Validation status

- Weight file size and SHA-256: verified in this build environment.
- Header, record count, expert table, and prototype ranges: verified by the Python loader.
- Held-out domain probes: 16/16 in the included evaluation run.
- Rust compilation: **not executed in this build environment because Rust/Cargo was unavailable**. Run `cargo test --release` on your machine before treating the Rust runtime as verified.

## License

Perci source and the generated Perci cognitive weights are available under **MIT OR Apache-2.0**.
