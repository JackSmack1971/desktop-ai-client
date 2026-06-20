#!/usr/bin/env python3
"""Run a dependency-free smoke test for all bundled CODEOWNERS scripts."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path


def run(args: list[str], cwd: Path, expected: int = 0) -> subprocess.CompletedProcess[str]:
    proc = subprocess.run(args, cwd=str(cwd), check=False, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != expected:
        raise RuntimeError(
            f"Expected exit {expected}, got {proc.returncode}: {' '.join(args)}\n"
            f"stdout:\n{proc.stdout}\nstderr:\n{proc.stderr}"
        )
    return proc


def main() -> int:
    scripts = Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix="generate-codeowners-test-") as temp:
        repo = Path(temp) / "fixture"
        repo.mkdir()
        run(["git", "init", "-q"], repo)
        run(["git", "config", "user.email", "fixture@example.invalid"], repo)
        run(["git", "config", "user.name", "Fixture Author"], repo)
        (repo / "services/payments").mkdir(parents=True)
        (repo / ".github/workflows").mkdir(parents=True)
        (repo / "services/payments/app.py").write_text("print('ok')\n", encoding="utf-8")
        (repo / ".github/workflows/ci.yml").write_text("name: CI\n", encoding="utf-8")
        (repo / "pyproject.toml").write_text("[project]\nname='fixture'\n", encoding="utf-8")
        run(["git", "add", "."], repo)
        run(["git", "commit", "-qm", "fixture"], repo)

        state = repo / ".git/claude-codeowners"
        inventory = state / "inventory.json"
        run([sys.executable, str(scripts / "analyze_repository.py"), "--repo", str(repo), "--output", str(inventory)], repo)
        if not inventory.is_file():
            raise RuntimeError("Inventory was not created")

        plan = {
            "version": 1,
            "repository": "octo-org/fixture",
            "archetype": "modular-application",
            "fallback": None,
            "verified_owners": [
                {
                    "handle": "@octo-org/octocats",
                    "kind": "team",
                    "verification": "owner-map",
                    "write_access": True,
                    "visible": True,
                }
            ],
            "sections": [
                {
                    "title": "Product",
                    "rules": [
                        {
                            "pattern": "/services/payments/",
                            "owners": ["@octo-org/octocats"],
                            "comment": "Payments service",
                            "source": "owner-map",
                            "intent": "owned",
                            "rationale": "Fixture product ownership",
                        }
                    ],
                },
                {
                    "title": "Repository governance",
                    "rules": [
                        {
                            "pattern": "/.github/",
                            "owners": ["@octo-org/octocats"],
                            "comment": "Repository governance and workflows",
                            "source": "owner-map",
                            "intent": "owned",
                            "rationale": "Fixture governance ownership",
                        }
                    ],
                },
                {
                    "title": "Root configuration",
                    "rules": [
                        {
                            "pattern": "/pyproject.toml",
                            "owners": ["@octo-org/octocats"],
                            "comment": "Python project configuration",
                            "source": "owner-map",
                            "intent": "owned",
                            "rationale": "Fixture toolchain ownership",
                        }
                    ],
                },
            ],
        }
        plan_path = state / "ownership-plan.json"
        state.mkdir(parents=True, exist_ok=True)
        plan_path.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")

        run([sys.executable, str(scripts / "render_codeowners.py"), "--repo", str(repo), "--plan", str(plan_path)], repo)
        target = repo / ".github/CODEOWNERS"
        if not target.is_file():
            raise RuntimeError("CODEOWNERS was not rendered")

        validation = state / "validation.json"
        run(
            [
                sys.executable,
                str(scripts / "validate_codeowners.py"),
                "--repo",
                str(repo),
                "--file",
                ".github/CODEOWNERS",
                "--json-out",
                str(validation),
                "--fail-on-unowned",
            ],
            repo,
        )
        report = json.loads(validation.read_text(encoding="utf-8"))
        if report["errors"]:
            raise RuntimeError(f"Unexpected validation errors: {report['errors']}")

        target.write_text(
            "/services/payments/ @octo-org/octocats\n* @octo-org/octocats\n",
            encoding="utf-8",
        )
        invalid = run(
            [sys.executable, str(scripts / "validate_codeowners.py"), "--repo", str(repo), "--file", ".github/CODEOWNERS"],
            repo,
            expected=1,
        )
        if "late-catch-all" not in invalid.stdout:
            raise RuntimeError("Negative validation test did not detect a late catch-all")

    print("generate-codeowners self-test: PASS")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(f"generate-codeowners self-test: FAIL\n{exc}", file=sys.stderr)
        raise SystemExit(1)
