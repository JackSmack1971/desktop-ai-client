# Codebase Structure

**Analysis Date:** 2026-06-13

## Directory Layout

```
desktop-ai-client/
├── src/                        # Svelte 5 / SvelteKit frontend renderer
│   ├── app.html                # Root HTML shell
│   ├── routes/                 # SvelteKit file-based routing
│   │   ├── +layout.svelte      # Root layout; hydrates surface store on mount
│   │   └── +page.svelte        # Root page; renders AppShell
│   └── lib/                    # Shared frontend modules
│       ├── accessibility/      # Accessibility helpers (scaffolded)
│       ├── api/                # IPC wrapper utilities (scaffolded)
│       ├── components/         # Svelte UI components
│       │   ├── AppShell.svelte         # Root layout shell (rail + content)
│       │   ├── SurfaceRail.svelte      # Side navigation rail
│       │   ├── SurfacePanel.svelte     # Active surface display area
│       │   ├── WorkspaceShell.svelte   # Workspace wrapper
│       │   ├── StatusRegion.svelte     # Accessible status announcements
│       │   └── surfaces/               # Per-surface view components
│       │       ├── ChatSurface.svelte
│       │       ├── HistorySurface.svelte
│       │       ├── SettingsSurface.svelte
│       │       └── ArtifactsSurface.svelte
│       ├── editor/             # Editor helpers (scaffolded)
│       └── stores/             # Svelte 5 reactive state stores
│           └── surface.ts      # Backend-owned surface preference store
│
├── src-tauri/                  # Tauri Rust crate (backend)
│   ├── src/                    # Rust source modules
│   │   ├── main.rs             # Crate entrypoint; Tauri builder bootstrap
│   │   ├── lib.rs              # Library root
│   │   ├── app_state.rs        # AppState, ShellState, Surface enum
│   │   ├── ipc/                # Frontend-facing command surface
│   │   │   ├── mod.rs          # Declares all IPC submodules
│   │   │   ├── app_shell.rs    # get_active_surface / set_active_surface (implemented)
│   │   │   ├── chat.rs         # Chat commands (scaffolded)
│   │   │   ├── files.rs        # File intake commands (scaffolded)
│   │   │   ├── history.rs      # History commands (scaffolded)
│   │   │   ├── inventory.rs    # Command inventory commands (scaffolded)
│   │   │   ├── privacy.rs      # Privacy commands (scaffolded)
│   │   │   └── providers.rs    # Provider commands (scaffolded)
│   │   ├── providers/          # Provider capability and routing
│   │   │   ├── mod.rs
│   │   │   ├── capabilities.rs # Capability detection (scaffolded)
│   │   │   ├── openrouter.rs   # OpenRouter provider adapter (scaffolded)
│   │   │   ├── routing.rs      # Routing policy (scaffolded)
│   │   │   └── sse.rs          # SSE streaming transport (scaffolded)
│   │   ├── security/           # Secrets, redaction, file tokens, sandboxing
│   │   │   ├── mod.rs
│   │   │   ├── artifact_sandbox.rs  # Artifact isolation (scaffolded)
│   │   │   ├── command_policy.rs    # Command execution policy (scaffolded)
│   │   │   ├── file_tokens.rs       # Opaque file access tokens (scaffolded)
│   │   │   ├── redaction.rs         # Content redaction (scaffolded)
│   │   │   └── secrets.rs           # Provider credential store (scaffolded)
│   │   ├── storage/            # SQLite persistence, migrations, FTS, retention
│   │   │   ├── mod.rs
│   │   │   ├── sqlite.rs       # SqlitePool + ShellPreferenceStore (implemented)
│   │   │   ├── migrations.rs   # Migration runner + MIGRATIONS slice (implemented)
│   │   │   ├── backup.rs       # Backup and export (scaffolded)
│   │   │   ├── fts.rs          # FTS5 full-text search (scaffolded)
│   │   │   └── retention.rs    # Retention and deletion policy (scaffolded)
│   │   └── telemetry/          # Audit logging and release evidence
│   │       ├── mod.rs
│   │       ├── audit_log.rs    # Audit log (scaffolded)
│   │       └── release_evidence.rs  # Release evidence capture (scaffolded)
│   ├── capabilities/           # Tauri capability grant files (JSON)
│   ├── migrations/             # SQL migration asset files
│   ├── permissions/            # Tauri permission definitions
│   ├── tests/                  # Rust integration tests
│   ├── Cargo.toml              # Rust crate manifest
│   ├── tauri.conf.json         # Tauri app configuration
│   └── AGENTS.md               # Backend subtree ownership rules
│
├── docs/                       # Product and agent-facing design context
│   ├── architecture.md         # Agent loop architecture (Planner/Executor model)
│   ├── privacy-boundaries.md   # Data access and redaction contract
│   ├── provider-routing.md     # Provider selection and fallback rules
│   ├── threat-model.md         # Security threat model
│   ├── command-inventory.md    # IPC command registry (source of truth)
│   ├── design-blueprint.md     # UI/UX design context
│   ├── implementation-plan.md  # Implementation milestones
│   ├── agent-context.md        # Agent operating context
│   ├── memory-loop.md          # Memory loop design
│   ├── multi-agent.md          # Multi-agent coordination design
│   ├── prompt-blueprint.md     # Prompt design rules
│   ├── release-evidence.md     # Release evidence format
│   └── prompt-templates/       # Prompt template files
│
├── .planning/                  # GSD planning layer
│   ├── PROJECT.md              # Project definition
│   ├── REQUIREMENTS.md         # Requirements
│   ├── ROADMAP.md              # Phase roadmap
│   ├── STATE.md                # Current project state
│   ├── config.json             # GSD config
│   ├── codebase/               # Codebase analysis documents (this file lives here)
│   └── phases/                 # Phase plan files
│
├── .claude/                    # Claude Code framework
│   ├── rules/                  # Path-scoped rule files
│   │   ├── architecture.md
│   │   ├── privacy.md
│   │   ├── provider-routing.md
│   │   ├── storage.md
│   │   ├── telemetry.md
│   │   └── testing.md
│   ├── agents/                 # Agent definitions
│   ├── commands/               # Custom slash commands
│   ├── hooks/                  # Lifecycle hooks
│   ├── output-styles/          # Output format definitions
│   ├── skills/                 # Skill modules
│   ├── workflows/              # Workflow definitions
│   └── worktrees/              # Worktree configuration
│
├── tests/                      # Top-level test directory
│   ├── e2e/                    # End-to-end tests
│   ├── fixtures/               # Test fixture data
│   ├── rust/                   # Rust integration tests (top-level)
│   └── security/               # Security-focused tests
│
├── scripts/                    # Build and utility scripts
├── AGENTS.md                   # Root ownership and invariants
├── CLAUDE.md                   # Claude Code project instructions
└── package.json                # Frontend manifest (SvelteKit/Vite)
```

