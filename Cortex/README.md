<p align="center">
  <img src="assets/cortex-neural-brain.png" alt="Cortex neural interlink brain" width="100%" />
</p>

<p align="center">
  <a href="https://github.com/jacksonjp0311-gif/Cortex/actions"><img src="https://img.shields.io/badge/verification-tested-22c55e?style=for-the-badge" alt="Tests verified" /></a>
  <img src="https://img.shields.io/badge/routing-Thalamus-8b5cf6?style=for-the-badge" alt="Thalamus routing" />
  <img src="https://img.shields.io/badge/storage-local--first-111827?style=for-the-badge" alt="Local first" />
  <img src="https://img.shields.io/badge/authority-recommend--only-f8fafc?style=for-the-badge&labelColor=111827" alt="Recommend only" />
</p>

# Cortex Neural Interlink

**Verified repository assimilation, selective memory, environment learning, and sparse neural interlinking for AI agent systems.**

Cortex is a portable local memory organ for coding agents. Drop the folder into or beside a repository, run one PowerShell or Bash command, and Cortex inventories the repository, learns its development environment, indexes supported content, extracts relationships, imports bounded Git telemetry, compiles those relationships into a sparse neural interlink, validates retrieval, installs repository-local agent instructions, and issues a bootstrap certificate.

After bootstrap, an agent does not need to rediscover the whole environment on every task. Cortex checks for drift, refreshes only changed surfaces, retrieves task-relevant evidence, propagates activation through bounded structural associations, and emits a compact context packet with provenance.

```text
FIRST RUN — VERIFIED ASSIMILATION
repository
  -> inventory and classification
  -> environment learning
  -> content indexing and embeddings
  -> symbol and relationship extraction
  -> Git telemetry
  -> sparse neural interlink compilation
  -> retrieval probes and verification
  -> bootstrap certificate

LATER RUNS — SELECTIVE NEURAL RECALL
current task
  -> manifest drift check
  -> incremental refresh when required
  -> lexical + semantic retrieval
  -> deterministic sparse spreading activation
  -> structural support-path expansion
  -> Governor trust control
  -> bounded evidence packet
  -> agent work
  -> episodic events
  -> Discovery Card consolidation
```

Cortex is local-first, SQLite-backed, dependency-free in its core installation, and designed to integrate with existing repositories without replacing their source, tests, governance, or authorization rules.

## Thalamus routing

Every normal activation now passes through a local, deterministic Thalamus route plan before retrieval. The plan classifies the task, allocates attention across source, tests, structure, documentation, Git, runtime, and other memory lanes, and records numerical inhibition for generated, duplicate, or out-of-scope evidence. It is an engineering routing analogy—not a biological model—and it cannot grant mutation authority. Inspect a plan with `cortex thalamus --repo <name> --task "<task>" --json`.

Run the reproducible before/after routing benchmark with `python benchmarks/thalamus_before_after.py --files 250 --runs 5`. Its committed chart and raw results are in [`benchmarks/results/`](benchmarks/results/).

Run `python -m cortex self-test --json` to clone Cortex as a host, place a second Cortex clone inside it as the active engine, and verify that the nested engine is excluded while the real outer Cortex repository bootstraps and activates.

See [cross-domain analysis](docs/CROSS_DOMAIN_ANALYSIS.md) for the evidence-informed attention/inhibition analogy and the computational-work telemetry reported with every context packet.

Use `cortex thalamus-feedback --repo <repository> --memory-id <id> --outcome helpful --json` to record bounded evidence feedback. Feedback adjusts only future routing priority; it never alters source truth or mutation authority.

## What changed in the neural edition

The previous standalone `neuron` repository has been integrated as an internal Cortex organ rather than kept as a competing system.

Cortex remains responsible for:

- repository identity and assimilation;
- semantic, structural, temporal, and episodic memory;
- provenance and retrieval;
- working sessions and Discovery Card consolidation;
- trust reduction through the Governor;
- NexusGate packet production;
- the authority boundary.

The internal neural interlink adds:

- file-level neural nodes compiled from indexed repository surfaces;
- bounded synapses compiled from imports, resolved references, tests, documentation, calls, and co-change history;
- deterministic sparse activation seeded by hybrid retrieval;
- bounded support-path expansion;
- optional bounded Hebbian association strengthening;
- a hash-chained neural event ledger;
- replayable activation packets and state hashes.

There is one database, one episodic path, one consolidation path, and one authority boundary. The neural layer does not maintain a second memory store.

## Why this matters

A coding agent usually faces two inefficient choices:

