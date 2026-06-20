#!/usr/bin/env python3
"""Stop hook: validate .github only when the worktree contains .github changes."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def main() -> int:
    project = Path(os.environ.get("CLAUDE_PROJECT_DIR", ".")).resolve()
    status = subprocess.run(
        ["git", "status", "--porcelain=v1", "--", ".github"],
        cwd=project,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if status.returncode != 0 or not status.stdout.strip():
        return 0

    validator = Path(__file__).resolve().parent / "validate_setup.py"
    result = subprocess.run(
        [sys.executable, str(validator), "--repo", str(project), "--format", "markdown"],
        cwd=project,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode == 0:
        return 0

    message = result.stdout.strip() or result.stderr.strip() or ".github validation failed."
    print(message, file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
