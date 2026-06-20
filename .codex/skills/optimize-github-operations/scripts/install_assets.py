#!/usr/bin/env python3
"""Preview or conservatively install canonical .github assets without deleting files."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

SPECIALIZED = {"security", "release", "migration"}


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


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def safe_target(root: Path, relative: str) -> Path:
    target = (root / relative).resolve()
    try:
        target.relative_to(root)
    except ValueError as exc:
        raise ValueError(f"Target escapes repository root: {relative}") from exc
    if not relative.startswith(".github/"):
        raise ValueError(f"Target is outside .github: {relative}")
    return target


def parse_include(value: str) -> list[str]:
    if not value.strip():
        return []
    items = [item.strip().lower() for item in value.split(",") if item.strip()]
    unknown = sorted(set(items) - SPECIALIZED)
    if unknown:
        raise ValueError(f"Unknown specialized profile(s): {', '.join(unknown)}")
    return sorted(set(items))


def plan_assets(root: Path, include: list[str], skip_workflow: bool) -> list[dict[str, object]]:
    skill_root = Path(__file__).resolve().parent.parent
    mappings = [
        ("assets/PULL_REQUEST_TEMPLATE.md", ".github/PULL_REQUEST_TEMPLATE.md"),
    ]
    if not skip_workflow:
        mappings.append(("assets/pr-contract.yml", ".github/workflows/pr-contract.yml"))
    profile_map = {
        "security": ("assets/security-sensitive.md", ".github/PULL_REQUEST_TEMPLATE/security-sensitive.md"),
        "release": ("assets/release.md", ".github/PULL_REQUEST_TEMPLATE/release.md"),
        "migration": ("assets/migration.md", ".github/PULL_REQUEST_TEMPLATE/migration.md"),
    }
    mappings.extend(profile_map[item] for item in include)

    plan: list[dict[str, object]] = []
    for source_relative, target_relative in mappings:
        source = (skill_root / source_relative).resolve()
        if not source.is_file():
            raise FileNotFoundError(f"Bundled asset is missing: {source}")
        target = safe_target(root, target_relative)
        if not target.exists():
            action = "create"
            same = False
        elif target.is_file():
            same = sha256(source) == sha256(target)
            action = "unchanged" if same else "conflict"
        else:
            same = False
            action = "conflict-non-file"
        plan.append(
            {
                "source": str(source),
                "target": target_relative,
                "action": action,
                "same_content": same,
                "source_sha256": sha256(source),
            }
        )
    return plan


def apply_plan(
    root: Path,
    plan: list[dict[str, object]],
    overwrite_existing: bool,
) -> tuple[list[str], str | None]:
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    backup_root = root / ".github" / ".optimizer-backups" / timestamp
    changed: list[str] = []
    backup_used = False

    for item in plan:
        action = str(item["action"])
        source = Path(str(item["source"]))
        target_relative = str(item["target"])
        target = safe_target(root, target_relative)

        if action == "unchanged":
            continue
        if action == "conflict-non-file":
            raise RuntimeError(f"Refusing to replace non-file target: {target_relative}")
        if action == "conflict" and not overwrite_existing:
            raise RuntimeError(
                f"Existing file differs: {target_relative}. Reconcile it surgically or rerun with --overwrite-existing."
            )
        if action == "conflict":
            backup = backup_root / target.relative_to(root / ".github")
            backup.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(target, backup)
            backup_used = True

        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        changed.append(target_relative)

    return changed, str(backup_root.relative_to(root)) if backup_used else None


def to_markdown(result: dict[str, object]) -> str:
    lines = [
        "# Asset Installation Plan",
        "",
        f"- Repository: `{result['repository_root']}`",
        f"- Mode: `{result['mode']}`",
        f"- Specialized profiles: {', '.join(result['include']) if result['include'] else 'none'}",
        "",
        "| Action | Target | Source SHA-256 |",
        "| --- | --- | --- |",
    ]
    for item in result["plan"]:
        lines.append(f"| {item['action']} | `{item['target']}` | `{item['source_sha256']}` |")
    if result.get("changed"):
        lines.extend(["", "## Changed", *[f"- `{path}`" for path in result["changed"]]])
    if result.get("backup_root"):
        lines.extend(["", f"Backup root: `{result['backup_root']}`"])
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Path inside the target Git repository.")
    parser.add_argument("--include", default="", help="Comma-separated: security,release,migration.")
    parser.add_argument("--skip-workflow", action="store_true", help="Do not plan the PR validator workflow.")
    parser.add_argument("--apply", action="store_true", help="Write planned assets. Default is read-only preview.")
    parser.add_argument(
        "--overwrite-existing",
        action="store_true",
        help="Replace differing managed targets after creating timestamped backups.",
    )
    parser.add_argument("--format", choices=("json", "markdown"), default="json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        root = resolve_repo(args.repo)
        include = parse_include(args.include)
        plan = plan_assets(root, include, args.skip_workflow)
        changed: list[str] = []
        backup_root: str | None = None
        if args.apply:
            changed, backup_root = apply_plan(root, plan, args.overwrite_existing)
        result = {
            "repository_root": str(root),
            "mode": "apply" if args.apply else "preview",
            "include": include,
            "plan": plan,
            "changed": changed,
            "backup_root": backup_root,
        }
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"installation error: {exc}", file=sys.stderr)
        return 2

    if args.format == "markdown":
        print(to_markdown(result), end="")
    else:
        print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
