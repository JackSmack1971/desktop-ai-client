#!/usr/bin/env python3
"""Claude Code Stop hook: block completion when generated CODEOWNERS is invalid."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        hook_input = json.load(sys.stdin)
    except (json.JSONDecodeError, TypeError):
        hook_input = {}
    if isinstance(hook_input, dict) and hook_input.get("stop_hook_active") is True:
        return 0
    repo = Path(args.repo).resolve()
    target = repo / ".github/CODEOWNERS"
    if not target.is_file():
        return 0
    validator = Path(__file__).with_name("validate_codeowners.py")
    proc = subprocess.run(
        [sys.executable, str(validator), "--repo", str(repo), "--file", ".github/CODEOWNERS"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if proc.returncode == 0:
        return 0
    message = proc.stderr.strip() or proc.stdout.strip() or "CODEOWNERS validation failed"
    print(message, file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
