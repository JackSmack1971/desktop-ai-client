#!/usr/bin/env python3
"""Discover read-only GitHub owner candidates with explicit repository access."""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path
from typing import Any

from codeowners_common import atomic_write_json, git_path, parse_github_remote, resolve_repo, run

WRITE_PERMISSIONS = {"push", "maintain", "admin"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".", help="Path inside the target Git worktree")
    parser.add_argument("--output", help="JSON output path; defaults to the Git metadata state directory")
    return parser.parse_args()


def gh_json(repo: Path, endpoint: str, *, paginate: bool = False) -> Any:
    command = ["gh", "api"]
    if paginate:
        command.append("--paginate")
    command.extend([endpoint, "--header", "Accept: application/vnd.github+json"])
    proc = run(command, repo, check=False)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or proc.stdout.strip() or f"GitHub API failed: {endpoint}")
    text = proc.stdout.strip()
    if not text:
        return None
    if paginate:
        # gh --paginate can concatenate top-level JSON arrays. Decode all documents safely.
        decoder = json.JSONDecoder()
        index = 0
        combined: list[Any] = []
        while index < len(text):
            while index < len(text) and text[index].isspace():
                index += 1
            if index >= len(text):
                break
            value, end = decoder.raw_decode(text, index)
            if isinstance(value, list):
                combined.extend(value)
            else:
                combined.append(value)
            index = end
        return combined
    return json.loads(text)


def write_result(output: Path, payload: dict[str, Any]) -> int:
    output.parent.mkdir(parents=True, exist_ok=True)
    atomic_write_json(output, payload)
    print(json.dumps({"output": str(output), **{key: payload.get(key) for key in ("available", "reason")}}, indent=2))
    return 0


def main() -> int:
    args = parse_args()
    try:
        repo = resolve_repo(args.repo)
        output = (
            Path(args.output).expanduser().resolve()
            if args.output
            else git_path(repo, "claude-codeowners") / "github-owners.json"
        )

        remote_proc = run(["git", "remote", "get-url", "origin"], repo, check=False)
        if remote_proc.returncode != 0:
            return write_result(
                output,
                {"schema_version": 1, "available": False, "reason": "origin remote is not configured"},
            )
        parsed = parse_github_remote(remote_proc.stdout.strip())
        if not parsed:
            return write_result(
                output,
                {"schema_version": 1, "available": False, "reason": "origin is not a supported github.com remote"},
            )
        owner, repository = parsed

        if shutil.which("gh") is None:
            return write_result(
                output,
                {
                    "schema_version": 1,
                    "available": False,
                    "reason": "GitHub CLI is not installed",
                    "repository": f"{owner}/{repository}",
                },
            )

        auth = run(["gh", "auth", "status", "--hostname", "github.com"], repo, check=False)
        if auth.returncode != 0:
            return write_result(
                output,
                {
                    "schema_version": 1,
                    "available": False,
                    "reason": "GitHub CLI is not authenticated for github.com",
                    "repository": f"{owner}/{repository}",
                },
            )

        metadata = gh_json(repo, f"repos/{owner}/{repository}")
        if not isinstance(metadata, dict):
            raise RuntimeError("GitHub repository metadata response was not an object")

        owner_type = str((metadata.get("owner") or {}).get("type") or "Unknown")
        teams: list[dict[str, Any]] = []
        team_error: str | None = None
        if owner_type == "Organization":
            try:
                raw_teams = gh_json(repo, f"repos/{owner}/{repository}/teams?per_page=100", paginate=True)
                if isinstance(raw_teams, list):
                    for team in raw_teams:
                        if not isinstance(team, dict):
                            continue
                        permission = str(team.get("permission") or "")
                        organization = str((team.get("organization") or {}).get("login") or owner)
                        slug = str(team.get("slug") or "")
                        if not slug:
                            continue
                        privacy = str(team.get("privacy") or "")
                        privacy_verified = bool(privacy)
                        if not privacy:
                            try:
                                team_details = gh_json(repo, f"orgs/{organization}/teams/{slug}")
                                if isinstance(team_details, dict):
                                    privacy = str(team_details.get("privacy") or "")
                                    privacy_verified = bool(privacy)
                            except RuntimeError:
                                privacy_verified = False
                        visible = privacy_verified and privacy.lower() != "secret"
                        teams.append(
                            {
                                "handle": f"@{organization}/{slug}",
                                "name": team.get("name"),
                                "permission": permission,
                                "write_access": permission in WRITE_PERMISSIONS,
                                "privacy": privacy or None,
                                "privacy_verified": privacy_verified,
                                "visible": visible,
                                "verification": "github-api",
                            }
                        )
            except RuntimeError as exc:
                team_error = str(exc)

        personal_owner: dict[str, Any] | None = None
        if owner_type == "User":
            permission = str((metadata.get("permissions") or {}).get("admin") and "admin" or "")
            # Repository metadata does not expose the owner's effective permission as a named field.
            # The repository owner is authoritative for a user-owned repository.
            personal_owner = {
                "handle": f"@{owner}",
                "kind": "user",
                "write_access": True,
                "visible": True,
                "verification": "personal-repository-owner",
                "note": "Verified from the owner of a user-owned GitHub repository.",
            }

        payload: dict[str, Any] = {
            "schema_version": 1,
            "available": True,
            "repository": f"{owner}/{repository}",
            "repository_owner_type": owner_type,
            "visibility": metadata.get("visibility") or ("private" if metadata.get("private") else "public"),
            "default_branch": metadata.get("default_branch"),
            "archived": bool(metadata.get("archived")),
            "teams": sorted(teams, key=lambda item: item["handle"].lower()),
            "eligible_teams": sorted(
                [item for item in teams if item["write_access"] and item["visible"]],
                key=lambda item: item["handle"].lower(),
            ),
            "personal_repository_owner": personal_owner,
            "team_discovery_error": team_error,
            "notes": [
                "Only teams with verified non-secret visibility and push, maintain, or admin permission are eligible team owners.",
                "This script performs read-only GitHub API calls and never changes repository access.",
            ],
        }
        return write_result(output, payload)
    except (ValueError, RuntimeError, OSError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