## Directory Purposes

**`src/`:**
- Purpose: Svelte 5 / SvelteKit frontend renderer code
- Contains: Routes, layout components, surface view components, reactive stores
- Key files: `src/routes/+layout.svelte` (bootstrap hydration), `src/lib/stores/surface.ts` (IPC bridge), `src/lib/components/AppShell.svelte` (root layout)

**`src-tauri/src/`:**
- Purpose: All Rust backend behavior
- Contains: IPC command handlers, provider adapters, security modules, storage layer, telemetry
- Key files: `src-tauri/src/main.rs` (bootstrap), `src-tauri/src/app_state.rs` (shared state), `src-tauri/src/ipc/app_shell.rs` (only fully implemented IPC module)

**`src-tauri/src/ipc/`:**
- Purpose: Frontend-facing command boundary
- Contains: One `.rs` file per domain, all implementing `#[tauri::command]` functions
- Key files: `mod.rs` (declares submodules), `app_shell.rs` (get/set active surface)

**`src-tauri/src/storage/`:**
- Purpose: All SQLite persistence
- Contains: Connection pool, typed domain stores, migration runner
- Key files: `sqlite.rs` (SqlitePool, ShellPreferenceStore), `migrations.rs` (MIGRATIONS slice and runner)

**`src-tauri/capabilities/`:**
- Purpose: Tauri capability grants (defense-in-depth, not sole enforcement)
- Generated: No — maintained manually alongside command registration

**`docs/`:**
- Purpose: Source-of-truth contracts for architecture, privacy, provider routing, threat model, and command inventory
- Key files: `architecture.md`, `privacy-boundaries.md`, `command-inventory.md`, `threat-model.md`
- Rule: Treat as the contract when source code is scaffolded; update when behavior changes

**`.planning/`:**
- Purpose: GSD project management layer
- Contains: Requirements, roadmap, phase plans, codebase analysis
- Key files: `PROJECT.md`, `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`

**`.claude/rules/`:**
- Purpose: Path-scoped Claude Code behavior rules
- Load order: `architecture` + `testing` always; add `privacy`, `provider-routing`, `storage`, `telemetry` as needed

## Key File Locations

**Entry Points:**
- `src-tauri/src/main.rs`: Rust/Tauri bootstrap entrypoint
- `src/routes/+layout.svelte`: SvelteKit root layout; triggers surface hydration on mount
- `src/routes/+page.svelte`: Root SvelteKit page

**Shared Backend State:**
- `src-tauri/src/app_state.rs`: `AppState`, `ShellState`, `Surface` enum — the single source of truth for runtime state shape

**IPC (Only Implemented Command):**
- `src-tauri/src/ipc/app_shell.rs`: `get_active_surface` and `set_active_surface` with window-label enforcement and typed errors

