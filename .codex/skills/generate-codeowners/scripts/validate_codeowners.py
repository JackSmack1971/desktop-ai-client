#!/usr/bin/env python3
"""Validate GitHub CODEOWNERS syntax, precedence, path coverage, and governance risks."""

from __future__ import annotations

import argparse
import collections
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from codeowners_common import (
    active_codeowners_path,
    atomic_write_json,
    list_tracked_files,
    pattern_matches,
    resolve_repo,
    split_codeowners_line,
    unescape_pattern_token,
    validate_owner_handle,
)

MAX_BYTES = 3 * 1024 * 1024


@dataclass
class Rule:
    line: int
    raw_pattern: str
    pattern: str
    owners: list[str]
    raw_line: str


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Path inside the target Git worktree")
    parser.add_argument("--file", help="CODEOWNERS path; defaults to GitHub's active precedence location")
    parser.add_argument("--json-out", help="Optional JSON report path")
    parser.add_argument(
        "--fail-on-unowned",
        action="store_true",
        help="Treat any unowned tracked file as an error. Do not use when deliberate blank-owner exceptions exist.",
    )
    return parser.parse_args()


def is_broad(pattern: str) -> bool:
    normalized = unescape_pattern_token(pattern).strip()
    return normalized in {"*", "**", "/**", "/*"}


def is_extension_glob(pattern: str) -> bool:
    normalized = unescape_pattern_token(pattern).strip()
    return normalized.startswith("*.") and "/" not in normalized


def parse_rules(path: Path) -> tuple[list[Rule], list[dict[str, Any]], list[dict[str, Any]]]:
    rules: list[Rule] = []
    errors: list[dict[str, Any]] = []
    warnings: list[dict[str, Any]] = []

    text = path.read_text(encoding="utf-8-sig")
    for number, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        tokens = split_codeowners_line(raw)
        if not tokens:
            continue
        raw_pattern = tokens[0]
        pattern = unescape_pattern_token(raw_pattern)
        owners = tokens[1:]

        if raw_pattern.startswith("\\#") or pattern.startswith("#"):
            errors.append({"line": number, "code": "leading-hash-pattern", "message": "GitHub cannot escape a pattern that starts with #"})
        if pattern.startswith("!"):
            errors.append({"line": number, "code": "negation", "message": "GitHub CODEOWNERS does not support ! negation"})
        if "[" in pattern or "]" in pattern:
            errors.append({"line": number, "code": "character-range", "message": "GitHub CODEOWNERS does not support [ ] character ranges"})
        if not pattern:
            errors.append({"line": number, "code": "empty-pattern", "message": "Pattern is empty"})

        for owner in owners:
            if owner.startswith("#"):
                errors.append({"line": number, "code": "inline-comment", "message": "Comments must be on their own line"})
            elif not validate_owner_handle(owner):
                errors.append({"line": number, "code": "invalid-owner", "message": f"Invalid or placeholder-like owner: {owner}"})

        if not owners:
            warnings.append({"line": number, "code": "blank-owner", "message": f"Pattern intentionally removes inherited ownership: {pattern}"})

        rules.append(Rule(number, raw_pattern, pattern, owners, raw))

    return rules, errors, warnings


