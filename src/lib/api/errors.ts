/**
 * Shared IPC error normalization.
 *
 * Backend IPC errors serialize as `{ code: "SCREAMING_SNAKE_CASE", message: string }`
 * (see `#[serde(tag = "code", content = "message", ...)]` on every domain error enum
 * in `src-tauri/src/ipc/*.rs`). This is the single place that turns that shape (or a
 * raw string, or an unrecognized rejection) into a user-facing message. Every store
 * that calls `invoke(...)` should import this instead of redefining it.
 */
export function normalizeIpcError(e: unknown): string {
	if (typeof e === 'string') return e;
	if (e && typeof e === 'object') {
		const obj = e as Record<string, unknown>;
		if (typeof obj['message'] === 'string') return obj['message'];
		if (typeof obj['code'] === 'string') return `Error: ${obj['code']}`;
	}
	return 'An unexpected error occurred.';
}