**Storage (Implemented):**
- `src-tauri/src/storage/sqlite.rs`: `SqlitePool` (WAL-mode connection) and `ShellPreferenceStore`
- `src-tauri/src/storage/migrations.rs`: `MIGRATIONS` slice and `run_migrations()` runner

**Frontend Store:**
- `src/lib/stores/surface.ts`: Singleton Svelte 5 store; all IPC round-trips for surface state live here

**UI Components:**
- `src/lib/components/AppShell.svelte`: Root shell layout (rail + content)
- `src/lib/components/SurfaceRail.svelte`: Side navigation
- `src/lib/components/StatusRegion.svelte`: Accessible status announcements
- `src/lib/components/surfaces/*.svelte`: Per-surface view components

**Docs (Contracts):**
- `docs/command-inventory.md`: Authoritative list of all IPC commands
- `docs/privacy-boundaries.md`: What data the client may access, store, or transmit
- `docs/provider-routing.md`: How the client selects providers and handles fallback

## Naming Conventions

**Files:**
- Rust modules: `snake_case.rs` matching the module name (e.g. `app_shell.rs`, `file_tokens.rs`)
- Svelte components: `PascalCase.svelte` (e.g. `AppShell.svelte`, `SurfaceRail.svelte`)
- TypeScript stores and utilities: `camelCase.ts` (e.g. `surface.ts`)
- Surface view components: `<Name>Surface.svelte` under `src/lib/components/surfaces/`

**Directories:**
- Rust subsystem modules: singular `snake_case` (e.g. `ipc/`, `providers/`, `security/`, `storage/`, `telemetry/`)
- Frontend feature areas: plural or descriptive `camelCase` (e.g. `components/`, `stores/`, `routes/`)

**Types:**
- Rust: `PascalCase` for structs, enums, and traits; `snake_case` for functions and fields
- TypeScript: `PascalCase` for types/interfaces; `camelCase` for functions and variables
- Svelte stores: factory function returning an object (e.g. `createSurfaceStore()` → exported singleton `surfaceStore`)

## Where to Add New Code

**New IPC command domain (e.g. chat):**
1. Add `pub mod <domain>;` to `src-tauri/src/ipc/mod.rs`
2. Implement commands in `src-tauri/src/ipc/<domain>.rs` using `#[tauri::command]`
3. Register in `tauri::generate_handler![...]` in `src-tauri/src/main.rs`
4. Add a capability entry in `src-tauri/capabilities/`
5. Add entry to `docs/command-inventory.md`

**New surface (e.g. Files):**
1. Add variant to `Surface` enum in `src-tauri/src/app_state.rs`
2. Add corresponding migration in `src-tauri/src/storage/migrations.rs`
3. Add `type Surface` literal in `src/lib/stores/surface.ts`
4. Add `<Name>Surface.svelte` under `src/lib/components/surfaces/`

**New backend subsystem module (e.g. new security helper):**
- Add `pub mod <name>;` to the relevant `mod.rs` (e.g. `src-tauri/src/security/mod.rs`)
- Implement in `src-tauri/src/security/<name>.rs`
- Do not import from `ipc/` — dependency direction is one-way

**New SQLite table:**
1. Append a new `Migration` to `MIGRATIONS` in `src-tauri/src/storage/migrations.rs`
2. Add a typed domain store in `src-tauri/src/storage/sqlite.rs` (or a new file in `storage/`)
3. IPC handlers call store methods, never raw SQL

**New Svelte component:**
- UI components: `src/lib/components/<Name>.svelte`
- Surface views: `src/lib/components/surfaces/<Name>Surface.svelte`
- Shared stores: `src/lib/stores/<name>.ts` (factory pattern, exported singleton)

**Tests:**
- Rust unit tests: `#[cfg(test)] mod tests` block at the bottom of the relevant `.rs` file
- Rust integration tests: `src-tauri/tests/` or `tests/rust/`
- E2E tests: `tests/e2e/`
- Security tests: `tests/security/`

## Special Directories

**`src-tauri/migrations/`:**
- Purpose: Raw SQL migration asset files (distinct from the Rust migration runner)
- Generated: No
- Committed: Yes

**`src-tauri/capabilities/`:**
- Purpose: Tauri JSON capability grant files — control which commands each window can invoke
- Generated: No
- Committed: Yes
- Note: Backend-side window-label enforcement in IPC handlers is the primary control; capabilities are defense-in-depth

**`.svelte-kit/`:**
- Purpose: SvelteKit generated build artifacts and type stubs
- Generated: Yes
- Committed: No

**`target/`:**
- Purpose: Rust/Cargo build output
- Generated: Yes
- Committed: No

**`.planning/codebase/`:**
- Purpose: GSD codebase analysis documents consumed by `/gsd-plan-phase` and `/gsd-execute-phase`
- Generated: Yes (by `/gsd-map-codebase`)
- Committed: Yes

---

*Structure analysis: 2026-06-13*
