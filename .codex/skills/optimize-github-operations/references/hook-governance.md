# Claude Code Hook Governance

Use hooks only for a project-scoped installation at:

```text
.claude/skills/optimize-github-operations/
```

Do not enable these commands for a personal installation because the project-relative script paths would not exist.

Add the following event handlers as siblings inside the existing `hooks` object in `.claude/settings.json`. Merge them; do not replace unrelated hooks.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/skills/optimize-github-operations/scripts/hook_guard.py\""
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/skills/optimize-github-operations/scripts/hook_post_write.py\""
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/skills/optimize-github-operations/scripts/hook_stop_verify.py\""
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "general-purpose",
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Allow this subagent to stop only when its final response identifies the repository root, selected template profiles, files changed, files preserved, validation results, and unresolved blockers. Return {\"ok\": false, \"reason\": \"Provide the missing optimization handoff fields.\"} when any field is absent."
          }
        ]
      }
    ]
  }
}
```

Behavior:

- `PreToolUse` blocks destructive Bash commands that can erase or discard `.github/` or the worktree.
- `PostToolUse` records `.github/` writes in `.git/claude-github-optimizer.log`; it does not modify repository content.
- `Stop` validates only when `.github/` has changed. Validation failures return exit code 2 so Claude continues working.
- `SubagentStop` enforces a complete forked-agent handoff.

Verification:

1. Run `/hooks` and confirm all four event types appear.
2. Invoke the skill in preview mode and confirm no repository file changes.
3. In a disposable branch, edit `.github/PULL_REQUEST_TEMPLATE.md`; confirm the audit log records the file.
4. Introduce a missing required heading and attempt to finish; confirm the Stop hook blocks completion.
5. Restore the heading and confirm completion succeeds.

Do not rely on hooks as the sole security boundary. Keep Claude Code permission deny rules for destructive Git and shell commands because hook argument filtering is best-effort.
