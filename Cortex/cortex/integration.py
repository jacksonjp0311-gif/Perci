from __future__ import annotations

import shlex
import stat
import sys
from pathlib import Path
from typing import Any

from .config import RepoConfig, save_repo_config

MANAGED_BEGIN = "<!-- CORTEX:MANAGED:BEGIN -->"
MANAGED_END = "<!-- CORTEX:MANAGED:END -->"

AGENT_BLOCK = f"""{MANAGED_BEGIN}
## Cortex Repository Memory Protocol

This repository uses Cortex for verified repository assimilation, selective recall, and sparse neural interlinking.
Every activation is first routed through the local deterministic Thalamus planner, which allocates memory lanes and inhibits irrelevant evidence.

### Mandatory startup sequence

Before broad repository reading, planning, editing, or code generation:

1. Run `.\\.cortex\\bin\\cortex.ps1 activate -Task \"<current task>\"` on Windows PowerShell, or `./.cortex/bin/cortex.sh activate --task \"<current task>\"` on Bash.
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
.\\.cortex\\bin\\cortex.ps1 activate -Task "<task>"
.\\.cortex\\bin\\cortex.ps1 query -Query "<narrow question>"
.\\.cortex\\bin\\cortex.ps1 remember -Kind decision -Text "<decision>"
.\\.cortex\\bin\\cortex.ps1 consolidate
```

```bash
./.cortex/bin/cortex.sh activate --task "<task>"
./.cortex/bin/cortex.sh query --query "<narrow question>"
./.cortex/bin/cortex.sh remember --kind decision --text "<decision>"
./.cortex/bin/cortex.sh consolidate
```
{MANAGED_END}"""

