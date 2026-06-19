# Plan 010: Scope `ArtifactReady` events to the conversation that produced them

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 6f79372..HEAD -- src-tauri/src/ipc/chat.rs src/lib/stores/artifacts.ts src/lib/stores/chat.ts`
> If any in-scope file changed since this plan was written, compare the
> "Current state" excerpts below against the live code before proceeding; on
> a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P3
- **Effort**: S-M
- **Risk**: LOW
- **Depends on**: `plans/001-guard-concurrent-chat-send.md` (soft — landing 001 first shrinks this bug's real-world exposure since concurrent sends become impossible from the UI, but this plan's fix is independent and still worth doing as defense-in-depth for any future multi-conversation feature).
- **Category**: bug
- **Planned at**: commit `6f79372`, 2026-06-17

## Why this matters

`ChatEvent::ArtifactReady` carries no conversation or request identifier — just `artifact_id`, `content_type`, `preview`. `artifactsStore.receiveArtifact` (the frontend handler) unconditionally overwrites whatever artifact is currently displayed whenever this event arrives, with no check against which conversation is actually active. Today this is low-exposure because the frontend only tracks one conversation/stream at a time, but it's the same root cause as the bug fixed in `plans/001-guard-concurrent-chat-send.md`: nothing in the artifact-event path enforces ".claude/rules/frontend.md"'s "treat stale stream events as inert once the active stream or attempt changes" rule. If a user ever navigates away from a streaming conversation (e.g. to the History surface) while it's still producing a response, and that response contains an artifact, the artifact panel for whatever the user is currently looking at gets silently replaced.

## Current state

- `src-tauri/src/ipc/chat.rs:75-80` — `ChatEvent::ArtifactReady` variant definition:
  ```rust
  ArtifactReady {
      artifact_id: String,
      content_type: ArtifactContentType,
      preview: String,
  },
  ```
- `src-tauri/src/ipc/chat.rs:281-306` — where it's sent, inside the `chat_send` spawned task's `Ok(())` branch, after `artifact_store.save_artifact(...)`:
  ```rust
  if let Some(detected) = artifacts::detect_artifact(&accumulated_text) {
      let artifact_store = app_handle.state::<ArtifactStore>();
      let artifact_id = Uuid::new_v4().to_string();
      if let Err(e) = artifact_store.save_artifact(
          &artifact_id, &effective_conv_id, Some(&assistant_message_id),
          &detected.content_type, &detected.raw_source,
      ) {
          eprintln!("[chat] failed to persist artifact: {e}");
      } else {
          match artifact_store.get_artifact_preview(&artifact_id) {
              Ok(preview) => {
                  let _ = channel.send(ChatEvent::ArtifactReady {
                      artifact_id: preview.artifact_id,
                      content_type: preview.content_type,
                      preview: preview.srcdoc,
                  });
              }
              Err(e) => { eprintln!("[chat] failed to build artifact preview: {e}"); }
          }
      }
  }
  ```
  Note: `effective_conv_id` (the conversation this artifact belongs to) is already in scope at this point in the function — it's used two lines earlier in `save_artifact`. It is simply not threaded into the `ArtifactReady` event payload.
- `src/lib/stores/artifacts.ts:32-41` (`receiveArtifact`):
  ```ts
  function receiveArtifact(
    event: Extract<ChatEvent, { type: 'ArtifactReady' }>,
  ): void {
    artifactId = event.artifact_id;
    contentType = event.content_type;
    preview = event.preview;
    error = null;
    state = event.preview.trim() ? 'ready' : 'error';
    if (!event.preview.trim()) {
      error = 'Artifact could not be displayed safely.';
    }
  }
  ```
  No check against which conversation is currently active.
- `src/lib/stores/chat.ts:133-136` — where `receiveArtifact` is invoked, inside `handleEvent`'s `ArtifactReady` case:
  ```ts
  case 'ArtifactReady': {
      artifactsStore.receiveArtifact(event);
      break;
  }
  ```
- The frontend's notion of "currently active conversation" lives in `historyStore` (`src/lib/stores/history.ts` — read this file yourself to find the exact getter name for the active/loaded conversation id; do not guess it) — `chat.ts` itself doesn't track a `conversationId` per se today (it only tracks `streamingId`/`requestId` for the single in-flight message). You will likely need to introduce a `conversationId` concept into `chatStore` as part of this fix, since the artifact-scoping check needs _some_ notion of "which conversation is the user looking at right now" to compare against.

## Commands you will need

| Purpose                                                         | Command                                                           | Expected on success |
| --------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------- |
| Compile check (backend)                                         | `cargo check --manifest-path src-tauri/Cargo.toml`                | exit 0              |
| Backend tests                                                   | `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` | all pass            |
| Type-check (frontend)                                           | `pnpm check`                                                      | exit 0              |
| Frontend tests (if `plans/005-add-test-coverage.md` has landed) | `pnpm test`                                                       | all pass            |

## Scope

**In scope**:

- `src-tauri/src/ipc/chat.rs` — add `conversation_id` to `ChatEvent::ArtifactReady`.
- `src/lib/api/chat.ts` (or wherever the `ChatEvent` TypeScript type is defined/generated to mirror the Rust enum — read this file first to find the exact type location) — mirror the new field.
- `src/lib/stores/chat.ts` — track the current conversation id and pass it through to the artifact handler.
- `src/lib/stores/artifacts.ts` — `receiveArtifact` ignores events whose `conversation_id` doesn't match the currently active one.

**Out of scope**:

- Building full multi-conversation support (tabs, parallel chats) — this plan only adds the scoping _check_; it does not change the app from single-conversation-at-a-time to multi-conversation. The check is a no-op safety net for the current UI and becomes load-bearing only if/when multi-conversation support is added later.
- `src-tauri/src/storage/artifacts.rs` — no schema or storage change; `effective_conv_id` is already stored against the artifact row (via `save_artifact`'s second argument) — this plan only adds it to the _event payload_, which is a separate, smaller thing.
- `src/lib/components/surfaces/ArtifactsSurface.svelte` — no UI change expected; if the scoping check causes a visible behavior difference here, that's the intended fix working, not a regression to chase further.

## Git workflow

- Branch: `advisor/010-scope-artifact-events-to-conversation`
- Commit per layer (backend, then frontend), message style:
  - `fix(chat): include conversation_id in ArtifactReady events`
  - `fix(artifacts): ignore ArtifactReady events for a non-active conversation`
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add `conversation_id` to the Rust `ChatEvent::ArtifactReady` variant

In `src-tauri/src/ipc/chat.rs`, change the enum variant (lines 76-80) to:

```rust
ArtifactReady {
    conversation_id: String,
    artifact_id: String,
    content_type: ArtifactContentType,
    preview: String,
},
```

Update the send site (inside the `Ok(())` branch, currently lines 295-299) to include it:

```rust
let _ = channel.send(ChatEvent::ArtifactReady {
    conversation_id: effective_conv_id.clone(),
    artifact_id: preview.artifact_id,
    content_type: preview.content_type,
    preview: preview.srcdoc,
});
```

Update the existing test `chat_event_artifact_ready_serializes_with_type_field` (in the `#[cfg(test)] mod tests` block, currently around line 646-658) to include the new field when constructing the test event:

