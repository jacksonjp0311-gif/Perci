#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
PYTHON="${PYTHON:-python3}"
"$PYTHON" -m venv .venv
VENV_PYTHON="$ROOT/.venv/bin/python"
if [[ "${CORTEX_WITH_SEMANTIC:-0}" == "1" ]]; then
  "$VENV_PYTHON" -m pip install ".[semantic]"
fi
if [[ "${CORTEX_WITH_SEMANTIC:-0}" != "1" ]]; then
  "$VENV_PYTHON" -m pip install .
fi
"$VENV_PYTHON" -m cortex init --json
"$VENV_PYTHON" -m cortex doctor --json
printf '\nCortex installed successfully.\n'
printf 'Activate with: source "%s/.venv/bin/activate"\n' "$ROOT"
printf 'Bootstrap with: ./scripts/bash/bootstrap-cortex-repo.sh /path/to/repo MyProject\n'
