# Cortex v2.0.0 Test Report

**Release validation date:** July 14, 2026

## v2.0 release validation

- `python -m pytest`: 26 passed
- Python compilation: passed
- `ruff check cortex tests`: passed
- Outcome learning: activation is observational; a verified outcome receives bounded credit, passes the replay gate, updates only in an allowed Governor mode, and appends to the hash-chained ledger.
- Context protocol: `cortex protocol` exposes `cortex-context/1.0` with an explicit authority boundary.

The v1.3.1 report below is retained as release lineage.

# Cortex Neural Interlink v1.3.1 Test Report

**Test date:** July 11, 2026  
**Primary validation environment:** Linux, CPython 3.13, SQLite with FTS5

## Automated suite

## Current release addendum (July 12, 2026)

- Python compilation: passed
- `python -m unittest discover -s tests -q`: 26 passed
- Ruff: passed
- `cortex benchmark --verify --json`: passed

The historic v1.1.0 validation details below are retained as lineage, not as the current release claim.

Commands:

```bash
python -m compileall -q cortex tests
python -m unittest discover -s tests -v
bash -n cortex.sh cortex-all-one.sh scripts/bash/*.sh
```

Result:

```text
Ran 17 tests
OK
```

The suite covers all original Cortex behavior plus the neural integration:

- managed `AGENTS.md` and repository-local wrapper installation;
- bootstrap certificate issuance and manifest integrity;
- supported-content indexing and unsupported-surface reporting;
- hybrid retrieval with path, line, hash, type, and score provenance;
- structural graph resolution and source-to-test relationships;
- bounded Git history and co-change telemetry;
- repository drift detection and incremental refresh;
- read-only fallback when refresh is disabled;
- active sessions, episodic events, and Discovery Card consolidation;
- repository identity reuse without stale-memory contamination;
- CLI and generated Bash wrapper execution, including protection from inherited Cortex-home and Python-engine redirection;
- learned environment profiles;
- single-database neural node and synapse compilation;
- deterministic sparse activation with plasticity disabled;
- explicit structural propagation from a retrieved seed into a non-retrieved support file;
- bounded Hebbian plasticity;
- neural event-ledger integrity and tamper detection;
- neural context and NexusGate packet integration;
- embedded engine exclusion from host assimilation.

## Portable nested-folder smoke test

A clean copy of Cortex was placed at:

```text
HostRepo/CortexEngine/
```

The command below was run without installing the package into the virtual environment:

```bash
./cortex-all-one.sh \
  --name HostRepo \
  --task "Find the host entrypoint" \
  --run-tests
```

Observed outcome:

- engine virtual environment created;
- 17 tests passed;
- parent host repository inferred;
- `CortexEngine` automatically added to host exclusions;
- repository bootstrap completed;
- environment profile written;
- neural interlink compiled;
- bootstrap certificate status: `verified`;
- doctor database check: passed;
- neural ledger check: passed;
- first context packet written.

## Wheel validation

A wheel was built and installed into a clean virtual environment:

```text
cortex_memory-1.1.0-py3-none-any.whl
```

A separate sample repository completed:

```text
wheel install -> bootstrap -> environment learning -> neural compilation
-> verified certificate -> activation -> doctor
```

Observed checks:

```json
{
  "certificate": "verified",
  "environment_ecosystems": ["python"],
  "neural_node_coverage": 1.0,
  "database_integrity": true,
  "neural_ledger_integrity": true
}
```

## Deterministic benchmark validation

The 250-module sparse benchmark was run twice. Activation metrics and state hash were identical across runs when plasticity was disabled.

See `BENCHMARK_REPORT.md`.

## PowerShell validation boundary

The build environment does not contain Windows PowerShell or `pwsh`. PowerShell files were reviewed for Windows PowerShell 5.1-compatible syntax and kept free of PowerShell 7-only constructs. The GitHub Actions matrix includes Windows launcher execution for repository-hosted validation.

## Claim boundary

These results validate the implemented assimilation, environment learning, graph compilation, retrieval, sparse activation, bounded plasticity, consolidation, integration, drift refresh, packaging, and integrity behavior. They do not prove target-repository correctness, security, semantic completeness, biological fidelity, or authority to mutate source.
