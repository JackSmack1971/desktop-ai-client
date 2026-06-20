#!/usr/bin/env python3
"""Claude Code PreToolUse hook for the generate-codeowners workflow."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

DESTRUCTIVE = re.compile(
    r"(?:^|[;&|]\s*)(?:rm\s+-[^\n]*r|git\s+(?:reset\s+--hard|clean\s+-|checkout\s+--\s+\.|restore\s+--staged\s+--worktree)|gh\s+(?:repo|api).*\s-(?:X|f)\s*(?:DELETE|PUT|PATCH|POST))",
    re.IGNORECASE,
)
PERMISSION_MUTATION = re.compile(
    r"\bgh\s+api\b(?=[^\n]*(?:collaborators|teams/.*/repos|rulesets|branches/.*/protection))(?=[^\n]*(?:-X|--method)\s*(?:POST|PUT|PATCH|DELETE)|[^\n]*(?:-f|--field|--raw-field)\s)",
    re.IGNORECASE,
)


def deny(reason: str) -> int:
    payload = {
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }
    print(json.dumps(payload))
    return 0


def repo_root() -> Path | None:
    proc = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    if proc.returncode != 0:
        return None
    return Path(proc.stdout.strip()).resolve()


def git_state(root: Path) -> Path | None:
    proc = subprocess.run(
        ["git", "rev-parse", "--git-path", "claude-codeowners"],
        cwd=str(root),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    if proc.returncode != 0:
        return None
    path = Path(proc.stdout.strip())
    return path.resolve() if path.is_absolute() else (root / path).resolve()


def within(path: Path, root: Path) -> bool:
    try:
        path.resolve().relative_to(root.resolve())
        return True
    except ValueError:
        return False


def main() -> int:
    try:
        data: dict[str, Any] = json.load(sys.stdin)
    except (json.JSONDecodeError, TypeError):
        return deny("Hook input was not valid JSON")

    tool = str(data.get("tool_name") or "")
    tool_input = data.get("tool_input") or {}
    if not isinstance(tool_input, dict):
        return deny("Hook input did not contain a tool_input object")

    if tool in {"Bash", "WorkspaceBash"}:
        command = str(tool_input.get("command") or tool_input.get("bash_command") or "")
        if DESTRUCTIVE.search(command):
            return deny("Destructive cleanup or history rewriting is outside the CODEOWNERS workflow")
        if PERMISSION_MUTATION.search(command):
            return deny("The CODEOWNERS workflow is read-only with respect to permissions, rulesets, and branch protection")
        return 0

    if tool in {"Write", "Edit", "MultiEdit", "NotebookEdit"}:
        raw_path = tool_input.get("file_path") or tool_input.get("path")
        if not isinstance(raw_path, str) or not raw_path:
            return deny("Write/Edit target path was missing")
        root = repo_root()
        if root is None:
            return deny("Write/Edit is blocked because the Git repository root could not be resolved")
        target = Path(raw_path)
        if not target.is_absolute():
            target = (Path.cwd() / target).resolve()
        allowed_target = (root / ".github/CODEOWNERS").resolve()
        state = git_state(root)
        if target == allowed_target or (state is not None and within(target, state)):
            return 0
        return deny("The workflow may write only .github/CODEOWNERS and the Git metadata state directory")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
