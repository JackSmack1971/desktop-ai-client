# Memory Loop

## Goal

The memory loop exists to help the agent improve without drifting.

It should answer three questions:

- what should be remembered
- what should be forgotten
- what should be reused

## Memory types

`factual`

- stable, verified facts
- example: API behavior, project invariants, limits

`episodic`

- what happened in a specific run
- example: task context, result, failure mode

`procedural`

- a reusable workflow or method
- example: a reliable sequence of steps for a task type

`caution`

- a known trap or failure pattern
- example: a bad retrieval pattern or a brittle assumption

## Candidate record shape

Every memory candidate should capture:

- type
- summary
- source run
- tags
- confidence
- utility
- recency
- verification state
- expiry

## Retrieval policy

- load only a small set of memories
- prefer memories that match the current task type
- prefer higher-confidence items
- prefer more recent items when relevance is close
- do not load expired items unless explicitly requested

## Promotion policy

A candidate should be promoted only when:

- the trace supports it
- it is not a duplicate
- the judge approves it
- it has a clear future-use condition

Suggested promotion rules:

- episodic to procedural: repeated success or a strong judge-approved pattern
- episodic to caution: repeated failure
- factual: externally verified or trace-supported and stable

## Consolidation policy

Run consolidation on a schedule:

- dedupe repeated items
- merge overlapping summaries
- rewrite weak memories into compact lessons
- expire stale items
- keep raw traces untouched

## Anti-drift rules

- If a memory is not reusable, do not promote it.
- If a memory is not verified, do not treat it as fact.
- If a memory is stale, do not load it by default.
- If a memory conflicts with current behavior, record the conflict and resolve it explicitly.

## Phase 1 implementation status (shadow mode)

Phase 1 implements this document's storage shape — run traces, the four
candidate kinds, retrieval, dedup, promotion, and a decision ledger — in
`src-tauri/src/storage/memory.rs` (migration 0006, see
`docs/architecture.md`'s "Evidence-Gated Memory Engine" section for the
full pipeline). It is **not** wired into `chat_send`: nothing in this phase
changes what a live prompt sees. Until a later phase explicitly decides
retrieval quality clears that bar, the only consumers of this module are
its own tests and the deterministic replay harness in
`src-tauri/src/telemetry/memory_replay.rs`.

What's mechanical vs. aspirational right now:

- **Promotion's "judge"** is the deterministic `promotion_rule` function
  (confidence/utility/verification thresholds per kind), not an LLM judge.
  This doc's "the judge approves it" is a future upgrade, not Phase 1's
  behavior — see the `ponytail:` comment on `promotion_rule` for the
  upgrade path.
- **Duplicate detection** is exact-match on normalized `(kind, summary)`
  text, not semantic/fuzzy matching.
- **Consolidation** (dedupe, merge, rewrite, expire on a schedule) is only
  partially implemented: `expire_stale()` exists; merge/rewrite do not.
- **Contradiction handling** rejects both sides of a detected conflict;
  there is no reconciliation step to pick a winner yet.

Verification commands:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib memory
cargo run --manifest-path src-tauri/Cargo.toml --bin memory-replay
```

## Inspecting stored memory metadata (read-only)

A human can inspect what the shadow-mode memory engine has stored by running
read-only SQL directly against the app's SQLite database file. This is
**not** a Rust code path, IPC command, or UI surface — it is an operator-local
diagnostic, run by hand, that crosses no IPC boundary and adds no new
program-reachable surface (the shadow-mode invariant above is preserved:
`grep -rn 'memory::' src-tauri/src/ipc/` still returns nothing).

Open the database file in read-only mode so inspection cannot mutate state.
Using the `sqlite3` CLI:

```sh
sqlite3 -readonly /path/to/app.db
```

(Find the app's database file location for your platform via the Tauri app
data directory; the path is not hardcoded here since it varies by OS and
build.)

### Candidate metadata (`memory_candidates`)

```sql
SELECT id, kind, summary, source_run_trace_id, confidence, utility, status, verification_state, contradiction_state, expires_at, created_at, updated_at, tags FROM memory_candidates
ORDER BY created_at DESC
LIMIT 50;
```

### Decision ledger (`memory_decisions`)

```sql
SELECT
  candidate_id,
  action,
  reason,
  decided_at
FROM memory_decisions
ORDER BY decided_at DESC
LIMIT 50;
```

### Run traces (`memory_run_traces`)

```sql
SELECT
  id,
  conversation_id,
  turn_id,
  task_summary,
  outcome,
  created_at
FROM memory_run_traces
ORDER BY created_at DESC
LIMIT 50;
```

### Why this is safe to expose

Each query above names an explicit column list against a single
`memory_*` table — never `SELECT *`, and never a join to `messages`,
`conversations`, `turns`, `turn_attempts`, `attachments`, `artifacts`, or any
other non-`memory_*` table. Per `docs/privacy-boundaries.md`'s "Memory engine
storage" section, the `memory_*` tables persist only derived summaries
(`task_summary`, `summary`, and now caller-supplied `tags`) and chain back to
their source conversation via `conversation_id` / `source_run_trace_id` —
they never hold a secret, credential, or raw file path. Those live entirely
in the attachment/file-token intake path (`docs/privacy-boundaries.md`'s
"Attachment intake" section), which none of the queries above read from.