POWERSHELL_WRAPPER = r'''param(
    [Parameter(Position=0)]
    [ValidateSet("activate", "bootstrap", "query", "remember", "consolidate", "verify", "status", "graph", "telemetry", "environment", "thalamus", "interlink", "neural-replay", "doctor")]
    [string]$Command = "activate",
    [string]$Task = "",
    [string]$Query = "",
    [string]$Kind = "discovery",
    [string]$Text = "",
    [int]$Budget = 1200,
    [switch]$Learn
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$ConfigPath = Join-Path $RepoRoot ".cortex\config.json"
if (-not (Test-Path $ConfigPath)) {
    throw "Cortex config is missing: $ConfigPath. Re-run repository bootstrap."
}

$Config = Get-Content $ConfigPath -Raw | ConvertFrom-Json
$RepoName = [string]$Config.repository_name
$CortexHome = [string]$Config.cortex_home
if ([string]::IsNullOrWhiteSpace($CortexHome)) { $CortexHome = [string]$env:CORTEX_HOME }
if ([string]::IsNullOrWhiteSpace($CortexHome)) { $CortexHome = '__CORTEX_HOME_PS__' }
$EngineModuleRoot = [string]$Config.engine_module_root
if ([string]::IsNullOrWhiteSpace($EngineModuleRoot)) { $EngineModuleRoot = '__CORTEX_ENGINE_MODULE_ROOT_PS__' }
if (-not [string]::IsNullOrWhiteSpace($EngineModuleRoot) -and (Test-Path $EngineModuleRoot)) {
    if ([string]::IsNullOrWhiteSpace($env:PYTHONPATH)) { $env:PYTHONPATH = $EngineModuleRoot }
    if (-not [string]::IsNullOrWhiteSpace($env:PYTHONPATH) -and -not $env:PYTHONPATH.StartsWith($EngineModuleRoot)) {
        $env:PYTHONPATH = "$EngineModuleRoot;$env:PYTHONPATH"
    }
}

$EnginePython = [string]$Config.engine_python
if ([string]::IsNullOrWhiteSpace($EnginePython)) { $EnginePython = [string]$env:CORTEX_PYTHON }
if ([string]::IsNullOrWhiteSpace($EnginePython)) { $EnginePython = '__CORTEX_ENGINE_PYTHON_PS__' }

$ResolvedPython = $null
if (Test-Path $EnginePython) { $ResolvedPython = (Resolve-Path $EnginePython).Path }
if ($null -eq $ResolvedPython) {
    $PythonCommand = Get-Command $EnginePython -ErrorAction SilentlyContinue
    if ($null -ne $PythonCommand) { $ResolvedPython = $PythonCommand.Source }
}
if ($null -eq $ResolvedPython) {
    $PythonCommand = Get-Command python -ErrorAction SilentlyContinue
    if ($null -ne $PythonCommand) { $ResolvedPython = $PythonCommand.Source }
}
if ($null -eq $ResolvedPython) {
    throw "Cortex Python was not found. Set CORTEX_PYTHON or re-run repository bootstrap."
}

& $ResolvedPython -c "import cortex" 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "The selected Python cannot import Cortex. Set CORTEX_PYTHON or re-run repository bootstrap."
}

$ArgsList = @("-m", "cortex", "--home", $CortexHome)
if ($Command -eq "activate") {
    if ([string]::IsNullOrWhiteSpace($Task)) { throw "-Task is required for activate." }
    $ArgsList += @("activate", "--repo", $RepoName, "--task", $Task, "--budget", "$Budget", "--json")
}
if ($Command -eq "bootstrap") { $ArgsList += @("bootstrap", $RepoRoot, "--name", $RepoName, "--json") }
if ($Command -eq "query") {
    if ([string]::IsNullOrWhiteSpace($Query)) { throw "-Query is required for query." }
    $ArgsList += @("query", $Query, "--repo", $RepoName, "--json")
}
if ($Command -eq "remember") {
    if ([string]::IsNullOrWhiteSpace($Text)) { throw "-Text is required for remember." }
    $ArgsList += @("remember", "--repo", $RepoName, "--kind", $Kind, "--text", $Text, "--json")
}
if ($Command -eq "consolidate") { $ArgsList += @("consolidate", "--repo", $RepoName, "--json") }
if ($Command -eq "verify") { $ArgsList += @("verify", "--repo", $RepoName, "--json") }
if ($Command -eq "status") { $ArgsList += @("status", "--repo", $RepoName, "--json") }
if ($Command -eq "graph") { $ArgsList += @("graph", "--repo", $RepoName, "--json") }
if ($Command -eq "telemetry") { $ArgsList += @("telemetry", "--repo", $RepoName, "--json") }
if ($Command -eq "environment") { $ArgsList += @("environment", "--repo", $RepoName, "--json") }
if ($Command -eq "thalamus") {
    if ([string]::IsNullOrWhiteSpace($Task)) { throw "-Task is required for thalamus." }
    $ArgsList += @("thalamus", "--repo", $RepoName, "--task", $Task, "--budget", "$Budget", "--json")
}
if ($Command -eq "doctor") { $ArgsList += @("doctor", "--repo", $RepoName, "--json") }
if ($Command -eq "neural-replay") { $ArgsList += @("neural-replay", "--repo", $RepoName, "--json") }
if ($Command -eq "interlink") {
    if ([string]::IsNullOrWhiteSpace($Task)) { throw "-Task is required for interlink." }
    $ArgsList += @("interlink", "--repo", $RepoName, "--task", $Task, "--json")
    if ($Learn) { $ArgsList += "--learn" }
}

& $ResolvedPython @ArgsList
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
'''

