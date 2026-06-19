# providers/AGENTS.md

This subtree owns provider integrations and routing.

## Read first

Before editing code here, read:
1. `../../../AGENTS.md`
2. `../../../docs/provider-routing.md`
3. `../../../docs/privacy-boundaries.md` — stub; see `../security/AGENTS.md`.
4. `../AGENTS.md`

## Purpose

Owns provider capability/model selection (`routing.rs`), the OpenRouter HTTP adapter (`openrouter.rs`), and SSE stream parsing (`sse.rs`). `capabilities.rs` and `mod.rs` are unimplemented stubs (1 and 4 lines) — capability-based provider selection is deferred past Phase 2; routing currently always selects OpenRouter.

## Contracts & Invariants

- `sse.rs` must stay free of Tauri imports — it takes a `reqwest::Response` plus a generic callback and never calls `channel.send()` itself. All `Channel` sends happen in `ipc::chat`.
- Dependency direction is unidirectional: `ipc` → `providers`, never the reverse. `providers::openrouter` must not import from `ipc::chat`.
- `DEFAULT_MODEL` is pinned to a specific string validated by a dedicated unit test tied to "decision D-13" — changing it is a reviewable, test-gated action, not a casual edit.
- Keep provider capability detection explicit and routing logic deterministic and testable.
- Do not store secrets in provider modules — that's `security::secrets`'s job.
- Do not let provider choice leak into unrelated layers.

## Pitfalls

- SSE line buffering must accumulate `line_buf` across HTTP chunks ("Pitfall 8" in code comments) — TCP fragmentation can split a single SSE line across multiple `bytes_stream()` items. Don't parse each chunk in isolation.
- A mid-stream SSE error object (HTTP 200 with an `error` key in the JSON body) takes precedence over any `delta.content` in the same chunk. Both can theoretically be present; the error must win (see `parse_sse_line_detects_mid_stream_error`).

