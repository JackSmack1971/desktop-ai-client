#!/usr/bin/env python3
"""Shared helpers for the generate-codeowners skill."""

from __future__ import annotations

import fnmatch
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any, Iterable

OWNER_RE = re.compile(
    r"^(?:@[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})(?:/[A-Za-z0-9](?:[A-Za-z0-9-]{0,98}))?|[^\s@]+@[^\s@]+\.[^\s@]+)$"
)
PLACEHOLDER_OWNER_RE = re.compile(
    r"(?:example|your[-_]?org|your[-_]?team|account/team|org/team|owner_username|placeholder|todo|changeme)",
    re.IGNORECASE,
)


def run(
    args: list[str],
    cwd: Path,
    *,
    check: bool = True,
    text: bool = True,
) -> subprocess.CompletedProcess[str]:
    """Run a command without invoking a shell."""
    proc = subprocess.run(
        args,
        cwd=str(cwd),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=text,
    )
    if check and proc.returncode != 0:
        command = " ".join(args)
        detail = proc.stderr.strip() or proc.stdout.strip() or "unknown failure"
        raise RuntimeError(f"Command failed ({proc.returncode}): {command}\n{detail}")
    return proc


def resolve_repo(path: str | Path) -> Path:
    candidate = Path(path).expanduser().resolve()
    if not candidate.exists():
        raise ValueError(f"Repository path does not exist: {candidate}")
    proc = run(["git", "rev-parse", "--show-toplevel"], candidate, check=False)
    if proc.returncode != 0:
        raise ValueError(f"Not inside a Git worktree: {candidate}")
    return Path(proc.stdout.strip()).resolve()


def git_path(repo: Path, name: str) -> Path:
    value = run(["git", "rev-parse", "--git-path", name], repo).stdout.strip()
    path = Path(value)
    if not path.is_absolute():
        path = (repo / path).resolve()
    return path


def atomic_write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temp.write_text(content, encoding="utf-8", newline="\n")
    os.replace(temp, path)


def atomic_write_json(path: Path, payload: Any) -> None:
    atomic_write_text(path, json.dumps(payload, indent=2, sort_keys=False) + "\n")


def ensure_within(path: Path, root: Path) -> Path:
    resolved = path.expanduser().resolve()
    root_resolved = root.expanduser().resolve()
    try:
        resolved.relative_to(root_resolved)
    except ValueError as exc:
        raise ValueError(f"Path escapes allowed root: {resolved}") from exc
    return resolved


def parse_github_remote(url: str) -> tuple[str, str] | None:
    """Parse HTTPS or SSH GitHub remote URL into owner/repository."""
    value = url.strip()
    patterns = (
        r"^https://github\.com/([^/]+)/([^/]+?)(?:\.git)?$",
        r"^ssh://git@github\.com/([^/]+)/([^/]+?)(?:\.git)?$",
        r"^git@github\.com:([^/]+)/([^/]+?)(?:\.git)?$",
    )
    for pattern in patterns:
        match = re.match(pattern, value)
        if match:
            return match.group(1), match.group(2)
    return None


def list_tracked_files(repo: Path, max_files: int | None = None) -> list[str]:
    proc = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=str(repo),
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.decode("utf-8", errors="replace").strip())
    files = [
        item.decode("utf-8", errors="surrogateescape")
        for item in proc.stdout.split(b"\0")
        if item
    ]
    if max_files is not None and len(files) > max_files:
        raise ValueError(
            f"Repository has {len(files)} tracked files, exceeding the configured limit of {max_files}. "
            "Raise --max-files deliberately after reviewing runtime impact."
        )
    return files


def unescape_pattern_token(token: str) -> str:
    result: list[str] = []
    index = 0
    while index < len(token):
        if token[index] == "\\" and index + 1 < len(token):
            result.append(token[index + 1])
            index += 2
        else:
            result.append(token[index])
            index += 1
    return "".join(result)


def split_codeowners_line(line: str) -> list[str]:
    """Split on unescaped ASCII whitespace while retaining escaped characters."""
    tokens: list[str] = []
    current: list[str] = []
    escaped = False
    for char in line.rstrip("\n\r"):
        if escaped:
            current.extend(["\\", char])
            escaped = False
            continue
        if char == "\\":
            escaped = True
            continue
        if char in " \t":
            if current:
                tokens.append("".join(current))
                current = []
            continue
        current.append(char)
    if escaped:
        current.append("\\")
    if current:
        tokens.append("".join(current))
    return tokens


def validate_owner_handle(owner: str) -> bool:
    return bool(OWNER_RE.fullmatch(owner)) and not PLACEHOLDER_OWNER_RE.search(owner)


def escape_codeowners_pattern(pattern: str) -> str:
    """Escape spaces and literal # characters in non-leading positions."""
    if pattern.startswith("#"):
        raise ValueError("GitHub cannot represent a CODEOWNERS pattern that starts with '#'")
    return pattern.replace("\\", "\\\\").replace(" ", "\\ ").replace("#", "\\#")


def _glob_to_regex(pattern: str) -> str:
    parts: list[str] = []
    i = 0
    while i < len(pattern):
        char = pattern[i]
        if char == "*":
            if i + 1 < len(pattern) and pattern[i + 1] == "*":
                while i + 1 < len(pattern) and pattern[i + 1] == "*":
                    i += 1
                parts.append(".*")
            else:
                parts.append("[^/]*")
        elif char == "?":
            parts.append("[^/]")
        else:
            parts.append(re.escape(char))
        i += 1
    return "".join(parts)


def compile_codeowners_pattern(raw_pattern: str) -> re.Pattern[str]:
    pattern = unescape_pattern_token(raw_pattern)
    anchored = pattern.startswith("/")
    if anchored:
        pattern = pattern[1:]
    directory = pattern.endswith("/")
    if directory:
        pattern = pattern[:-1]

    has_slash = "/" in pattern
    body = _glob_to_regex(pattern)

    if directory:
        suffix = r"(?:/.*)?"
    else:
        suffix = ""

    if anchored or has_slash:
        expression = rf"^{body}{suffix}$"
    else:
        expression = rf"^(?:.*/)?{body}{suffix}$"
    return re.compile(expression)


def pattern_matches(raw_pattern: str, path: str) -> bool:
    try:
        return bool(compile_codeowners_pattern(raw_pattern).match(path))
    except re.error:
        return False


def active_codeowners_path(repo: Path) -> Path | None:
    for relative in (Path(".github/CODEOWNERS"), Path("CODEOWNERS"), Path("docs/CODEOWNERS")):
        candidate = repo / relative
        if candidate.is_file():
            return candidate
    return None


def is_probably_generated(path: str) -> bool:
    lower = path.lower()
    segments = lower.split("/")
    generated_segments = {
        "dist",
        "build",
        "generated",
        "gen",
        "vendor",
        "coverage",
        "out",
        "target",
        ".next",
        "public/build",
    }
    return any(segment in generated_segments for segment in segments) or any(
        marker in lower
        for marker in (".generated.", "/generated/", ".min.js", ".min.css", ".snap")
    )


def unique_preserve(values: Iterable[str]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if value not in seen:
            seen.add(value)
            result.append(value)
    return result
