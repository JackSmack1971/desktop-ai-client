# Phase 6: Release — Plan

**Phase:** 06-release
**Goal:** Make the project release-ready with reviewed command exposure and adversarial evidence
**Mode:** mvp
**Requirements:** REL-01, REL-02

---

## Phase Goal

**As a** maintainer preparing a release, **I want to** prove the packaged app exposes only reviewed commands and ships with the expected hardening evidence, **so that** release readiness is gated by verified backend policy rather than build success alone.

Research for this phase is already captured in [`06-CONTEXT.md`](./06-CONTEXT.md) and the adjacent discussion log. This plan translates those decisions into an executable verifier, a source-controlled command inventory, and a first-pass release evidence bundle.

---

## Success Criteria

1. Every registered custom command is listed in the reviewed inventory and matched against the release capability set.
2. The release gate fails on inventory drift, capability drift, missing negative-test coverage, or debug-only commands in production.
3. The release evidence bundle includes the expected security, routing, storage, accessibility, and adversarial fixture outputs for implemented paths.
4. A build alone is never treated as release-ready unless the reviewed inventory and evidence bundle both pass.

---

## Locked Decision Coverage

| Decision | Covered In |
|----------|------------|
| D-01 deny-by-inventory release gate | T-1, T-2 |
| D-02 reviewed inventory shape in `security/command-inventory.toml` | T-1, T-2 |
| D-03 full registered command set is in scope | T-1, T-2, T-3 |
| D-04 explicit release/dev capability split | T-1, T-2 |
| D-05 `main.json` remains the release-selected baseline unless expanded intentionally | T-1, T-2 |
| D-06 first-pass evidence bundle only requires implemented paths | T-3 |
| D-07 evidence bundle preserves hardening-spec categories | T-3 |
| D-08 exact fixture families remain the target shape where implementation exists | T-3 |

---

## Threat Model

### Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Rust command registration → reviewed inventory | Every compiled custom command must be declared in the source-controlled inventory before release |
| capability files → release selection | Capability inclusion must be explicit; folder presence alone is not a release decision |
| test outputs / fixtures → release evidence | Evidence files must reflect actual implemented-path verification, not fabricated coverage |

### STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-06-01 | Elevation of Privilege | unreviewed command surface | mitigate | inventory verifier fails if any registered custom command is missing from `security/command-inventory.toml` or the selected release capability set |
| T-06-02 | Tampering | capability drift | mitigate | classify every capability file as release-selected or dev-only and fail release if the classification is ambiguous |
| T-06-03 | Information Disclosure | release evidence placeholders | mitigate | evidence bundle must cite real test outputs and implemented-path fixtures only |
| T-06-04 | Denial of Service | partial release gate | mitigate | release gate fails on missing negative-test coverage for commands and on missing fixture families |

---

## Wave Structure

| Wave | Tasks | Description | Parallel? |
|------|-------|-------------|-----------|
| 1 | T-1 | Reviewed inventory + explicit capability selection catalog | — |
| 2 | T-2 | Command-inventory verifier + compiled-command allowlist checks | — |
| 3 | T-3 | Release evidence bundle + fixture-backed verification | — |

---

## Tasks

---

### Wave 1: Inventory and Capability Selection

#### T-1 — Create the reviewed command inventory and explicit capability selection catalog

**Files:**
- `security/command-inventory.toml`
- `security/release-capabilities.toml`
- `docs/command-inventory.md`
- `src-tauri/capabilities/main.json`
- `src-tauri/tauri.conf.json`

**Description:**

Define the release authority for the current command surface in source-controlled data. Enumerate every command currently registered in `src-tauri/src/main.rs`:

- `get_active_surface`
- `set_active_surface`
- `chat_send`
- `chat_cancel`
- `artifact_get`
- `artifact_dismiss`
- `history_list`
- `history_get`
- `history_delete`
- `history_search`
- `privacy_set_provider_key`
- `privacy_get_credential_status`
- `privacy_clear_provider_key`
- `files_open_dialog`
- `files_read_token`

For each command, record at least:

- owning module
- allowed window labels
- production or debug status
- argument schema summary
- sensitivity class
- expected capability grant
- required negative-test coverage

Create a separate release-capability catalog so release selection is explicit instead of inferred from the directory layout. `src-tauri/capabilities/main.json` remains the release-selected baseline unless the phase intentionally adds more release capabilities later. If no dev-only capability files exist yet, record that explicitly rather than leaving the split implicit.

Update `docs/command-inventory.md` so the review file explains the canonical fields and how the release/dev split is interpreted by the verifier.

**Verification:**
```
cat security/command-inventory.toml
cat security/release-capabilities.toml
```

**Done:** The reviewed inventory exists, the release-selected capability set is explicit, and the docs describe how to interpret both.

---

### Wave 2: Deny-by-Inventory Verification

#### T-2 — Implement the command-inventory verifier and compiled-command cross-check

**Files:**
- `src-tauri/build.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/src/ipc/inventory.rs`
- `src-tauri/src/ipc/mod.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/bin/verify-command-inventory.rs`

**Description:**

