# Bootstrap Protocol

## Objective

Bootstrap turns an unknown repository into a verified Cortex target that an agent can query without repeatedly rediscovering the whole environment.

## Portable entry points

PowerShell:

```powershell
.\Cortex-All-One.ps1 -RepositoryPath "C:\path\to\repo" -Name "MyRepo" -RunTests
```

Bash:

```bash
./cortex-all-one.sh --repository-path /path/to/repo --name MyRepo --run-tests
```

When Cortex is dropped inside a host repository and no path is supplied, the all-one launchers attempt to identify the parent repository. The embedded Cortex engine directory is added to the target exclusion rules so the engine does not assimilate itself as host source.

## Sequence

1. Resolve the Cortex engine root and target repository.
2. Create or reuse the engine virtual environment.
3. Install Cortex locally.
4. Initialize the shared Cortex home and database.
5. Assign a stable repository ID derived from the resolved target path.
6. Preserve existing `.cortex/config.json` policy when re-bootstrapping.
7. Install repository-local launchers and the managed `AGENTS.md` block.
8. Inventory every non-excluded file.
9. Record unsupported, binary, oversized, and unreadable surfaces.
10. Index supported text with line ranges and content hashes.
11. Extract symbols and relationships.
12. Resolve repository-local structural edges.
13. Import bounded Git telemetry and co-change edges.
14. Learn the repository environment profile.
15. Compile file nodes and bounded synapses from the verified graph.
16. Run retrieval probes.
17. Validate database, integration, manifest, coverage, environment, neural node coverage, and neural ledger.
18. Issue the bootstrap certificate.
19. Run doctor and verification checks.
20. Emit the first bounded activation packet.

## Certificate meaning

A `verified` certificate confirms the implemented inventory, supported-content indexing, environment profiling, relationship extraction, neural compilation, integration files, and retrieval probes for the recorded manifest.

It does not prove:

- source correctness;
- security or safety;
- semantic completeness;
- build success;
- test success in the target repository;
- authorization to mutate source.

## Drift handling

At activation, Cortex compares the current manifest with the stored manifest.

- `refresh=auto`: changed surfaces are re-indexed and the environment/neural graph are refreshed.
- `refresh=always`: the refresh sequence runs even when the manifest is current.
- `refresh=never`: drift remains visible and the Governor forces `read_only`.

## Idempotence

Re-bootstrap preserves explicit repository configuration, refreshes managed integration files, removes stale indexed records, and recompiles the neural graph from current evidence. Existing neural weights survive when the same synapse remains valid and are clamped to current bounds.
