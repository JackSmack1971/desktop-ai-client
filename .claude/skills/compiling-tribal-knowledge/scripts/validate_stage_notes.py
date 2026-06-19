#!/usr/bin/env python3
"""Validate a tribal-knowledge staging document without modifying the repository."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ALLOWED_FILENAMES = {"RAW_CONTEXT.md", "STAGE_NOTES.md"}
REQUIRED_MARKER = "<!-- generated-by: compiling-tribal-knowledge -->"
REQUIRED_TITLE = "# Repository Tribal Knowledge"
REQUIRED_TOP_LEVEL = {"Evidence Rules", "Cross-Cutting Contracts"}
REQUIRED_DIMENSIONS = (
    "Core Ownership",
    "Architectural Invariants",
    "Historical Pitfalls",
    "Stylistic Standards",
    "Hidden Contracts",
)
PLACEHOLDER_PATTERNS = (
    re.compile(r"\bTODO\b", re.IGNORECASE),
    re.compile(r"\bTBD\b", re.IGNORECASE),
    re.compile(r"\bFIXME\b", re.IGNORECASE),
    re.compile(r"\?\?\?"),
)
LABEL_PATTERN = re.compile(r"\[(CONFIRMED|INFERRED|UNRESOLVED)\]")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-root", required=True, help="Resolved Git repository root")
    parser.add_argument("--file", required=True, help="RAW_CONTEXT.md or STAGE_NOTES.md")
    return parser.parse_args()


def fail(message: str) -> None:
    print(json.dumps({"valid": False, "errors": [message]}, indent=2))
    raise SystemExit(2)


def section_spans(lines: list[str]) -> list[tuple[int, int, str]]:
    headings: list[tuple[int, str]] = []
    for index, line in enumerate(lines):
        if line.startswith("## ") and not line.startswith("### "):
            headings.append((index, line[3:].strip()))
    spans: list[tuple[int, int, str]] = []
    for position, (start, title) in enumerate(headings):
        end = headings[position + 1][0] if position + 1 < len(headings) else len(lines)
        spans.append((start, end, title))
    return spans


def main() -> int:
    args = parse_args()
    root = Path(args.project_root).expanduser().resolve()
    requested = Path(args.file)

    if requested.name not in ALLOWED_FILENAMES or requested.parent not in (Path("."), Path("")):
        fail("--file must be RAW_CONTEXT.md or STAGE_NOTES.md with no directory component")
    if not root.is_dir():
        fail("project root does not exist or is not a directory")

    target = root / requested.name
    if target.is_symlink():
        fail("staging file must not be a symbolic link")
    if not target.is_file():
        fail("staging file does not exist")
    if target.parent.resolve() != root:
        fail("staging file is not directly under the repository root")
    if target.stat().st_size > 2 * 1024 * 1024:
        fail("staging file exceeds the 2 MiB safety limit")

    try:
        text = target.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        fail("staging file is not valid UTF-8")

    lines = text.splitlines()
    errors: list[str] = []

    if not lines or lines[0].strip() != REQUIRED_MARKER:
        errors.append("missing generated-by marker on the first line")
    if REQUIRED_TITLE not in lines[:5]:
        errors.append("missing '# Repository Tribal Knowledge' near the top")

    for pattern in PLACEHOLDER_PATTERNS:
        match = pattern.search(text)
        if match:
            errors.append(f"placeholder-like content detected: {match.group(0)!r}")

    spans = section_spans(lines)
    top_titles = {title for _, _, title in spans}
    for required in sorted(REQUIRED_TOP_LEVEL):
        if required not in top_titles:
            errors.append(f"missing required repository-wide section: {required}")

    reserved = {"Capture Metadata", "Evidence Rules", "Cross-Cutting Contracts"}
    directory_sections = [(start, end, title) for start, end, title in spans if title not in reserved]
    if not directory_sections:
        errors.append("no major directory sections were found")

    unresolved_count = 0
    claim_count = 0
    for start, end, title in directory_sections:
        body = lines[start + 1 : end]
        subsection_indexes: dict[str, int] = {}
        for relative_index, line in enumerate(body):
            if line.startswith("### "):
                subsection_indexes[line[4:].strip()] = relative_index

        for dimension in REQUIRED_DIMENSIONS:
            if dimension not in subsection_indexes:
                errors.append(f"{title}: missing subsection '{dimension}'")
                continue
            subsection_start = subsection_indexes[dimension] + 1
            later = [index for name, index in subsection_indexes.items() if index > subsection_indexes[dimension]]
            subsection_end = min(later) if later else len(body)
            subsection_text = "\n".join(body[subsection_start:subsection_end]).strip()
            if not subsection_text:
                errors.append(f"{title}/{dimension}: subsection is empty")
                continue
            if "Evidence:" not in subsection_text:
                errors.append(f"{title}/{dimension}: missing Evidence line")
            labels = LABEL_PATTERN.findall(subsection_text)
            if not labels:
                errors.append(f"{title}/{dimension}: missing confidence label")
            claim_count += len(labels)
            unresolved_count += sum(1 for label in labels if label == "UNRESOLVED")

    result = {
        "valid": not errors,
        "file": requested.name,
        "directory_sections": len(directory_sections),
        "claims": claim_count,
        "unresolved_claims": unresolved_count,
        "errors": errors,
    }
    print(json.dumps(result, indent=2))
    return 0 if not errors else 1


if __name__ == "__main__":
    sys.exit(main())
