#!/usr/bin/env python3
"""PreToolUse hook: block destructive Bash commands that can discard .github or worktree state."""

from __future__ import annotations

import json
import re
import sys

BLOCKED = [
    (r"\bgit\s+reset\s+--hard\b", "git reset --hard can discard unrelated work"),
    (r"\bgit\s+clean\b", "git clean can delete untracked repository files"),
    (r"\bgit\s+(?:checkout|restore)\b[^\n]*(?:--\s+)?(?:\.github|\.)", "destructive checkout or restore is not allowed"),
    (r"\brm\s+[^\n]*(?:-r|-rf|-fr)[^\n]*(?:\.github|\s\.\s*$)", "recursive deletion of repository governance is not allowed"),
    (r"\b(?:rmdir|del)\b[^\n]*\.github", "deletion of .github is not allowed"),
    (r"\bmv\s+[^\n]*\.github(?:\s|/|$)", "moving .github is not allowed"),
]


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        print("Hook guard received invalid JSON; blocking the command.", file=sys.stderr)
        return 2

    command = str(payload.get("tool_input", {}).get("command", ""))
    for pattern, reason in BLOCKED:
        if re.search(pattern, command, flags=re.IGNORECASE | re.MULTILINE):
            print(f"Blocked by .github optimizer policy: {reason}.", file=sys.stderr)
            return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
