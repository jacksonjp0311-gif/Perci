# Perci + Cortex integration

Perci v0.1.1 vendors Cortex as its governed selective-memory engine.

## Responsibility boundary

```text
Perci
  explicit command precedence
  exact-tool dispatch
  Bitwork cognitive classification
  backend composition
  response coordination

Cortex
  repository assimilation
  semantic, structural, temporal, and episodic memory
  deterministic sparse activation
  bounded provenance-bearing context packets
  Governor trust reduction
```

Cortex supplies evidence. It does not authorize source mutation. Current source,
tests, compiler output, runtime evidence, and explicit human permission remain
controlling.

## Local initialization

Windows:

```powershell
powershell -ExecutionPolicy Bypass -File .\Initialize-Perci-Cortex.ps1
```

Or:

```powershell
.\Start-Perci.cmd -Mode cortex-init
```

Cortex creates local runtime state under:

```text
.cortex/
.perci/cortex-home/
Cortex/.venv/
```

These paths are intentionally ignored by Git because they contain machine-bound
configuration, a Python environment, a local SQLite database, packets, sessions,
and repository paths.

## Runtime behavior

For ordinary chat, Perci asks Cortex for a bounded `cortex-context/1.0` packet.
Only selected evidence text, relative paths, line ranges, hashes, and governance
mode are passed to the language backend.

For explicit memory writes:

```text
remember that ...
```

Perci appends a JSONL memory record and asks Cortex to record the same explicit
episodic event. No probabilistic route can trigger a memory write.

For explicit recall:

```text
recall ...
```

Perci strips the command prefix, searches local JSONL memory, and requests a
bounded Cortex packet.

## Environment overrides

```text
PERCI_CORTEX_ROOT
PERCI_CORTEX_PYTHON
PERCI_CORTEX_HOME
PERCI_CORTEX_REPO
```

## Upstream synchronization

`Cortex/UPSTREAM.json` records the vendored upstream repository and commit.
Re-run the project evolution/synchronization script to update the snapshot.