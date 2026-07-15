# AGENTS.md

<!-- CORTEX:MANAGED:BEGIN -->
## Cortex Repository Memory Protocol

This repository uses Cortex for verified repository assimilation, selective recall, and sparse neural interlinking.
Every activation is first routed through the local deterministic Thalamus planner, which allocates memory lanes and inhibits irrelevant evidence.

### Mandatory startup sequence

Before broad repository reading, planning, editing, or code generation:

1. Run `.\.cortex\bin\cortex.ps1 activate -Task "<current task>"` on Windows PowerShell, or `./.cortex/bin/cortex.sh activate --task "<current task>"` on Bash.
2. Inspect the returned bootstrap status, governor mode, learned environment, evidence references, neural support paths, and structural neighborhood.
3. If the bootstrap certificate is missing, degraded, or stale, run the wrapper's `bootstrap` command before relying on memory.
4. Read only the cited files and line ranges first. Expand context only when the packet is insufficient.
5. Treat repository source, tests, compiler output, and current runtime evidence as more authoritative than summaries.
6. Record decisions, discoveries, invariants, failures, fixes, and outcomes with the wrapper's `remember` command.
7. Run `consolidate` at task completion to create a provenance-bearing Discovery Card.

### Authority boundary

Cortex provides memory, relationships, telemetry, sparse activation, and evidence references. Neural plasticity changes only bounded internal association weights; it never authorizes durable source mutation. The host repository's rules and explicit human authorization remain controlling.

### Required commands

```powershell
.\.cortex\bin\cortex.ps1 activate -Task "<task>"
.\.cortex\bin\cortex.ps1 query -Query "<narrow question>"
.\.cortex\bin\cortex.ps1 remember -Kind decision -Text "<decision>"
.\.cortex\bin\cortex.ps1 consolidate
```

```bash
./.cortex/bin/cortex.sh activate --task "<task>"
./.cortex/bin/cortex.sh query --query "<narrow question>"
./.cortex/bin/cortex.sh remember --kind decision --text "<decision>"
./.cortex/bin/cortex.sh consolidate
```
<!-- CORTEX:MANAGED:END -->
