#!/usr/bin/env python3
"""Read-only inventory and policy signal extraction for a repository's .github folder."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Iterable

REDUNDANT_TEMPLATE_STEMS = {
    "bug-fix",
    "bugfix",
    "feature",
    "refactor",
    "docs",
    "documentation",
    "test",
    "tests",
    "chore",
}

IGNORE_DIRS = {
    ".git",
    "node_modules",
    "vendor",
    ".venv",
    "venv",
    "dist",
    "build",
    "target",
    ".next",
    ".cache",
}


def run_git(cwd: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git command failed")
    return result.stdout.strip()


def resolve_repo(value: str) -> Path:
    candidate = Path(value).expanduser().resolve()
    if candidate.is_file():
        candidate = candidate.parent
    if not candidate.exists():
        raise ValueError(f"Repository path does not exist: {candidate}")
    root = Path(run_git(candidate, "rev-parse", "--show-toplevel")).resolve()
    if not root.is_dir():
        raise ValueError(f"Resolved Git root is not a directory: {root}")
    return root


def relative_files(root: Path, start: Path, max_files: int = 20000) -> list[str]:
    if not start.exists():
        return []
    files: list[str] = []
    for current, dirs, names in os.walk(start, followlinks=False):
        dirs[:] = sorted(d for d in dirs if d not in IGNORE_DIRS)
        for name in sorted(names):
            path = Path(current) / name
            try:
                relative = path.relative_to(root).as_posix()
            except ValueError:
                continue
            files.append(relative)
            if len(files) >= max_files:
                return files
    return files


def first_existing(root: Path, candidates: Iterable[str]) -> list[str]:
    return [candidate for candidate in candidates if (root / candidate).is_file()]


def directories_named(root: Path, names: set[str], max_depth: int = 5) -> list[str]:
    hits: list[str] = []
    root_parts = len(root.parts)
    for current, dirs, _ in os.walk(root, followlinks=False):
        current_path = Path(current)
        depth = len(current_path.parts) - root_parts
        dirs[:] = [d for d in dirs if d not in IGNORE_DIRS and depth < max_depth]
        for directory in dirs:
            if directory.lower() in names:
                hits.append((current_path / directory).relative_to(root).as_posix())
    return sorted(set(hits))


def workflow_signals(root: Path, github_files: list[str]) -> dict[str, list[str]]:
    workflows = [path for path in github_files if path.startswith(".github/workflows/")]
    release = [
        path
        for path in workflows
        if any(token in Path(path).stem.lower() for token in ("release", "publish", "package", "deploy"))
    ]
    security = [
        path
        for path in workflows
        if any(token in Path(path).stem.lower() for token in ("security", "codeql", "scan", "sast", "dependency-review"))
    ]
    migration = [
        path
        for path in workflows
        if any(token in Path(path).stem.lower() for token in ("migrate", "migration", "schema"))
    ]
    return {"all": workflows, "release": release, "security": security, "migration": migration}


def collect_inventory(root: Path) -> dict[str, object]:
    github_dir = root / ".github"
    github_files = relative_files(root, github_dir)
    all_top_files = relative_files(root, root)

    default_templates = first_existing(
        root,
        [
            ".github/PULL_REQUEST_TEMPLATE.md",
            ".github/pull_request_template.md",
            "PULL_REQUEST_TEMPLATE.md",
            "pull_request_template.md",
            "docs/PULL_REQUEST_TEMPLATE.md",
            "docs/pull_request_template.md",
        ],
    )

    template_dir = github_dir / "PULL_REQUEST_TEMPLATE"
    specialized_templates = []
    redundant_templates = []
    other_templates = []
    if template_dir.is_dir():
        for path in sorted(template_dir.iterdir()):
            if not path.is_file() or path.suffix.lower() not in {".md", ".txt"}:
                continue
            relative = path.relative_to(root).as_posix()
            stem = path.stem.lower().replace("_", "-")
            if stem in {"security-sensitive", "security", "release", "migration"}:
                specialized_templates.append(relative)
            elif stem in REDUNDANT_TEMPLATE_STEMS:
                redundant_templates.append(relative)
            else:
                other_templates.append(relative)

    workflow = workflow_signals(root, github_files)
    security_policy = first_existing(root, ["SECURITY.md", ".github/SECURITY.md", "docs/SECURITY.md"])
    release_docs = first_existing(
        root,
        ["CHANGELOG.md", "RELEASE.md", "RELEASING.md", "docs/RELEASING.md", ".github/release.yml"],
    )
    maintainer_docs = first_existing(
        root,
        ["MAINTAINERS.md", "OWNERS", "OWNERS.md", ".github/MAINTAINERS.md", "CONTRIBUTING.md"],
    )
    codeowners = first_existing(root, [".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS"])

    security_dirs = directories_named(
        root,
        {"auth", "authentication", "authorization", "security", "permissions", "crypto", "cryptography", "sandbox"},
    )
    migration_dirs = directories_named(
        root,
        {"migrations", "migration", "alembic", "schema-migrations", "db-migrations"},
    )
    release_files = [
        path
        for path in all_top_files
        if Path(path).name.lower()
        in {
            "pyproject.toml",
            "package.json",
            "cargo.toml",
            "goreleaser.yml",
            ".goreleaser.yml",
            "dockerfile",
            "docker-bake.hcl",
        }
    ][:30]
    schema_files = [
        path
        for path in all_top_files
        if Path(path).suffix.lower() in {".sql", ".prisma"}
        or Path(path).name.lower() in {"schema.rb", "schema.json", "schema.yaml", "schema.yml"}
    ][:50]

    signals = {
        "security": sorted(set(security_policy + workflow["security"] + security_dirs)),
        "release": sorted(set(release_docs + workflow["release"] + release_files)),
        "migration": sorted(set(workflow["migration"] + migration_dirs + schema_files)),
    }

    recommendations: list[str] = []
    if not default_templates:
        recommendations.append("Create .github/PULL_REQUEST_TEMPLATE.md as the universal PR evidence contract.")
    elif len(default_templates) > 1:
        recommendations.append("Consolidate duplicate default PR template locations around .github/PULL_REQUEST_TEMPLATE.md.")
    if redundant_templates:
        recommendations.append("Review change-class templates for consolidation into the universal default; do not delete automatically.")
    if not (root / ".github/workflows/pr-contract.yml").is_file():
        recommendations.append("Add .github/workflows/pr-contract.yml with read-only PR-body validation.")
    if signals["security"] and not any("security" in item.lower() for item in specialized_templates):
        recommendations.append("Repository signals justify evaluating a security-sensitive PR template.")
    if signals["release"] and not any(Path(item).stem.lower() == "release" for item in specialized_templates):
        recommendations.append("Repository signals justify evaluating a release PR template.")
    if signals["migration"] and not any(Path(item).stem.lower() == "migration" for item in specialized_templates):
        recommendations.append("Repository signals justify evaluating a migration PR template.")
    if not codeowners:
        recommendations.append("Create CODEOWNERS only after concrete GitHub user or team handles are verified.")

    dirty = run_git(root, "status", "--porcelain=v1", "--untracked-files=all").splitlines()
    branch = run_git(root, "branch", "--show-current")

    return {
        "repository_root": str(root),
        "branch": branch or "DETACHED_HEAD",
        "dirty_entries": dirty,
        "github_exists": github_dir.is_dir(),
        "github_files": github_files,
        "default_pr_templates": default_templates,
        "specialized_pr_templates": specialized_templates,
        "redundant_change_class_templates": redundant_templates,
        "other_pr_templates": other_templates,
        "codeowners_files": codeowners,
        "maintainer_files": maintainer_docs,
        "workflow_files": workflow["all"],
        "signals": signals,
        "recommendations": recommendations,
    }


def to_markdown(report: dict[str, object]) -> str:
    lines = [
        "# .github Inventory",
        "",
        f"- Repository root: `{report['repository_root']}`",
        f"- Branch: `{report['branch']}`",
        f"- Dirty entries: {len(report['dirty_entries'])}",
        f"- Existing .github files: {len(report['github_files'])}",
        "",
    ]

    for title, key in (
        ("Default PR templates", "default_pr_templates"),
        ("Specialized PR templates", "specialized_pr_templates"),
        ("Redundant change-class templates", "redundant_change_class_templates"),
        ("CODEOWNERS files", "codeowners_files"),
        ("Workflows", "workflow_files"),
    ):
        values = report[key]
        lines.append(f"## {title}")
        lines.extend(f"- `{value}`" for value in values) if values else lines.append("- None")
        lines.append("")

    lines.append("## Repository signals")
    for profile, values in report["signals"].items():
        lines.append(f"### {profile.title()}")
        lines.extend(f"- `{value}`" for value in values) if values else lines.append("- None")
    lines.append("")

    lines.append("## Recommendations")
    recommendations = report["recommendations"]
    lines.extend(f"- {value}" for value in recommendations) if recommendations else lines.append("- No structural recommendations.")
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Path inside the target Git repository.")
    parser.add_argument("--format", choices=("json", "markdown"), default="json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        root = resolve_repo(args.repo)
        report = collect_inventory(root)
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"inventory error: {exc}", file=sys.stderr)
        return 2

    if args.format == "markdown":
        print(to_markdown(report), end="")
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
