---
status: testing
phase: 02-routing
source: [02-SUMMARY.md]
started: 2026-06-16T04:14:58.2132382Z
updated: 2026-06-16T04:14:58.2132382Z
---

## Current Test

<!-- OVERWRITE each test - shows where we are -->

number: 1
name: Cold Start Smoke Test
expected: |
Kill any running app or backend process, clear ephemeral local state if present,
and start the application from scratch.

The app boots without startup errors, the shell loads, and a basic chat action
still reaches live UI state after the fresh start.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test

expected: |
Kill any running app or backend process, clear ephemeral local state if present,
and start the application from scratch.

The app boots without startup errors, the shell loads, and a basic chat action
still reaches live UI state after the fresh start.
result: [pending]

### 2. Send a Prompt Without Frontend Secrets

expected: |
Open the chat surface and submit a prompt without entering any API key in the
renderer.

The request is accepted, no secret entry field appears, and the user sees the
assistant response path begin through the normal chat UI.
result: [pending]

### 3. Streaming Output Arrives In Order

expected: |
Start a response that produces multiple chunks.

The placeholder state changes into a streaming bubble on the first delta, text
appears incrementally in the same message, and the visible order of output is
preserved.
result: [pending]

### 4. Cancel An In-Flight Response

expected: |
While a response is still streaming, use the cancel control in the chat input
area.

The request stops, the conversation remains intact, and the interrupted message
is marked as cancelled or incomplete instead of continuing to stream.
result: [pending]

### 5. Typed Errors Surface Cleanly

expected: |
Trigger a provider or routing error while sending a prompt.

The UI shows a clear typed error in the alert region, and the active chat list
remains usable without corruption.
result: [pending]

### 6. Completion Clears Cancel State

expected: |
Let a response finish normally.

The assistant message settles into a completed state, the streaming indicator
disappears, and the cancel control is no longer available for that request.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps

[none yet]
