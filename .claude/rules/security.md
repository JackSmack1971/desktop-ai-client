---
name: security
description: Secret handling, PII, exposure control, and security-sensitive config rules.
paths:
  - "src/hooks.server.{ts,js}"
  - "src/app.d.ts"
  - "src/lib/server/**/*.{ts,js}"
  - "src/routes/**/+*.server.{ts,js}"
  - "src/routes/**/+server.{ts,js}"
  - "svelte.config.{js,ts}"
  - "src-tauri/tauri.conf.json"
  - "src-tauri/tauri.conf.*.json"
  - "src-tauri/capabilities/**/*.json"
  - "src-tauri/permissions/**/*.{json,toml}"
  - "src-tauri/src/security/**"
  - "src-tauri/src/ipc/**"
  - "src-tauri/src/telemetry/**"
  - "tests/security/**"
---

# Security Rules

## Privacy and Data Handling

- Keep secrets backend-owned. Do not expose provider credentials, raw tokens, or secret stores to the renderer.
- Use opaque tokens or Rust-owned selection for file intake. Do not let the frontend claim raw path authority.
- Redact sensitive content before logs, telemetry, or release evidence.
- Treat hostile renderer behavior as expected, not exceptional.
- Keep preview surfaces sandboxed and recoverable.
- Review any command, IPC, or storage change that can widen data exposure.

## Server Boundaries and Request Protection

- Store sensitive modules under `$lib/server/**` or give them a `.server.ts` or `.server.js` suffix.
- Read private runtime configuration through `$env/static/private` or `$env/dynamic/private` inside server-bound modules.
- Expose browser-readable configuration only through `PUBLIC_` environment names and public `$env` imports.
- Populate `event.locals` in `src/hooks.server.ts` for per-request authentication, session, and authorization context.
- Define the `App.Locals` shape in `src/app.d.ts` when `event.locals` gains or changes a property.
- Authorize protected pages in `+layout.server.ts` or `+page.server.ts` with `locals` checks and explicit `error(401, ...)` or `error(403, ...)` branches.
- Validate every mutating form action and `+server.ts` handler before state changes, with the validated object used for downstream logic.
- Keep SvelteKit CSRF origin checking active for production form submissions that use POST, PUT, PATCH, or DELETE.
- Record each configured CSRF trusted origin in `svelte.config.*` with a short reason beside the origin entry.
- Set session cookies through SvelteKit `cookies.set(...)` with an explicit `path` and reviewed `httpOnly`, `sameSite`, and `secure` attributes.
- Return client-safe error messages from route handlers, while detailed diagnostics go to the project logger or server trace output.

## Tauri Capabilities and Browser Exposure

- Map every frontend-callable command in `src-tauri/src/**/*.rs` to an explicit permission or capability entry under `src-tauri/capabilities/**/*.json` or `src-tauri/permissions/**/*.{json,toml}`.
- Include `identifier`, `description`, and `commands.allow` in each custom permission file created for project commands.
- Add `scope.allow` and `scope.deny` entries for every permission that accepts filesystem, asset, URL, or shell-like parameters.
- Register release capabilities explicitly in `app.security.capabilities` inside `src-tauri/tauri.conf.json` or the active environment-specific config.
- Match every `@tauri-apps/api/*` frontend import in `src/**/*.{ts,tsx}` to a capability permission that grants that API surface to the intended window.
- Bind each capability to concrete window or webview labels that exist in `app.windows[]`.
- Configure `app.security.csp` with explicit `default-src`, `connect-src`, `img-src`, and `style-src` directives before enabling remote or custom asset loading.
- Enable `app.security.assetProtocol` with `scope.allow` entries tied to app-owned directories and review every `**/*` asset glob before release.
- Store updater settings under `plugins.updater` with an HTTPS endpoint template and a configured `pubkey` for release builds.
- Create new permission files with `pnpm tauri permission new` or `cargo tauri permission new` so identifier, description, and allow-list structure stay consistent.
- Validate security-sensitive config changes with `pnpm tauri build` or `cargo tauri build` before merging release-bound work.

## Telemetry and Evidence

- Keep telemetry separate from user content and prompt history.
- Redact sensitive data before persistence or export.
- Treat evidence capture as a privacy-sensitive operation.
- Never use telemetry to smuggle secrets, raw paths, or prompt payloads across boundaries.
- Keep release evidence minimal, specific, and reviewable.

