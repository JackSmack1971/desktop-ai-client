---
name: frontend
description: UI/UX standards, component patterns, and frontend build rules for the desktop AI client.
paths:
  - "src/lib/components/**/*.{svelte,ts,js}"
  - "src/lib/stores/**/*.{ts,js}"
  - "src/routes/**/+*.{svelte,ts,js}"
  - "src/**/*.d.ts"
  - "vite.config.{ts,js,mts,mjs}"
  - "index.html"
  - "public/**/*"
  - "package.json"
---

# Frontend Rules

## Architecture and Boundaries

- Treat the repository as a Tauri desktop client scaffold with a docs-led contract.
- Keep frontend, backend, provider routing, storage, telemetry, and security concerns separated.
- Do not invent module boundaries that conflict with `docs/architecture.md` or `.planning/` docs.
- Treat missing manifests as a fact, not a temporary inconvenience. Do not assume the app is runnable until the repo actually contains the relevant build files.
- Update the affected docs when behavior, ownership, or boundaries change.

## Component State

- Put state shared by 2 or more components in a dedicated module instead of duplicating it inside component-local `$state(...)`.
- Keep shared state APIs small and typed. Export the narrowest writable, readable, or derived surface needed by consumers.
- Prefer a single source of truth for UI state that spans multiple components or surfaces.
- Use `derived` state for values computed from shared state, not for side effects.
- Keep subscription setup and teardown explicit so shared stores do not leak listeners across navigation or component churn.
- Avoid coupling shared state modules to Svelte component markup or route-specific concerns.

## Svelte Component Contracts

- Use `<script lang="ts">` in every Svelte component that declares script logic.
- Define a local `interface Props` or inline generic prop object for every component that calls `$props()`.
- Import snippet types with `import type { Snippet } from 'svelte'` when component props include children, render props, or reusable snippets.
- Use `<script lang="ts" generics="T">` for reusable collection components where `data: T[]` and `row: Snippet<[T]>` must stay type-aligned.
- Type event callback props as functions with explicit arguments, for example `onSelect: (item: T) => void`.
- Capture forwarded attributes with a rest prop typed as `[key: string]: unknown` or a narrower declared attribute interface.
- Render typed snippets with `{@render ...}` calls whose arguments match the `Snippet<[...]>` tuple type.
- Keep all component-local data transformations typed before values enter markup expressions.

## Svelte 5 Reactivity

- Declare component-local mutable primitives, arrays, and objects with `$state(...)` inside `<script lang="ts">`.
- Compute values that are pure functions of existing state with `$derived(...)`.
- Reserve `$effect(...)` for browser integration, subscriptions, timers, analytics, and imperative APIs.
- Return a teardown function from every `$effect(...)` that creates an interval, subscription, listener, observer, animation frame, or external resource.
- Use `onMount` for browser-only initialization that must start after DOM creation; return synchronous cleanup when setup owns resources.
- Keep side effects out of `$derived(...)`; place imperative work in event handlers, `onMount`, or `$effect(...)` with cleanup.
- Keep each `$effect(...)` body focused on 1 external concern; split unrelated browser APIs into separate effects.
- Store DOM node references from `bind:this` in local component state and interact with them inside `onMount` or `$effect(...)`.
- Keep shared state out of component-local `$state(...)`; move cross-component stores and shared runes modules to `component-state.md`.
- Use `tick()` after state changes when reading DOM layout that depends on the just-updated state.
- Give every `{#each}` block a stable key when rendering mutable lists or lists received through props.

## Routing and Data

- Scope every `src/routes/**` route to SvelteKit file conventions: `+page.svelte`, `+layout.svelte`, `+page.ts`, `+page.server.ts`, `+layout.ts`, `+layout.server.ts`, and `+server.ts`.
- Use generated `./$types` imports for every route module type: `PageLoad`, `PageServerLoad`, `LayoutLoad`, `LayoutServerLoad`, `Actions`, or `RequestHandler`.
- Type page and layout components with generated route types such as `PageProps`, `PageData`, or `LayoutData` from `./$types`.
- Keep browser-safe data loading in universal `+page.ts` or `+layout.ts` files when the returned values are serializable route data.
- Place database queries, private environment access, cookies, filesystem reads, and server SDK calls in `+page.server.ts`, `+layout.server.ts`, `+server.ts`, or `$lib/server/**`.
- Return serializable objects from every `load` function, with explicit keys consumed by the nearest `+page.svelte` or `+layout.svelte` component.
- Use `await parent()` inside child `load` functions when nested route data depends on layout data, then destructure the required parent keys explicitly.
- Raise expected route failures with SvelteKit `error(status, message)` using concrete HTTP statuses such as `404`, `401`, or `403`.
- Keep route parameter handling inside the matching route directory, with `params` reads scoped to that route's generated `$types` contract.
- Add or update the nearest `+error.svelte` when a route introduces a new user-visible `error(...)` status path.

