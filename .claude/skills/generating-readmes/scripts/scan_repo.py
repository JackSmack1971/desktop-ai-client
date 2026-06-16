#!/usr/bin/env python3
"""Safely scan repository metadata for README generation.

This script is read-only and deliberately ignores secret-bearing files. It uses
only the Python standard library so it can run in restricted Claude Code hosts.
"""
from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None  # type: ignore[assignment]

IGNORE_DIRS = {
    ".git",
    ".hg",
    ".svn",
    "node_modules",
    ".pnpm-store",
    ".yarn/cache",
    "vendor",
    "dist",
    "build",
    "out",
    "coverage",
    "target",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".cache",
    ".venv",
    "venv",
    "env",
    "__pycache__",
}

SECRET_FILENAMES = {
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".npmrc",
    ".pypirc",
    ".netrc",
    "id_rsa",
    "id_ed25519",
    "credentials",
    "credentials.json",
    "secrets.json",
}

SAFE_ENV_EXAMPLES = {
    ".env.example",
    ".env.sample",
    ".env.template",
    "example.env",
    "sample.env",
}

IMPORTANT_FILES = {
    "package.json",
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "bun.lockb",
    "pyproject.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "setup.py",
    "setup.cfg",
    "Cargo.toml",
    "Cargo.lock",
    "go.mod",
    "go.sum",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yml",
    "compose.yaml",
    "Makefile",
    "makefile",
    "justfile",
    "Justfile",
    "Taskfile.yml",
    "Taskfile.yaml",
    "LICENSE",
    "LICENSE.md",
    "COPYING",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "CODE_OF_CONDUCT.md",
    "README.md",
    "AGENTS.md",
    "CLAUDE.md",
}

LANGUAGE_EXTENSIONS = {
    ".ts": "TypeScript",
    ".tsx": "TypeScript",
    ".js": "JavaScript",
    ".jsx": "JavaScript",
    ".py": "Python",
    ".rs": "Rust",
    ".go": "Go",
    ".java": "Java",
    ".kt": "Kotlin",
    ".cs": "C#",
    ".rb": "Ruby",
    ".php": "PHP",
    ".swift": "Swift",
    ".svelte": "Svelte",
    ".vue": "Vue",
    ".sol": "Solidity",
}


def resolve_root(root: str) -> Path:
    path = Path(root).expanduser().resolve()
    if not path.exists() or not path.is_dir():
        raise SystemExit(f"root is not a directory: {path}")
    return path


def is_ignored_dir(path: Path) -> bool:
    parts = set(path.parts)
    return any(ignored in parts for ignored in IGNORE_DIRS)


