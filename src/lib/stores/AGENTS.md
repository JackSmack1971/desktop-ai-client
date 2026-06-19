# src/lib/stores/AGENTS.md

This subtree owns frontend reactive state.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/privacy-boundaries.md`

## Purpose

Owns frontend reactive state for surface navigation (`surface.ts`), chat streaming (`chat.ts`), artifact preview lifecycle (`artifacts.ts`), conversation history (`history.ts`), and credential status (`settings.ts`). Each store is a singleton factory function (`createXStore()` → exported instance), not a class or Svelte 4 `writable`. Does not own IPC command implementations — those live in `src-tauri/src/ipc/*`.

## Contracts & Invariants

- No store here reads or writes `localStorage`/`sessionStorage` — all persisted state round-trips through `invoke()`. This is a privacy invariant, not a style preference.
- `chatStore.handleEvent` is the only code path permitted to mutate `messages`/`streamingId`/`requestId` in response to backend events; `sendMessage`/`cancelRequest` only initiate IPC calls or apply pre-stream state (terminal state arrives via channel only, mirroring the backend D-03 invariant).
- `chatStore.sendMessage` filters out the just-inserted placeholder assistant message and excludes the system prompt entirely when building the IPC payload — the backend is the sole place the system prompt is attached.
- `settingsStore`'s `privacyStore` never caches or returns the API key value itself — only `CONFIGURED`/`MISSING` status (`settings.ts:3-6`).
- Every store factory returns only getters (`get x() { return x; }`) plus action methods — never a raw mutable reference to internal `$state` — so external code cannot bypass store invariants by mutating state directly.
- `artifactsStore.reload()` uses a monotonically increasing `requestNonce`, checked before applying any IPC response, to discard stale in-flight responses superseded by a newer `reload()`/`dismiss()` call.

## Anti-patterns

- Redefining `normalizeIpcError` locally instead of importing it from `$lib/api/errors` (see Pitfalls — this was fixed once already; don't reintroduce a local copy).
- Mutating `messages`/`streamingId`/`requestId` from anywhere other than `chatStore.handleEvent`.
- Rolling back optimistic state the same way in every store — `surface.ts::setSurface` rolls back to the prior value on IPC rejection; `chat.ts::sendMessage` instead removes the placeholder assistant message entirely on failure. These are deliberately different strategies per store, not a shared pattern to copy verbatim.

## Related Context

- Backend IPC commands these stores call: `../../../src-tauri/src/ipc/AGENTS.md`
