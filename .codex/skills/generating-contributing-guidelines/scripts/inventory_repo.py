#!/usr/bin/env python3
"""Create a bounded, read-only repository inventory for CONTRIBUTING.md generation."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any

MAX_TRACKED_FILES = 50000
MAX_COMMITS = 80
MAX_CI_COMMANDS = 160

SENSITIVE_NAMES = {
    ".env",
    ".env.local",
    ".env.production",
    ".env.development",
    "id_rsa",
    "id_ed25519",
    "credentials.json",
    "secrets.json",
}

SIGNAL_NAMES = {
    "readme.md",
    "readme.rst",
    "readme.txt",
    "package.json",
    "pyproject.toml",
    "cargo.toml",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "gemfile",
    "composer.json",
    "makefile",
    "justfile",
    "taskfile.yml",
    "taskfile.yaml",
    "dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
    ".editorconfig",
    ".pre-commit-config.yaml",
    "biome.json",
    "biome.jsonc",
    "eslint.config.js",
    "eslint.config.mjs",
    "eslint.config.cjs",
    "ruff.toml",
    "pytest.ini",
    "tox.ini",
    "noxfile.py",
    "vitest.config.ts",
    "jest.config.js",
    "jest.config.ts",
    "codeowners",
    "claude.md",
    "agents.md",
}

LOCKFILE_MANAGERS = {
    "pnpm-lock.yaml": "pnpm",
    "yarn.lock": "yarn",
    "package-lock.json": "npm",
    "npm-shrinkwrap.json": "npm",
    "bun.lock": "bun",
    "bun.lockb": "bun",
    "uv.lock": "uv",
    "poetry.lock": "poetry",
    "pdm.lock": "pdm",
    "pipfile.lock": "pipenv",
    "cargo.lock": "cargo",
    "go.sum": "go",
    "gemfile.lock": "bundler",
    "composer.lock": "composer",
}

EXTENSION_LANGUAGES = {
    ".ts": "TypeScript",
    ".tsx": "TypeScript",
    ".js": "JavaScript",
    ".jsx": "JavaScript",
    ".mjs": "JavaScript",
    ".cjs": "JavaScript",
    ".py": "Python",
    ".rs": "Rust",
    ".go": "Go",
    ".java": "Java",
    ".kt": "Kotlin",
    ".kts": "Kotlin",
    ".rb": "Ruby",
    ".php": "PHP",
    ".cs": "C#",
    ".cpp": "C++",
    ".cc": "C++",
    ".c": "C",
    ".h": "C/C++",
    ".swift": "Swift",
    ".scala": "Scala",
    ".sh": "Shell",
    ".ps1": "PowerShell",
    ".md": "Markdown",
    ".rst": "reStructuredText",
}


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


def tracked_files(root: Path) -> tuple[list[str], bool]:
    process = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if process.returncode != 0:
        raise RuntimeError(process.stderr.decode("utf-8", errors="replace").strip())
    entries = [item.decode("utf-8", errors="replace") for item in process.stdout.split(b"\0") if item]
    truncated = len(entries) > MAX_TRACKED_FILES
    return entries[:MAX_TRACKED_FILES], truncated


def safe_text(root: Path, relative: str, limit: int = 200_000) -> str | None:
    path = (root / relative).resolve()
    try:
        path.relative_to(root)
    except ValueError:
        return None
    if path.name.lower() in SENSITIVE_NAMES or path.is_symlink() or not path.is_file():
        return None
    try:
        if path.stat().st_size > limit:
            return None
        data = path.read_bytes()
        if b"\x00" in data:
            return None
        return data.decode("utf-8", errors="replace")
    except OSError:
        return None


def parse_package_json(root: Path, files: set[str]) -> dict[str, Any] | None:
    if "package.json" not in files:
        return None
    text = safe_text(root, "package.json")
    if text is None:
        return None
    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        return {"parse_error": str(exc)}
    scripts = data.get("scripts") if isinstance(data.get("scripts"), dict) else {}
    workspaces = data.get("workspaces")
    return {
        "name": data.get("name"),
        "private": data.get("private"),
        "package_manager": data.get("packageManager"),
        "engines": data.get("engines") if isinstance(data.get("engines"), dict) else {},
        "scripts": dict(sorted((str(k), str(v)) for k, v in scripts.items())),
        "workspaces": workspaces,
    }


def parse_pyproject(root: Path, files: set[str]) -> dict[str, Any] | None:
    if "pyproject.toml" not in files:
        return None
    try:
        import tomllib
    except ModuleNotFoundError:
        return {"present": True, "parse_error": "Python 3.11+ is required for TOML parsing"}
    path = root / "pyproject.toml"
    try:
        with path.open("rb") as handle:
            data = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        return {"present": True, "parse_error": str(exc)}
    project = data.get("project") if isinstance(data.get("project"), dict) else {}
    tool = data.get("tool") if isinstance(data.get("tool"), dict) else {}
    return {
        "project_name": project.get("name"),
        "requires_python": project.get("requires-python"),
        "build_system": data.get("build-system"),
        "tool_sections": sorted(str(key) for key in tool.keys()),
    }


def parse_cargo(root: Path, files: set[str]) -> dict[str, Any] | None:
    if "Cargo.toml" not in files:
        return None
    try:
        import tomllib
    except ModuleNotFoundError:
        return {"present": True, "parse_error": "Python 3.11+ is required for TOML parsing"}
    try:
        with (root / "Cargo.toml").open("rb") as handle:
            data = tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        return {"present": True, "parse_error": str(exc)}
    package = data.get("package") if isinstance(data.get("package"), dict) else {}
    workspace = data.get("workspace") if isinstance(data.get("workspace"), dict) else {}
    return {
        "name": package.get("name"),
        "edition": package.get("edition"),
        "rust_version": package.get("rust-version"),
        "workspace_members": workspace.get("members"),
    }


def parse_go_mod(root: Path, files: set[str]) -> dict[str, Any] | None:
    if "go.mod" not in files:
        return None
    text = safe_text(root, "go.mod") or ""
    module = re.search(r"(?m)^module\s+(.+)$", text)
    version = re.search(r"(?m)^go\s+([0-9.]+)$", text)
    return {
        "module": module.group(1).strip() if module else None,
        "go_version": version.group(1) if version else None,
    }


def extract_make_targets(root: Path, files: set[str]) -> list[str]:
    name = next((item for item in ("Makefile", "makefile", "GNUmakefile") if item in files), None)
    if name is None:
        return []
    text = safe_text(root, name) or ""
    targets: list[str] = []
    for line in text.splitlines():
        match = re.match(r"^([A-Za-z0-9][A-Za-z0-9_.-]*):(?:\s|$)", line)
        if match and not match.group(1).startswith("."):
            targets.append(match.group(1))
    return sorted(set(targets))[:200]


def extract_just_targets(root: Path, files: set[str]) -> list[str]:
    name = next((item for item in ("justfile", "Justfile") if item in files), None)
    if name is None:
        return []
    text = safe_text(root, name) or ""
    targets: list[str] = []
    for line in text.splitlines():
        if line.startswith((" ", "\t", "#", "@", "[")):
            continue
        match = re.match(r"^([A-Za-z0-9][A-Za-z0-9_-]*)(?:\s+[^:]*)?:", line)
        if match:
            targets.append(match.group(1))
    return sorted(set(targets))[:200]


def extract_ci_commands(root: Path, paths: list[str]) -> list[dict[str, str]]:
    commands: list[dict[str, str]] = []
    for relative in sorted(paths):
        text = safe_text(root, relative, limit=500_000)
        if text is None:
            continue
        lines = text.splitlines()
        index = 0
        while index < len(lines):
            line = lines[index]
            match = re.match(r"^(\s*)(?:-\s*)?run:\s*(.*)$", line)
            if not match:
                index += 1
                continue
            indent = len(match.group(1))
            value = match.group(2).strip()
            if value and value not in {"|", ">", "|-", ">-"}:
                commands.append({"file": relative, "command": value})
                index += 1
                continue
            block: list[str] = []
            index += 1
            while index < len(lines):
                following = lines[index]
                if not following.strip():
                    block.append("")
                    index += 1
                    continue
                current_indent = len(following) - len(following.lstrip())
                if current_indent <= indent:
                    break
                block.append(following.strip())
                index += 1
            command = "\n".join(block).strip()
            if command:
                commands.append({"file": relative, "command": command})
            if len(commands) >= MAX_CI_COMMANDS:
                return commands
    return commands


def runtime_files(root: Path, files: set[str]) -> dict[str, str]:
    candidates = [
        ".nvmrc",
        ".node-version",
        ".python-version",
        ".ruby-version",
        ".tool-versions",
        "rust-toolchain",
        "rust-toolchain.toml",
        "mise.toml",
    ]
    result: dict[str, str] = {}
    for relative in candidates:
        if relative not in files:
            continue
        text = safe_text(root, relative, limit=20_000)
        if text is not None:
            result[relative] = text.strip()[:4000]
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, help="Exact Git repository root")
    args = parser.parse_args()

    try:
        root = resolve_root(args.root)
        tracked, truncated = tracked_files(root)
    except (ValueError, RuntimeError) as exc:
        print(json.dumps({"ok": False, "error": str(exc)}, indent=2))
        return 2

    file_set = set(tracked)
    lower_map = {path.lower(): path for path in tracked}
    extension_counts: Counter[str] = Counter()
    language_counts: Counter[str] = Counter()
    for relative in tracked:
        suffix = Path(relative).suffix.lower()
        if suffix:
            extension_counts[suffix] += 1
            language = EXTENSION_LANGUAGES.get(suffix)
            if language:
                language_counts[language] += 1

    signal_files: list[str] = []
    policy_files: list[str] = []
    workflow_files: list[str] = []
    issue_template_files: list[str] = []
    pr_template_files: list[str] = []
    test_files: list[str] = []

    for relative in tracked:
        path = Path(relative)
        lower = relative.lower()
        name = path.name.lower()
        if name in SENSITIVE_NAMES:
            continue
        if name in SIGNAL_NAMES or name.startswith(("readme", "license", "contributing", "security", "code_of_conduct")):
            signal_files.append(relative)
        if name.startswith(("contributing", "security", "code_of_conduct", "support", "governance", "maintainers")) or name in {"codeowners", "pull_request_template.md"}:
            policy_files.append(relative)
        if lower.startswith(".github/workflows/") and path.suffix.lower() in {".yml", ".yaml"}:
            workflow_files.append(relative)
        if lower.startswith(".github/issue_template/"):
            issue_template_files.append(relative)
        if name.startswith("pull_request_template") or "/pull_request_template/" in lower:
            pr_template_files.append(relative)
        if re.search(r"(^|/)(test|tests|spec|specs)(/|$)", lower) or re.search(r"(?:^|[._-])(test|spec)\.[^.]+$", name):
            test_files.append(relative)

    lockfiles = [relative for relative in tracked if Path(relative).name.lower() in LOCKFILE_MANAGERS]
    managers = sorted({LOCKFILE_MANAGERS[Path(relative).name.lower()] for relative in lockfiles})

    code, current_branch = run_git(root, ["branch", "--show-current"])
    if code != 0:
        current_branch = ""
    code, remote_head = run_git(root, ["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"])
    default_branch = remote_head.removeprefix("origin/") if code == 0 and remote_head else None
    code, status = run_git(root, ["status", "--short"])
    status_lines = status.splitlines() if code == 0 and status else []
    code, log = run_git(root, ["log", f"-{MAX_COMMITS}", "--pretty=format:%s"])
    commit_subjects = log.splitlines() if code == 0 and log else []

    inventory = {
        "ok": True,
        "repository_root": str(root),
        "tracked_file_count_examined": len(tracked),
        "tracked_file_list_truncated": truncated,
        "current_branch": current_branch or None,
        "default_branch_from_local_origin_head": default_branch,
        "pre_existing_status": status_lines,
        "top_languages_by_tracked_files": language_counts.most_common(12),
        "top_extensions": extension_counts.most_common(20),
        "package_managers_from_lockfiles": managers,
        "lockfiles": sorted(lockfiles),
        "runtime_version_files": runtime_files(root, file_set),
        "manifests": {
            "package_json": parse_package_json(root, file_set),
            "pyproject": parse_pyproject(root, file_set),
            "cargo": parse_cargo(root, file_set),
            "go_mod": parse_go_mod(root, file_set),
        },
        "task_runners": {
            "make_targets": extract_make_targets(root, file_set),
            "just_targets": extract_just_targets(root, file_set),
        },
        "high_signal_files": sorted(set(signal_files))[:500],
        "policy_and_governance_files": sorted(set(policy_files))[:300],
        "ci_workflows": sorted(workflow_files)[:300],
        "ci_run_commands": extract_ci_commands(root, workflow_files),
        "issue_templates": sorted(issue_template_files)[:300],
        "pull_request_templates": sorted(pr_template_files)[:100],
        "representative_test_files": sorted(test_files)[:120],
        "recent_commit_subjects": commit_subjects,
        "root_directory_entries": sorted({Path(item).parts[0] for item in tracked if Path(item).parts})[:300],
        "notes": [
            "Inventory is read-only and bounded.",
            "File presence is not proof of policy; read relevant files directly.",
            "Default branch is reported only when local origin/HEAD metadata exists.",
            "No network access or secret-bearing file reads were performed.",
        ],
    }
    print(json.dumps(inventory, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
