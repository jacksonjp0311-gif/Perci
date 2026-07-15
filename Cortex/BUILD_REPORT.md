# Build Report

## Release

- Name: Cortex Neural Interlink
- Version: 1.1.0
- Date: July 11, 2026
- Package: `cortex-memory`
- Python: 3.10+
- License: MIT

## Integration decision

The standalone Neuron repository was not copied into Cortex as a second independent memory service. Its useful capabilities were adapted into `cortex.neuron` and connected to Cortex's existing graph, SQLite store, hippocampal sessions, Bridge, Governor, and Nexus packet.

This preserves the original Cortex mission and prevents:

- duplicate databases;
- duplicate episodic memory;
- competing consolidation logic;
- conflicting governance modes;
- divergent provenance;
- ambiguous ownership between Cortex and Neuron.

## Added modules

- `cortex/environment.py`
- `cortex/neuron/models.py`
- `cortex/neuron/compiler.py`
- `cortex/neuron/engine.py`
- `cortex/neuron/plasticity.py`
- `cortex/neuron/__init__.py`
- `benchmarks/sparse_activation_benchmark.py`

## Main evolved behaviors

- environment learning during bootstrap;
- file nodes compiled from indexed surfaces;
- bounded synapses compiled from current graph evidence;
- sparse deterministic task activation;
- neural support-path expansion under the existing context budget;
- bounded plasticity in normal/constrained modes;
- hash-chained neural replay ledger;
- neural and environment fields in NexusGate packets;
- portable no-install all-one flow;
- automatic embedded-engine exclusion;
- repository-local wrapper binding that resists inherited Cortex-home and Python-engine redirection.

## Validation summary

- Python compile: passed
- Automated tests: 17 passed
- Bash syntax: passed
- Portable nested-folder smoke: passed
- Wheel build: passed
- Clean wheel install/bootstrap/activate: passed
- Database integrity: passed
- Neural ledger integrity: passed
- Deterministic benchmark repeat: passed

## Known boundaries

- PowerShell was statically reviewed but not executed in the Linux build environment.
- Environment commands are inferred and are not automatically executed during bootstrap.
- Neural relationships are limited by parser and graph quality.
- Sparse activation is an engineering routing mechanism, not a biological simulation.
