# Coding Conventions

**Analysis Date:** 2026-06-13

## Rust Conventions

### Module Organization

Top-level modules declared in `src-tauri/src/lib.rs`:

- `app_state` — shared runtime state structs and enums
- `ipc` — all frontend-facing command handlers, one domain per submodule
- `providers` — model provider routing, capabilities, SSE transport
- `security` — file tokens, redaction, artifact sandbox, command policy
- `storage` — SQLite pool, migrations, FTS, backup, retention
- `telemetry` — audit log, release evidence

Each submodule uses `pub mod <name>;` declared in the parent `mod.rs` or `lib.rs`. Domain submodules expose a narrow typed API; internal helpers are private. Placeholder scaffolds use a single `pub fn <module_name>() { // Scaffold placeholder. }` until real behavior is added.

### Naming Patterns

**Structs:** `PascalCase` — `AppState`, `ShellState`, `SqlitePool`, `ShellPreferenceStore`

**Enums:** `PascalCase` with `snake_case` serde rename — `Surface` serializes as `"chat"`, `"history"`, etc. via `#[serde(rename_all = "snake_case")]`

**Error enums:** `PascalCase` with `SCREAMING_SNAKE_CASE` serde code field:

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShellError {
    #[error("storage error: {0}")]
    StorageError(String),
}
```

**Functions:** `snake_case` — `run_migrations`, `save_active_surface`, `assert_main_window`

**Constants / statics:** `SCREAMING_SNAKE_CASE` — `MIGRATIONS`

**IPC command functions:** registered with `#[tauri::command]`, named in `snake_case` matching the JS `invoke` string (`get_active_surface`, `set_active_surface`).

### Error Handling

- All IPC commands return `Result<T, DomainError>` — never `unwrap()` or raw panics in command handlers.
- Domain errors use `thiserror::Error` for `Display` and implement `serde::Serialize` so they cross the IPC boundary as `{ code, message }` objects.
- Internal storage functions return `rusqlite::Result<T>` and are converted at the IPC boundary with `.map_err(|e| ShellError::StorageError(e.to_string()))`.
- Mutex poison in `SqlitePool::with_conn` propagates as a panic rather than a misleading error (connection state is unknown after poison).
- `debug_assert!` is used for dev-only invariant checks (e.g., migration id character set in `src-tauri/src/storage/migrations.rs`).

Example pattern (`src-tauri/src/ipc/app_shell.rs`):

```rust
let mut shell = state.shell.lock().map_err(|e| {
    ShellError::StorageError(format!("shell state lock poisoned: {e}"))
})?;
```

### Documentation Standards

- Every public struct, enum, and function carries `///` doc comments explaining purpose and invariants.
- Privacy invariants are explicitly called out: "Privacy invariant: AppState must never expose provider credentials… to the frontend."
- Lock ordering is documented inline where multiple locks are acquired (shell lock → sqlite lock in `get_active_surface`).
- `src-tauri/src/ipc/mod.rs` carries a header listing invariants all commands must satisfy.
- `SAFETY:` comments label patterns that would be dangerous if generalized (e.g., static migration SQL embedding).

### Serde Patterns

- Enums that cross the IPC boundary derive both `serde::Serialize` and `serde::Deserialize`.
- `#[serde(rename_all = "snake_case")]` on domain enums (`Surface`).
- `#[serde(tag = "code", content = "message")]` on error enums for structured frontend consumption.

### Formatting and Linting

- No `rustfmt.toml` or `.clippy.toml` detected — defaults apply.
- Edition 2021, rust-version 1.77 minimum (`src-tauri/Cargo.toml`).
- Release profile: `lto = true`, `opt-level = "z"`, `strip = true`, `panic = "abort"`, `codegen-units = 1`.

---

## TypeScript / Svelte Conventions

### Language and Framework

- Svelte 5 with SvelteKit. Rune-based reactivity: `$state`, `$derived`, `$effect`, `$props`.
- TypeScript throughout. Explicit type annotations on all exported types and function signatures.
- No ESLint or Prettier config detected. `svelte-check` (`npm run check`) is the type and template validation tool.

### Store Pattern

Stores are factory functions returning plain objects with reactive getters. No Svelte 4 `writable`/`readable`.

Pattern from `src/lib/stores/surface.ts`:

```typescript
function createSurfaceStore() {
	let surface = $state<Surface>('chat');
	return {
		get surface() {
			return surface;
		},
		hydrate,
		setSurface,
	};
}
export const surfaceStore = createSurfaceStore();
```

### IPC Call Pattern

All backend calls use `invoke` from `@tauri-apps/api/core`. Never use `fetch`, `localStorage`, or `sessionStorage` for backend-owned state.

```typescript
import { invoke } from '@tauri-apps/api/core';
const persisted = await invoke<Surface>('get_active_surface');
await invoke<void>('set_active_surface', { surface: next });
```

IPC errors are normalized via a `normalizeIpcError(e: unknown): string` helper before surfacing to the user — handles both plain strings and structured `{ code, message }` objects from the Rust backend. Defined in `src/lib/stores/surface.ts`.

### Optimistic Update Pattern

Stores apply an optimistic update before the IPC call, then roll back on failure:

```typescript
const previous = surface;
surface = next; // optimistic
try {
	await invoke<void>('set_active_surface', { surface: next });
} catch (e) {
	surface = previous; // rollback
	error = normalizeIpcError(e);
}
```

### Component Patterns

- Props declared via `interface Props {}` and destructured with `let { prop }: Props = $props();`.
- Reactive derived values: `let x = $derived(store.field)`.
- Side effects: `$effect(() => { ... })`.
- Event handlers use Svelte 5 syntax: `onclick`, `onkeydown` (not `on:click`).
- Component children rendered with `{@render children?.()}` (Svelte 5 snippet API).
- `bind:this` used to collect typed DOM element ref arrays.

### Naming Patterns

**Files:** `PascalCase.svelte` for components (`AppShell.svelte`, `SurfaceRail.svelte`). `camelCase.ts` for stores and utilities (`surface.ts`). SvelteKit route conventions for `+layout.svelte`, `+page.svelte`.

**Types:** `PascalCase` (`Surface`, `Props`). String union literals for domain enums mirroring Rust enums.

**Constants:** `SCREAMING_SNAKE_CASE` for module-level maps (`SURFACE_LABELS`).

**Functions:** `camelCase` (`createSurfaceStore`, `normalizeIpcError`, `setSurface`, `hydrate`).

### Import Organization

1. Svelte lifecycle imports (`import { onMount } from 'svelte'`)
2. Store imports from `$lib/stores/`
3. Component imports from `$lib/components/`

`$lib` alias resolves to `src/lib/` via SvelteKit default path resolution.

### Accessibility Standards

- ARIA landmarks on structural components: `role="tablist"`, `role="tab"`, `aria-label`, `aria-selected`, `aria-controls`.
- Roving tabindex pattern on interactive groups (tab rail in `src/lib/components/SurfaceRail.svelte`).
- `.sr-only` helper class defined per-component for screen-reader-only text.
- `aria-live` regions for loading and error status announcements.
- `focus-visible` outline on interactive elements (not `focus`).

### CSS Conventions

- Scoped `<style>` blocks per component.
- BEM-style class naming: `block__element` and `block--modifier` (e.g., `rail-button__icon`, `rail-button--active`).
- Hardcoded hex values in a dark palette (`#0f0f0f`, `#1a1a1a`, `#2a2a2a`, `#4a9eff`). No CSS custom properties yet.
- No global stylesheet beyond SvelteKit injection.

---

_Convention analysis: 2026-06-13_