## Streaming UI

- Render partial failure as a distinct state, not as a completed assistant message.
- Track the visible states `Streaming`, `Interrupted - partial response saved`, `Retrying`, `Completed`, `Cancelled`, and `Failed`.
- Show recovery actions only when the backend has emitted partial-failure metadata.
- Keep retry mode selection explicit; do not imply uninterrupted generation after a failure.
- Treat stale stream events as inert once the active stream or attempt changes.

## TypeScript Baseline

- Keep `strict: true`, `verbatimModuleSyntax: true`, `moduleResolution: "bundler"`, `exactOptionalPropertyTypes: true`, and `noUncheckedIndexedAccess: true` aligned across every project `tsconfig*.json`.
- Verify TypeScript edits with the repository check script or `tsc --noEmit` before reporting completion for touched `.ts` and `.svelte` paths.
- Use `import type` and `export type` for type-only symbols in every `.ts` file and every `<script lang="ts">` block.
- Use `satisfies` for route config, component variant maps, feature-flag maps, and literal lookup objects that must retain literal inference while conforming to a declared shape.
- Represent finite async UI states as discriminated unions with explicit `state` tags for `idle`, `loading`, `success`, and `error` branches.
- Narrow `unknown` values with type guards, schema parsers, or `instanceof` checks before property access in browser input, route param, and fetch response paths.
- Give every exported helper in `src/lib/**/*.ts` explicit parameter types and an explicit return type.
- Model recoverable network, auth, and form failures with typed result objects that expose `ok: true` or `ok: false`.

## Vite and Client Build

- Keep `vite` pinned in `package.json` as `^6.0.0` for this project scope.
- Define Vite configuration with `defineConfig(...)` in `vite.config.ts` or an equivalent Vite-supported config file.
- Configure `server.port` as `1420` and `server.strictPort` as `true` in every Vite config for this frontend.
- Configure `server.host` as `127.0.0.1` for local, Dev Container, and VS Code port-forwarded development.
- Align HMR proxy settings by setting `server.hmr.clientPort` to `1420` when a reverse proxy or container forwards the dev server.
- Represent `server.cors` as an object with explicit `origin` entries for the backend or app shell origins used by this project.
- Keep browser access origins limited to localhost, `127.0.0.1`, `::1`, and checked-in backend/app-shell origins.
- Document each non-local CORS origin beside the Vite config with the owning service name and environment.
- Configure backend integration origins in `server.cors.origin` when the frontend is served through another local process.
- Keep Vite client configuration exposed through the default `VITE_` env prefix or an explicit non-empty `envPrefix` array.
- Store client-visible values in `VITE_*` variables and parse each value as a string before boolean, number, or URL use.
- Store secrets, tokens, database URLs, and provider keys in unprefixed backend environment variables.
- Track `.env.*.local` files in `.gitignore` and keep committed `.env.example` values as placeholders only.
- Represent intentionally exposed unprefixed constants with `define` and `JSON.stringify(...)` in `vite.config.*`.
- Enable `build.manifest` when a backend, Tauri shell, or server renderer consumes hashed Vite assets.
- Configure `build.rollupOptions.input` explicitly when this frontend uses a non-default HTML or JavaScript entry.
- Resolve bundled static assets with ESM imports or `new URL("./asset.ext", import.meta.url)` using statically analyzable paths.
- Place files required at stable root URLs in `public/` and reference them with root-absolute paths.
- Run `pnpm vite build` or the project `pnpm build` script after changing `vite.config.*`, `index.html`, `public/**/*`, or asset import paths.