BASH_WRAPPER = r'''#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-activate}"
if [[ $# -gt 0 ]]; then shift; fi
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG_PATH="$REPO_ROOT/.cortex/config.json"
ENGINE_PYTHON=__CORTEX_ENGINE_PYTHON_SH__
ENGINE_MODULE_ROOT=__CORTEX_ENGINE_MODULE_ROOT_SH__
CORTEX_HOME_PATH=__CORTEX_HOME_SH__

if [[ ! -f "$CONFIG_PATH" ]]; then
  echo "Cortex config is missing: $CONFIG_PATH. Re-run repository bootstrap." >&2
  exit 2
fi

if [[ -z "$ENGINE_PYTHON" && -n "${CORTEX_PYTHON:-}" ]]; then ENGINE_PYTHON="$CORTEX_PYTHON"; fi
if [[ -z "$CORTEX_HOME_PATH" && -n "${CORTEX_HOME:-}" ]]; then CORTEX_HOME_PATH="$CORTEX_HOME"; fi
if [[ -d "$ENGINE_MODULE_ROOT" ]]; then export PYTHONPATH="$ENGINE_MODULE_ROOT${PYTHONPATH:+:$PYTHONPATH}"; fi
if [[ "$ENGINE_PYTHON" == */* && ! -x "$ENGINE_PYTHON" ]]; then ENGINE_PYTHON=""; fi
if [[ -z "$ENGINE_PYTHON" ]] && command -v python3 >/dev/null 2>&1; then ENGINE_PYTHON="$(command -v python3)"; fi
if [[ -z "$ENGINE_PYTHON" ]] && command -v python >/dev/null 2>&1; then ENGINE_PYTHON="$(command -v python)"; fi
if [[ -z "$ENGINE_PYTHON" ]]; then
  echo "Cortex Python was not found. Set CORTEX_PYTHON or re-run repository bootstrap." >&2
  exit 2
fi
if ! "$ENGINE_PYTHON" -c 'import cortex' >/dev/null 2>&1; then
  echo "The selected Python cannot import Cortex. Set CORTEX_PYTHON or re-run repository bootstrap." >&2
  exit 2
fi

REPO_NAME="$("$ENGINE_PYTHON" -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["repository_name"])' "$CONFIG_PATH")"

case "$COMMAND" in
  activate)
    TASK=""
    BUDGET="1200"
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --task) TASK="${2:-}"; shift 2 ;;
        --budget) BUDGET="${2:-1200}"; shift 2 ;;
        *) echo "Unknown activate argument: $1" >&2; exit 2 ;;
      esac
    done
    [[ -n "$TASK" ]] || { echo "--task is required" >&2; exit 2; }
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" activate --repo "$REPO_NAME" --task "$TASK" --budget "$BUDGET" --json
    ;;
  bootstrap)
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" bootstrap "$REPO_ROOT" --name "$REPO_NAME" --json
    ;;
  query)
    QUERY=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --query) QUERY="${2:-}"; shift 2 ;;
        *) echo "Unknown query argument: $1" >&2; exit 2 ;;
      esac
    done
    [[ -n "$QUERY" ]] || { echo "--query is required" >&2; exit 2; }
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" query "$QUERY" --repo "$REPO_NAME" --json
    ;;
  remember)
    KIND="discovery"
    TEXT=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --kind) KIND="${2:-discovery}"; shift 2 ;;
        --text) TEXT="${2:-}"; shift 2 ;;
        *) echo "Unknown remember argument: $1" >&2; exit 2 ;;
      esac
    done
    [[ -n "$TEXT" ]] || { echo "--text is required" >&2; exit 2; }
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" remember --repo "$REPO_NAME" --kind "$KIND" --text "$TEXT" --json
    ;;
  interlink)
    TASK=""
    LEARN=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --task) TASK="${2:-}"; shift 2 ;;
        --learn) LEARN="--learn"; shift ;;
        *) echo "Unknown interlink argument: $1" >&2; exit 2 ;;
      esac
    done
    [[ -n "$TASK" ]] || { echo "--task is required" >&2; exit 2; }
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" interlink --repo "$REPO_NAME" --task "$TASK" ${LEARN:+$LEARN} --json
    ;;
  thalamus)
    TASK=""
    BUDGET="1200"
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --task) TASK="${2:-}"; shift 2 ;;
        --budget) BUDGET="${2:-1200}"; shift 2 ;;
        *) echo "Unknown thalamus argument: $1" >&2; exit 2 ;;
      esac
    done
    [[ -n "$TASK" ]] || { echo "--task is required" >&2; exit 2; }
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" thalamus --repo "$REPO_NAME" --task "$TASK" --budget "$BUDGET" --json
    ;;
  consolidate|verify|status|graph|telemetry|environment|neural-replay|doctor)
    exec "$ENGINE_PYTHON" -m cortex --home "$CORTEX_HOME_PATH" "$COMMAND" --repo "$REPO_NAME" --json
    ;;
  *) echo "Unknown command: $COMMAND" >&2; exit 2 ;;
esac
'''

REPO_CORTEX_README = """# Repository Cortex Integration

This directory is the repository-local integration surface for the installed Cortex engine.

- `config.json` identifies this repository, records the Cortex Python interpreter/module location, and controls assimilation.
- `bootstrap_certificate.json` records the latest verified inventory and coverage state.
- `bin/cortex.ps1` and `bin/cortex.sh` are stable entry points for Codex and other agents.
- `runtime/` contains generated context and learned-environment packets and is intentionally ignored by Git.

Cortex's global database normally lives at `~/.cortex/cortex.db`. The neural interlink shares that database and never creates a competing memory authority. Repository source remains authoritative.
"""

