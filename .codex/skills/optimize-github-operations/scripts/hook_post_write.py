#!/usr/bin/env python3
"""PostToolUse hook: log .github file writes without changing repository content."""

from __future__ import annotations

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        return 0

    file_path = payload.get("tool_input", {}).get("file_path")
    if not isinstance(file_path, str) or not file_path:
        return 0

    project = Path(os.environ.get("CLAUDE_PROJECT_DIR", ".")).resolve()
    target = Path(file_path).expanduser()
    if not target.is_absolute():
        target = (project / target).resolve()
    else:
        target = target.resolve()

    try:
        relative = target.relative_to(project)
    except ValueError:
        return 0
    if not relative.as_posix().startswith(".github/"):
        return 0

    git_dir = project / ".git"
    if not git_dir.is_dir():
        return 0
    log = git_dir / "claude-github-optimizer.log"
    timestamp = datetime.now(timezone.utc).isoformat()
    with log.open("a", encoding="utf-8") as handle:
        handle.write(f"{timestamp}\t{relative.as_posix()}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
