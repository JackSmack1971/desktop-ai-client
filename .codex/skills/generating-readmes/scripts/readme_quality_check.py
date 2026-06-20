#!/usr/bin/env python3
"""Heuristic README quality checker for operational repository manuals."""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any, Dict, List, Tuple

REQUIRED_SECTIONS = [
    "quickstart",
    "features",
    "architecture",
    "directory structure",
    "usage",
    "configuration",
    "developer command center",
    "testing & verification",
    "troubleshooting",
    "stack inventory",
    "reproducibility & maintenance",
    "contributing",
    "governance",
    "roadmap",
    "license",
]

PLACEHOLDER_PATTERNS = [
    r"<[^>\n]+>",
    r"\bTODO\b",
    r"INSERT[_ -]?HERE",
    r"your[-_ ]?(project|repo|command|url)",
]


def resolve_inside_root(root: Path, path_arg: str) -> Path:
    path = Path(path_arg)
    if not path.is_absolute():
        path = root / path
    resolved = path.resolve()
    try:
        resolved.relative_to(root)
    except ValueError as exc:
        raise SystemExit(f"path must be inside root: {resolved}") from exc
    return resolved


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9 &/-]", "", text.strip().lower()).replace("--", "-")


def headings(text: str) -> List[Tuple[int, str]]:
    result: List[Tuple[int, str]] = []
    for line in text.splitlines():
        match = re.match(r"^(#{1,6})\s+(.+?)\s*$", line)
        if match:
            result.append((len(match.group(1)), match.group(2).strip()))
    return result


def has_section(text: str, names: List[str]) -> bool:
    available = [slug(title) for _, title in headings(text)]
    return any(any(name in title for title in available) for name in names)


def section_text(text: str, section_name: str) -> str:
    pattern = re.compile(rf"^##\s+.*{re.escape(section_name)}.*$", re.IGNORECASE | re.MULTILINE)
    match = pattern.search(text)
    if not match:
        return ""
    start = match.end()
    next_match = re.search(r"^##\s+", text[start:], re.MULTILINE)
    end = start + next_match.start() if next_match else len(text)
    return text[start:end]


def count_table_rows(block: str) -> int:
    rows = [line for line in block.splitlines() if line.strip().startswith("|") and line.strip().endswith("|")]
    data_rows = [line for line in rows if not re.match(r"^\|\s*-+", line.strip()) and "---" not in line]
    return max(0, len(data_rows) - 1) if data_rows else 0


def local_links(text: str) -> List[str]:
    links = re.findall(r"\[[^\]]+\]\(([^)]+)\)", text)
    local: List[str] = []
    for link in links:
        if "://" in link or link.startswith("#") or link.startswith("mailto:"):
            continue
        clean = link.split("#", 1)[0]
        if clean:
            local.append(clean)
    return local


def broken_local_links(root: Path, text: str) -> List[str]:
    broken: List[str] = []
    for link in local_links(text):
        candidate = (root / link).resolve()
        try:
            candidate.relative_to(root)
        except ValueError:
            broken.append(link)
            continue
        if not candidate.exists():
            broken.append(link)
    return sorted(set(broken))