Add a project-owned verifier that compares the compiled command surface, `tauri::generate_handler![...]`, the reviewed inventory, and the explicit release-capability catalog. Use the build script to surface the compiled command allowlist where the pinned Tauri version supports `tauri_build::AppManifest::commands`; the runtime verifier should fail closed if the lists diverge, if a registered command is missing from inventory, if a capability grants a command that is not reviewed, or if a command is tagged as debug-only for release.

Keep the loader and comparison logic reusable so the same source of truth can power both CI and release evidence generation. Treat `src-tauri/src/ipc/inventory.rs` as the shared inventory snapshot module even though it is not a frontend-facing product feature.

Add tests that cover:

- a complete inventory snapshot round-trip
- missing-command drift
- extra-command drift
- release-vs-dev capability classification
- negative-test coverage metadata for each command

**Verification:**
```
cargo test --manifest-path src-tauri/Cargo.toml -- command_inventory
cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory
```

**Done:** Inventory drift fails deterministically, the compiled command surface is checked against source control, and the verifier exits nonzero when the release set is not clean.

---

### Wave 3: Release Evidence and Fixture Coverage

#### T-3 — Build the release evidence bundle and implement the first-pass evidence collector

**Files:**
- `src-tauri/src/telemetry/mod.rs`
- `src-tauri/src/telemetry/release_evidence.rs`
- `src-tauri/src/bin/collect-release-evidence.rs`
- `docs/release-evidence.md`
- `release-evidence/*`
- `tests/security/*`
- `tests/e2e/*`
- `tests/fixtures/adversarial-sse/*`
- `tests/fixtures/provider-drift/*`
- `tests/fixtures/fts-query-abuse/*`
- `tests/fixtures/srcdoc-escaping/*`
- `tests/fixtures/wal-recovery/*`
- `tests/fixtures/capability-drift/*`

**Description:**

Replace the placeholder release-evidence contract with a first-pass bundle generator that preserves the hardening-spec structure while only requiring evidence for implemented paths. Export `release_evidence` from `telemetry/mod.rs`, then have the collector assemble a source-controlled bundle under `release-evidence/` using the hardening-spec categories that are already implementable in this repo:

- security checks
- streaming tests
- database / storage evidence
- provider-routed evidence
- command-inventory verification
- artifact sandbox and accessibility evidence
- adversarial fixture coverage

Materialize the exact fixture families called out by the hardening spec wherever the matching implementation exists:

- SSE errors
- FTS query abuse
- `srcdoc` escaping
- WAL recovery
- capability drift

Keep the evidence bundle honest: if a path is not implemented yet, represent it as deferred structure instead of inventing proof. Update `docs/release-evidence.md` so the bundle contract and naming scheme are explicit for maintainers and CI.

**Verification:**
```
cargo test --manifest-path src-tauri/Cargo.toml
cargo run --manifest-path src-tauri/Cargo.toml --bin collect-release-evidence
```

**Done:** The bundle is reproducible, the evidence files correspond to real tests and fixtures, and the bundle format matches the hardening-spec categories without overclaiming coverage.

---

## Source Audit

| Source | Item | Covered By | Status |
|--------|------|------------|--------|
| GOAL | Release-ready command exposure | T-1, T-2 | COVERED |
| GOAL | Adversarial evidence before release | T-3 | COVERED |
| REQ REL-01 | Reviewed command inventory + explicit release capability selection | T-1, T-2 | COVERED |
| REQ REL-02 | Release evidence for security, routing, storage, and adversarial fixtures | T-3 | COVERED |
| D-01 deny-by-inventory | T-1, T-2 | COVERED |
| D-02 inventory as source-controlled artifact | T-1 | COVERED |
| D-03 full registered surface | T-1, T-2 | COVERED |
| D-04 explicit release/dev split | T-1, T-2 | COVERED |
| D-05 main.json baseline remains release-selected | T-1, T-2 | COVERED |
| D-06 first-pass evidence only for implemented paths | T-3 | COVERED |
| D-07 preserve hardening-spec evidence shape | T-3 | COVERED |
| D-08 exact fixture families remain the target shape | T-3 | COVERED |

---

## Verification

Before declaring the plan complete:

- [ ] `security/command-inventory.toml` exists and enumerates every registered command.
- [ ] `security/release-capabilities.toml` makes release selection explicit.
- [ ] The command-inventory verifier fails on missing, extra, and debug-only commands.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml -- command_inventory` succeeds.
- [ ] `cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory` succeeds on the clean tree.
- [ ] The release evidence bundle exists and reflects implemented-path test output only.
- [ ] Exact fixture families are represented where implementation exists.

---

## Success Criteria

- Command exposure is explicitly inventoried and cross-checked before release.
- The release gate includes the expected security, routing, storage, and fixture evidence.
- A build alone is not considered complete unless the verification evidence is present.
- The repo can point to a reviewed, reproducible release package instead of relying on informal checks.

---

## Output

Create `security/command-inventory.toml`, `security/release-capabilities.toml`, `release-evidence/`, and the supporting verifier and evidence files when executing this plan.
