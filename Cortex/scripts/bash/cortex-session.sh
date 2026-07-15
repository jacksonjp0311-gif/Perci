#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 2 ]]; then
  echo "Usage: $0 REPOSITORY_PATH TASK [BUDGET]" >&2
  exit 2
fi
REPO_PATH="$(cd "$1" && pwd)"
TASK="$2"
BUDGET="${3:-1200}"
WRAPPER="$REPO_PATH/.cortex/bin/cortex.sh"
if [[ ! -x "$WRAPPER" ]]; then
  echo "Cortex is not integrated into $REPO_PATH. Bootstrap it first." >&2
  exit 2
fi
exec "$WRAPPER" activate --task "$TASK" --budget "$BUDGET"
