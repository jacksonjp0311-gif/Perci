# Data Model

Cortex uses one SQLite database, normally `~/.cortex/cortex.db`.

## Repository assimilation

- `repositories`: stable target identity, path, manifest, and bootstrap status.
- `files`: complete visible inventory and status.
- `memories`: indexed chunks with line ranges, hashes, vectors, and metadata.
- `memories_fts`: FTS5 lexical index.
- `symbols`: extracted symbols and signatures.
- `edges`: structural and temporal relationships.
- `git_commits`: bounded commit summaries.
- `file_telemetry`: churn and co-change statistics.

## Episodic and consolidated memory

- `sessions`: active and completed tasks.
- `events`: append-only discoveries, decisions, failures, fixes, and outcomes.
- Discovery Cards are stored as indexed memories with source-session provenance.

## Environment learning

- `environment_profiles`: deterministic JSON profile and profile hash per repository.

## Neural interlink

- `neural_nodes`: one node per indexed file with kind, threshold, tags, and metadata.
- `neural_synapses`: bounded relationships compiled from existing graph edges.
- `neural_activations`: replayable activation packets and state hashes.
- `neural_ledger`: monotonic hash-chained interlink events.

## Bootstrap and configuration

- `bootstrap_runs`: certificate history and run state.
- `settings`: global or repository-scoped metadata.

## Provenance

Evidence chunks preserve path, line range, kind, content hash, file hash, embedding model, and selection source. Neural support never removes this provenance.
