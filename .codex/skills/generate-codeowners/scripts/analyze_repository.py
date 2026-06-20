#!/usr/bin/env python3
"""Build a bounded, machine-readable inventory for CODEOWNERS design."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

from codeowners_common import (
    active_codeowners_path,
    atomic_write_json,
    git_path,
    is_probably_generated,
    list_tracked_files,
    parse_github_remote,
    resolve_repo,
    run,
)

LANGUAGE_BY_SUFFIX = {
    ".py": "Python",
    ".pyi": "Python",
    ".js": "JavaScript",
    ".jsx": "JavaScript",
    ".mjs": "JavaScript",
    ".cjs": "JavaScript",
    ".ts": "TypeScript",
    ".tsx": "TypeScript",
    ".go": "Go",
    ".rs": "Rust",
    ".java": "Java",
    ".kt": "Kotlin",
    ".kts": "Kotlin",
    ".cs": "C#",
    ".cpp": "C++",
    ".cc": "C++",
    ".cxx": "C++",
    ".c": "C",
    ".h": "C/C++ Header",
    ".hpp": "C/C++ Header",
    ".rb": "Ruby",
    ".php": "PHP",
    ".swift": "Swift",
    ".scala": "Scala",
    ".sh": "Shell",
    ".ps1": "PowerShell",
    ".sql": "SQL",
    ".tf": "Terraform",
    ".hcl": "HCL",
    ".yaml": "YAML",
    ".yml": "YAML",
    ".json": "JSON",
    ".md": "Markdown",
    ".proto": "Protocol Buffers",
    ".graphql": "GraphQL",
    ".gql": "GraphQL",
}

MANIFEST_NAMES = {
    "package.json",
    "pnpm-workspace.yaml",
    "pnpm-workspace.yml",
    "yarn.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "Cargo.toml",
    "Cargo.lock",
    "go.mod",
    "go.work",
    "pyproject.toml",
    "requirements.txt",
    "Pipfile",
    "poetry.lock",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "Gemfile",
    "composer.json",
    "Makefile",
    "CMakeLists.txt",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
}

GOVERNANCE_NAMES = {
    "CODEOWNERS",
    "OWNERS",
    "MAINTAINERS",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "GOVERNANCE.md",
    "SUPPORT.md",
    "catalog-info.yaml",
    "catalog-info.yml",
}

RISK_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("ci-cd", re.compile(r"^(?:\.github/workflows/|\.github/actions/|\.gitlab-ci\.yml$|Jenkinsfile$|\.circleci/)")),
    ("release", re.compile(r"(?:^|/)(?:release|releases|publishing|publish|changesets?)(?:/|\.|$)", re.I)),
    ("infrastructure", re.compile(r"(?:^|/)(?:infra|infrastructure|terraform|k8s|kubernetes|helm|deploy|deployment)(?:/|$)", re.I)),
    ("database", re.compile(r"(?:^|/)(?:db|database|migrations?|schema)(?:/|$)|\.sql$", re.I)),
    ("authentication-authorization", re.compile(r"(?:^|/)(?:auth|authentication|authorization|iam|rbac|permissions?|policy)(?:/|$)", re.I)),
    ("cryptography-signing", re.compile(r"(?:^|/)(?:crypto|cryptography|signing|certificates?|pki)(?:/|$)", re.I)),
    ("security", re.compile(r"(?:^|/)(?:security|secrets?|vulnerability|sast|dast)(?:/|$)|SECURITY\.md$", re.I)),
    ("public-api-contract", re.compile(r"(?:^|/)(?:api|contracts?|protocols?|schemas?|openapi)(?:/|$)|\.(?:proto|graphql|gql)$", re.I)),
    ("dependency-toolchain", re.compile(r"(?:^|/)(?:package\.json|.*lock.*|Cargo\.toml|go\.mod|pyproject\.toml|pom\.xml|build\.gradle(?:\.kts)?|Makefile)$", re.I)),
    ("ownership-policy", re.compile(r"(?:^|/)CODEOWNERS$")),
]

DOMAIN_ROOTS = {"apps", "services", "packages", "libs", "modules", "components", "plugins", "crates", "cmd"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Path inside the target Git worktree")
    parser.add_argument("--output", help="JSON output path; defaults to the Git metadata state directory")
    parser.add_argument(
        "--max-files",
        type=int,
        default=50000,
        help="Safety bound for tracked files. The default keeps analysis responsive on typical repositories.",
    )
    parser.add_argument(
        "--max-commits",
        type=int,
        default=2000,
        help="Maximum recent commits used for domain concentration. The default balances signal with bounded runtime.",
    )
    return parser.parse_args()


def read_json_if_small(path: Path, max_bytes: int = 2_000_000) -> Any | None:
    try:
        if path.stat().st_size > max_bytes:
            return None
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return None


def detect_workspace(repo: Path, files: set[str]) -> dict[str, Any]:
    signals: list[str] = []
    workspace_members: list[str] = []

    package_path = repo / "package.json"
    if "package.json" in files:
        data = read_json_if_small(package_path)
        if isinstance(data, dict) and "workspaces" in data:
            signals.append("package.json workspaces")
            value = data.get("workspaces")
            if isinstance(value, list):
                workspace_members.extend(str(item) for item in value)
            elif isinstance(value, dict) and isinstance(value.get("packages"), list):
                workspace_members.extend(str(item) for item in value["packages"])

    for name, signal in (
        ("pnpm-workspace.yaml", "pnpm workspace"),
        ("pnpm-workspace.yml", "pnpm workspace"),
        ("go.work", "Go workspace"),
        ("lerna.json", "Lerna workspace"),
        ("nx.json", "Nx workspace"),
        ("turbo.json", "Turborepo workspace"),
    ):
        if name in files:
            signals.append(signal)

    if "Cargo.toml" in files:
        try:
            cargo_text = (repo / "Cargo.toml").read_text(encoding="utf-8")
            if re.search(r"^\s*\[workspace\]\s*$", cargo_text, re.MULTILINE):
                signals.append("Cargo workspace")
        except (OSError, UnicodeDecodeError):
            pass

    return {"detected": bool(signals), "signals": signals, "declared_members": workspace_members}


def build_domains(files: list[str]) -> list[dict[str, Any]]:
    domain_files: dict[str, list[str]] = collections.defaultdict(list)
    for file_path in files:
        parts = file_path.split("/")
        if len(parts) >= 2 and parts[0] in DOMAIN_ROOTS:
            domain = "/".join(parts[:2]) + "/"
        elif len(parts) >= 2:
            domain = parts[0] + "/"
        else:
            domain = "<root>"
        domain_files[domain].append(file_path)

    domains: list[dict[str, Any]] = []
    for domain, members in sorted(domain_files.items(), key=lambda item: (-len(item[1]), item[0])):
        languages = collections.Counter(
            LANGUAGE_BY_SUFFIX.get(Path(path).suffix.lower(), "Other") for path in members
        )
        risks = sorted(
            {
                category
                for path in members
                for category, pattern in RISK_PATTERNS
                if pattern.search(path)
            }
        )
        domains.append(
            {
                "path": domain,
                "tracked_files": len(members),
                "dominant_languages": [
                    {"language": language, "files": count}
                    for language, count in languages.most_common(5)
                    if language != "Other"
                ],
                "risk_categories": risks,
                "generated_files": sum(1 for path in members if is_probably_generated(path)),
            }
        )
    return domains


def history_concentration(repo: Path, max_commits: int, domains: list[dict[str, Any]]) -> dict[str, Any]:
    domain_paths = [item["path"] for item in domains]
    author_commits: collections.Counter[str] = collections.Counter()
    domain_authors: dict[str, collections.Counter[str]] = {
        path: collections.Counter() for path in domain_paths
    }

    proc = run(
        [
            "git",
            "log",
            f"--max-count={max_commits}",
            "--format=@@AUTHOR@@%aN",
            "--name-only",
            "--no-renames",
        ],
        repo,
        check=False,
    )
    if proc.returncode != 0:
        return {"available": False, "reason": proc.stderr.strip() or "git log failed"}

    current_author: str | None = None
    touched_by_commit: set[str] = set()
    commits_seen = 0

    def flush() -> None:
        nonlocal touched_by_commit, current_author, commits_seen
        if current_author is None:
            return
        author_commits[current_author] += 1
        for domain in touched_by_commit:
            domain_authors[domain][current_author] += 1
        commits_seen += 1
        touched_by_commit = set()

    for raw_line in proc.stdout.splitlines():
        if raw_line.startswith("@@AUTHOR@@"):
            flush()
            current_author = raw_line[len("@@AUTHOR@@") :].strip() or "Unknown author"
            continue
        path = raw_line.strip()
        if not path or current_author is None:
            continue
        for domain in domain_paths:
            if domain == "<root>":
                if "/" not in path:
                    touched_by_commit.add(domain)
            elif path.startswith(domain):
                touched_by_commit.add(domain)
    flush()

    domain_summary: list[dict[str, Any]] = []
    for domain, counts in domain_authors.items():
        total = sum(counts.values())
        leaders = counts.most_common(5)
        concentration = (leaders[0][1] / total) if total and leaders else 0.0
        domain_summary.append(
            {
                "path": domain,
                "commit_touches": total,
                "top_authors": [{"name": name, "touches": count} for name, count in leaders],
                "dominant_author_share": round(concentration, 4),
                "single_point_risk": bool(total >= 5 and concentration >= 0.75),
            }
        )

    return {
        "available": True,
        "commits_analyzed": commits_seen,
        "max_commits": max_commits,
        "repository_authors": [
            {"name": name, "commits": count} for name, count in author_commits.most_common(20)
        ],
        "domains": sorted(domain_summary, key=lambda item: item["path"]),
        "note": "Author names are boundary evidence only and must not be converted into GitHub handles.",
    }


def classify_archetype(
    files: list[str], domains: list[dict[str, Any]], workspace: dict[str, Any]
) -> dict[str, Any]:
    top_dirs = {path.split("/", 1)[0] for path in files if "/" in path}
    service_count = sum(1 for item in domains if item["path"].startswith("services/"))
    app_count = sum(1 for item in domains if item["path"].startswith("apps/"))
    package_count = sum(
        1
        for item in domains
        if item["path"].startswith(("packages/", "libs/", "modules/", "crates/", "plugins/"))
    )
    infra_files = sum(
        1
        for path in files
        if any(pattern.search(path) for category, pattern in RISK_PATTERNS if category == "infrastructure")
    )
    source_like = sum(
        1
        for path in files
        if Path(path).suffix.lower() in LANGUAGE_BY_SUFFIX
        and LANGUAGE_BY_SUFFIX[Path(path).suffix.lower()] not in {"Markdown", "JSON", "YAML"}
    )
    infrastructure_ratio = infra_files / max(1, len(files))

    signals: list[str] = []
    if workspace["detected"] and (service_count + app_count + package_count >= 6):
        archetype = "enterprise-monorepo"
        signals.extend(workspace["signals"])
        signals.append(f"{service_count + app_count + package_count} app/service/package domains")
    elif infrastructure_ratio >= 0.20 and app_count <= 1:
        archetype = "internal-platform"
        signals.append(f"infrastructure files are {infrastructure_ratio:.1%} of tracked files")
    elif service_count + app_count >= 2:
        archetype = "modular-application"
        signals.append(f"{service_count} service domains and {app_count} application domains")
    elif package_count >= 1 and app_count == 0 and service_count == 0:
        archetype = "focused-library"
        signals.append(f"{package_count} library/package domains without application domains")
    elif len(files) >= 1000 or len(top_dirs) >= 8:
        archetype = "modular-application"
        signals.append(f"repository scale: {len(files)} tracked files across {len(top_dirs)} top-level directories")
    else:
        archetype = "small-or-mixed"
        signals.append("insufficient structural evidence for a stronger classification")

    oss_signals = [
        name
        for name in ("CONTRIBUTING.md", "SECURITY.md", "CODE_OF_CONDUCT.md", "LICENSE", "LICENSE.md")
        if name in files
    ]
    return {
        "primary": archetype,
        "signals": signals,
        "open_source_signals": oss_signals,
        "open_source_status_requires_github_verification": True,
        "metrics": {
            "service_domains": service_count,
            "application_domains": app_count,
            "package_domains": package_count,
            "infrastructure_ratio": round(infrastructure_ratio, 4),
            "source_like_files": source_like,
        },
    }


def main() -> int:
    args = parse_args()
    try:
        repo = resolve_repo(args.repo)
        files = list_tracked_files(repo, args.max_files)
        file_set = set(files)

        remote_proc = run(["git", "remote", "get-url", "origin"], repo, check=False)
        remote_url = remote_proc.stdout.strip() if remote_proc.returncode == 0 else ""
        parsed_remote = parse_github_remote(remote_url) if remote_url else None

        manifests = sorted(path for path in files if Path(path).name in MANIFEST_NAMES)
        governance = sorted(path for path in files if Path(path).name in GOVERNANCE_NAMES)
        domains = build_domains(files)
        workspace = detect_workspace(repo, file_set)
        active = active_codeowners_path(repo)

        risk_paths: dict[str, list[str]] = collections.defaultdict(list)
        for path in files:
            for category, pattern in RISK_PATTERNS:
                if pattern.search(path):
                    risk_paths[category].append(path)

        existing: dict[str, Any] | None = None
        if active:
            content = active.read_bytes()
            existing = {
                "active_path": active.relative_to(repo).as_posix(),
                "size_bytes": len(content),
                "sha256": hashlib.sha256(content).hexdigest(),
                "all_locations": [
                    path
                    for path in (".github/CODEOWNERS", "CODEOWNERS", "docs/CODEOWNERS")
                    if (repo / path).is_file()
                ],
            }

        total_commits_proc = run(["git", "rev-list", "--count", "HEAD"], repo, check=False)
        total_commits = (
            int(total_commits_proc.stdout.strip())
            if total_commits_proc.returncode == 0 and total_commits_proc.stdout.strip().isdigit()
            else None
        )

        payload: dict[str, Any] = {
            "schema_version": 1,
            "repository": {
                "root": str(repo),
                "name": repo.name,
                "origin_url": remote_url or None,
                "github": (
                    {"owner": parsed_remote[0], "repository": parsed_remote[1]}
                    if parsed_remote
                    else None
                ),
                "tracked_files": len(files),
                "total_commits": total_commits,
            },
            "archetype": classify_archetype(files, domains, workspace),
            "workspace": workspace,
            "languages": [
                {"language": language, "files": count}
                for language, count in collections.Counter(
                    LANGUAGE_BY_SUFFIX.get(Path(path).suffix.lower(), "Other") for path in files
                ).most_common()
                if language != "Other"
            ],
            "manifests": manifests,
            "governance_files": governance,
            "existing_codeowners": existing,
            "domains": domains,
            "risk_paths": {
                category: {
                    "count": len(paths),
                    "examples": paths[:25],
                    "truncated": len(paths) > 25,
                }
                for category, paths in sorted(risk_paths.items())
            },
            "generated_or_vendored": {
                "count": sum(1 for path in files if is_probably_generated(path)),
                "examples": [path for path in files if is_probably_generated(path)][:50],
            },
            "history": history_concentration(repo, args.max_commits, domains),
            "analysis_limits": {
                "max_files": args.max_files,
                "max_commits": args.max_commits,
            },
        }

        output = (
            Path(args.output).expanduser().resolve()
            if args.output
            else git_path(repo, "claude-codeowners") / "inventory.json"
        )
        output.parent.mkdir(parents=True, exist_ok=True)
        atomic_write_json(output, payload)

        summary = {
            "output": str(output),
            "tracked_files": len(files),
            "domains": len(domains),
            "archetype": payload["archetype"]["primary"],
            "active_codeowners": existing["active_path"] if existing else None,
            "risk_categories": sorted(risk_paths),
        }
        print(json.dumps(summary, indent=2))
        return 0
    except (ValueError, RuntimeError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
