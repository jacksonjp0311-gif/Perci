#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON="${CORTEX_PYTHON:-$ROOT/.venv/bin/python}"
if [[ "$PYTHON" == */* && ! -x "$PYTHON" ]]; then PYTHON=""; fi
if [[ -z "$PYTHON" ]] && command -v python3 >/dev/null 2>&1; then PYTHON="$(command -v python3)"; fi
if [[ -z "$PYTHON" ]] && command -v python >/dev/null 2>&1; then PYTHON="$(command -v python)"; fi
if [[ -z "$PYTHON" ]]; then echo "Python 3.10+ is required." >&2; exit 2; fi
export PYTHONPATH="$ROOT${PYTHONPATH:+:$PYTHONPATH}"
exec "$PYTHON" -m cortex "$@"