1. load too much repository context and lose reasoning quality to token pressure; or
2. load too little and repeatedly rediscover architecture, commands, history, and prior decisions.

Cortex separates repository availability from prompt loading:

- the supported repository is assimilated once;
- every chunk retains path, line range, content hash, type, and metadata;
- unsupported, unreadable, binary, oversized, and unresolved surfaces remain visible;
- the environment profile records likely commands, ecosystems, frameworks, and entrypoints;
- structural and temporal relationships become reusable associations;
- only a sparse, task-relevant subset is activated and loaded;
- the AI receives evidence instead of an ungrounded recollection.

## Biological efficiency model

The terminology is an engineering analogy. Cortex does not claim biological fidelity, consciousness, or AGI.

| Component | Engineering role |
|---|---|
| Hippocampus | Active task focus and append-only episodic events |
| Durable cortex | Semantic, structural, temporal, and consolidated memory |
| Neural nodes | Indexed repository files and evidence surfaces |
| Synapses | Bounded structural and temporal associations |
| Sparse activation | Task-triggered selection and limited propagation |
| Plasticity | Bounded strengthening of repeatedly co-activated associations |
| Bridge | Deterministic consolidation into Discovery Cards |
| Governor | Negative feedback that narrows or blocks trust when memory drifts |
| Homeostasis | Manifest, database, integration, coverage, ledger, and retrieval verification |

The efficiency objective is not to simulate every neuron. It is to avoid scanning and loading every stored surface for every task.

## Single-substrate architecture

```text
AI agent / Codex / NexusGate
            |
            v
Cortex activation and Governor
            |
            +--> learned environment profile
            |
            +--> hybrid semantic retrieval
            |
            +--> sparse neural interlink
            |       nodes = indexed repository files
            |       synapses = existing graph relationships
            |       plasticity = bounded internal association updates
            |
            +--> bounded context packet with provenance
            |
            v
SQLite cortex.db
  repositories, files, memories, FTS5, vectors
  symbols, edges, Git telemetry
  sessions, events, Discovery Cards
  environment profiles
  neural nodes, synapses, activations, ledger
```

## Requirements

- Python 3.10 or newer
- SQLite with FTS5, included in normal Python distributions
- Git is optional but recommended for temporal and co-change telemetry
- Windows PowerShell 5.1+ or PowerShell 7
- Bash on Linux, macOS, WSL, or Git Bash

No API key, network service, vector server, or model download is required for the core system.

## Fastest setup: drop in and run

### Windows PowerShell

Place this Cortex folder inside the repository you want to integrate, or keep it beside the repository and pass a path.

When the folder is nested inside a host repository, Cortex automatically excludes its own engine directory from assimilation.

From the Cortex folder:

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
.\Cortex-All-One.ps1
```

With an explicit target:

```powershell
.\Cortex-All-One.ps1 `
    -RepositoryPath "C:\path\to\AgentRepository" `
    -Name "AgentRepository" `
    -Task "Map the architecture and prepare the first bounded context packet" `
    -RunTests
