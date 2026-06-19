# PR Triage Output Contract

## Markdown format

Return exactly these sections in this order:

```markdown
# PR Triage: #NUMBER — TITLE

- **Decision:** DECISION
- **Priority:** PRIORITY
- **Primary category:** CATEGORY
- **Secondary categories:** CATEGORY_LIST_OR_NONE
- **Confidence:** CONFIDENCE
- **Repository:** OWNER/REPO
- **Base → Head:** BASE → HEAD

## Triage rationale

A concise explanation of why this disposition is correct.

## Dimension assessment

| Dimension | Result | Evidence |
|---|---|---|
| Scope alignment | pass/concern/fail/unknown | Concrete evidence |
| Problem definition | pass/concern/fail/unknown | Concrete evidence |
| Implementation completeness | pass/concern/fail/unknown | Concrete evidence |
| Test completeness | pass/concern/fail/unknown | Concrete evidence |
| Documentation completeness | pass/concern/fail/unknown | Concrete evidence |
| Contribution compliance | pass/concern/fail/unknown | Concrete evidence |
| CI health | pass/concern/fail/unknown | Concrete evidence |
| Duplicate risk | pass/concern/fail/unknown | Concrete evidence |
| Spam risk | pass/concern/fail/unknown | Concrete evidence |
| Reviewability | pass/concern/fail/unknown | Concrete evidence |

## Required follow-up

### Blocking

- Concrete action, or `None.`

### Important

- Concrete action, or `None.`

### Advisory

- Concrete action, or `None.`

## Duplicate and scope search

State the searches performed and list matching pull requests or issues. Write `No substantial match found.` when appropriate.

## Evidence reviewed

- Repository policy files
- Pull request metadata and linked issues
- Changed-file inventory and patch coverage
- Checks and test evidence

## Maintainer routing

Name the next action and the appropriate reviewer domain or owner. Do not name a person unless CODEOWNERS, review requests, or repository policy supports it.
```

Replace all uppercase field labels with actual values. Do not output the template delimiters.

## JSON format

Return one JSON object and no surrounding prose:

```json
{
  "pull_request": {
    "number": 42,
    "url": "https://github.com/octo-org/octo-repo/pull/42",
    "title": "Prevent duplicate job scheduling",
    "repository": "octo-org/octo-repo",
    "base": "main",
    "head": "fix/deduplicate-jobs"
  },
  "decision": "needs-author-changes",
  "priority": "P2",
  "categories": {
    "primary": "bug",
    "secondary": ["tests"]
  },
  "confidence": "high",
  "rationale": "The fix is in scope, but the regression path lacks coverage required by the repository contribution guide.",
  "dimensions": {
    "scope_alignment": {"result": "pass", "evidence": ["README identifies scheduler correctness as supported scope."]},
    "problem_definition": {"result": "pass", "evidence": ["Linked issue includes reproduction steps and expected behavior."]},
    "implementation_completeness": {"result": "pass", "evidence": ["Patch covers both enqueue entry points."]},
    "test_completeness": {"result": "fail", "evidence": ["No regression test covers concurrent duplicate submissions."]},
    "documentation_completeness": {"result": "pass", "evidence": ["No user-facing behavior or configuration changed."]},
    "contribution_compliance": {"result": "fail", "evidence": ["CONTRIBUTING requires regression tests for bug fixes."]},
    "ci_health": {"result": "pass", "evidence": ["All configured checks passed."]},
    "duplicate_risk": {"result": "pass", "evidence": ["No substantially overlapping pull request or issue was found."]},
    "spam_risk": {"result": "pass", "evidence": ["Changes directly address the linked scheduler defect."]},
    "reviewability": {"result": "concern", "evidence": ["Review should wait for the missing regression test."]}
  },
  "follow_up": {
    "blocking": ["Add a regression test for concurrent duplicate submissions."],
    "important": [],
    "advisory": []
  },
  "duplicate_search": {
    "queries": ["duplicate job scheduling", "scheduler deduplication"],
    "matches": []
  },
  "evidence_reviewed": {
    "policy_files": ["README.md", "CONTRIBUTING.md"],
    "patch_review": "complete",
    "checks_reviewed": true
  },
  "maintainer_routing": "Return to the author for the missing regression test, then route to the scheduler code owner."
}
```

The JSON example defines types and shape. Replace every example value with evidence from the inspected pull request. Use empty arrays instead of null for absent list values. Use `unknown` dimension results when evidence is unavailable.