```rust
let event = ChatEvent::ArtifactReady {
    conversation_id: "conv-1".into(),
    artifact_id: "art-1".into(),
    content_type: ArtifactContentType::Html,
    preview: "<html></html>".into(),
};
```

and add an assertion that `conversation_id` is present in the serialized JSON.

**Verify**: `cargo check --manifest-path src-tauri/Cargo.toml` → exit 0. `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` → all pass.

### Step 2: Mirror the new field in the frontend `ChatEvent` type

Read `src/lib/api/chat.ts` to find where the `ChatEvent` discriminated union type is defined (it must mirror the Rust enum's `#[serde(tag = "type", rename_all = "PascalCase")]` shape exactly, per `.claude/rules/backend.md`'s "Match TypeScript payload keys exactly to Rust command parameter names" rule). Add `conversation_id: string;` to the `ArtifactReady` variant of that type.

**Verify**: `pnpm check` → exit 0 (confirms no existing code destructures `ArtifactReady` in a way that conflicts with the new required field — if `pnpm check` reports an error in a file you haven't touched yet, that's expected; proceed to fix it in the relevant step below rather than treating it as a STOP condition).

### Step 3: Track a `conversationId` in `chatStore` and pass it through

In `src/lib/stores/chat.ts`:

1. Add a new piece of state near `streamingId`/`requestId` (around line 50-54):
   ```ts
   let activeConversationId = $state<string | null>(null);
   ```
