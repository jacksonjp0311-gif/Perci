#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 1 ]]; then
  echo "Usage: $0 REPOSITORY_PATH [NAME] [--force]" >&2
  exit 2
fi
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PYTHONPATH="$ROOT${PYTHONPATH:+:$PYTHONPATH}"
REPO_PATH="$(cd "$1" && pwd)"
NAME="${2:-$(basename "$REPO_PATH")}" 
FORCE="${3:-}"
PYTHON="$ROOT/.venv/bin/python"
if [[ ! -x "$PYTHON" ]]; then PYTHON="${CORTEX_PYTHON:-python3}"; fi
ARGS=(-m cortex bootstrap "$REPO_PATH" --name "$NAME" --json)
if [[ "$FORCE" == "--force" ]]; then ARGS+=(--force); fi
"$PYTHON" "${ARGS[@]}"
printf '\nRepository bootstrap complete.\n'
printf 'Activate from the target repository with:\n'
printf '  ./.cortex/bin/cortex.sh activate --task "<current task>"\n'