def score_readme(root: Path, text: str) -> Dict[str, Any]:
    lower = text.lower()
    h = headings(text)
    missing_sections = [name for name in REQUIRED_SECTIONS if not has_section(text, [name])]
    code_blocks = len(re.findall(r"```", text)) // 2
    inferred_count = text.count("[INFERRED]")
    tbd_count = text.count("[TBD]")
    placeholder_hits: List[str] = []
    for pattern in PLACEHOLDER_PATTERNS:
        placeholder_hits.extend(re.findall(pattern, text, flags=re.IGNORECASE))
    broken_links = broken_local_links(root, text)

    scores: Dict[str, int] = {}
    notes: Dict[str, str] = {}

    title_present = bool(h and h[0][0] == 1)
    opening = "\n".join(text.splitlines()[1:10]) if title_present else ""
    if title_present and re.search(r"\bis\s+(an?|the)\b|\bhelps\b|\bfor\b", opening, re.IGNORECASE):
        scores["Positioning"] = 3
    elif title_present and opening.strip():
        scores["Positioning"] = 2
    elif title_present:
        scores["Positioning"] = 1
    else:
        scores["Positioning"] = 0
    notes["Positioning"] = "Title plus opening value proposition."

    quickstart = section_text(text, "Quickstart")
    if quickstart and "install" in quickstart.lower() and "run" in quickstart.lower() and "verify" in quickstart.lower() and "```" in quickstart:
        scores["Quickstart"] = 3
    elif quickstart and "```" in quickstart:
        scores["Quickstart"] = 2
    elif quickstart:
        scores["Quickstart"] = 1
    else:
        scores["Quickstart"] = 0
    notes["Quickstart"] = "Quickstart should include install, run, and verify commands."

    if placeholder_hits:
        scores["Accuracy"] = 0
    elif inferred_count == 0 and broken_links == []:
        scores["Accuracy"] = 3
    elif inferred_count <= 5 and len(broken_links) <= 1:
        scores["Accuracy"] = 2
    else:
        scores["Accuracy"] = 1
    notes["Accuracy"] = "Penalizes unresolved placeholders, many inferred claims, and broken local links."

    present_sections = len(REQUIRED_SECTIONS) - len(missing_sections)
    if present_sections >= 13 and "table of contents" in lower:
        scores["Structure"] = 3
    elif present_sections >= 10:
        scores["Structure"] = 2
    elif present_sections >= 5:
        scores["Structure"] = 1
    else:
        scores["Structure"] = 0
    notes["Structure"] = "Measures section coverage and table of contents."

    command_center = section_text(text, "Developer Command Center")
    testing = section_text(text, "Testing")
    if code_blocks >= 4 and command_center and testing:
        scores["Commands"] = 3
    elif code_blocks >= 2:
        scores["Commands"] = 2
    elif code_blocks == 1:
        scores["Commands"] = 1
    else:
        scores["Commands"] = 0
    notes["Commands"] = "Measures command code blocks and command inventory sections."

    architecture = section_text(text, "Architecture")
    if "```mermaid" in architecture.lower() and ("component" in architecture.lower() or "roles" in architecture.lower()):
        scores["Architecture"] = 3
    elif architecture and ("```mermaid" in architecture.lower() or "diagram" in architecture.lower() or "component" in architecture.lower()):
        scores["Architecture"] = 2
    elif architecture:
        scores["Architecture"] = 1
    else:
        scores["Architecture"] = 0
    notes["Architecture"] = "Rewards a useful diagram and component role explanation."

    troubleshooting = section_text(text, "Troubleshooting")
    rows = count_table_rows(troubleshooting)
    if rows >= 3:
        scores["Troubleshooting"] = 3
    elif rows >= 1:
        scores["Troubleshooting"] = 2
    elif troubleshooting:
        scores["Troubleshooting"] = 1
    else:
        scores["Troubleshooting"] = 0
    notes["Troubleshooting"] = "Requires at least three troubleshooting rows."

    maintenance = section_text(text, "Reproducibility")
    if maintenance and code_blocks >= 4 and ("fresh clone" in maintenance.lower() or "updating" in maintenance.lower()):
        scores["Maintenance"] = 3
    elif maintenance and "```" in maintenance:
        scores["Maintenance"] = 2
    elif maintenance:
        scores["Maintenance"] = 1
    else:
        scores["Maintenance"] = 0
    notes["Maintenance"] = "Measures reproducible maintenance workflows."

    governance = section_text(text, "Governance")
    license_section = section_text(text, "License")
    if governance and "security" in governance.lower() and "license" in governance.lower() and license_section:
        scores["Governance"] = 3
    elif governance or license_section:
        scores["Governance"] = 2 if "license" in lower else 1
    else:
        scores["Governance"] = 0
    notes["Governance"] = "Rewards contribution, security, support, and license status."

    if "stack inventory" in lower and ("developer command center" in lower or "api" in lower or "agents" in lower or "cli" in lower):
        scores["Adaptation"] = 3
    elif "stack inventory" in lower:
        scores["Adaptation"] = 2
    elif present_sections >= 8:
        scores["Adaptation"] = 1
    else:
        scores["Adaptation"] = 0
    notes["Adaptation"] = "Measures repo-aware details instead of generic documentation."

    total = sum(scores.values())
    return {
        "total": total,
        "max": 30,
        "scores": scores,
        "notes": notes,
        "missing_sections": missing_sections,
        "inferred_count": inferred_count,
        "tbd_count": tbd_count,
        "placeholder_hits": sorted(set(str(hit) for hit in placeholder_hits)),
        "broken_local_links": broken_links,
    }


def as_markdown(result: Dict[str, Any]) -> str:
    lines: List[str] = []
    lines.append(f"# README Quality Check: {result['total']}/{result['max']}")
    lines.append("")
    lines.append("| Category | Score | Note |")
    lines.append("|---|---:|---|")
    for category, score in result["scores"].items():
        lines.append(f"| {category} | {score} | {result['notes'][category]} |")
    lines.append("")
    if result["missing_sections"]:
        lines.append("## Missing sections")
        for section in result["missing_sections"]:
            lines.append(f"- {section}")
        lines.append("")
    if result["placeholder_hits"]:
        lines.append("## Placeholder-like text to remove")
        for hit in result["placeholder_hits"]:
            lines.append(f"- `{hit}`")
        lines.append("")
    if result["broken_local_links"]:
        lines.append("## Broken local links")
        for link in result["broken_local_links"]:
            lines.append(f"- `{link}`")
        lines.append("")
    lines.append(f"- `[INFERRED]` count: {result['inferred_count']}")
    lines.append(f"- `[TBD]` count: {result['tbd_count']}")
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description="Score README quality against the operational README rubric.")
    parser.add_argument("--root", default=".", help="Repository root. Defaults to current directory.")
    parser.add_argument("--readme", default="README.md", help="README path relative to root, or absolute path inside root.")
    parser.add_argument("--min-score", type=int, default=24, help="Minimum acceptable score. Defaults to 24.")
    parser.add_argument("--format", choices=["json", "markdown"], default="markdown", help="Output format.")
    args = parser.parse_args()

    root = Path(args.root).expanduser().resolve()
    if not root.exists() or not root.is_dir():
        raise SystemExit(f"root is not a directory: {root}")
    readme = resolve_inside_root(root, args.readme)
    if not readme.exists():
        raise SystemExit(f"README file not found: {readme}")
    text = readme.read_text(encoding="utf-8", errors="replace")
    result = score_readme(root, text)
    if args.format == "json":
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(as_markdown(result), end="")
    if result["total"] < args.min_score:
        raise SystemExit(f"README quality score {result['total']} is below required minimum {args.min_score}")


if __name__ == "__main__":
    main()
