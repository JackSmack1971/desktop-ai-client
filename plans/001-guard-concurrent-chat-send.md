# Plan 001: Prevent a second `sendMessage` from corrupting an in-flight chat stream

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- src/lib/stores/chat.ts src/lib/components/chat/ChatInput.svelte src/lib/components/surfaces/ChatSurface.svelte src-tauri/src/ipc/chat.rs`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

The chat send button is only disabled while `chatStore.loading` is `true`, and `loading` flips back to `false` as soon as the *first* streaming token (`Delta`) arrives (`src/lib/stores/chat.ts:84-92`). That means once a response starts streaming, the Send button re-enables even though a request is still in flight (`chatStore.canCancel` is true at this point, but nothing reads `canCancel` to gate the input). If the user clicks Send again, `sendMessage` reassigns the module-level `streamingId`/`requestId` to the brand-new message with no guard. Every `Delta`/`Done`/`Error` event for the *first* (now orphaned) request still matches `m.id === streamingId` checks... except `streamingId` has already moved on, so those events get silently dropped or, worse, applied to the new message if the timing lines up before `streamingId` updates again. The first message is left permanently stuck in a non-terminal streaming state (no `Done`/`Error` will ever match its id again), and tokens from two unrelated requests can interleave into one bubble. This directly violates the documented invariant in `.claude/rules/backend.md`: "Never let stale stream events mutate the active conversation."

The fix is small: disable the input for the full duration of an in-flight request (covering both the pre-first-token "loading" phase and the active-streaming phase), not just the pre-first-token phase.

## Current state

- `src/lib/stores/chat.ts` — the chat store (Svelte 5 runes, factory pattern). Relevant state:
  ```ts
  // lines 50-66
  let streamingId = $state<string | null>(null);
  let requestId = $state<string | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let canCancel = $derived(requestId !== null);
  ```
  The store already exposes `canCancel` as a getter (lines 229-231) — it is computed but currently only consumed to show/hide the Cancel button, never to gate Send.
- `sendMessage` (lines 146-191) has no guard at the top — it always proceeds to append messages and call `chatSend` regardless of current state.
- `src/lib/components/chat/ChatInput.svelte` — presentational component. Takes a single `disabled: boolean` prop (line 12) that gates both the textarea and the submit button (lines 39-43, 61-65). It has no own state; it just reflects whatever `disabled` the parent passes.
- `src/lib/components/surfaces/ChatSurface.svelte:93-96` — the parent wiring:
  ```svelte
  <ChatInput
      onsubmit={...}
      disabled={chatStore.loading}
      showCancel={chatStore.canCancel}
      ...
  />
  ```
  This is the bug: `disabled` tracks `loading` (true only until the first delta), not the full in-flight lifetime (`canCancel`, true until `Done`/`Error`/`CANCELLED`).

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Type-check | `pnpm check` | exit 0, no errors |
| Frontend dev smoke test | `pnpm frontend:dev` then manually exercise (see Step 3 verification) | app loads, no console errors |

(There is no `pnpm test` script in this repo yet — see plan `005-add-test-coverage.md`, which is independent of this plan and not a prerequisite.)

## Scope

**In scope** (the only files you should modify):
- `src/lib/stores/chat.ts`
- `src/lib/components/surfaces/ChatSurface.svelte`

**Out of scope** (do NOT touch, even though related):
- `src-tauri/src/ipc/chat.rs` — the backend already supports multiple concurrent `chat_send` calls correctly (each gets its own `request_id` and `CancellationToken` in `active_requests`); this plan only needs to stop the *frontend* from issuing a second one while the store is still tracking an active one. Do not add backend-side single-flight enforcement — that would be a behavior change beyond this bug's scope.
- `src/lib/components/chat/ChatInput.svelte` — its `disabled` prop contract is already correct (a boolean gate); no change needed there.
- `src/lib/stores/artifacts.ts` and the `ArtifactReady` handling — a related but distinct gap is tracked separately in `plans/010-scope-artifact-events-to-conversation.md`. Do not fix it here.

## Git workflow

- Branch: `advisor/001-guard-concurrent-chat-send`
- Commit message style: conventional commits, matching this repo's history (e.g. `fix(chat): disable input for the full in-flight request lifetime`)
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Change the disable condition in `ChatSurface.svelte` to cover the full in-flight lifetime

In `src/lib/components/surfaces/ChatSurface.svelte`, change line 95 from:

```svelte
disabled={chatStore.loading}
```

to:

```svelte
disabled={chatStore.loading || chatStore.canCancel}
```

This keeps the textarea/Send button disabled from the moment `sendMessage` is called until `Done`/`Error`/`CANCELLED` clears `requestId` back to `null` (which is exactly when `canCancel` becomes `false` again, per `chat.ts:106-109` and `chat.ts:128-131`).

**Verify**: `pnpm check` → exit 0, no new type errors. (This is a template-only change in a `.svelte` file with no new types, so `svelte-check` should pass unchanged.)

### Step 2: Add a defense-in-depth guard inside `sendMessage` itself

UI-level disabling is necessary but not sufficient — defend the store's own invariant in case any other caller (a future second composer, a keyboard shortcut, a test) calls `sendMessage` directly. In `src/lib/stores/chat.ts`, at the top of `sendMessage` (currently starting at line 146), add an early return:

```ts
async function sendMessage(content: string): Promise<void> {
    if (canCancel) {
        // A request is already in flight — ignore the new submission rather
        // than corrupting the active stream's identity (see plan 001).
        return;
    }

    const userId = crypto.randomUUID();
    // ...rest of the existing function body unchanged
```

Do not change anything else in the function body.

**Verify**: `pnpm check` → exit 0, no new type errors.

### Step 3: Manual smoke test (no automated test infra exists yet — see plan 005)

Run `pnpm dev` (or `pnpm frontend:dev` if you don't want to launch the full Tauri shell) and:
1. Send a message that will produce a multi-token streaming response.
2. While the response is still streaming (text visibly appearing token-by-token), confirm the Send button is disabled (greyed out, shows "Sending…") and the textarea is disabled.
3. Confirm the Cancel button is visible and clicking it still works (mark the message `incomplete` with the amber badge).
4. After the response completes, confirm the Send button re-enables and a second message can be sent normally.

**Verify**: All four observations hold. If the Send button is still clickable mid-stream after Step 1's edit, the drift check at the top of this plan likely found a mismatch — re-read `ChatSurface.svelte` before re-attempting.

## Test plan

No automated test infrastructure exists for the frontend yet (tracked in `plans/005-add-test-coverage.md`). Once that lands, add to `src/lib/stores/chat.test.ts` (created by plan 005):
- A test that calls `sendMessage` twice in quick succession (mocking `chatSend` to never resolve on the first call) and asserts the second call is a no-op — `messages` length only grows by 2 (one user, one assistant placeholder), not 4.
- Model the mock-`chatSend` pattern after however plan 005 sets up `$lib/api/chat` mocking; do not invent a different mocking approach here.

This plan does not block on plan 005 — ship the fix now with the manual verification in Step 3, and backfill the automated test when plan 005's infra lands.

## Done criteria

- [ ] `pnpm check` exits 0
- [ ] `src/lib/components/surfaces/ChatSurface.svelte` line ~95 reads `disabled={chatStore.loading || chatStore.canCancel}`
- [ ] `src/lib/stores/chat.ts`'s `sendMessage` returns early when `canCancel` is true, before appending any messages
- [ ] Manual smoke test in Step 3 passes
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:
- `ChatSurface.svelte` no longer has a `disabled={chatStore.loading}` line at all (the binding has been refactored differently than described) — re-read the current file fully before guessing where to apply the fix.
- `chatStore` no longer exposes a `canCancel` getter, or its semantics have changed (e.g. it now also covers something other than "request in flight").
- Fixing this surfaces a *different* pre-existing bug in `handleEvent` (e.g. you find `streamingId` mismatches even with the guard in place) — report it instead of trying to fix it under this plan; it's likely the related-but-distinct issue tracked in plan `010-scope-artifact-events-to-conversation.md`.

## Maintenance notes

- If a "multiple simultaneous conversations" feature is ever added (the backend already supports concurrent `request_id`s), this single-`streamingId`/`requestId`/`canCancel` model will need to become a map keyed by conversation, and this guard will need to move from "block all sends" to "block sends only for the conversation that's already streaming." Flag that as a larger redesign, not an incremental patch.
- A reviewer should manually re-run Step 3's smoke test on the PR — this bug class (state corruption under double-submit) is exactly the kind of thing that "looks fine" in a quick glance at the diff but needs the actual race exercised.