```

The all-one flow performs:

```text
virtual environment
-> portable engine binding with no package install required
-> database initialization
-> optional test suite
-> repository bootstrap
-> environment learning
-> neural interlink compilation
-> certificate verification
-> doctor checks
-> first activation
```

### Bash

```bash
chmod +x cortex-all-one.sh scripts/bash/*.sh
./cortex-all-one.sh
```

With an explicit target:

```bash
./cortex-all-one.sh \
  --repository-path /path/to/AgentRepository \
  --name AgentRepository \
  --task "Map the architecture and prepare the first bounded context packet" \
  --run-tests
```

## Install the engine without bootstrapping a target

### PowerShell

```powershell
.\scripts\powershell\Install-Cortex.ps1
```

### Bash

```bash
./scripts/bash/install-cortex.sh
```

Manual equivalent:

```bash
python -m venv .venv
# Windows: .venv\Scripts\activate
# Bash: source .venv/bin/activate
python -m pip install -e .
python -m cortex init --json
python -m cortex doctor --json
```

## Bootstrap a repository

```powershell
.\scripts\powershell\Bootstrap-CortexRepo.ps1 `
    -RepositoryPath "C:\path\to\repository" `
    -Name "MyProject"
```

```bash
./scripts/bash/bootstrap-cortex-repo.sh /path/to/repository MyProject
```

Direct Python form:

```bash
python -m cortex bootstrap /path/to/repository --name MyProject --json
```

## What bootstrap learns

Bootstrap builds a bounded environment profile that includes:

- indexed language distribution;
- source, test, documentation, configuration, and runtime-evidence counts;
- package and build manifests;
- detected ecosystems such as Python, Node, Rust, Go, Java, containers, and CI;
- likely frameworks from local manifests;
- likely test, build, and run commands;
- likely entrypoints;
- Git availability;
- FTS5 availability;
- local runtime and launcher capabilities.

The latest profile is written to:

```text
TargetRepository/.cortex/runtime/environment_latest.json
```

It is also stored in the shared Cortex database for later activation.

## What bootstrap installs into the target

```text
TargetRepository/
├── AGENTS.md
└── .cortex/
    ├── config.json
    ├── bootstrap_certificate.json
    ├── README.md
    ├── .gitignore
    ├── bin/
    │   ├── cortex.ps1
    │   └── cortex.sh
    └── runtime/
        ├── context_latest.json
        └── environment_latest.json
```

The global database normally remains outside the repository:

```text
~/.cortex/
├── cortex.db
├── cards/
├── certificates/
├── packets/
├── sessions/
└── logs/
```

Set `CORTEX_HOME` before installation or bootstrap to move that storage.

## Activate Cortex before agent work

From an integrated repository:

### PowerShell

```powershell
.\.cortex\bin\cortex.ps1 activate `
    -Task "Trace the authentication flow and identify the smallest safe repair surface"
```

### Bash

```bash
./.cortex/bin/cortex.sh activate \
  --task "Trace the authentication flow and identify the smallest safe repair surface"
```

Activation performs:

1. repository manifest comparison;
2. incremental refresh when drift is detected;
3. relationship and Git telemetry refresh;
4. environment-profile refresh;
5. neural interlink recompilation when needed;
6. certificate verification;
7. hippocampal session creation;
8. lexical and semantic retrieval;
9. deterministic sparse activation;
10. bounded support-path selection;
11. Governor evaluation;
12. context packet generation.

The packet is written to:

```text
TargetRepository/.cortex/runtime/context_latest.json
```

## Context selection

Cortex first performs hybrid retrieval:

```text
SQLite FTS5 lexical ranking
+ deterministic feature-hash semantic similarity
+ Reciprocal Rank Fusion
+ authoritative and telemetry quality factors
```

The highest-ranked evidence seeds the neural interlink. Activation then propagates only through bounded existing associations. Support paths may add relevant tests, callers, dependencies, documentation, or co-changing files without broad repository loading.

The packet reports:

- direct evidence;
- neural support evidence;
- fired paths;
- propagation records;
- sparse activation ratio;
- nodes considered versus total nodes;
- propagation depth and steps;
- graph and activation state hashes;
- bounded plasticity updates, when allowed;
- provenance for every evidence chunk.

## Determinism boundary

With the same:

- database state;
- repository graph;
- task text;
- retrieval ordering;
- configuration;
- plasticity setting;

the sparse activation state hash and fired paths are deterministic.

Activation ledger timestamps are operational metadata and are not part of the deterministic state hash.

## Bounded plasticity

When enabled and the Governor is `normal` or `constrained`, co-activated traversed synapses may strengthen using a bounded rule:

```text
delta = learning_rate × pre_activation × post_activation × remaining_capacity
new_weight = clamp(old_weight + delta, minimum_weight, maximum_weight)
```

Properties:

- weights cannot leave declared bounds;
- no new topology is invented during activation;
- only compiled repository relationships can strengthen;
- read-only mode blocks plasticity;
- updates are recorded in the neural ledger;
- source code is never mutated by plasticity.

## Episodic and long-term memory

Neuron does not create a second episodic memory system.

During a task, use the existing Cortex hippocampal flow:

```powershell
.\.cortex\bin\cortex.ps1 remember `
    -Kind decision `
    -Text "The authentication middleware owns token normalization."
```

```bash
./.cortex/bin/cortex.sh remember \
  --kind decision \
  --text "The authentication middleware owns token normalization."
```

At task completion:

```powershell
.\.cortex\bin\cortex.ps1 consolidate
```

```bash
./.cortex/bin/cortex.sh consolidate
```

The Bridge deterministically converts explicit task events into a provenance-bearing Discovery Card. Source and current tests remain authoritative.

## Governor modes

| Mode | Meaning |
|---|---|
| `normal` | Certificate verified, manifest current, active focus present, and trust sufficient |
| `constrained` | Smaller context and bounded dry-run-first behavior |
| `read_only` | Retrieval, inspection, replay, and proposals only; plasticity is disabled |

A missing, failed, degraded, or stale certificate forces `read_only` regardless of numeric stability.

Cortex never authorizes source mutation. Host repository rules, current tests, runtime evidence, and explicit human authorization remain controlling.

## Useful commands

```bash
python -m cortex status --repo MyProject --json
python -m cortex doctor --repo MyProject --json
python -m cortex environment --repo MyProject --json
python -m cortex query "Where is retry policy enforced?" --repo MyProject --json
python -m cortex interlink --repo MyProject --task "Trace retry policy" --json
python -m cortex interlink --repo MyProject --task "Trace retry policy" --learn --json
python -m cortex neural-replay --repo MyProject --limit 100 --json
python -m cortex graph --repo MyProject --json
python -m cortex verify --repo MyProject --json
python -m cortex nexus-packet --repo MyProject --task "Prepare gated evidence" --json
```

## NexusGate integration

Cortex is designed to become an evidence and memory organ inside NexusGate while preserving separation of responsibilities:

```text
Cortex
  assimilation
  environment learning
  semantic/structural/temporal/episodic memory
  sparse neural activation
  evidence packets

NexusGate
  intent routing
  evidence gates
  authority checks
  certificates
  mutation governance
```

Generate a packet shaped for NexusGate:

```bash
python -m cortex nexus-packet \
  --repo NexusGate \
  --task "Summarize the active wound and nearest passed certificate" \
  --json
```

The packet includes intent, evidence, learned environment, neural interlink state, structural context, and an explicit recommendation-only authority boundary.

## Repository configuration

The generated `.cortex/config.json` controls:

- repository name and stable ID;
- bound Python interpreter, engine root, and Cortex home;
- context budget;
- chunk size and overlap;
- file-size ceiling;
- Git history limit;
- supported extensions and excluded paths;
- authoritative and runtime-evidence paths;
- environment learning;
- neural interlink enablement;
- activation depth and node budget;
- bounded plasticity enablement and learning rate;
- verification thresholds.

## Optional semantic model

The core system works offline with deterministic feature hashing.

To enable a local SentenceTransformers model:

```bash
python -m pip install -e ".[semantic]"
```

PowerShell:

```powershell
$env:CORTEX_EMBEDDING_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
```

Bash:

```bash
export CORTEX_EMBEDDING_MODEL="sentence-transformers/all-MiniLM-L6-v2"
```

If loading fails, Cortex falls back to the dependency-free embedder.

## Tests

```powershell
.\scripts\powershell\Run-Tests.ps1
```

```bash
./scripts/bash/run-tests.sh
```

Manual:

```bash
python -m compileall -q cortex tests
python -m unittest discover -s tests -v
```

The current suite covers:

- original Cortex bootstrap, retrieval, graph, telemetry, drift, wrappers, sessions, and consolidation;
- learned environment profiles;
- single-database neural compilation;
- deterministic sparse activation;
- bounded plasticity;
- neural ledger integrity and tamper detection;
- neural context and NexusGate packet integration;
- embedded-engine exclusion from host assimilation.

## Sparse activation benchmark

A reproducible synthetic benchmark is included:

```bash
python benchmarks/sparse_activation_benchmark.py --files 250
```

In the recorded build run, 42 of 262 nodes were considered and 24 fired, with identical metrics and state hash across two plasticity-disabled runs. See `BENCHMARK_REPORT.md` for the exact workload and claim boundary.

## Security and privacy

- No network access is required.
- The database can contain repository source and history; protect `CORTEX_HOME`.
- Exclude secret-bearing files before bootstrap.
- Do not record credentials, secrets, personal data, or raw confidential logs as episodic events.
- Neural association strength is evidence-routing metadata, not truth.
- Generated memory and environment inference can be incomplete.
- Current repository source, tests, compiler output, and runtime evidence win.

See `docs/SECURITY.md` for the full threat model.

## Non-goals

- training large neural models;
- autonomous source mutation;
- autonomous topology creation during activation;
- replacing repository tests or governance;
- distributed execution in this release;
- perfect semantic understanding of every language and artifact;
- claims of consciousness, AGI, or biological fidelity.

## Documentation

- `docs/ARCHITECTURE.md` — single-substrate architecture and data flow
- `docs/BOOTSTRAP_PROTOCOL.md` — portable assimilation and certification sequence
- `docs/AI_INTEGRATION.md` — generic agent and NexusGate use
- `docs/DATA_MODEL.md` — SQLite entities and provenance
- `docs/SECURITY.md` — trust, privacy, and authority boundaries
- `docs/TROUBLESHOOTING.md` — common setup and runtime problems
- `docs/NEURAL_INTERLINK.md` — sparse activation and bounded plasticity

## License

MIT License. See `LICENSE`.
