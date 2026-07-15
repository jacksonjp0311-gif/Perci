# AI Integration

## Required startup behavior

Before broad repository reading, planning, or code generation, an agent should run the repository-local activation wrapper.

PowerShell:

```powershell
.\.cortex\bin\cortex.ps1 activate -Task "<current task>"
```

Bash:

```bash
./.cortex/bin/cortex.sh activate --task "<current task>"
```

The returned packet contains:

- bootstrap and manifest status;
- Governor mode;
- learned environment summary;
- directly retrieved evidence;
- neural support evidence;
- sparse activation state and metrics;
- structural neighborhood;
- provenance hashes and line ranges;
- explicit authority instructions.

## Agent operating sequence

1. Read the packet before broad source exploration.
2. Start with directly retrieved evidence.
3. Use neural support paths as bounded expansion candidates.
4. Open full files only when cited chunks are insufficient.
5. Treat current source, tests, compiler output, and runtime evidence as authoritative.
6. Record durable discoveries, decisions, failures, fixes, and outcomes.
7. Consolidate at task completion.

## Generic agent command

```bash
python -m cortex activate \
  --repo MyProject \
  --task "Explain the failing release path" \
  --budget 1200 \
  --json
```

## Inspect the learned environment

```bash
python -m cortex environment --repo MyProject --json
```

## Inspect sparse activation

```bash
python -m cortex interlink \
  --repo MyProject \
  --task "Trace authentication ownership" \
  --json
```

Add `--learn` only when bounded internal association strengthening is desired. Normal activation already obeys repository configuration and Governor mode.

## NexusGate packet

```bash
python -m cortex nexus-packet \
  --repo NexusGate \
  --task "Find the active wound, controlling invariant, and nearest certificate" \
  --json
```

Cortex contributes evidence and context. NexusGate remains responsible for intent routing, gates, certificates, and mutation governance.

## Authority boundary

Cortex may index, retrieve, activate, remember, consolidate, and adjust bounded internal association weights. It may not authorize durable source changes. The host repository and explicit human authorization control mutation.
