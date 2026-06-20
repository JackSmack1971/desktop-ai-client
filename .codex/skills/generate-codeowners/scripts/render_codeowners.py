#!/usr/bin/env python3
"""Validate an ownership plan and atomically render .github/CODEOWNERS."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import shutil
import sys
from pathlib import Path
from typing import Any

from codeowners_common import (
    atomic_write_text,
    escape_codeowners_pattern,
    git_path,
    parse_github_remote,
    resolve_repo,
    run,
    validate_owner_handle,
)

ALLOWED_ARCHETYPES = {
    "focused-library",
    "modular-application",
    "enterprise-monorepo",
    "open-source",
    "internal-platform",
    "small-or-mixed",
}
ALLOWED_VERIFICATION = {
    "github-api",
    "existing-codeowners",
    "owner-map",
    "personal-repository-owner",
}
ALLOWED_KINDS = {"team", "user", "email"}
ALLOWED_INTENTS = {"owned", "unowned"}
ALLOWED_SOURCES = ALLOWED_VERIFICATION
MAX_BYTES = 3 * 1024 * 1024


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Path inside the target Git worktree")
    parser.add_argument("--plan", required=True, help="Ownership plan JSON path")
    parser.add_argument(
        "--output",
        default=".github/CODEOWNERS",
        help="Output path. The renderer intentionally accepts only .github/CODEOWNERS.",
    )
    parser.add_argument(
        "--allow-organization-individuals",
        action="store_true",
        help="Permit explicit individual owners in an organization repository. Use only with documented authorization.",
    )
    return parser.parse_args()


def require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def load_plan(path: Path) -> dict[str, Any]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ValueError(f"Plan does not exist: {path}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"Plan is not valid JSON: {exc}") from exc
    if not isinstance(payload, dict):
        raise ValueError("Plan root must be a JSON object")
    return payload


def validate_plan(
    plan: dict[str, Any], repo: Path, allow_org_individuals: bool
) -> tuple[list[str], list[str], dict[str, dict[str, Any]]]:
    errors: list[str] = []
    warnings: list[str] = []

    require(plan.get("version") == 1, "version must be exactly 1", errors)
    require(plan.get("archetype") in ALLOWED_ARCHETYPES, "archetype is missing or unsupported", errors)
    require(isinstance(plan.get("repository"), str) and bool(plan.get("repository")), "repository is required", errors)

    remote_proc = run(["git", "remote", "get-url", "origin"], repo, check=False)
    parsed = parse_github_remote(remote_proc.stdout.strip()) if remote_proc.returncode == 0 else None
    if parsed:
        expected = f"{parsed[0]}/{parsed[1]}"
        require(plan.get("repository") == expected, f"repository must match origin: {expected}", errors)
        organization_repo = True
        github_state = git_path(repo, "claude-codeowners") / "github-owners.json"
        if github_state.is_file():
            try:
                data = json.loads(github_state.read_text(encoding="utf-8"))
                organization_repo = data.get("repository_owner_type") == "Organization"
            except (OSError, json.JSONDecodeError):
                organization_repo = False
    else:
        organization_repo = "/" in str(plan.get("repository", ""))
        warnings.append("GitHub origin could not be verified; repository identity relies on the plan")

    raw_verified = plan.get("verified_owners")
    require(isinstance(raw_verified, list) and bool(raw_verified), "verified_owners must be a non-empty array", errors)
    verified: dict[str, dict[str, Any]] = {}
    if isinstance(raw_verified, list):
        for index, item in enumerate(raw_verified):
            prefix = f"verified_owners[{index}]"
            if not isinstance(item, dict):
                errors.append(f"{prefix} must be an object")
                continue
            handle = item.get("handle")
            require(isinstance(handle, str) and validate_owner_handle(handle), f"{prefix}.handle is invalid or placeholder-like", errors)
            require(item.get("kind") in ALLOWED_KINDS, f"{prefix}.kind is unsupported", errors)
            require(item.get("verification") in ALLOWED_VERIFICATION, f"{prefix}.verification is unsupported", errors)
            require(item.get("write_access") is True, f"{prefix}.write_access must be true", errors)
            if item.get("kind") == "team":
                require(item.get("visible") is True, f"{prefix}.visible must be true for teams", errors)
                require(isinstance(handle, str) and "/" in handle, f"{prefix}.handle must use @organization/team format", errors)
            if item.get("kind") == "user" and organization_repo and not allow_org_individuals:
                errors.append(
                    f"{prefix} uses individual owner {handle} in an organization repository; "
                    "supply --allow-organization-individuals only after explicit authorization"
                )
            if isinstance(handle, str):
                if handle in verified:
                    errors.append(f"duplicate verified owner: {handle}")
                verified[handle] = item

    fallback = plan.get("fallback")
    if fallback is not None:
        if not isinstance(fallback, dict):
            errors.append("fallback must be null or an object")
        else:
            owners = fallback.get("owners")
            require(isinstance(owners, list) and bool(owners), "fallback.owners must be non-empty", errors)
            require(isinstance(fallback.get("rationale"), str) and bool(fallback.get("rationale", "").strip()), "fallback.rationale is required", errors)
            if isinstance(owners, list):
                for owner in owners:
                    require(owner in verified, f"fallback owner is not verified: {owner}", errors)

    sections = plan.get("sections")
    require(isinstance(sections, list) and bool(sections), "sections must be a non-empty array", errors)
    if isinstance(sections, list):
        seen_patterns: set[str] = set()
        for section_index, section in enumerate(sections):
            section_prefix = f"sections[{section_index}]"
            if not isinstance(section, dict):
                errors.append(f"{section_prefix} must be an object")
                continue
            require(isinstance(section.get("title"), str) and bool(section.get("title", "").strip()), f"{section_prefix}.title is required", errors)
            rules = section.get("rules")
            require(isinstance(rules, list) and bool(rules), f"{section_prefix}.rules must be non-empty", errors)
            if not isinstance(rules, list):
                continue
            for rule_index, rule in enumerate(rules):
                prefix = f"{section_prefix}.rules[{rule_index}]"
                if not isinstance(rule, dict):
                    errors.append(f"{prefix} must be an object")
                    continue
                pattern = rule.get("pattern")
                require(isinstance(pattern, str) and bool(pattern.strip()), f"{prefix}.pattern is required", errors)
                if isinstance(pattern, str):
                    require(not pattern.startswith("!"), f"{prefix}.pattern cannot use negation", errors)
                    require("[" not in pattern and "]" not in pattern, f"{prefix}.pattern cannot use character ranges", errors)
                    require(not pattern.startswith("#"), f"{prefix}.pattern cannot start with #", errors)
                    if pattern in seen_patterns:
                        errors.append(f"duplicate pattern: {pattern}")
                    seen_patterns.add(pattern)
                intent = rule.get("intent")
                require(intent in ALLOWED_INTENTS, f"{prefix}.intent is unsupported", errors)
                owners = rule.get("owners")
                require(isinstance(owners, list), f"{prefix}.owners must be an array", errors)
                require(rule.get("source") in ALLOWED_SOURCES, f"{prefix}.source is unsupported", errors)
                require(isinstance(rule.get("rationale"), str) and bool(rule.get("rationale", "").strip()), f"{prefix}.rationale is required", errors)
                comment = rule.get("comment")
                if comment is not None:
                    require(isinstance(comment, str) and "\n" not in comment and "\r" not in comment, f"{prefix}.comment must be one line", errors)
                if isinstance(owners, list):
                    if intent == "owned":
                        require(bool(owners), f"{prefix} owned rule requires at least one owner", errors)
                    elif intent == "unowned":
                        require(not owners, f"{prefix} unowned rule must have an empty owner list", errors)
                    for owner in owners:
                        require(owner in verified, f"{prefix} references unverified owner: {owner}", errors)

    return errors, warnings, verified


def render(plan: dict[str, Any]) -> str:
    lines: list[str] = [
        "# CODEOWNERS",
        "# Generated from repository evidence. Edit ownership sources, then regenerate.",
        "# Rules are ordered broad to specific because the last matching rule wins.",
        "",
    ]

    fallback = plan.get("fallback")
    if isinstance(fallback, dict):
        lines.extend(
            [
                "# Repository fallback",
                f"# {fallback['rationale'].strip()}",
                "* " + " ".join(fallback["owners"]),
                "",
            ]
        )

    for section in plan["sections"]:
        title = str(section["title"]).strip()
        lines.append(f"# {title}")
        for rule in section["rules"]:
            comment = rule.get("comment")
            if isinstance(comment, str) and comment.strip():
                lines.append(f"# {comment.strip()}")
            pattern = escape_codeowners_pattern(str(rule["pattern"]).strip())
            owners = " ".join(rule["owners"])
            lines.append(f"{pattern} {owners}".rstrip())
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    args = parse_args()
    try:
        repo = resolve_repo(args.repo)
        output_relative = Path(args.output)
        if output_relative.as_posix() != ".github/CODEOWNERS":
            raise ValueError("Output must be exactly .github/CODEOWNERS")
        output = (repo / output_relative).resolve()
        if output.parent != (repo / ".github").resolve():
            raise ValueError("Output path escaped .github")

        plan_path = Path(args.plan).expanduser().resolve()
        plan = load_plan(plan_path)
        errors, warnings, _ = validate_plan(plan, repo, args.allow_organization_individuals)
        if errors:
            for error in errors:
                print(f"ERROR: {error}", file=sys.stderr)
            return 2

        content = render(plan)
        encoded = content.encode("utf-8")
        if len(encoded) >= MAX_BYTES:
            raise ValueError(f"Rendered CODEOWNERS is {len(encoded)} bytes; GitHub requires less than {MAX_BYTES} bytes")

        output.parent.mkdir(parents=True, exist_ok=True)
        backup: Path | None = None
        if output.exists():
            timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
            backup_dir = git_path(repo, "claude-codeowners") / "backups"
            backup_dir.mkdir(parents=True, exist_ok=True)
            backup = backup_dir / f"CODEOWNERS.{timestamp}.bak"
            shutil.copy2(output, backup)

        atomic_write_text(output, content)
        result = {
            "output": str(output),
            "size_bytes": len(encoded),
            "backup": str(backup) if backup else None,
            "warnings": warnings,
            "sections": len(plan["sections"]),
            "rules": sum(len(section["rules"]) for section in plan["sections"]) + (1 if plan.get("fallback") else 0),
        }
        print(json.dumps(result, indent=2))
        return 0
    except (ValueError, OSError, RuntimeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
