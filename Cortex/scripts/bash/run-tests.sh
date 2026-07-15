#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
PYTHON="$ROOT/.venv/bin/python"
if [[ ! -x "$PYTHON" ]]; then PYTHON="${CORTEX_PYTHON:-python3}"; fi
"$PYTHON" -m compileall -q cortex tests
"$PYTHON" -m unittest discover -s tests -v
printf 'Cortex tests passed.\n'
