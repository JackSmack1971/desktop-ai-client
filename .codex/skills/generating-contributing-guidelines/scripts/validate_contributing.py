#!/usr/bin/env python3
"""Validate a repository-root CONTRIBUTING.md without network access or writes."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlparse

PLACEHOLDER_PATTERNS = {
    "template project name": re.compile(r"\{+\s*project(?:[-_ ]?name)?\s*\}+", re.IGNORECASE),
    "insert marker": re.compile(r"\b(?:insert|replace)[-_ ](?:link|url|command|name|path|version|here)\b", re.IGNORECASE),
    "example username": re.compile(r"\byour[-_ ]username\b", re.IGNORECASE),
    "example project": re.compile(r"\byour[-_ ]project\b", re.IGNORECASE),
    "angle-bracket placeholder": re.compile(r"<(?:your|insert|project|owner|username|command|version|path)[^>]*>", re.IGNORECASE),
    "unfinished marker": re.compile(r"(?m)^\s*(?:TODO|TBD|FIXME)\s*:\s*.+$", re.IGNORECASE),
    "mustache placeholder": re.compile(r"\{\{[^{}]+\}\}"),
}

REQUIRED_THEME_PATTERNS = {
    "local setup or prerequisites": re.compile(r"(?im)^#{2,4}\s+.*(?:setup|prerequisite|development environment|getting started)"),
    "quality validation": re.compile(r"(?im)^#{2,4}\s+.*(?:test|quality|validation|checks|standards)"),
    "pull request workflow": re.compile(r"(?im)^#{2,4}\s+.*(?:pull request|submitting changes|submission workflow)"),
}

FENCE_PATTERN = re.compile(r"```(?:bash|sh|shell|console|powershell|pwsh|zsh)?\s*\n(.*?)```", re.IGNORECASE | re.DOTALL)
LINK_PATTERN = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def run_git(root: Path, args: list[str]) -> tuple[int, str]:
    process = subprocess.run(
        ["git", "-C", str(root), *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )
    return process.returncode, process.stdout.strip()


def resolve_root(raw_root: str) -> Path:
    candidate = Path(raw_root).expanduser().resolve()
    if not candidate.is_dir():
        raise ValueError(f"Repository root is not a directory: {candidate}")
    code, detected = run_git(candidate, ["rev-parse", "--show-toplevel"])
    if code != 0 or not detected:
        raise ValueError(f"No Git repository found at or above: {candidate}")
    root = Path(detected).resolve()
    if root != candidate:
        raise ValueError(f"Pass the exact repository root: {root}")
    return root


def package_scripts(root: Path) -> dict[str, str]:
    path = root / "package.json"
    if not path.is_file():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    scripts = data.get("scripts")
    if not isinstance(scripts, dict):
        return {}
    return {str(key): str(value) for key, value in scripts.items()}


def make_targets(root: Path) -> set[str]:
    path = next((root / name for name in ("Makefile", "makefile", "GNUmakefile") if (root / name).is_file()), None)
    if path is None:
        return set()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return set()
    return {
        match.group(1)
        for line in text.splitlines()
        if (match := re.match(r"^([A-Za-z0-9][A-Za-z0-9_.-]*):(?:\s|$)", line))
    }


def just_targets(root: Path) -> set[str]:
    path = next((root / name for name in ("justfile", "Justfile") if (root / name).is_file()), None)
    if path is None:
        return set()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return set()
    result: set[str] = set()
    for line in text.splitlines():
        if line.startswith((" ", "\t", "#", "@", "[")):
            continue
        match = re.match(r"^([A-Za-z0-9][A-Za-z0-9_-]*)(?:\s+[^:]*)?:", line)
        if match:
            result.add(match.group(1))
    return result


def validate_links(root: Path, source: Path, text: str) -> list[str]:
    errors: list[str] = []
    for raw_target in LINK_PATTERN.findall(text):
        target = raw_target.strip().split(maxsplit=1)[0].strip("<>")
        if not target or target.startswith("#"):
            continue
        parsed = urlparse(target)
        if parsed.scheme in {"http", "https", "mailto"} or target.startswith("//"):
            continue
        relative_part = unquote(target.split("#", 1)[0].split("?", 1)[0])
        if not relative_part:
            continue
        candidate = (source.parent / relative_part).resolve()
        try:
            candidate.relative_to(root)
        except ValueError:
            errors.append(f"Relative link escapes repository root: {target}")
            continue
        if not candidate.exists():
            errors.append(f"Broken relative link: {target}")
    return errors


def extract_commands(text: str) -> list[str]:
    commands: list[str] = []
    for block in FENCE_PATTERN.findall(text):
        for raw_line in block.splitlines():
            line = raw_line.strip()
            if not line or line.startswith(("#", "$", ">")):
                continue
            if line.endswith("\\"):
                line = line[:-1].strip()
            commands.append(line)
    return commands


def validate_task_commands(root: Path, commands: list[str]) -> list[str]:
    errors: list[str] = []
    scripts = package_scripts(root)
    make = make_targets(root)
    just = just_targets(root)

    package_patterns = [
        re.compile(r"^(?:npm\s+run|pnpm\s+(?:run\s+)?|yarn\s+(?:run\s+)?|bun\s+run)\s+([A-Za-z0-9:_-]+)(?:\s|$)"),
        re.compile(r"^npm\s+(test|start|stop|restart)(?:\s|$)"),
    ]
    for command in commands:
        stripped = re.sub(r"^(?:sudo\s+)?", "", command)
        for pattern in package_patterns:
            match = pattern.match(stripped)
            if match:
                script = match.group(1)
                if script not in scripts:
                    errors.append(f"Documented package script is not defined in root package.json: {script}")
                break
        make_match = re.match(r"^make\s+([A-Za-z0-9_.-]+)(?:\s|$)", stripped)
        if make_match and make and make_match.group(1) not in make:
            errors.append(f"Documented Make target is not defined: {make_match.group(1)}")
        just_match = re.match(r"^just\s+([A-Za-z0-9_-]+)(?:\s|$)", stripped)
        if just_match and just and just_match.group(1) not in just:
            errors.append(f"Documented Just recipe is not defined: {just_match.group(1)}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, help="Exact Git repository root")
    parser.add_argument("--file", required=True, help="CONTRIBUTING.md path")
    args = parser.parse_args()

    errors: list[str] = []
    warnings: list[str] = []
    facts: dict[str, Any] = {}

    try:
        root = resolve_root(args.root)
    except ValueError as exc:
        print(json.dumps({"ok": False, "errors": [str(exc)], "warnings": []}, indent=2))
        return 2

    target = Path(args.file).expanduser().resolve()
    expected = (root / "CONTRIBUTING.md").resolve()
    if target != expected:
        errors.append(f"Target must be the repository-root CONTRIBUTING.md: {expected}")
    if not target.is_file():
        errors.append(f"File does not exist: {target}")
        print(json.dumps({"ok": False, "errors": errors, "warnings": warnings}, indent=2))
        return 1

    try:
        text = target.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        errors.append(f"Unable to read UTF-8 Markdown: {exc}")
        print(json.dumps({"ok": False, "errors": errors, "warnings": warnings}, indent=2))
        return 1

    facts["character_count"] = len(text)
    facts["line_count"] = len(text.splitlines())
    facts["command_count"] = len(extract_commands(text))

    if len(text.strip()) < 500:
        errors.append("CONTRIBUTING.md is too short to provide an actionable contributor workflow")
    first_content = next((line.strip() for line in text.splitlines() if line.strip()), "")
    if not re.match(r"^#\s+Contributing\b", first_content, re.IGNORECASE):
        errors.append("First content line must be an H1 beginning with 'Contributing'")

    for label, pattern in PLACEHOLDER_PATTERNS.items():
        if pattern.search(text):
            errors.append(f"Unresolved {label} detected")

    for theme, pattern in REQUIRED_THEME_PATTERNS.items():
        if not pattern.search(text):
            errors.append(f"Missing required contributor theme: {theme}")

    errors.extend(validate_links(root, target, text))
    commands = extract_commands(text)
    errors.extend(validate_task_commands(root, commands))

    lower = text.lower()
    policy_checks = [
        ((root / "SECURITY.md").is_file() or (root / ".github/SECURITY.md").is_file(), "security", "Repository has a security policy but CONTRIBUTING.md does not reference security reporting"),
        ((root / "CODE_OF_CONDUCT.md").is_file() or (root / ".github/CODE_OF_CONDUCT.md").is_file(), "code of conduct", "Repository has a Code of Conduct but CONTRIBUTING.md does not reference it"),
    ]
    for present, needle, message in policy_checks:
        if present and needle not in lower:
            warnings.append(message)

    has_license = any(path.is_file() for path in (root / "LICENSE", root / "LICENSE.md", root / "LICENSE.txt", root / "COPYING"))
    if has_license and "license" not in lower:
        warnings.append("Repository has a license file but CONTRIBUTING.md does not mention contribution licensing")

    code, diff_names = run_git(root, ["diff", "--name-only"])
    if code == 0:
        facts["unstaged_changed_files"] = diff_names.splitlines() if diff_names else []
    code, cached_names = run_git(root, ["diff", "--cached", "--name-only"])
    if code == 0:
        facts["staged_changed_files"] = cached_names.splitlines() if cached_names else []
        if "CONTRIBUTING.md" in facts["staged_changed_files"]:
            errors.append("CONTRIBUTING.md is staged; the skill must not stage files")

    result = {
        "ok": not errors,
        "file": str(target),
        "errors": sorted(set(errors)),
        "warnings": sorted(set(warnings)),
        "facts": facts,
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    sys.exit(main())
