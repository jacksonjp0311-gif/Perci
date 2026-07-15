#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ORIGINAL_DIR="$(pwd)"
REPOSITORY_PATH=""
NAME=""
TASK="Map this repository, learn its environment, and prepare bounded agent context"
FORCE=0
RUN_TESTS=0
WITH_SEMANTIC=0
PYTHON_BIN="${PYTHON:-python3}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repository-path) REPOSITORY_PATH="${2:-}"; shift 2 ;;
    --name) NAME="${2:-}"; shift 2 ;;
    --task) TASK="${2:-}"; shift 2 ;;
    --python) PYTHON_BIN="${2:-python3}"; shift 2 ;;
    --force) FORCE=1; shift ;;
    --run-tests) RUN_TESTS=1; shift ;;
    --with-semantic-model) WITH_SEMANTIC=1; shift ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

is_project_root() {
  local path="$1"
  local marker
  for marker in .git pyproject.toml package.json Cargo.toml go.mod pom.xml build.gradle Makefile README.md; do
    [[ -e "$path/$marker" ]] && return 0
  done
  return 1
}

TARGET=""
if [[ -n "$REPOSITORY_PATH" ]]; then TARGET="$(cd "$REPOSITORY_PATH" && pwd)"; fi
PARENT="$(cd "$ROOT/.." && pwd)"
if [[ -z "$TARGET" ]] && is_project_root "$PARENT"; then TARGET="$PARENT"; fi
if [[ -z "$TARGET" && "$ORIGINAL_DIR" != "$ROOT" ]] && is_project_root "$ORIGINAL_DIR"; then TARGET="$ORIGINAL_DIR"; fi
if [[ -z "$TARGET" ]]; then TARGET="$ROOT"; fi

if [[ -z "$NAME" ]]; then NAME="$(basename "$TARGET")"; fi

cd "$ROOT"
if [[ ! -d .venv ]]; then "$PYTHON_BIN" -m venv .venv; fi
VENV_PYTHON="$ROOT/.venv/bin/python"
export PYTHONPATH="$ROOT${PYTHONPATH:+:$PYTHONPATH}"
if [[ "$WITH_SEMANTIC" == "1" ]]; then
  "$VENV_PYTHON" -m pip install -e ".[semantic]"
fi
"$VENV_PYTHON" -c 'import cortex; print(cortex.__version__)'
"$VENV_PYTHON" -m cortex init --json

if [[ "$RUN_TESTS" == "1" ]]; then
  "$VENV_PYTHON" -m compileall -q cortex tests
  "$VENV_PYTHON" -m unittest discover -s tests -v
fi

BOOTSTRAP_ARGS=(-m cortex bootstrap "$TARGET" --name "$NAME" --json)
if [[ "$FORCE" == "1" ]]; then BOOTSTRAP_ARGS+=(--force); fi
"$VENV_PYTHON" "${BOOTSTRAP_ARGS[@]}"
"$VENV_PYTHON" -m cortex doctor --repo "$NAME" --json
"$VENV_PYTHON" -m cortex verify --repo "$NAME" --json
"$VENV_PYTHON" -m cortex activate --repo "$NAME" --task "$TASK" --json

cd "$ORIGINAL_DIR"
printf '\nCORTEX + NEURAL INTERLINK READY\n'
printf 'Engine: %s\n' "$ROOT"
printf 'Integrated repository: %s\n' "$TARGET"
printf 'Repository name: %s\n' "$NAME"
printf 'Use from the integrated repository:\n'
printf '  ./.cortex/bin/cortex.sh activate --task "<current task>"\n'
