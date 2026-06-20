#!/usr/bin/env bash
set -euo pipefail

if command -v python3 >/dev/null 2>&1; then
  exec python3 "$@"
fi

if command -v python >/dev/null 2>&1; then
  exec python "$@"
fi

printf '%s\n' 'ERROR: Python 3.10 or later is required and must be available as python3 or python.' >&2
exit 127
