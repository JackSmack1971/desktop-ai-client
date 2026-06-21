# Phase 2: Memory Model - Context

**Gathered:** 2026-06-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Close the gap between Phase 2's stated success criteria and what `src-tauri/src/storage/memory.rs` (migration 0006) already implements. This phase does NOT build the memory engine from scratch — most of it shipped already, in shadow mode, ahead of the roadmap's phase numbering. The remaining work is narrow: add a `tags` field to candidate memories, and provide a documented way for a human to inspect stored memory metadata. No change to live chat behavior; shadow-mode-only constraint stays in force.

</domain>

<decisions>
## Implementation Decisions

### Scope
- **D-01:** Phase 2 scope is both remaining gaps: tags + inspection. The other three roadmap success criteria (separate storage, full candidate metadata minus tags, shadow-mode isolation) are already satisfied by existing code and require no new work.

### Tags
- **D-02:** Tags are free-form, caller-supplied strings (e.g. `Vec<String>` on `propose_candidate`), not a fixed taxonomy and not auto-derived. Whatever code proposes a candidate chooses its own tags. No validation/allow-list.
- **D-03:** Add a `tags` column to `memory_candidates` (new migration, additive — do not alter migration 0006). Store as a serialized list (e.g. JSON array or comma-joined TEXT); exact encoding is Claude's discretion at planning time, no user preference expressed.

### Inspection
- **D-04:** Inspection is via direct read-only SQL queries against the SQLite file — no new Rust code, no new IPC command, no UI. Deliverable is documentation (e.g. a short doc or README section) showing the query pattern against `memory_candidates`/`memory_decisions`/`memory_run_traces` that a human can run to inspect metadata.
- **D-05:** The documented query pattern must demonstrate it surfaces only `memory_*` table columns — no joins or column selections that would pull in secrets, credentials, or raw file paths from other tables, per the "without exposing backend-owned secrets or raw file paths" success criterion.

### Claude's Discretion
- Exact tags column encoding (JSON vs delimited TEXT) and migration numbering.
- Where the inspection doc lives (docs/memory-loop.md update vs new file) — match existing doc conventions.
- Whether `propose_candidate`'s signature takes tags as a required or optional/defaulted parameter — should not break existing call sites without updating them.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Memory engine (existing implementation)
- `docs/memory-loop.md` — defines candidate record shape (type, summary, source, tags, confidence, utility, recency, verification state, expiry), retrieval/promotion/consolidation policy, and the "Phase 1 implementation status (shadow mode)" section documenting what's mechanical vs. aspirational today.
- `docs/architecture.md` §"Evidence-Gated Memory Engine" — full pipeline description.
- `src-tauri/src/storage/memory.rs` — `MemoryStore`, `CandidateRow`, `propose_candidate`, `decide_promotion`, `expire_stale`, `bounded_retrieve`. This is the file to extend, not replace.
- `src-tauri/src/storage/migrations.rs` (migration 0006, `memory_candidates` table definition, lines ~206-230) — schema to extend additively.
- `src-tauri/src/telemetry/memory_replay.rs` — existing consumer/test harness; pattern to follow if any future inspection tooling becomes code-based.

### Project-level boundaries
- `.planning/PROJECT.md` — "evidence-gated memory engine stays in shadow mode until a later phase explicitly decides it can influence live behavior."
- `docs/privacy-boundaries.md` — backend-owned secrets/paths boundary, relevant to D-05.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `MemoryStore::propose_candidate` — extend its signature/SQL to accept and persist tags; this is the only write path for new candidates.
- `memory_candidates` table (migration 0006) — additive ALTER/new migration adds the `tags` column; existing columns (confidence, utility, status, verification_state, contradiction_state, expires_at) are untouched.

### Established Patterns
- Migrations are append-only and tested via `memory_candidates_and_decisions_tables_exist_after_migration` style tests in `src-tauri/src/storage/migrations.rs` — a new migration should follow the same `CREATE TABLE IF NOT EXISTS` / `ALTER TABLE` + index pattern.
- `memory.rs` has inline unit tests (`decide_promotion_*` etc.) — any new behavior (tag storage/round-trip) should get an equivalent unit test in the same file.

### Integration Points
- No IPC integration exists today (`grep` for `memory::` in `src-tauri/src/ipc/*.rs` returned nothing) — confirms shadow-mode isolation is currently real, not just documented. This phase must not add an IPC command (per D-04), preserving that isolation.

</code_context>

<specifics>
## Specific Ideas

No specific UI/format requirements expressed — user chose the minimal-surface options (free-form tags, direct-SQL inspection) over more structured alternatives (fixed taxonomy, IPC command + debug UI).

</specifics>

<deferred>
## Deferred Ideas

- A Tauri IPC command + Svelte debug UI for memory inspection was considered and explicitly not chosen for this phase — defer to a later phase if/when shadow mode graduates toward live behavior and a UI-based inspection surface becomes worth the IPC + UI cost.
- A fixed tag taxonomy (closed enum, validated) was considered and not chosen — revisit only if free-form tags prove too unstructured in practice (e.g. during consolidation work in Phase 5).

### Reviewed Todos (not folded)
None — `todo.match-phase 2` returned zero matches.

</deferred>

---

*Phase: 2-memory-model*
*Context gathered: 2026-06-21*