2. In `sendMessage`, set it alongside `streamingId = assistantId;` (line 168) — read how `conversation_id` is currently passed to `chatSend` (check `src/lib/api/chat.ts`'s `chatSend` signature and `sendMessage`'s call at line 182) to determine the right value: if this is a brand-new conversation, the id may not be known until the backend's `Ack` event or a later response — check whether `chat_send`'s `conversation_id: Option<String>` parameter is ever populated by the frontend today, or always `None` (always-new-conversation). Based on what you find, either set `activeConversationId` synchronously in `sendMessage` (if the frontend already generates/knows the id upfront) or set it when the first event referencing the conversation arrives. **If this turns out to be more involved than a 1-line addition because the frontend doesn't currently track any conversation id at all**, that confirms the "Current state" section's caveat above — proceed by adding a minimal `activeConversationId` that's set from whatever value `ArtifactReady`/other events carry, and treat it as best-effort scoping rather than a fully precise model; do not redesign the broader conversation-id flow under this plan.
3. In `handleEvent`'s `ArtifactReady` case (currently line 133-136), pass the comparison value through:
   ```ts
   case 'ArtifactReady': {
       artifactsStore.receiveArtifact(event, activeConversationId);
       break;
   }
   ```

**Verify**: `pnpm check` → exit 0.

### Step 4: `artifactsStore.receiveArtifact` ignores mismatched conversations

In `src/lib/stores/artifacts.ts`, change `receiveArtifact`'s signature and add the guard:

```ts
function receiveArtifact(
	event: Extract<ChatEvent, { type: 'ArtifactReady' }>,
	activeConversationId: string | null,
): void {
	if (
		activeConversationId !== null &&
		event.conversation_id !== activeConversationId
	) {
		// Stale event from a conversation the user has navigated away from —
		// do not let it mutate the artifact panel for whatever is active now.
		return;
	}
	artifactId = event.artifact_id;
	contentType = event.content_type;
	preview = event.preview;
	error = null;
	state = event.preview.trim() ? 'ready' : 'error';
	if (!event.preview.trim()) {
		error = 'Artifact could not be displayed safely.';
	}
}
```

The `activeConversationId !== null` check is deliberate: if the frontend doesn't yet have a confident notion of the active conversation (e.g. very first message of a new conversation, before any id is known), fail open (apply the artifact) rather than fail closed (silently drop every artifact) — matching this plan's "best-effort scoping" framing from Step 3.

**Verify**: `pnpm check` → exit 0. If `plans/005-add-test-coverage.md` has landed and a `artifacts.test.ts` exists, add a test asserting a mismatched `conversation_id` is ignored and a matching one is applied; if no test file exists yet, skip this (do not create a whole new test file as a side effect of this plan — that's plan 005's scope).

## Test plan

If `plans/005-add-test-coverage.md` has landed:

- Add to `src/lib/stores/chat.test.ts` (or a new `artifacts.test.ts` only if one already exists from a prior session — do not create the file fresh under this plan): a test sending an `ArtifactReady` event with a `conversation_id` that doesn't match `activeConversationId`, asserting the artifact store's `artifactId` remains unchanged from its prior value.

If plan 005 has not landed: rely on manual verification — send a message, let an artifact-producing response complete, confirm the artifact panel shows it; this is a regression check on existing behavior, not new coverage for the fix itself (which is inherently hard to manually trigger without simulating the race).

## Done criteria

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --lib ipc::chat` exits 0
- [ ] `pnpm check` exits 0
- [ ] `ChatEvent::ArtifactReady` (both Rust and TypeScript) carries `conversation_id`
- [ ] `artifactsStore.receiveArtifact` ignores events whose `conversation_id` doesn't match the currently active one (when one is known)
- [ ] No files outside the in-scope list are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

- The frontend turns out to have no usable notion of "active conversation id" anywhere (not in `chatStore`, not in `historyStore`) and adding one is clearly a larger design decision than "track one new `$state` variable" — stop and report rather than improvising a half-correct model that could introduce its own bugs.
- `chat_send`'s `conversation_id` parameter handling doesn't match what's described in "Current state" — re-read `chat.rs:149-202` fully before proceeding with Step 3.

## Maintenance notes

- This plan's `activeConversationId` tracking in `chatStore` is intentionally minimal (best-effort, fails open when unknown). If multi-conversation support is ever built, this will need to become a proper per-conversation state model — flag this plan's `activeConversationId` as a placeholder that a multi-conversation feature should replace, not extend.
- Related to `plans/001-guard-concurrent-chat-send.md` — both fix instances of the same root-cause class (stale stream events not checked against current state). A reviewer should look at both PRs together if they land close in time, since they touch adjacent code in `chat.ts`.