def main() -> int:
    args = parse_args()
    try:
        repo = resolve_repo(args.repo)
        if args.file:
            candidate = Path(args.file)
            path = candidate if candidate.is_absolute() else repo / candidate
            path = path.resolve()
            try:
                path.relative_to(repo)
            except ValueError as exc:
                raise ValueError(f"CODEOWNERS path is outside repository: {path}") from exc
        else:
            active = active_codeowners_path(repo)
            if active is None:
                raise ValueError("No CODEOWNERS file exists in .github/, root, or docs/")
            path = active

        if not path.is_file():
            raise ValueError(f"CODEOWNERS file does not exist: {path}")

        size = path.stat().st_size
        errors: list[dict[str, Any]] = []
        warnings: list[dict[str, Any]] = []
        if size >= MAX_BYTES:
            errors.append({"line": None, "code": "file-size", "message": f"File is {size} bytes; GitHub requires less than {MAX_BYTES} bytes"})

        rules, parse_errors, parse_warnings = parse_rules(path)
        errors.extend(parse_errors)
        warnings.extend(parse_warnings)

        if not rules:
            errors.append({"line": None, "code": "no-rules", "message": "No ownership rules were found"})

        duplicates: dict[str, list[int]] = collections.defaultdict(list)
        for rule in rules:
            duplicates[rule.pattern].append(rule.line)
        for pattern, lines in duplicates.items():
            if len(lines) > 1:
                errors.append({"line": lines[-1], "code": "duplicate-pattern", "message": f"Duplicate pattern {pattern!r} on lines {lines}"})

        for index, rule in enumerate(rules):
            if index > 0 and is_broad(rule.raw_pattern):
                errors.append({"line": rule.line, "code": "late-catch-all", "message": "Catch-all appears after specific rules and overrides them"})
            if index > 0 and is_extension_glob(rule.raw_pattern):
                warnings.append({"line": rule.line, "code": "late-extension-glob", "message": "A late extension glob can override earlier domain ownership"})

        files = list_tracked_files(repo)
        relative_target = path.relative_to(repo).as_posix()
        if relative_target not in files:
            files.append(relative_target)

        coverage: dict[str, dict[str, Any]] = {}
        raw_match_counts = [0 for _ in rules]
        selected_counts = [0 for _ in rules]
        for file_path in files:
            selected: Rule | None = None
            selected_index: int | None = None
            for index, rule in enumerate(rules):
                if pattern_matches(rule.raw_pattern, file_path):
                    raw_match_counts[index] += 1
                    selected = rule
                    selected_index = index
            if selected is None:
                coverage[file_path] = {"owned": False, "owners": [], "line": None, "pattern": None, "reason": "no-match"}
            else:
                assert selected_index is not None
                selected_counts[selected_index] += 1
                coverage[file_path] = {
                    "owned": bool(selected.owners),
                    "owners": selected.owners,
                    "line": selected.line,
                    "pattern": selected.pattern,
                    "reason": "owned" if selected.owners else "blank-owner-rule",
                }

        for index, raw_count in enumerate(raw_match_counts):
            if raw_count == 0:
                warnings.append({"line": rules[index].line, "code": "dead-rule", "message": f"Pattern matches no tracked file: {rules[index].pattern}"})
            elif selected_counts[index] == 0:
                warnings.append({"line": rules[index].line, "code": "fully-shadowed-rule", "message": f"Pattern matches {raw_count} tracked files but never wins last-match evaluation: {rules[index].pattern}"})

        target_coverage = coverage.get(".github/CODEOWNERS")
        if path.relative_to(repo).as_posix() == ".github/CODEOWNERS":
            if not target_coverage or not target_coverage["owned"]:
                errors.append({"line": target_coverage.get("line") if target_coverage else None, "code": "self-unowned", "message": ".github/CODEOWNERS must resolve to at least one owner"})
        else:
            warnings.append({"line": None, "code": "non-preferred-location", "message": f"Active file is {relative_target}; .github/CODEOWNERS is the preferred protected location"})

        owner_kinds = collections.Counter()
        owner_rule_counts = collections.Counter()
        for rule in rules:
            for owner in rule.owners:
                if owner.startswith("@") and "/" in owner:
                    owner_kinds["team"] += 1
                elif owner.startswith("@"):
                    owner_kinds["user"] += 1
                else:
                    owner_kinds["email"] += 1
                owner_rule_counts[owner] += 1
        if owner_kinds["user"]:
            warnings.append({"line": None, "code": "individual-owners", "message": f"File contains {owner_kinds['user']} individual-owner references; organization repositories should prefer teams"})

        unowned = [path for path, item in coverage.items() if not item["owned"]]
        no_match = [path for path, item in coverage.items() if item["reason"] == "no-match"]
        blank_unowned = [path for path, item in coverage.items() if item["reason"] == "blank-owner-rule"]

        if no_match:
            warnings.append({"line": None, "code": "unmatched-files", "message": f"{len(no_match)} tracked files match no rule"})
        if args.fail_on_unowned and unowned:
            errors.append({"line": None, "code": "unowned-files", "message": f"{len(unowned)} tracked files are unowned"})

        report: dict[str, Any] = {
            "schema_version": 1,
            "file": str(path),
            "relative_file": relative_target,
            "size_bytes": size,
            "rules": len(rules),
            "errors": errors,
            "warnings": warnings,
            "stats": {
                "tracked_files_evaluated": len(files),
                "owned_files": len(files) - len(unowned),
                "unowned_files": len(unowned),
                "no_match_files": len(no_match),
                "blank_owner_files": len(blank_unowned),
                "owner_kinds": dict(owner_kinds),
                "unique_owners": len(owner_rule_counts),
            },
            "samples": {
                "unmatched": no_match[:50],
                "blank_owner": blank_unowned[:50],
            },
            "owner_rule_counts": dict(sorted(owner_rule_counts.items())),
        }

        if args.json_out:
            output = Path(args.json_out).expanduser().resolve()
            output.parent.mkdir(parents=True, exist_ok=True)
            atomic_write_json(output, report)

        print(json.dumps(report, indent=2))
        return 1 if errors else 0
    except (ValueError, OSError, RuntimeError, UnicodeDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
