#!/usr/bin/env python3
"""Skill-scoped PreToolUse guard for destructive shell commands."""

from __future__ import annotations

import json
import re
import sys


def deny(reason: str) -> None:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }
        )
    )


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError):
        deny("Workspace hygiene guard could not parse the tool request.")
        return 0

    if payload.get("tool_name") != "Bash":
        return 0

    command = str(payload.get("tool_input", {}).get("command", ""))
    normalized = " ".join(command.lower().split())

    approved = (
        "workspace_hygiene.py" in normalized
        and " apply " in f" {normalized} "
        and "--confirm purge" in normalized
    )
    if approved:
        return 0

    destructive_patterns = (
        r"(^|[;&|]\s*)rm\s+.*-[a-z]*r[a-z]*f",
        r"(^|[;&|]\s*)git\s+clean\b",
        r"\bremove-item\b.*\b-recurse\b",
        r"\brmdir\b.*(?:/s|/q)",
        r"(^|[;&|]\s*)del\s+.*(?:/s|/q)",
        r"\bfind\b.*\s-delete\b",
    )
    if any(re.search(pattern, normalized, flags=re.IGNORECASE) for pattern in destructive_patterns):
        deny(
            "Direct destructive cleanup is blocked. Use the bundled "
            "workspace_hygiene.py apply --confirm PURGE command."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
