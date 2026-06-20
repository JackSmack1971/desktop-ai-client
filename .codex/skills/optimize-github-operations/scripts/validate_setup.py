#!/usr/bin/env python3
"""Validate the optimized .github pull-request governance contract."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

DEFAULT_HEADINGS = {
    "purpose",
    "change summary",
    "scope",
    "classification",
    "design and behavior",
    "risk and compatibility",
    "rollback or recovery",
    "verification",
    "regression coverage",
    "reviewer guidance",
    "user-facing evidence",
    "documentation, release, and follow-up",
    "author checklist",
}

PROFILE_HEADINGS = {
    "security-sensitive.md": {
        "security objective",
        "change summary",
        "scope",
        "classification",
        "assets and trust boundaries",
        "attacker model and abuse cases",
        "authorization and validation flow",
        "secrets and sensitive data",
        "failure behavior",
        "compatibility and migration",
        "security verification",
        "residual risk",
        "rollback and incident recovery",
        "reviewer guidance",
        "documentation, release, and follow-up",
        "checklist",
    },
    "release.md": {
        "release objective",
        "release identity",
        "included changes",
        "excluded or deferred changes",
        "artifact matrix",
        "provenance and signing",
        "compatibility and migration",
        "risk and rollback",
        "release verification",
        "post-release validation",
        "reviewer guidance",
        "follow-up work",
        "checklist",
    },
    "migration.md": {
        "migration objective",
        "representation change",
        "scope",
        "forward migration",
        "mixed-version compatibility",
        "idempotency and interruption",
        "data validation and reconciliation",
        "backup and restore",
        "rollback or downgrade",
        "risk and observability",
        "verification",
        "reviewer guidance",
        "documentation, release, and follow-up",
        "checklist",
    },
}

REDUNDANT_STEMS = {
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

GENERIC_OWNERS = {
    "@owner",
    "@owners",
    "@maintainer",
    "@maintainers",
    "@team",
    "@org/team",
    "@organization/team",
}


def run_git(cwd: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False
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
    return Path(run_git(candidate, "rev-parse", "--show-toplevel")).resolve()


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError as exc:
        raise ValueError(f"File is not valid UTF-8: {path}") from exc


def headings(text: str) -> set[str]:
    return {
        re.sub(r"\s+", " ", match.group(1).strip()).lower()
        for match in re.finditer(r"(?m)^#{2,4}\s+(.+?)\s*$", text)
    }


def validate_template(path: Path, required: set[str], errors: list[str]) -> None:
    if not path.is_file():
        errors.append(f"Missing required template: {path.as_posix()}")
        return
    text = read_text(path)
    present = headings(text)
    missing = sorted(required - present)
    if missing:
        errors.append(f"{path.as_posix()} is missing headings: {', '.join(missing)}")
    if "N/A" not in text:
        errors.append(f"{path.as_posix()} does not instruct authors to provide a reasoned N/A.")
    if "<!--" not in text:
        errors.append(f"{path.as_posix()} should keep author instructions in HTML comments.")


def validate_workflow(path: Path, errors: list[str]) -> None:
    if not path.is_file():
        errors.append(f"Missing required workflow: {path.as_posix()}")
        return
    text = read_text(path)
    lowered = text.lower()
    required_fragments = [
        "pull_request_target:",
        "permissions:",
        "contents: read",
        "pull-requests: read",
        "github_event_path",
        "timeout-minutes:",
    ]
    for fragment in required_fragments:
        if fragment not in lowered:
            errors.append(f"{path.as_posix()} is missing required workflow fragment: {fragment}")
    forbidden_patterns = {
        "checkout action": r"actions/checkout",
        "write-all permission": r"permissions\s*:\s*write-all",
        "write permission": r"(?m)^\s*[a-z-]+\s*:\s*write\s*$",
        "secret reference": r"secrets\.",
        "pull-request head checkout": r"pull_request\.head|github\.head_ref|refs/pull",
        "package installation": r"\b(?:npm|pnpm|yarn|pip|uv)\s+(?:install|add)\b",
    }
    for label, pattern in forbidden_patterns.items():
        if re.search(pattern, lowered, flags=re.IGNORECASE):
            errors.append(f"{path.as_posix()} contains forbidden {label}.")


def validate_codeowners(path: Path, errors: list[str], warnings: list[str]) -> None:
    if not path.exists():
        warnings.append("CODEOWNERS is absent; ownership routing remains unresolved until handles are verified.")
        return
    if not path.is_file():
        errors.append(f"CODEOWNERS path is not a regular file: {path.as_posix()}")
        return

    entries = 0
    for number, raw in enumerate(read_text(path).splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        entries += 1
        parts = line.split()
        if len(parts) < 2:
            errors.append(f"{path.as_posix()}:{number} has no owner token.")
            continue
        owners = parts[1:]
        for owner in owners:
            normalized = owner.lower()
            if not owner.startswith("@"):
                errors.append(f"{path.as_posix()}:{number} owner must start with @: {owner}")
            if normalized in GENERIC_OWNERS or "todo" in normalized or "example" in normalized:
                errors.append(f"{path.as_posix()}:{number} contains a generic owner placeholder: {owner}")
    if entries == 0:
        errors.append(f"{path.as_posix()} contains no ownership entries.")


def validate(root: Path) -> dict[str, object]:
    errors: list[str] = []
    warnings: list[str] = []
    github = root / ".github"
    if not github.is_dir():
        errors.append(".github directory does not exist.")
        return {"errors": errors, "warnings": warnings, "checked": []}

    checked: list[str] = []
    default_template = github / "PULL_REQUEST_TEMPLATE.md"
    validate_template(default_template, DEFAULT_HEADINGS, errors)
    checked.append(default_template.relative_to(root).as_posix())

    templates_dir = github / "PULL_REQUEST_TEMPLATE"
    if templates_dir.is_dir():
        for path in sorted(templates_dir.iterdir()):
            if not path.is_file() or path.suffix.lower() not in {".md", ".txt"}:
                continue
            stem = path.stem.lower().replace("_", "-")
            if path.name in PROFILE_HEADINGS:
                validate_template(path, PROFILE_HEADINGS[path.name], errors)
                checked.append(path.relative_to(root).as_posix())
            elif stem in REDUNDANT_STEMS:
                warnings.append(
                    f"Redundant change-class template should be reviewed for consolidation: {path.relative_to(root).as_posix()}"
                )
            else:
                warnings.append(f"Unclassified PR template requires policy review: {path.relative_to(root).as_posix()}")

    workflow = github / "workflows" / "pr-contract.yml"
    validate_workflow(workflow, errors)
    checked.append(workflow.relative_to(root).as_posix())

    codeowners_candidates = [github / "CODEOWNERS", root / "CODEOWNERS", root / "docs" / "CODEOWNERS"]
    existing_codeowners = next((path for path in codeowners_candidates if path.exists()), github / "CODEOWNERS")
    validate_codeowners(existing_codeowners, errors, warnings)
    if existing_codeowners.exists():
        checked.append(existing_codeowners.relative_to(root).as_posix())

    duplicate_defaults = [
        path.relative_to(root).as_posix()
        for path in [
            root / "PULL_REQUEST_TEMPLATE.md",
            root / "pull_request_template.md",
            root / "docs" / "PULL_REQUEST_TEMPLATE.md",
            root / "docs" / "pull_request_template.md",
            github / "pull_request_template.md",
        ]
        if path.is_file()
    ]
    if duplicate_defaults:
        warnings.append("Additional default PR template locations exist: " + ", ".join(duplicate_defaults))

    return {"errors": errors, "warnings": warnings, "checked": checked}


def to_markdown(root: Path, result: dict[str, object]) -> str:
    status = "PASS" if not result["errors"] else "FAIL"
    lines = [
        "# .github Validation",
        "",
        f"- Repository: `{root}`",
        f"- Status: **{status}**",
        f"- Errors: {len(result['errors'])}",
        f"- Warnings: {len(result['warnings'])}",
        "",
        "## Checked files",
    ]
    lines.extend(f"- `{path}`" for path in result["checked"]) if result["checked"] else lines.append("- None")
    lines.append("")
    lines.append("## Errors")
    lines.extend(f"- {message}" for message in result["errors"]) if result["errors"] else lines.append("- None")
    lines.append("")
    lines.append("## Warnings")
    lines.extend(f"- {message}" for message in result["warnings"]) if result["warnings"] else lines.append("- None")
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
        result = validate(root)
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"validation error: {exc}", file=sys.stderr)
        return 2

    payload = {"repository_root": str(root), **result, "valid": not result["errors"]}
    if args.format == "markdown":
        print(to_markdown(root, result), end="")
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))
    return 0 if payload["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
