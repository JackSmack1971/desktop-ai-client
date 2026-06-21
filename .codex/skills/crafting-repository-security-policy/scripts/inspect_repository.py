#!/usr/bin/env python3
"""Generate a read-only security-policy profile for a software repository."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

EXCLUDED_DIRS = {
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vscode",
    "node_modules",
    "vendor",
    "dist",
    "build",
    "target",
    "coverage",
    ".coverage",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    ".venv",
    "venv",
    "env",
}

SENSITIVE_NAMES = {
    ".env",
    ".env.local",
    ".env.production",
    "id_rsa",
    "id_ed25519",
    "credentials",
    "credentials.json",
    "secrets.json",
}

TEXT_SUFFIXES = {
    ".md",
    ".txt",
    ".json",
    ".jsonc",
    ".yaml",
    ".yml",
    ".toml",
    ".ini",
    ".cfg",
    ".conf",
    ".xml",
    ".gradle",
    ".kts",
    ".py",
    ".pyi",
    ".js",
    ".jsx",
    ".ts",
    ".tsx",
    ".rs",
    ".go",
    ".java",
    ".kt",
    ".c",
    ".cc",
    ".cpp",
    ".h",
    ".hpp",
    ".cs",
    ".rb",
    ".php",
    ".swift",
    ".sh",
    ".ps1",
    ".sql",
}

MANIFEST_RULES = {
    "javascript/typescript": ["package.json", "pnpm-lock.yaml", "yarn.lock", "package-lock.json", "bun.lockb", "bun.lock"],
    "python": ["pyproject.toml", "requirements.txt", "setup.py", "setup.cfg", "Pipfile", "poetry.lock", "uv.lock"],
    "rust": ["Cargo.toml", "Cargo.lock"],
    "go": ["go.mod", "go.sum"],
    "java/kotlin": ["pom.xml", "build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"],
    "dotnet": ["*.csproj", "*.fsproj", "*.sln", "global.json"],
    "ruby": ["Gemfile", "Gemfile.lock", "*.gemspec"],
    "php": ["composer.json", "composer.lock"],
    "swift": ["Package.swift"],
    "elixir": ["mix.exs", "mix.lock"],
    "c/c++": ["CMakeLists.txt", "meson.build", "Makefile", "configure.ac"],
}

DOMAIN_SIGNALS = {
    "ai-agent": {
        "tokens": ["anthropic", "openai", "langchain", "llamaindex", "transformers", "modelcontextprotocol", "mcp", "tool_call", "function_call"],
        "paths": ["prompts", "agents", "models", "tools", "mcp"],
    },
    "web-api": {
        "tokens": ["express", "fastapi", "flask", "django", "next", "nestjs", "spring-boot", "axum", "actix-web", "gin-gonic", "echo"],
        "paths": ["routes", "controllers", "api", "server", "middleware"],
    },
    "desktop-local": {
        "tokens": ["tauri", "electron", "wry", "webview", "keyring", "keychain"],
        "paths": ["src-tauri", "electron", "desktop", "updater"],
    },
    "cli-developer-tool": {
        "tokens": ["clap", "argparse", "click", "typer", "commander", "cobra", "urfave/cli"],
        "paths": ["cli", "cmd", "commands", "bin"],
    },
    "cryptography": {
        "tokens": ["openssl", "ring", "rustls", "libsodium", "cryptography", "pyca", "jsonwebtoken", "jwt", "aes", "ed25519", "secp256k1"],
        "paths": ["crypto", "cryptography", "keys", "signing"],
    },
    "embedded-iot": {
        "tokens": ["platformio", "arduino", "zephyr", "freertos", "esp-idf", "mbed"],
        "paths": ["firmware", "bootloader", "boards", "hal", "drivers"],
    },
    "infrastructure-cloud": {
        "tokens": ["kubernetes", "helm", "terraform", "pulumi", "ansible", "docker", "containerd"],
        "paths": ["charts", "k8s", "kubernetes", "terraform", "deploy", "infra"],
    },
    "browser-extension": {
        "tokens": ["webextension", "chrome.runtime", "browser.runtime", "manifest_version"],
        "paths": ["extension", "browser-extension"],
    },
    "mobile": {
        "tokens": ["react-native", "flutter", "android", "ios", "swiftui", "jetpack compose"],
        "paths": ["android", "ios", "mobile"],
    },
}

RISK_PATTERNS = {
    "authentication/authorization": [r"\bauth(?:entication|orization)?\b", r"\brbac\b", r"\bpermission(?:s)?\b", r"\bsession\b", r"\boauth\b"],
    "shell/process execution": [r"subprocess", r"child_process", r"Command::new", r"os\.system", r"exec\(", r"spawn\("],
    "filesystem access": [r"readFile", r"writeFile", r"std::fs", r"pathlib", r"open\(", r"File::open", r"fs\."],
    "network listener/client": [r"listen\(", r"bind\(", r"http[s]?://", r"reqwest", r"requests\.", r"fetch\(", r"socket"],
    "secret/key handling": [r"api[_-]?key", r"secret", r"token", r"keychain", r"keyring", r"credential"],
    "archive/parsing": [r"tarfile", r"zipfile", r"archive", r"deserialize", r"unmarshal", r"parser"],
    "plugin/tool execution": [r"plugin", r"extension", r"tool[_-]?call", r"function[_-]?call", r"mcp"],
    "update/release integrity": [r"updater", r"auto[_-]?update", r"signature", r"checksum", r"provenance", r"release"],
    "database/persistence": [r"sql", r"database", r"sqlite", r"postgres", r"mysql", r"mongodb", r"redis"],
    "multi-user/tenancy": [r"tenant", r"multi[_-]?user", r"organization", r"workspace", r"account_id", r"user_id"],
}

CONTROL_RULES = {
    "codeql": [".github/workflows/codeql", ".github/workflows/codeql-analysis"],
    "dependency-review": [".github/workflows/dependency-review"],
    "dependabot": [".github/dependabot.yml", ".github/dependabot.yaml"],
    "renovate": ["renovate.json", "renovate.json5", ".renovaterc", ".renovaterc.json"],
    "scorecard": [".github/workflows/scorecard", ".github/workflows/scorecards"],
    "allstar": [".allstar/", "allstar.yaml", "allstar.yml"],
    "sbom": ["sbom", "cyclonedx", "spdx"],
    "vex": ["vex", "openvex"],
    "slsa-provenance": ["slsa", "provenance"],
    "sigstore-cosign": ["cosign", "sigstore"],
    "signed-releases": ["gpg", "minisign", "signing", "signature"],
}

POLICY_LOCATIONS = ["SECURITY.md", ".github/SECURITY.md", "docs/SECURITY.md"]
HIGH_SIGNAL_NAMES = {
    "README.md",
    "README.rst",
    "README.txt",
    "CONTRIBUTING.md",
    "GOVERNANCE.md",
    "SUPPORT.md",
    "CODEOWNERS",
    "CHANGELOG.md",
    "RELEASE.md",
    "RELEASING.md",
    "Dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "tauri.conf.json",
    "tauri.conf.json5",
    "electron-builder.yml",
    "electron-builder.yaml",
    "platformio.ini",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repository", nargs="?", default=".", help="Repository directory (default: current directory)")
    parser.add_argument("--output", help="Write JSON profile to this path; stdout when omitted")
    parser.add_argument("--max-files", type=int, default=20000, help="Maximum files to inventory (default: 20000)")
    args = parser.parse_args()
    if args.max_files < 100:
        parser.error("--max-files must be at least 100")
    return args


def run_git(repo: Path, args: list[str]) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), *args],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            timeout=10,
        )
        return result.stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def resolve_root(path: Path) -> tuple[Path, bool]:
    path = path.expanduser().resolve()
    if not path.exists() or not path.is_dir():
        raise ValueError(f"repository directory does not exist: {path}")
    git_root = run_git(path, ["rev-parse", "--show-toplevel"])
    if git_root:
        return Path(git_root).resolve(), True
    return path, False


def inventory_files(root: Path, max_files: int) -> tuple[list[Path], bool]:
    files: list[Path] = []
    truncated = False
    for current_root, dirs, names in os.walk(root):
        dirs[:] = [d for d in dirs if d not in EXCLUDED_DIRS and not d.startswith(".cache")]
        for name in names:
            if len(files) >= max_files:
                truncated = True
                return files, truncated
            p = Path(current_root, name)
            try:
                rel = p.relative_to(root)
            except ValueError:
                continue
            if any(part in EXCLUDED_DIRS for part in rel.parts):
                continue
            files.append(p)
    return files, truncated


def rel(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def safe_read(path: Path, limit: int = 512_000) -> str:
    if path.name in SENSITIVE_NAMES or path.suffix.lower() in {".pem", ".key", ".p12", ".pfx"}:
        return ""
    try:
        if path.stat().st_size > limit:
            return ""
        data = path.read_bytes()
        if b"\x00" in data[:4096]:
            return ""
        return data.decode("utf-8", errors="replace")
    except OSError:
        return ""


def matches_glob_name(path: Path, pattern: str) -> bool:
    if "*" in pattern:
        return path.match(pattern)
    return path.name == pattern or path.as_posix().endswith("/" + pattern)


def detect_ecosystems(root: Path, files: list[Path]) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for ecosystem, patterns in MANIFEST_RULES.items():
        matches = sorted({rel(root, p) for p in files for pattern in patterns if matches_glob_name(p, pattern)})
        if matches:
            results.append({"name": ecosystem, "classification": "verified", "sources": matches[:20]})
    return results


def selected_text_corpus(root: Path, files: list[Path]) -> tuple[str, list[tuple[str, str]]]:
    chunks: list[str] = []
    sources: list[tuple[str, str]] = []
    candidates: list[Path] = []
    for p in files:
        rp = rel(root, p)
        name = p.name
        if (
            name in HIGH_SIGNAL_NAMES
            or name in {item for values in MANIFEST_RULES.values() for item in values if "*" not in item}
            or rp.startswith(".github/workflows/")
            or rp in POLICY_LOCATIONS
            or (p.suffix.lower() in TEXT_SUFFIXES and any(part in {"src", "app", "server", "backend", "cmd", "crates"} for part in p.parts))
        ):
            candidates.append(p)
    for p in candidates[:500]:
        text = safe_read(p, 160_000)
        if not text:
            continue
        rp = rel(root, p)
        lowered = text.lower()
        chunks.append(lowered)
        sources.append((rp, lowered))
    return "\n".join(chunks), sources


def detect_domains(root: Path, files: list[Path], corpus: str) -> list[dict[str, Any]]:
    rel_paths = [rel(root, p).lower() for p in files]
    results: list[dict[str, Any]] = []
    for domain, rule in DOMAIN_SIGNALS.items():
        token_hits = [token for token in rule["tokens"] if token.lower() in corpus]
        path_hits = sorted({p for p in rel_paths if any(segment in p.split("/") for segment in rule["paths"])})
        score = len(token_hits) * 2 + min(len(path_hits), 3)
        if score >= 2:
            results.append(
                {
                    "name": domain,
                    "classification": "inferred",
                    "signals": token_hits[:10],
                    "sources": path_hits[:10],
                }
            )
    return results


def detect_risks(source_texts: list[tuple[str, str]]) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for risk, patterns in RISK_PATTERNS.items():
        matched_paths: list[str] = []
        matched_patterns: list[str] = []
        for path, text in source_texts:
            for pattern in patterns:
                if re.search(pattern, text, re.IGNORECASE):
                    matched_paths.append(path)
                    matched_patterns.append(pattern)
                    break
        if matched_paths:
            results.append(
                {
                    "name": risk,
                    "classification": "inferred",
                    "sources": sorted(set(matched_paths))[:15],
                    "signals": sorted(set(matched_patterns))[:8],
                }
            )
    return results


def detect_policies(root: Path) -> list[dict[str, Any]]:
    policies: list[dict[str, Any]] = []
    for location in POLICY_LOCATIONS:
        p = root / location
        if not p.is_file():
            continue
        text = safe_read(p)
        emails = sorted(set(re.findall(r"[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}", text, re.IGNORECASE)))
        pvr = bool(re.search(r"private vulnerability reporting|report a vulnerability", text, re.IGNORECASE))
        safe_harbor = bool(re.search(r"safe harbor|will not (?:initiate|pursue|support).*legal", text, re.IGNORECASE | re.DOTALL))
        fixed_timeline = bool(re.search(r"within\s+\d+\s+(?:hours?|business days?|days?|weeks?)", text, re.IGNORECASE))
        policies.append(
            {
                "path": location,
                "emails": emails,
                "mentions_github_pvr": pvr,
                "contains_safe_harbor_language": safe_harbor,
                "contains_fixed_response_timeline": fixed_timeline,
                "classification": "verified",
            }
        )
    return policies


def detect_controls(root: Path, files: list[Path], source_texts: list[tuple[str, str]]) -> list[dict[str, Any]]:
    rel_paths = [rel(root, p).lower() for p in files]
    results: list[dict[str, Any]] = []
    for control, needles in CONTROL_RULES.items():
        path_hits: set[str] = set()
        content_hits: set[str] = set()
        for rp in rel_paths:
            if any(needle.lower() in rp for needle in needles):
                path_hits.add(rp)
        if control in {"sbom", "vex", "slsa-provenance", "sigstore-cosign", "signed-releases"}:
            for path, text in source_texts:
                if any(needle.lower() in text for needle in needles):
                    content_hits.add(path)
        if path_hits or content_hits:
            results.append(
                {
                    "name": control,
                    "classification": "configured",
                    "sources": sorted(path_hits | content_hits)[:20],
                }
            )
    return results


def detect_release_support(root: Path, is_git: bool, files: list[Path]) -> dict[str, Any]:
    tags: list[str] = []
    branches: list[str] = []
    if is_git:
        tag_output = run_git(root, ["tag", "--sort=-version:refname"])
        branch_output = run_git(root, ["branch", "--format=%(refname:short)"])
        if tag_output:
            tags = [line for line in tag_output.splitlines() if line][:20]
        if branch_output:
            branches = [line for line in branch_output.splitlines() if line][:50]
    rel_paths = {rel(root, p) for p in files}
    changelog = sorted(p for p in rel_paths if Path(p).name.lower().startswith(("changelog", "changes", "history")))[:10]
    release_files = sorted(
        p
        for p in rel_paths
        if "release" in p.lower() or p.lower().startswith(".github/workflows/") and "publish" in p.lower()
    )[:20]
    maintenance_branches = [b for b in branches if re.search(r"(?:release|maint|support|stable|v?\d+[._-]x)", b, re.IGNORECASE)]

    if maintenance_branches:
        model = "maintained_lines"
        classification = "inferred"
    elif tags:
        model = "latest_release"
        classification = "inferred"
    else:
        model = "rolling"
        classification = "inferred"

    return {
        "suggested_model": model,
        "classification": classification,
        "tags": tags,
        "branches": branches,
        "maintenance_branch_signals": maintenance_branches,
        "changelog_files": changelog,
        "release_files": release_files,
        "warning": "Support commitments require direct confirmation; tags and branches are evidence, not a complete lifecycle policy.",
    }


def detect_high_signal_files(root: Path, files: list[Path]) -> list[str]:
    selected: set[str] = set()
    for p in files:
        rp = rel(root, p)
        if (
            p.name in HIGH_SIGNAL_NAMES
            or rp in POLICY_LOCATIONS
            or rp.startswith(".github/workflows/")
            or rp.startswith(".github/ISSUE_TEMPLATE/")
            or p.name in {item for values in MANIFEST_RULES.values() for item in values if "*" not in item}
            or p.suffix in {".csproj", ".fsproj", ".gemspec"}
        ):
            selected.add(rp)
    return sorted(selected)[:200]


def parse_remote(remote: str | None) -> dict[str, Any]:
    if not remote:
        return {"url": None, "host": None, "owner_repo": None}
    host = None
    owner_repo = None
    patterns = [
        r"^(?:https?|ssh)://(?:[^@/]+@)?([^/:]+)(?::\d+)?/(.+?)(?:\.git)?$",
        r"^[^@]+@([^:]+):(.+?)(?:\.git)?$",
    ]
    for pattern in patterns:
        match = re.match(pattern, remote)
        if match:
            host = match.group(1).lower()
            owner_repo = match.group(2).removesuffix(".git")
            break
    return {"url": remote, "host": host, "owner_repo": owner_repo}


def build_recommendations(profile: dict[str, Any]) -> list[dict[str, str]]:
    recommendations: list[dict[str, str]] = []
    policies = profile["evidence"]["existing_policies"]
    remote_host = profile["repository"]["remote"]["host"]
    if policies:
        recommendations.append({"priority": "normal", "action": "Update the existing canonical policy surgically; preserve verified contacts and approved legal language."})
    else:
        target = ".github/SECURITY.md" if remote_host == "github.com" else "SECURITY.md"
        recommendations.append({"priority": "normal", "action": f"Create {target}."})
    if remote_host == "github.com" and not any(p.get("mentions_github_pvr") for p in policies):
        recommendations.append({"priority": "high", "action": "Verify whether GitHub Private Vulnerability Reporting is enabled before naming it as the intake route."})
    if not any(p.get("emails") or p.get("mentions_github_pvr") for p in policies):
        recommendations.append({"priority": "high", "action": "Configure and verify a private reporting channel; do not invent an address."})
    if profile["evidence"]["supply_chain_controls"]:
        recommendations.append({"priority": "normal", "action": "Inspect configured supply-chain controls and mention only artifacts that are actually published and usable."})
    return recommendations


def main() -> int:
    try:
        args = parse_args()
        root, is_git = resolve_root(Path(args.repository))
        files, truncated = inventory_files(root, args.max_files)
        corpus, source_texts = selected_text_corpus(root, files)
        remote = parse_remote(run_git(root, ["remote", "get-url", "origin"]) if is_git else None)
        current_branch = run_git(root, ["branch", "--show-current"]) if is_git else None
        default_branch = None
        if is_git:
            symbolic = run_git(root, ["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
            if symbolic and "/" in symbolic:
                default_branch = symbolic.split("/", 1)[1]

        profile: dict[str, Any] = {
            "status": "ok",
            "schema_version": "1.0",
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "repository": {
                "root": str(root),
                "is_git": is_git,
                "current_branch": current_branch,
                "default_branch": default_branch,
                "remote": remote,
                "file_count": len(files),
                "inventory_truncated": truncated,
            },
            "evidence": {
                "ecosystems": detect_ecosystems(root, files),
                "domains": detect_domains(root, files, corpus),
                "risk_boundaries": detect_risks(source_texts),
                "release_support": detect_release_support(root, is_git, files),
                "existing_policies": detect_policies(root),
                "supply_chain_controls": detect_controls(root, files, source_texts),
                "high_signal_files": detect_high_signal_files(root, files),
            },
            "limitations": [
                "Remote repository settings, including GitHub Private Vulnerability Reporting, are not verified by local inspection.",
                "Detected domains and risk boundaries are heuristics and require direct source review.",
                "Configured workflows do not prove that release artifacts are successfully published.",
                "Support models inferred from tags or branches require maintainer confirmation before becoming commitments.",
            ],
            "warnings": [],
        }
        if truncated:
            profile["warnings"].append(f"File inventory stopped at --max-files={args.max_files}; some evidence may be missing.")
        if not is_git:
            profile["warnings"].append("Directory is not a Git repository; branch, tag, and remote evidence is unavailable.")
        profile["recommendations"] = build_recommendations(profile)

        encoded = json.dumps(profile, indent=2, sort_keys=False) + "\n"
        if args.output:
            output = Path(args.output).expanduser()
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(encoded, encoding="utf-8")
            print(json.dumps({"status": "ok", "output": str(output), "repository": str(root)}))
        else:
            sys.stdout.write(encoded)
        return 0
    except SystemExit:
        raise
    except (OSError, ValueError) as exc:
        print(json.dumps({"status": "error", "error": str(exc)}), file=sys.stderr)
        return 1
    except Exception as exc:  # defensive boundary for predictable CLI behavior
        print(json.dumps({"status": "error", "error": f"unexpected inspection failure: {exc}"}), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