def safe_read_text(path: Path, max_bytes: int = 512_000) -> str:
    if path.name in SECRET_FILENAMES and path.name not in SAFE_ENV_EXAMPLES:
        return ""
    try:
        if path.stat().st_size > max_bytes:
            return ""
        return path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def rel(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def find_existing(root: Path, names: Iterable[str]) -> List[Path]:
    found: List[Path] = []
    for name in names:
        candidate = root / name
        if candidate.exists() and candidate.is_file():
            found.append(candidate)
    return sorted(found, key=lambda p: p.name.lower())


def iter_files(root: Path, max_files: int) -> Iterable[Path]:
    count = 0
    for current, dirs, files in os.walk(root):
        current_path = Path(current)
        dirs[:] = [d for d in dirs if not is_ignored_dir(current_path / d)]
        for filename in files:
            path = current_path / filename
            if path.name in SECRET_FILENAMES and path.name not in SAFE_ENV_EXAMPLES:
                continue
            yield path
            count += 1
            if count >= max_files:
                return


def parse_package_json(path: Path) -> Dict[str, Any]:
    data: Dict[str, Any] = {}
    text = safe_read_text(path)
    if not text:
        return data
    try:
        raw = json.loads(text)
    except json.JSONDecodeError as exc:
        return {"parse_error": str(exc)}
    for key in ["name", "version", "description", "license", "type", "packageManager", "bin", "main", "module", "exports", "engines"]:
        if key in raw:
            data[key] = raw[key]
    data["scripts"] = raw.get("scripts", {}) if isinstance(raw.get("scripts"), dict) else {}
    data["dependencies"] = sorted((raw.get("dependencies") or {}).keys())
    data["devDependencies"] = sorted((raw.get("devDependencies") or {}).keys())
    return data


def parse_pyproject(path: Path) -> Dict[str, Any]:
    text = safe_read_text(path)
    if not text:
        return {}
    if tomllib is not None:
        try:
            raw = tomllib.loads(text)
            project = raw.get("project", {}) if isinstance(raw, dict) else {}
            tool = raw.get("tool", {}) if isinstance(raw, dict) else {}
            return {
                "name": project.get("name"),
                "version": project.get("version"),
                "description": project.get("description"),
                "requires_python": project.get("requires-python"),
                "dependencies": project.get("dependencies", []),
                "optional_dependencies": project.get("optional-dependencies", {}),
                "scripts": project.get("scripts", {}),
                "tool_sections": sorted(tool.keys()) if isinstance(tool, dict) else [],
                "build_system": raw.get("build-system", {}),
            }
        except Exception as exc:  # noqa: BLE001
            return {"parse_error": str(exc)}
    fallback: Dict[str, Any] = {}
    for field in ["name", "version", "description"]:
        match = re.search(rf'^\s*{field}\s*=\s*["\']([^"\']+)["\']', text, re.MULTILINE)
        if match:
            fallback[field] = match.group(1)
    return fallback


def parse_cargo(path: Path) -> Dict[str, Any]:
    text = safe_read_text(path)
    data: Dict[str, Any] = {}
    for field in ["name", "version", "edition", "license", "description"]:
        match = re.search(rf'^\s*{field}\s*=\s*["\']([^"\']+)["\']', text, re.MULTILINE)
        if match:
            data[field] = match.group(1)
    return data


def parse_go_mod(path: Path) -> Dict[str, Any]:
    text = safe_read_text(path)
    data: Dict[str, Any] = {}
    module = re.search(r"^module\s+(\S+)", text, re.MULTILINE)
    version = re.search(r"^go\s+(\S+)", text, re.MULTILINE)
    if module:
        data["module"] = module.group(1)
    if version:
        data["go"] = version.group(1)
    return data


def parse_make_targets(path: Path) -> List[str]:
    text = safe_read_text(path)
    targets: List[str] = []
    for line in text.splitlines():
        if line.startswith("\t") or line.lstrip().startswith("#"):
            continue
        match = re.match(r"^([A-Za-z0-9_.-]+)\s*:(?![=])", line)
        if match and not match.group(1).startswith("."):
            targets.append(match.group(1))
    return sorted(set(targets))


def parse_env_example(path: Path) -> List[str]:
    text = safe_read_text(path, max_bytes=128_000)
    names: List[str] = []
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or line.startswith("export ") is False and "=" not in line:
            continue
        if line.startswith("export "):
            line = line[len("export ") :].strip()
        key = line.split("=", 1)[0].strip()
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", key):
            names.append(key)
    return sorted(set(names))


def detect_package_manager(root: Path, package_json: Dict[str, Any]) -> Dict[str, str]:
    package_manager = package_json.get("packageManager")
    if isinstance(package_manager, str):
        return {"name": package_manager.split("@", 1)[0], "source": "package.json packageManager"}
    lock_order = [
        ("pnpm-lock.yaml", "pnpm"),
        ("bun.lockb", "bun"),
        ("yarn.lock", "yarn"),
        ("package-lock.json", "npm"),
    ]
    for filename, name in lock_order:
        if (root / filename).exists():
            return {"name": name, "source": filename}
    if (root / "package.json").exists():
        return {"name": "npm", "source": "package.json [INFERRED]"}
    return {}


def command_for_script(pm: str, script: str) -> str:
    if pm == "npm":
        return f"npm run {script}"
    if pm == "pnpm":
        return f"pnpm {script}"
    if pm == "yarn":
        return f"yarn {script}"
    if pm == "bun":
        return f"bun run {script}"
    return f"<package-manager> run {script}"


def candidate_commands(root: Path, package_json: Dict[str, Any], pyproject: Dict[str, Any], make_targets: List[str]) -> List[Dict[str, str]]:
    commands: List[Dict[str, str]] = []
    pm_info = detect_package_manager(root, package_json)
    pm = pm_info.get("name", "")
    if package_json:
        if pm:
            if pm == "npm":
                install = "npm ci" if (root / "package-lock.json").exists() else "npm install"
            elif pm == "pnpm":
                install = "pnpm install --frozen-lockfile" if (root / "pnpm-lock.yaml").exists() else "pnpm install"
            elif pm == "yarn":
                install = "yarn install --frozen-lockfile" if (root / "yarn.lock").exists() else "yarn install"
            elif pm == "bun":
                install = "bun install"
            else:
                install = f"{pm} install"
            commands.append({"category": "install", "command": install, "source": pm_info.get("source", "package manager")})
        scripts = package_json.get("scripts", {}) if isinstance(package_json.get("scripts"), dict) else {}
        script_categories = {
            "dev": "run/dev",
            "start": "run",
            "build": "build",
            "test": "test",
            "lint": "lint",
            "typecheck": "typecheck",
            "check": "verification",
            "format": "format",
        }
        for script, category in script_categories.items():
            if script in scripts:
                commands.append({"category": category, "command": command_for_script(pm or "npm", script), "source": f"package.json scripts.{script}"})
    if (root / "requirements.txt").exists():
        commands.append({"category": "install", "command": "python -m pip install -r requirements.txt", "source": "requirements.txt"})
    if pyproject:
        if pyproject.get("name"):
            commands.append({"category": "install", "command": "python -m pip install -e .", "source": "pyproject.toml [INFERRED]"})
        scripts = pyproject.get("scripts", {})
        if isinstance(scripts, dict):
            for name in sorted(scripts):
                commands.append({"category": "cli", "command": name, "source": f"pyproject.toml project.scripts.{name}"})
        tool_sections = set(pyproject.get("tool_sections", [])) if isinstance(pyproject.get("tool_sections"), list) else set()
        if "pytest" in tool_sections:
            commands.append({"category": "test", "command": "python -m pytest", "source": "pyproject.toml tool.pytest [INFERRED]"})
        if "ruff" in tool_sections:
            commands.append({"category": "lint", "command": "python -m ruff check .", "source": "pyproject.toml tool.ruff [INFERRED]"})
        if "mypy" in tool_sections:
            commands.append({"category": "typecheck", "command": "python -m mypy .", "source": "pyproject.toml tool.mypy [INFERRED]"})
    for target in make_targets:
        category = "automation"
        lowered = target.lower()
        if lowered in {"test", "tests"}:
            category = "test"
        elif lowered in {"lint", "typecheck", "check", "verify", "build", "dev", "run", "start", "install"}:
            category = lowered
        commands.append({"category": category, "command": f"make {target}", "source": "Makefile target"})
    return commands


def detect_license(root: Path, package_json: Dict[str, Any], cargo: Dict[str, Any]) -> Dict[str, str]:
    for filename in ["LICENSE", "LICENSE.md", "COPYING"]:
        path = root / filename
        if path.exists():
            text = safe_read_text(path, max_bytes=64_000).lower()
            if "mit license" in text:
                return {"name": "MIT", "source": filename}
            if "apache license" in text:
                return {"name": "Apache", "source": filename}
            if "gnu general public license" in text:
                return {"name": "GPL", "source": filename}
            if "bsd" in text and "redistribution" in text:
                return {"name": "BSD", "source": filename}
            return {"name": "Present", "source": filename}
    for source_name, data in [("package.json", package_json), ("Cargo.toml", cargo)]:
        license_value = data.get("license") if isinstance(data, dict) else None
        if isinstance(license_value, str) and license_value.strip():
            return {"name": license_value.strip(), "source": source_name}
    return {"name": "Not found", "source": "No LICENSE file or manifest license field found"}


def count_languages(root: Path, max_files: int) -> Dict[str, int]:
    counts: Dict[str, int] = {}
    for path in iter_files(root, max_files=max_files):
        language = LANGUAGE_EXTENSIONS.get(path.suffix.lower())
        if language:
            counts[language] = counts.get(language, 0) + 1
    return dict(sorted(counts.items(), key=lambda item: item[1], reverse=True))


def detect_repo_type(root: Path, package_json: Dict[str, Any], pyproject: Dict[str, Any], languages: Dict[str, int]) -> Dict[str, str]:
    deps = set(package_json.get("dependencies", [])) | set(package_json.get("devDependencies", []))
    if ".claude" in [p.name for p in root.iterdir() if p.is_dir()] or (root / "AGENTS.md").exists() or (root / "CLAUDE.md").exists():
        return {"type": "AI agent / automation", "source": "Claude/agent project files [INFERRED]"}
    if any(dep in deps for dep in ["next", "react", "svelte", "@sveltejs/kit", "vite", "vue", "angular"]):
        return {"type": "Web app", "source": "package dependencies [INFERRED]"}
    if package_json.get("bin") or pyproject.get("scripts"):
        return {"type": "CLI tool", "source": "CLI entry point metadata [INFERRED]"}
    if (root / "go.mod").exists() or (root / "Cargo.toml").exists() or pyproject.get("name"):
        return {"type": "Library/package or backend", "source": "language package manifest [INFERRED]"}
    if "Solidity" in languages:
        return {"type": "Smart contract / Web3", "source": "Solidity files [INFERRED]"}
    return {"type": "Unknown", "source": "Insufficient evidence"}


def workflow_files(root: Path) -> List[str]:
    workflow_dir = root / ".github" / "workflows"
    if not workflow_dir.exists():
        return []
    return sorted(rel(path, root) for path in workflow_dir.glob("*.y*ml") if path.is_file())


def major_directories(root: Path) -> List[Dict[str, str]]:
    dirs: List[Dict[str, str]] = []
    for path in sorted([p for p in root.iterdir() if p.is_dir()], key=lambda p: p.name.lower()):
        if is_ignored_dir(path) or path.name.startswith(".") and path.name not in {".github", ".claude"}:
            continue
        sample_files = []
        try:
            for child in sorted(path.iterdir(), key=lambda p: p.name.lower()):
                if child.is_file() and child.name not in SECRET_FILENAMES:
                    sample_files.append(child.name)
                if len(sample_files) >= 3:
                    break
        except OSError:
            pass
        dirs.append({"path": path.name + "/", "sample_files": sample_files})
    return dirs


def governance_files(root: Path) -> Dict[str, Optional[str]]:
    mapping = {
        "license": ["LICENSE", "LICENSE.md", "COPYING"],
        "contributing": ["CONTRIBUTING.md", ".github/CONTRIBUTING.md"],
        "security": ["SECURITY.md", ".github/SECURITY.md"],
        "code_of_conduct": ["CODE_OF_CONDUCT.md", ".github/CODE_OF_CONDUCT.md"],
    }
    result: Dict[str, Optional[str]] = {}
    for key, candidates in mapping.items():
        result[key] = None
        for candidate in candidates:
            if (root / candidate).exists():
                result[key] = candidate
                break
    return result


def build_inventory(root: Path, max_files: int) -> Dict[str, Any]:
    package_json = parse_package_json(root / "package.json") if (root / "package.json").exists() else {}
    pyproject = parse_pyproject(root / "pyproject.toml") if (root / "pyproject.toml").exists() else {}
    cargo = parse_cargo(root / "Cargo.toml") if (root / "Cargo.toml").exists() else {}
    go_mod = parse_go_mod(root / "go.mod") if (root / "go.mod").exists() else {}
    make_targets = []
    for makefile in [root / "Makefile", root / "makefile"]:
        if makefile.exists():
            make_targets = parse_make_targets(makefile)
            break
    important = [rel(path, root) for path in find_existing(root, IMPORTANT_FILES | SAFE_ENV_EXAMPLES)]
    safe_env = []
    for path in find_existing(root, SAFE_ENV_EXAMPLES):
        safe_env.append({"path": rel(path, root), "variables": parse_env_example(path)})
    languages = count_languages(root, max_files=max_files)
    inventory = {
        "root": root.as_posix(),
        "project_name_candidates": [
            value for value in [package_json.get("name"), pyproject.get("name"), cargo.get("name"), go_mod.get("module"), root.name] if value
        ],
        "repo_type": detect_repo_type(root, package_json, pyproject, languages),
        "languages": languages,
        "package_manager": detect_package_manager(root, package_json),
        "package_json": package_json,
        "pyproject": pyproject,
        "cargo": cargo,
        "go_mod": go_mod,
        "commands": candidate_commands(root, package_json, pyproject, make_targets),
        "important_files": important,
        "major_directories": major_directories(root),
        "ci_workflows": workflow_files(root),
        "env_examples": safe_env,
        "license": detect_license(root, package_json, cargo),
        "governance": governance_files(root),
    }
    return inventory


def as_markdown(inventory: Dict[str, Any]) -> str:
    lines: List[str] = []
    lines.append("# Repository Evidence Inventory")
    lines.append("")
    lines.append(f"- Root: `{inventory['root']}`")
    names = inventory.get("project_name_candidates", [])
    if names:
        lines.append(f"- Project name candidates: {', '.join(f'`{name}`' for name in names)}")
    repo_type = inventory.get("repo_type", {})
    lines.append(f"- Repo type: {repo_type.get('type', 'Unknown')} ({repo_type.get('source', 'Unknown source')})")
    languages = inventory.get("languages", {})
    lines.append(f"- Languages by sampled file count: {', '.join(f'{k}: {v}' for k, v in languages.items()) or 'None detected'}")
    pm = inventory.get("package_manager", {})
    lines.append(f"- Package manager: {pm.get('name', 'Unknown')} ({pm.get('source', 'No package manager evidence')})")
    license_info = inventory.get("license", {})
    lines.append(f"- License: {license_info.get('name', 'Unknown')} ({license_info.get('source', 'Unknown source')})")
    lines.append("")
    lines.append("## Important files")
    for item in inventory.get("important_files", []):
        lines.append(f"- `{item}`")
    if not inventory.get("important_files"):
        lines.append("- None detected")
    lines.append("")
    lines.append("## Candidate commands")
    commands = inventory.get("commands", [])
    if commands:
        lines.append("| Category | Command | Source |")
        lines.append("|---|---|---|")
        for command in commands:
            lines.append(f"| {command['category']} | `{command['command']}` | {command['source']} |")
    else:
        lines.append("No grounded commands detected.")
    lines.append("")
    lines.append("## Major directories")
    for item in inventory.get("major_directories", []):
        samples = ", ".join(item.get("sample_files", [])) or "no sampled files"
        lines.append(f"- `{item['path']}` — sample files: {samples}")
    if not inventory.get("major_directories"):
        lines.append("- None detected")
    lines.append("")
    lines.append("## CI workflows")
    for workflow in inventory.get("ci_workflows", []):
        lines.append(f"- `{workflow}`")
    if not inventory.get("ci_workflows"):
        lines.append("- None detected")
    lines.append("")
    lines.append("## Environment examples")
    env_examples = inventory.get("env_examples", [])
    if env_examples:
        for item in env_examples:
            vars_text = ", ".join(f"`{name}`" for name in item.get("variables", [])) or "no variables parsed"
            lines.append(f"- `{item['path']}` — {vars_text}")
    else:
        lines.append("- No safe environment example files detected")
    lines.append("")
    lines.append("## Governance")
    governance = inventory.get("governance", {})
    for key in ["license", "contributing", "security", "code_of_conduct"]:
        value = governance.get(key)
        lines.append(f"- {key.replace('_', ' ').title()}: `{value}`" if value else f"- {key.replace('_', ' ').title()}: [TBD]")
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description="Safely scan repository metadata for README generation.")
    parser.add_argument("--root", default=".", help="Repository root to scan. Defaults to current directory.")
    parser.add_argument("--format", choices=["json", "markdown"], default="markdown", help="Output format.")
    parser.add_argument("--max-files", type=int, default=2500, help="Maximum non-ignored files to sample for language counts.")
    args = parser.parse_args()

    root = resolve_root(args.root)
    inventory = build_inventory(root, max_files=max(1, args.max_files))
    if args.format == "json":
        print(json.dumps(inventory, indent=2, sort_keys=True))
    else:
        print(as_markdown(inventory), end="")


if __name__ == "__main__":
    main()