GITIGNORE = """runtime/
*.tmp
*.lock
"""


def install_integration(root: Path, config: RepoConfig) -> dict[str, Any]:
    cortex_dir = root / ".cortex"
    bin_dir = cortex_dir / "bin"
    runtime_dir = cortex_dir / "runtime"
    bin_dir.mkdir(parents=True, exist_ok=True)
    runtime_dir.mkdir(parents=True, exist_ok=True)

    engine_python = config.engine_python or str(Path(sys.executable))
    engine_module_root = config.engine_module_root or str(Path(__file__).resolve().parent.parent)
    cortex_home = config.cortex_home or str((Path.home() / ".cortex").resolve())
    config.engine_python = engine_python
    config.engine_module_root = engine_module_root
    config.cortex_home = cortex_home
    save_repo_config(root, config)

    (cortex_dir / "README.md").write_text(REPO_CORTEX_README, encoding="utf-8")
    (cortex_dir / ".gitignore").write_text(GITIGNORE, encoding="utf-8")
    (runtime_dir / ".gitkeep").write_text("", encoding="utf-8")

    ps_path = bin_dir / "cortex.ps1"
    sh_path = bin_dir / "cortex.sh"
    ps_engine = engine_python.replace("'", "''")
    ps_module_root = engine_module_root.replace("'", "''")
    ps_cortex_home = cortex_home.replace("'", "''")
    ps_content = POWERSHELL_WRAPPER.replace("__CORTEX_ENGINE_PYTHON_PS__", ps_engine)
    ps_content = ps_content.replace("__CORTEX_ENGINE_MODULE_ROOT_PS__", ps_module_root)
    ps_content = ps_content.replace("__CORTEX_HOME_PS__", ps_cortex_home)
    sh_content = BASH_WRAPPER.replace(
        "__CORTEX_ENGINE_PYTHON_SH__", shlex.quote(engine_python)
    )
    sh_content = sh_content.replace(
        "__CORTEX_ENGINE_MODULE_ROOT_SH__", shlex.quote(engine_module_root)
    )
    sh_content = sh_content.replace("__CORTEX_HOME_SH__", shlex.quote(cortex_home))
    ps_path.write_text(ps_content, encoding="utf-8")
    sh_path.write_text(sh_content, encoding="utf-8", newline="\n")
    sh_path.chmod(sh_path.stat().st_mode | stat.S_IEXEC)

    agents_path = root / "AGENTS.md"
    existing = (
        agents_path.read_text(encoding="utf-8", errors="replace")
        if agents_path.exists()
        else "# AGENTS.md\n\n"
    )
    if MANAGED_BEGIN in existing and MANAGED_END in existing:
        before, rest = existing.split(MANAGED_BEGIN, 1)
        _, after = rest.split(MANAGED_END, 1)
        updated = before.rstrip() + "\n\n" + AGENT_BLOCK + after
    else:
        updated = existing.rstrip() + "\n\n" + AGENT_BLOCK + "\n"
    agents_path.write_text(updated, encoding="utf-8")

    return {
        "config": str(cortex_dir / "config.json"),
        "agents": str(agents_path),
        "powershell_wrapper": str(ps_path),
        "bash_wrapper": str(sh_path),
        "runtime_directory": str(runtime_dir),
        "engine_python": engine_python,
        "engine_module_root": engine_module_root,
        "cortex_home": cortex_home,
    }


def integration_status(root: Path) -> dict[str, Any]:
    required = [
        root / ".cortex" / "config.json",
        root / ".cortex" / "README.md",
        root / ".cortex" / "bin" / "cortex.ps1",
        root / ".cortex" / "bin" / "cortex.sh",
        root / "AGENTS.md",
    ]
    agents_text = (
        (root / "AGENTS.md").read_text(encoding="utf-8", errors="replace")
        if (root / "AGENTS.md").exists()
        else ""
    )
    managed = MANAGED_BEGIN in agents_text and MANAGED_END in agents_text
    return {
        "required_files": {str(path.relative_to(root)): path.exists() for path in required},
        "agents_managed_block": managed,
        "complete": all(path.exists() for path in required) and managed,
    }
