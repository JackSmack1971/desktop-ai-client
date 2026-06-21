#!/usr/bin/env python3
"""Validate a repository SECURITY.md against an evidence profile and policy plan."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

CLASSIFICATIONS = {"verified", "inferred", "unknown", "not-applicable"}
REQUIRED_PLAN_KEYS = {
    "schema_version",
    "mode",
    "target_path",
    "repository_facts",
    "policy_decisions",
    "unresolved_todos",
}
REQUIRED_DECISIONS = {
    "support",
    "intake",
    "scope",
    "out_of_scope",
    "report_requirements",
    "response",
    "safe_harbor",
    "attribution",
    "supply_chain",
    "omitted_modules",
}

PLACEHOLDER_PATTERNS = [
    (r"\b(?:TODO|TBD|FIXME)\b", "unresolved TODO/TBD/FIXME placeholder"),
    (r"example\.(?:com|org|net)", "example domain remains"),
    (r"security@(?:example|yourdomain)", "sample security email remains"),
    (r"0x123456789ABCDEF0", "sample PGP fingerprint remains"),
    (r"\[(?:VERIFIED|APPROVED|INSERT|SELECT|OPTIONAL|REPOSITORY|SUPPORTED|PROJECT|TODO)[^\]]*\]", "template bracket remains"),
    (r"<[^>]*(?:email|url|version|project|contact|replace|insert)[^>]*>", "angle-bracket placeholder remains"),
]

LEGAL_COMMITMENT_PATTERNS = [
    r"will not (?:initiate|pursue|support).*legal action",
    r"authorize you to perform security testing",
    r"consider your research authorized",
    r"waive.*(?:claim|right)",
]

FIXED_TIMELINE_PATTERN = re.compile(
    r"\b(?:will|shall|commit(?:s|ted)? to|guarantee(?:s|d)?)\b[^.\n]{0,100}\bwithin\s+\d+\s+(?:hours?|business days?|days?|weeks?)",
    re.IGNORECASE,
)

EMAIL_PATTERN = re.compile(r"[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}", re.IGNORECASE)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", required=True, help="Path to SECURITY.md")
    parser.add_argument("--profile", help="Path to inspect_repository.py JSON output")
    parser.add_argument("--plan", help="Path to security-policy-plan JSON")
    parser.add_argument("--strict", action="store_true", help="Promote deployment blockers and unsupported claims to errors")
    parser.add_argument("--format", choices=("json", "text"), default="json")
    return parser.parse_args()


def load_json(path: str | None, label: str) -> dict[str, Any] | None:
    if not path:
        return None
    p = Path(path)
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
    except OSError as exc:
        raise ValueError(f"cannot read {label} file {p}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise ValueError(f"invalid JSON in {label} file {p}: {exc}") from exc
    if not isinstance(data, dict):
        raise ValueError(f"{label} file must contain a JSON object: {p}")
    return data


def add_issue(collection: list[dict[str, str]], code: str, message: str) -> None:
    collection.append({"code": code, "message": message})


def validate_evidence_item(item: Any, path: str, errors: list[dict[str, str]]) -> None:
    if not isinstance(item, dict):
        add_issue(errors, "PLAN_ITEM_TYPE", f"{path} must be an object")
        return
    for key in ("statement", "classification", "sources"):
        if key not in item:
            add_issue(errors, "PLAN_ITEM_REQUIRED", f"{path}.{key} is required")
    classification = item.get("classification")
    if classification not in CLASSIFICATIONS:
        add_issue(errors, "PLAN_CLASSIFICATION", f"{path}.classification must be one of {sorted(CLASSIFICATIONS)}")
    sources = item.get("sources")
    if not isinstance(sources, list):
        add_issue(errors, "PLAN_SOURCES_TYPE", f"{path}.sources must be an array")
    elif classification == "verified" and not sources:
        add_issue(errors, "PLAN_VERIFIED_WITHOUT_SOURCE", f"{path} is verified but has no source")


def validate_plan(plan: dict[str, Any], errors: list[dict[str, str]], warnings: list[dict[str, str]]) -> None:
    missing = sorted(REQUIRED_PLAN_KEYS - set(plan))
    for key in missing:
        add_issue(errors, "PLAN_REQUIRED", f"plan.{key} is required")
    if plan.get("schema_version") != "1.0":
        add_issue(errors, "PLAN_SCHEMA_VERSION", "plan.schema_version must be '1.0'")
    if plan.get("mode") not in {"create", "update", "audit"}:
        add_issue(errors, "PLAN_MODE", "plan.mode must be create, update, or audit")
    if not isinstance(plan.get("target_path"), str) or not plan.get("target_path", "").strip():
        add_issue(errors, "PLAN_TARGET", "plan.target_path must be a non-empty string")

    facts = plan.get("repository_facts")
    if not isinstance(facts, list) or not facts:
        add_issue(errors, "PLAN_FACTS", "plan.repository_facts must be a non-empty array")
    else:
        for index, item in enumerate(facts):
            validate_evidence_item(item, f"plan.repository_facts[{index}]", errors)

    decisions = plan.get("policy_decisions")
    if not isinstance(decisions, dict):
        add_issue(errors, "PLAN_DECISIONS", "plan.policy_decisions must be an object")
        return
    for key in sorted(REQUIRED_DECISIONS - set(decisions)):
        add_issue(errors, "PLAN_DECISION_REQUIRED", f"plan.policy_decisions.{key} is required")

    for name in ("support", "response"):
        value = decisions.get(name)
        if not isinstance(value, dict):
            add_issue(errors, "PLAN_DECISION_TYPE", f"plan.policy_decisions.{name} must be an object")
            continue
        for key in ("statement", "classification", "sources"):
            if key not in value:
                add_issue(errors, "PLAN_DECISION_REQUIRED", f"plan.policy_decisions.{name}.{key} is required")
        if value.get("classification") not in CLASSIFICATIONS:
            add_issue(errors, "PLAN_CLASSIFICATION", f"plan.policy_decisions.{name}.classification is invalid")
        if value.get("classification") == "verified" and not value.get("sources"):
            add_issue(errors, "PLAN_VERIFIED_WITHOUT_SOURCE", f"plan.policy_decisions.{name} is verified but has no source")

    intake = decisions.get("intake")
    if isinstance(intake, dict):
        for key in ("preferred_type", "preferred_value", "verified", "sources"):
            if key not in intake:
                add_issue(errors, "PLAN_INTAKE_REQUIRED", f"plan.policy_decisions.intake.{key} is required")
        if intake.get("verified") is True and not intake.get("sources"):
            add_issue(errors, "PLAN_INTAKE_SOURCE", "verified intake route requires at least one source")
        if intake.get("preferred_type") == "unknown" and intake.get("verified") is True:
            add_issue(errors, "PLAN_INTAKE_CONFLICT", "unknown intake route cannot be verified")
    elif intake is not None:
        add_issue(errors, "PLAN_INTAKE_TYPE", "plan.policy_decisions.intake must be an object")

    for name in ("scope", "out_of_scope", "supply_chain"):
        value = decisions.get(name)
        if not isinstance(value, list):
            add_issue(errors, "PLAN_DECISION_TYPE", f"plan.policy_decisions.{name} must be an array")
            continue
        if name in {"scope", "out_of_scope"} and not value:
            add_issue(errors, "PLAN_DECISION_EMPTY", f"plan.policy_decisions.{name} must not be empty")
        for index, item in enumerate(value):
            validate_evidence_item(item, f"plan.policy_decisions.{name}[{index}]", errors)

    safe_harbor = decisions.get("safe_harbor")
    if isinstance(safe_harbor, dict):
        if safe_harbor.get("mode") not in {"research_guidelines", "approved_safe_harbor", "preserve_existing"}:
            add_issue(errors, "PLAN_SAFE_HARBOR_MODE", "safe_harbor.mode is invalid")
        if not isinstance(safe_harbor.get("approved"), bool):
            add_issue(errors, "PLAN_SAFE_HARBOR_APPROVAL", "safe_harbor.approved must be boolean")
        if safe_harbor.get("approved") and not safe_harbor.get("sources"):
            add_issue(errors, "PLAN_SAFE_HARBOR_SOURCE", "approved safe harbor requires a source")
    elif safe_harbor is not None:
        add_issue(errors, "PLAN_SAFE_HARBOR_TYPE", "safe_harbor must be an object")

    todos = plan.get("unresolved_todos")
    if not isinstance(todos, list):
        add_issue(errors, "PLAN_TODOS_TYPE", "plan.unresolved_todos must be an array")
    else:
        blocking = [item for item in todos if isinstance(item, dict) and item.get("blocking") is True]
        if blocking:
            add_issue(warnings, "PLAN_BLOCKING_TODOS", f"plan contains {len(blocking)} blocking unresolved TODO(s)")


def headings(text: str) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    in_fence = False
    fence_marker = ""
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith(("```", "~~~")):
            marker = stripped[:3]
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
            continue
        if in_fence:
            continue
        match = re.match(r"^(#{1,6})\s+(.+?)\s*$", line)
        if match:
            found.append((len(match.group(1)), re.sub(r"[*_`]", "", match.group(2)).strip()))
    return found


def has_heading(all_headings: list[tuple[int, str]], patterns: list[str]) -> bool:
    return any(any(re.search(pattern, title, re.IGNORECASE) for pattern in patterns) for _, title in all_headings)


def plan_intake(plan: dict[str, Any] | None) -> dict[str, Any]:
    if not plan:
        return {}
    decisions = plan.get("policy_decisions")
    return decisions.get("intake", {}) if isinstance(decisions, dict) else {}


def verified_plan_text(plan: dict[str, Any] | None) -> str:
    if not plan:
        return ""
    chunks: list[str] = []
    for item in plan.get("repository_facts", []):
        if isinstance(item, dict) and item.get("classification") == "verified":
            chunks.append(str(item.get("statement", "")))
    decisions = plan.get("policy_decisions", {})
    if isinstance(decisions, dict):
        for name in ("support", "response"):
            item = decisions.get(name)
            if isinstance(item, dict) and item.get("classification") == "verified":
                chunks.append(str(item.get("statement", "")))
        for name in ("scope", "out_of_scope", "supply_chain"):
            for item in decisions.get(name, []) if isinstance(decisions.get(name), list) else []:
                if isinstance(item, dict) and item.get("classification") == "verified":
                    chunks.append(str(item.get("statement", "")))
    return "\n".join(chunks).lower()


def profile_controls(profile: dict[str, Any] | None) -> set[str]:
    if not profile:
        return set()
    evidence = profile.get("evidence", {})
    controls = evidence.get("supply_chain_controls", []) if isinstance(evidence, dict) else []
    return {str(item.get("name", "")).lower() for item in controls if isinstance(item, dict)}


def validate_policy(
    policy_path: Path,
    text: str,
    profile: dict[str, Any] | None,
    plan: dict[str, Any] | None,
    strict: bool,
    errors: list[dict[str, str]],
    warnings: list[dict[str, str]],
    checks: list[dict[str, Any]],
) -> None:
    all_headings = headings(text)
    h1s = [title for level, title in all_headings if level == 1]
    if len(h1s) != 1:
        add_issue(errors, "HEADING_H1_COUNT", f"policy must contain exactly one top-level heading; found {len(h1s)}")
    elif not re.search(r"security\s+policy", h1s[0], re.IGNORECASE):
        add_issue(errors, "HEADING_H1_TITLE", "top-level heading should be 'Security Policy'")
    checks.append({"name": "single_h1", "passed": len(h1s) == 1})

    required_sections = {
        "support": [r"supported versions?", r"support policy", r"version support"],
        "reporting": [r"reporting a vulnerability", r"report a vulnerability", r"vulnerability reporting"],
        "scope": [r"security scope", r"threat model", r"security boundar"],
        "out_of_scope": [r"out of scope", r"not in scope"],
        "expectations": [r"what to expect", r"response", r"handling process"],
        "research": [r"research guidelines", r"safe harbor", r"security research"],
    }
    for name, patterns in required_sections.items():
        passed = has_heading(all_headings, patterns)
        checks.append({"name": f"section_{name}", "passed": passed})
        if not passed:
            add_issue(errors, "MISSING_SECTION", f"missing required section: {name}")

    recommended_sections = {
        "report_quality": [r"report quality", r"report requirements", r"include.*report"],
        "coordination": [r"coordinated disclosure", r"disclosure"],
        "attribution": [r"attribution", r"credit", r"acknowledg"],
    }
    for name, patterns in recommended_sections.items():
        passed = has_heading(all_headings, patterns)
        checks.append({"name": f"section_{name}", "passed": passed})
        if not passed:
            add_issue(warnings, "RECOMMENDED_SECTION", f"recommended section missing: {name}")

    lower = text.lower()
    placeholder_hits: list[str] = []
    for pattern, label in PLACEHOLDER_PATTERNS:
        if re.search(pattern, text, re.IGNORECASE):
            placeholder_hits.append(label)
    for label in sorted(set(placeholder_hits)):
        target = errors if strict else warnings
        add_issue(target, "PLACEHOLDER", label)
    checks.append({"name": "no_placeholders", "passed": not placeholder_hits})

    private_channel = False
    pvr_mentioned = bool(re.search(r"private vulnerability reporting|report a vulnerability", text, re.IGNORECASE))
    emails = sorted(set(EMAIL_PATTERN.findall(text)))
    non_example_emails = [email for email in emails if not re.search(r"(?:example|yourdomain)\.", email, re.IGNORECASE)]
    portal_mentioned = bool(re.search(r"https?://\S+", text) and re.search(r"(?:security|vulnerab|bug bounty|hackerone|bugcrowd|portal)", text, re.IGNORECASE))
    private_channel = pvr_mentioned or bool(non_example_emails) or portal_mentioned
    has_channel_todo = bool(re.search(r"TODO[^\n]*(?:private|report|security|contact)", text, re.IGNORECASE))
    if not private_channel:
        target = errors if strict or not has_channel_todo else warnings
        add_issue(target, "PRIVATE_CHANNEL", "no usable private vulnerability reporting channel was detected")
    checks.append({"name": "private_reporting_channel", "passed": private_channel})

    intake = plan_intake(plan)
    if pvr_mentioned and not (intake.get("preferred_type") == "github_pvr" and intake.get("verified") is True):
        add_issue(errors if strict else warnings, "PVR_UNVERIFIED", "policy names GitHub Private Vulnerability Reporting but the plan does not mark it verified")
    planned_values = {str(intake.get("preferred_value", "")).lower(), str(intake.get("fallback_value", "")).lower()}
    for email in non_example_emails:
        if plan and email.lower() not in planned_values:
            add_issue(errors if strict else warnings, "CONTACT_NOT_IN_PLAN", f"reporting email is not verified by the plan: {email}")

    for line_number, line in enumerate(text.splitlines(), start=1):
        if re.search(r"\b(?:open|file|submit|create)\b[^\n]{0,50}\bpublic\s+(?:issue|discussion|pull request)", line, re.IGNORECASE):
            if not re.search(r"\b(?:do not|don't|never|avoid|must not)\b", line, re.IGNORECASE):
                add_issue(errors, "PUBLIC_REPORTING", f"line {line_number} appears to direct security reports to a public channel")
    checks.append({"name": "no_public_reporting_route", "passed": not any(i["code"] == "PUBLIC_REPORTING" for i in errors)})

    if not re.search(r"(?:proof of concept|reproduc|reproduction|human[- ]verified|exploit path)", text, re.IGNORECASE):
        add_issue(warnings, "REPORT_EVIDENCE", "policy should require a human-verified reproducer or feasible exploit path")
    if not re.search(r"(?:raw scanner|fuzzer|automated|AI-generated|theoretical|speculative)", text, re.IGNORECASE):
        add_issue(warnings, "ANTI_NOISE", "policy does not clearly address unverified automated or speculative reports")

    response = plan.get("policy_decisions", {}).get("response", {}) if plan else {}
    if FIXED_TIMELINE_PATTERN.search(text):
        if response.get("model") == "best_effort" or response.get("classification") != "verified":
            add_issue(errors, "UNSUPPORTED_TIMELINE", "policy contains a fixed response promise not supported by a verified response plan")

    safe_harbor = plan.get("policy_decisions", {}).get("safe_harbor", {}) if plan else {}
    legal_commitment = any(re.search(pattern, text, re.IGNORECASE | re.DOTALL) for pattern in LEGAL_COMMITMENT_PATTERNS)
    if legal_commitment and not (safe_harbor.get("approved") is True or safe_harbor.get("mode") == "preserve_existing"):
        add_issue(errors, "UNAPPROVED_SAFE_HARBOR", "policy creates formal legal/safe-harbor commitments without plan approval")

    private_key_markers = ["-----BEGIN PRIVATE KEY-----", "-----BEGIN OPENSSH PRIVATE KEY-----", "-----BEGIN PGP PRIVATE KEY BLOCK-----"]
    if any(marker in text for marker in private_key_markers):
        add_issue(errors, "PRIVATE_KEY_EXPOSURE", "policy contains private key material")

    controls = profile_controls(profile)
    verified_text = verified_plan_text(plan)
    artifact_rules = {
        "sbom": (r"\b(?:SBOM|software bill of materials|CycloneDX|SPDX)\b", {"sbom"}),
        "vex": (r"\b(?:VEX|OpenVEX|vulnerability exploitability exchange)\b", {"vex"}),
        "slsa": (r"\bSLSA\b|\bprovenance attestation", {"slsa-provenance"}),
        "cosign": (r"\b(?:Cosign|Sigstore)\b", {"sigstore-cosign"}),
        "signed releases": (r"\b(?:signed releases?|release signatures?|signed artifacts?)\b", {"signed-releases", "sigstore-cosign"}),
    }
    for label, (pattern, expected_controls) in artifact_rules.items():
        if not re.search(pattern, text, re.IGNORECASE):
            continue
        plan_supports = label in verified_text or any(token in verified_text for token in expected_controls)
        profile_supports = bool(expected_controls & controls)
        if not plan_supports and not profile_supports:
            add_issue(errors if strict else warnings, "UNSUPPORTED_SUPPLY_CHAIN_CLAIM", f"policy mentions {label} without profile or verified plan evidence")
        elif profile_supports and not plan_supports:
            add_issue(warnings, "CONFIGURED_NOT_PUBLISHED", f"{label} appears configured, but publication/usability should be verified before making a strong claim")

    if len(text.splitlines()) > 300:
        add_issue(warnings, "POLICY_LENGTH", f"policy is {len(text.splitlines())} lines; consider reducing it below 250 lines")

    if plan:
        target_path = str(plan.get("target_path", ""))
        if target_path and Path(target_path).name != policy_path.name:
            add_issue(warnings, "TARGET_PATH_MISMATCH", f"plan target path '{target_path}' does not match policy filename '{policy_path.name}'")
        blocking_todos = [item for item in plan.get("unresolved_todos", []) if isinstance(item, dict) and item.get("blocking") is True]
        if blocking_todos:
            target = errors if strict else warnings
            add_issue(target, "BLOCKING_TODOS", f"{len(blocking_todos)} blocking plan TODO(s) remain")


def render_text(result: dict[str, Any]) -> str:
    lines = [f"Status: {result['status'].upper()}"]
    lines.append(f"Errors: {len(result['errors'])}; Warnings: {len(result['warnings'])}")
    if result["errors"]:
        lines.append("\nErrors:")
        for item in result["errors"]:
            lines.append(f"- [{item['code']}] {item['message']}")
    if result["warnings"]:
        lines.append("\nWarnings:")
        for item in result["warnings"]:
            lines.append(f"- [{item['code']}] {item['message']}")
    lines.append("\nChecks:")
    for check in result["checks"]:
        lines.append(f"- {'PASS' if check['passed'] else 'FAIL'} {check['name']}")
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    try:
        policy_path = Path(args.policy)
        text = policy_path.read_text(encoding="utf-8")
        if not text.strip():
            raise ValueError(f"policy file is empty: {policy_path}")
        profile = load_json(args.profile, "profile")
        plan = load_json(args.plan, "plan")
    except (OSError, ValueError) as exc:
        error = {"status": "error", "error": str(exc)}
        if args.format == "json":
            print(json.dumps(error, indent=2))
        else:
            print(f"Input error: {exc}", file=sys.stderr)
        return 2

    errors: list[dict[str, str]] = []
    warnings: list[dict[str, str]] = []
    checks: list[dict[str, Any]] = []

    if profile is not None and profile.get("status") != "ok":
        add_issue(warnings, "PROFILE_STATUS", "profile status is not 'ok'")
    if plan is not None:
        validate_plan(plan, errors, warnings)

    validate_policy(policy_path, text, profile, plan, args.strict, errors, warnings, checks)

    result = {
        "status": "pass" if not errors else "fail",
        "strict": args.strict,
        "inputs": {
            "policy": str(policy_path),
            "profile": args.profile,
            "plan": args.plan,
        },
        "errors": errors,
        "warnings": warnings,
        "checks": checks,
    }
    if args.format == "json":
        print(json.dumps(result, indent=2))
    else:
        sys.stdout.write(render_text(result))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
