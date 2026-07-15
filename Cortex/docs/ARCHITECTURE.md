# Cortex Neural Interlink Architecture

## Purpose

Cortex is a local repository-assimilation and selective-memory organ. It inventories and verifies a repository once, then supplies bounded task context to an AI agent. The neural interlink is an internal routing layer over Cortex's existing memory and graph; it is not a second brain, database, or authority system.

## Core invariant

```text
one repository identity
one SQLite database
one episodic event path
one consolidation path
one Governor
one authority boundary
```

The neural layer may strengthen bounded internal associations. It may not authorize or perform source mutation.

## Components

### Thalamus

`thalamus` is the mandatory deterministic request-routing layer for normal context activation. It classifies task intent without an LLM, assigns bounded memory-lane weights and token budgets, and applies auditable inhibition to retrieved candidates before neural propagation and context selection. Its route plans are advisory only: Governor and host-repository authority boundaries remain controlling.

### Assimilator

`cortex.bootstrap` coordinates repository identity, target integration, indexing, graph resolution, Git telemetry, environment learning, neural compilation, verification, and certificate issuance.

### Inventory and indexer

`cortex.indexer` walks the repository with explicit exclusions, classifies every visible file, hashes supported content, records unsupported surfaces, chunks text by line, and stores lexical and semantic retrieval material.

### Environment learner

`cortex.environment` derives a bounded profile from indexed files and local manifests. It detects language distribution, package/build ecosystems, frameworks, likely commands, entrypoints, CI surfaces, and runtime capabilities. It does not execute inferred project commands during bootstrap.

### Structural parser and graph

`cortex.parsers` extracts symbols and raw relationships. `cortex.graph` resolves repository-local imports and adds test, documentation, reference, call, and co-change relationships.

### Temporal telemetry

`cortex.telemetry` reads bounded Git history and derives churn and co-change relationships. Git is optional and its availability is explicit in the certificate.

### Durable store

`cortex.store` owns the complete SQLite schema:

- repositories and file inventory;
- chunk memories and FTS5;
- symbols and edges;
- Git telemetry;
- sessions and events;
- environment profiles;
- neural nodes, synapses, activations, and ledger;
- bootstrap runs and settings.

### Hippocampus

`cortex.hippocampus` maintains active task focus and append-only task events.

### Bridge

`cortex.bridge` consolidates explicit task events into Discovery Cards. Neural activations do not bypass this consolidation path.

### Neural compiler

`cortex.neuron.compiler` maps indexed files to nodes and existing repository relationships to bounded synapses. It does not invent arbitrary topology.

### Sparse activation engine

`cortex.neuron.engine` seeds activation from hybrid retrieval, propagates through a limited number of existing associations, applies thresholding, records support paths, and optionally strengthens traversed co-activated synapses.

### Governor

`cortex.governor` evaluates integrity, certificate status, manifest freshness, active focus, retrieval confidence, and continuity. It selects `normal`, `constrained`, or `read_only` behavior.

### Activation

`cortex.activation` checks drift, refreshes changed surfaces, refreshes the environment and neural graph when needed, verifies the repository, starts a session, and emits a bounded packet.

## Bootstrap data flow

```text
repository root
  -> stable repository identity
  -> repository-local integration files
  -> file inventory and manifest hash
  -> chunk index and embeddings
  -> symbols and raw edges
  -> resolved structural graph
  -> Git telemetry and co-change edges
  -> learned environment profile
  -> neural node/synapse compilation
  -> retrieval probes and integrity checks
  -> bootstrap certificate
```

## Activation data flow

```text
current task
  -> manifest drift check
  -> incremental refresh if required
  -> hybrid retrieval
  -> Governor evaluation
  -> sparse neural activation
  -> bounded support-path expansion
  -> evidence selection under token budget
  -> structural neighborhood
  -> context packet
```

## Sparse activation boundary

Nodes represent indexed files. Synapses represent existing evidence-bearing relationships. Initial activation comes only from retrieved evidence. Propagation is limited by configured depth and node budget.

This keeps work proportional to the active neighborhood rather than the full repository.

## Plasticity boundary

Plasticity can only adjust the weight of an existing compiled synapse. It cannot:

- create new source files;
- alter repository code;
- invent a new relationship during activation;
- exceed minimum or maximum weight bounds;
- run in `read_only` mode;
- bypass the neural ledger.

## Trust order

```text
current repository source and tests
  > current compiler and runtime evidence
  > verified inventory, graph, environment, and telemetry
  > Discovery Cards
  > neural association weights
  > inference
```

Association strength affects routing efficiency. It is not proof of truth.
