#!/usr/bin/env python3
"""Audit, align, and safely purge generated workspace artifacts.

Standard-library only. The script never deletes tracked files and never follows
symlinks outside the repository root.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from typing import Any, Iterable

BEGIN = "# BEGIN token-dead-weight managed block"
END = "# END token-dead-weight managed block"
REPORT_DIR = Path(".claude/hygiene")
SETTINGS_PATH = Path(".claude/settings.json")

GENERATED_DIR_NAMES = {
    "node_modules",
    ".pnpm-store",
    ".venv",
    "venv",
    "pods",
    ".bundle",
    "dist",
    "build",
    "out",
    "target",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".astro",
    ".vite",
    ".parcel-cache",
    ".turbo",
    "coverage",
    "htmlcov",
    ".gradle",
    ".cache",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".nox",
    "__pycache__",
}

GENERATED_FILE_PATTERNS = (
    "*.log",
    "npm-debug.log*",
    "yarn-debug.log*",
    "yarn-error.log*",
    "pnpm-debug.log*",
    "*.tmp",
    "*.temp",
    "*.pyc",
    "*.pyo",
    "*.o",
    "*.obj",
    "*.class",
    "*.so",
    "*.dylib",
    "*.dll",
    "*.exe",
    "*.pdb",
    "*.ilk",
)

AMBIGUOUS_FILE_PATTERNS = ("*.min.js", "*.min.css", "*.map")
NOISY_DIR_TOKENS = {"fixture", "fixtures", "__fixtures__", "mock-data", "mocks", "testdata", "datasets", "dataset"}
LARGE_FILE_BYTES = 10 * 1024 * 1024
NOISY_DIR_BYTES = 5 * 1024 * 1024
NOISY_DIR_FILES = 500

BASE_GITIGNORE = [
    "# Dependency installations",
    "node_modules/",
    ".pnpm-store/",
    ".yarn/cache/",
    ".venv/",
    "venv/",
    "Pods/",
    ".bundle/",
    "",
    "# Build outputs and caches",
    "dist/",
    "build/",
    "out/",
    "target/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    ".astro/",
    ".vite/",
    ".parcel-cache/",
    ".turbo/",
    "coverage/",
    "htmlcov/",
    ".gradle/",
    ".cache/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".tox/",
    ".nox/",
    "__pycache__/",
    "*.py[cod]",
    "",
    "# Logs, temporaries, and compiled intermediates",
    "*.log",
    "npm-debug.log*",
    "yarn-debug.log*",
    "yarn-error.log*",
    "pnpm-debug.log*",
    "*.tmp",
    "*.temp",
    "*.o",
    "*.obj",
    "*.class",
    "*.so",
    "*.dylib",
    "*.dll",
    "*.exe",
    "*.pdb",
    "*.ilk",
    "",
    "# Local hygiene evidence",
    ".claude/hygiene/",
]

DOCKERIGNORE = [
    ".git/",
    ".claude/hygiene/",
    "node_modules/",
    ".pnpm-store/",
    ".yarn/cache/",
    ".venv/",
    "venv/",
    ".cache/",
    "*.log",
    "*.tmp",
]

TOOL_IGNORE = [
    "node_modules/",
    ".pnpm-store/",
    ".yarn/cache/",
    ".venv/",
    "venv/",
    "dist/",
    "build/",
    "out/",
    "target/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    ".astro/",
    ".vite/",
    ".parcel-cache/",
    ".turbo/",
    "coverage/",
    "htmlcov/",
    ".cache/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".tox/",
    ".nox/",
    "__pycache__/",
    "*.min.js",
    "*.min.css",
    "*.map",
]

BASE_CLAUDE_DENY = [
    "Read(./**/node_modules/**)",
    "Read(./**/.pnpm-store/**)",
    "Read(./**/.yarn/cache/**)",
    "Read(./**/.venv/**)",
    "Read(./**/venv/**)",
    "Read(./**/vendor/**)",
    "Read(./**/Pods/**)",
    "Read(./**/.bundle/**)",
    "Read(./**/dist/**)",
    "Read(./**/build/**)",
    "Read(./**/out/**)",
    "Read(./**/target/**)",
    "Read(./**/.next/**)",
    "Read(./**/.nuxt/**)",
    "Read(./**/.svelte-kit/**)",
    "Read(./**/.astro/**)",
    "Read(./**/.vite/**)",
    "Read(./**/.parcel-cache/**)",
    "Read(./**/.turbo/**)",
    "Read(./**/coverage/**)",
    "Read(./**/htmlcov/**)",
    "Read(./**/.gradle/**)",
    "Read(./**/.cache/**)",
    "Read(./**/.pytest_cache/**)",
    "Read(./**/.mypy_cache/**)",
    "Read(./**/.ruff_cache/**)",
    "Read(./**/.tox/**)",
    "Read(./**/.nox/**)",
    "Read(./**/__pycache__/**)",
    "Read(./**/*.min.js)",
    "Read(./**/*.min.css)",
    "Read(./**/*.map)",
    "Read(./**/*.log)",
]


class HygieneError(RuntimeError):
    pass


def run_git(root: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if check and result.returncode != 0:
        raise HygieneError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return result


def repository_root(candidate: Path) -> Path:
    candidate = candidate.expanduser().resolve()
    result = run_git(candidate, "rev-parse", "--show-toplevel")
    root = Path(result.stdout.strip()).resolve()
    home = Path.home().resolve()
    anchor = Path(root.anchor).resolve()
    if root == home or root == anchor:
        raise HygieneError(f"refusing unsafe repository root: {root}")
    if not (root / ".git").exists():
        raise HygieneError("repository root has no .git entry")
    return root


def relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def tracked_files(root: Path) -> set[str]:
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise HygieneError(result.stderr.decode(errors="replace").strip())
    return {p.decode(errors="surrogateescape") for p in result.stdout.split(b"\0") if p}


def path_has_tracked(rel: str, tracked: set[str]) -> bool:
    prefix = rel.rstrip("/") + "/"
    return rel in tracked or any(item.startswith(prefix) for item in tracked)


def is_within(root: Path, path: Path) -> bool:
    try:
        path.resolve(strict=False).relative_to(root)
        return True
    except ValueError:
        return False


def dir_stats(path: Path) -> tuple[int, int]:
    total = 0
    count = 0
    if path.is_symlink():
        try:
            return path.lstat().st_size, 1
        except OSError:
            return 0, 1
    for current, dirs, files in os.walk(path, followlinks=False):
        dirs[:] = [d for d in dirs if not (Path(current) / d).is_symlink()]
        for name in files:
            item = Path(current) / name
            try:
                total += item.lstat().st_size
                count += 1
            except OSError:
                continue
    return total, count


def file_matches(name: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatch(name, pattern) for pattern in patterns)


def vendor_is_generated(root: Path) -> bool:
    return (root / "composer.json").exists() and not (root / "go.mod").exists()


def scan(root: Path) -> dict[str, Any]:
    tracked = tracked_files(root)
    candidates: list[dict[str, Any]] = []
    preserved: list[dict[str, str]] = []
    review: list[dict[str, Any]] = []
    noisy_dirs: list[str] = []
    generated_roots: set[str] = set()

    for current, dirs, files in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        rel_current = relative(root, current_path) if current_path != root else ""

        if rel_current == ".git" or rel_current.startswith(".git/"):
            dirs[:] = []
            continue
        if rel_current == ".claude/skills" or rel_current.startswith(".claude/skills/"):
            dirs[:] = []
            continue

        retained_dirs: list[str] = []
        for dirname in dirs:
            path = current_path / dirname
            rel = relative(root, path)
            lower = dirname.lower()
            recognized = lower in GENERATED_DIR_NAMES or (
                lower == "vendor" and vendor_is_generated(root)
            )
            if recognized:
                size, count = dir_stats(path)
                item = {"path": rel, "kind": "directory", "bytes": size, "files": count}
                generated_roots.add(rel)
                if path_has_tracked(rel, tracked):
                    preserved.append({"path": rel, "reason": "contains tracked files"})
                elif not is_within(root, path):
                    preserved.append({"path": rel, "reason": "resolves outside repository"})
                else:
                    candidates.append(item)
                continue

            retained_dirs.append(dirname)
            if lower in NOISY_DIR_TOKENS:
                size, count = dir_stats(path)
                if size >= NOISY_DIR_BYTES or count >= NOISY_DIR_FILES:
                    noisy_dirs.append(rel)
                    review.append(
                        {
                            "path": rel,
                            "reason": "large low-signal fixture, mock, or dataset directory",
                            "bytes": size,
                            "files": count,
                        }
                    )
        dirs[:] = retained_dirs

        inside_generated = any(
            rel_current == base or rel_current.startswith(base + "/") for base in generated_roots
        )
        for filename in files:
            path = current_path / filename
            rel = relative(root, path)
            try:
                size = path.lstat().st_size
            except OSError:
                size = 0

            if file_matches(filename, GENERATED_FILE_PATTERNS):
                if rel in tracked:
                    preserved.append({"path": rel, "reason": "tracked file"})
                elif is_within(root, path):
                    candidates.append({"path": rel, "kind": "file", "bytes": size, "files": 1})
                continue

            if file_matches(filename, AMBIGUOUS_FILE_PATTERNS):
                if inside_generated and rel not in tracked and is_within(root, path):
                    candidates.append({"path": rel, "kind": "file", "bytes": size, "files": 1})
                elif size >= LARGE_FILE_BYTES:
                    review.append(
                        {
                            "path": rel,
                            "reason": "large minified asset or source map outside generated output",
                            "bytes": size,
                            "files": 1,
                        }
                    )
                continue

            if size >= LARGE_FILE_BYTES and rel not in tracked:
                review.append(
                    {
                        "path": rel,
                        "reason": "large untracked file with unknown provenance",
                        "bytes": size,
                        "files": 1,
                    }
                )

    candidates.sort(key=lambda item: item["path"])
    preserved.sort(key=lambda item: item["path"])
    review.sort(key=lambda item: item["path"])
    return {
        "root": str(root),
        "candidates": candidates,
        "candidate_count": len(candidates),
        "candidate_bytes": sum(int(item["bytes"]) for item in candidates),
        "preserved": preserved,
        "review": review,
        "agent_noise_paths": sorted(set(noisy_dirs)),
    }


def managed_content(lines: list[str]) -> str:
    return "\n".join([BEGIN, *lines, END])


def replace_managed_block(original: str, lines: list[str]) -> str:
    block = managed_content(lines)
    if BEGIN in original or END in original:
        if original.count(BEGIN) != 1 or original.count(END) != 1:
            raise HygieneError("ignore file has malformed or duplicate managed block markers")
        start = original.index(BEGIN)
        finish = original.index(END, start) + len(END)
        result = original[:start].rstrip() + "\n\n" + block + original[finish:].lstrip("\n")
    else:
        result = original.rstrip() + ("\n\n" if original.strip() else "") + block
    return result.rstrip() + "\n"


def write_if_changed(path: Path, content: str, changed: list[str]) -> None:
    old = path.read_text(encoding="utf-8") if path.exists() else ""
    if old == content:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")
    changed.append(path.as_posix())


def project_patterns(root: Path) -> list[str]:
    patterns = list(BASE_GITIGNORE)
    if vendor_is_generated(root):
        insertion = patterns.index("Pods/")
        patterns.insert(insertion, "vendor/")
    return patterns


def sync_ignore_files(root: Path, noisy_paths: list[str]) -> list[str]:
    changed: list[str] = []

    gitignore = root / ".gitignore"
    write_if_changed(
        gitignore,
        replace_managed_block(gitignore.read_text(encoding="utf-8") if gitignore.exists() else "", project_patterns(root)),
        changed,
    )

    container_files = [root / "Dockerfile", root / "docker-compose.yml", root / "docker-compose.yaml", root / "compose.yml", root / "compose.yaml"]
    dockerignore = root / ".dockerignore"
    if dockerignore.exists() or any(path.exists() for path in container_files):
        write_if_changed(
            dockerignore,
            replace_managed_block(dockerignore.read_text(encoding="utf-8") if dockerignore.exists() else "", DOCKERIGNORE),
            changed,
        )

    for name in (".prettierignore", ".eslintignore", ".rgignore"):
        path = root / name
        if path.exists():
            write_if_changed(path, replace_managed_block(path.read_text(encoding="utf-8"), TOOL_IGNORE), changed)

    settings = root / SETTINGS_PATH
    if settings.exists():
        try:
            data = json.loads(settings.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise HygieneError(f"invalid {SETTINGS_PATH.as_posix()}: {exc}") from exc
        if not isinstance(data, dict):
            raise HygieneError(f"{SETTINGS_PATH.as_posix()} must contain a JSON object")
    else:
        data = {}

    data.setdefault("$schema", "https://json.schemastore.org/claude-code-settings.json")
    permissions = data.setdefault("permissions", {})
    if not isinstance(permissions, dict):
        raise HygieneError("permissions in .claude/settings.json must be an object")
    deny = permissions.setdefault("deny", [])
    if not isinstance(deny, list) or not all(isinstance(item, str) for item in deny):
        raise HygieneError("permissions.deny in .claude/settings.json must be a string array")

    exact_noise_rules = [f"Read(./{path}/**)" for path in noisy_paths]
    required = BASE_CLAUDE_DENY + exact_noise_rules
    merged = list(deny)
    seen = set(merged)
    for rule in required:
        if rule not in seen:
            merged.append(rule)
            seen.add(rule)
    permissions["deny"] = merged
    content = json.dumps(data, indent=2, ensure_ascii=False) + "\n"
    write_if_changed(settings, content, changed)

    return [relative(root, Path(path)) if Path(path).is_absolute() else path for path in changed]


def delete_candidate(root: Path, item: dict[str, Any], tracked: set[str]) -> tuple[bool, str]:
    rel = str(item["path"])
    path = root / rel
    if path_has_tracked(rel, tracked):
        return False, "became tracked"
    if not path.exists() and not path.is_symlink():
        return False, "already absent"
    if not is_within(root, path):
        return False, "resolves outside repository"
    if rel == ".git" or rel.startswith(".git/") or rel == ".claude/skills" or rel.startswith(".claude/skills/"):
        return False, "protected path"

    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.is_dir():
        shutil.rmtree(path)
    else:
        return False, "unsupported file type"
    return True, "deleted"


def settings_audit(root: Path, noisy_paths: list[str]) -> dict[str, Any]:
    missing: list[str] = []
    settings = root / SETTINGS_PATH
    required = BASE_CLAUDE_DENY + [f"Read(./{path}/**)" for path in noisy_paths]
    if not settings.exists():
        missing.extend(required)
        return {"path": SETTINGS_PATH.as_posix(), "valid": False, "missing_rules": missing}
    try:
        data = json.loads(settings.read_text(encoding="utf-8"))
        deny = data.get("permissions", {}).get("deny", [])
        valid = isinstance(deny, list) and all(isinstance(item, str) for item in deny)
    except (json.JSONDecodeError, AttributeError):
        return {"path": SETTINGS_PATH.as_posix(), "valid": False, "missing_rules": required}
    if valid:
        missing = [rule for rule in required if rule not in deny]
    else:
        missing = required
    return {"path": SETTINGS_PATH.as_posix(), "valid": valid, "missing_rules": missing}


def ignore_audit(root: Path) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for name in (".gitignore", ".dockerignore", ".prettierignore", ".eslintignore", ".rgignore"):
        path = root / name
        if not path.exists():
            result[name] = {"exists": False, "managed_blocks": 0}
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        result[name] = {
            "exists": True,
            "managed_blocks": min(text.count(BEGIN), text.count(END)),
            "begin_markers": text.count(BEGIN),
            "end_markers": text.count(END),
        }
    return result


def emit_report(root: Path, report: dict[str, Any], write: bool) -> None:
    report["generated_at"] = datetime.now(timezone.utc).isoformat()
    if write:
        target = root / REPORT_DIR / "latest.json"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        report["report_path"] = relative(root, target)
    print(json.dumps(report, indent=2, ensure_ascii=False))


def command_audit(root: Path) -> int:
    result = scan(root)
    result["settings"] = settings_audit(root, result["agent_noise_paths"])
    result["ignore_files"] = ignore_audit(root)
    result["mode"] = "audit"
    emit_report(root, result, write=False)
    return 0


def command_apply(root: Path, confirm: str) -> int:
    if confirm != "PURGE":
        raise HygieneError("apply requires --confirm PURGE")
    before = scan(root)
    changed = sync_ignore_files(root, before["agent_noise_paths"])
    tracked = tracked_files(root)
    deleted: list[dict[str, Any]] = []
    skipped: list[dict[str, str]] = []
    reclaimed = 0
    for item in before["candidates"]:
        success, reason = delete_candidate(root, item, tracked)
        if success:
            deleted.append(item)
            reclaimed += int(item["bytes"])
        else:
            skipped.append({"path": str(item["path"]), "reason": reason})

    after = scan(root)
    report = {
        "mode": "apply",
        "root": str(root),
        "changed_ignore_files": changed,
        "deleted": deleted,
        "deleted_count": len(deleted),
        "reclaimed_bytes": reclaimed,
        "skipped": skipped,
        "preserved": before["preserved"],
        "review": after["review"],
        "remaining_candidates": after["candidates"],
        "settings": settings_audit(root, after["agent_noise_paths"]),
        "ignore_files": ignore_audit(root),
        "git_status_short": run_git(root, "status", "--short").stdout.splitlines(),
    }
    emit_report(root, report, write=True)
    return 0 if not after["candidates"] else 2


def command_verify(root: Path) -> int:
    current = scan(root)
    settings = settings_audit(root, current["agent_noise_paths"])
    ignores = ignore_audit(root)
    git_block_ok = ignores.get(".gitignore", {}).get("begin_markers") == 1 and ignores.get(".gitignore", {}).get("end_markers") == 1
    malformed = [
        name
        for name, info in ignores.items()
        if info.get("exists") and (info.get("begin_markers") != info.get("end_markers") or info.get("begin_markers", 0) > 1)
    ]
    ok = not current["candidates"] and settings["valid"] and not settings["missing_rules"] and git_block_ok and not malformed
    report = {
        "mode": "verify",
        "root": str(root),
        "ok": ok,
        "remaining_candidates": current["candidates"],
        "settings": settings,
        "ignore_files": ignores,
        "malformed_ignore_files": malformed,
        "review": current["review"],
        "git_status_short": run_git(root, "status", "--short").stdout.splitlines(),
    }
    emit_report(root, report, write=True)
    return 0 if ok else 2


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("audit", "verify"):
        child = subparsers.add_parser(command)
        child.add_argument("--root", default=".")
    apply_parser = subparsers.add_parser("apply")
    apply_parser.add_argument("--root", default=".")
    apply_parser.add_argument("--confirm", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        root = repository_root(Path(args.root))
        if args.command == "audit":
            return command_audit(root)
        if args.command == "apply":
            return command_apply(root, args.confirm)
        return command_verify(root)
    except HygieneError as exc:
        print(json.dumps({"ok": False, "error": str(exc)}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
