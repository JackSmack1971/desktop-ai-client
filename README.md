# desktop-ai-client

A memory-aware Tauri desktop client for long-running AI agent tasks with persistent episodic, procedural, and caution memory.

## Overview

**desktop-ai-client** is a docs-led desktop application scaffold for AI agents that learn from mistakes, reuse workflows, and maintain context across sessions.

- **Memory-first**: Working, episodic, procedural, and caution memory types guide behavior
- **Provider-agnostic**: Backend routes API calls; provider selection stays backend-owned
- **Security-hardened**: Command inventory, capability grants reviewed before release

## Quickstart

### Prerequisites

- Node.js 18+
- pnpm 8+
- Rust 1.77+
- Tauri CLI 2.x

### Install

```bash
git clone https://github.com/JackSmack1971/desktop-ai-client.git
cd desktop-ai-client
pnpm install --frozen-lockfile
```

### Run

```bash
pnpm dev
pnpm build
```

### Verify

```bash
pnpm check
cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory
```

## Features

- **Memory-aware task execution**: Four distinct memory types for agent behavior
- **Persistent state**: Chat history and lessons via SQLite backend
- **Provider routing**: API calls route through backend; providers are swappable
- **Secure IPC**: Communication validated against command inventory
- **Privacy-first**: Sensitive data redacted before logging
- **Extensible**: Custom skills, workflows, hooks via blueprints

## Architecture

Long-running agent with persistent memory:

- **Planner**: Breaks work into steps
- **Executor**: Performs tool calls, records traces
- **Memory Writer**: Converts traces to memories
- **Memory Manager**: Dedupes and promotes memories
- **Retriever**: Loads relevant memories
- **Judge**: Validates candidate memories

See docs/architecture.md for complete design.

## Directory Structure

```
desktop-ai-client/
├── docs/                          # Context layer
├── src/                           # SvelteKit frontend
├── src-tauri/                     # Rust backend
├── security/                      # Release gates
├── AGENTS.md                      # Intent layer
└── package.json
```

## Usage

Main commands:
- pnpm dev: Development with hot reload
- pnpm build: Production binary
- pnpm check: Type-check
- pnpm check:watch: Continuous type-check
- pnpm frontend:dev: SvelteKit dev server

Read AGENTS.md before editing code in any directory.

## Configuration

| Variable | Required | Default | Notes |
|---|---|---|---|
| RUST_LOG | No | info | Backend log level |
| Provider keys | No | None | Via system keyring |

## Developer Command Center

| Script | Purpose |
|---|---|
| pnpm dev | Dev server |
| pnpm build | Production binary |
| pnpm check | Type-check |
| cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory | Verify commands |

## Testing & Verification

```bash
pnpm check
cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory
```

## Troubleshooting

| Symptom | Fix |
|---|---|
| pnpm dev hangs | rustup update && pnpm install |
| Frontend fails to connect | Check port 1420 in vite.config.ts |
| Type-check fails | pnpm check or pnpm dlx svelte-kit sync |
| Command inventory fails | Add to security/command-inventory.toml |
| SQLite error | Check database path in app_state.rs |
| Keyring fails | Check system credential manager |

## Stack Inventory

| Component | Tech | Version |
|---|---|---|
| Desktop | Tauri | 2.0.0 |
| Frontend | SvelteKit | 2.0.0 |
| UI | Svelte | 5.0.0 |
| Build | Vite | 6.0.0 |
| Backend | Rust | 1.77+ |
| Database | SQLite | bundled |
| Runtime | Tokio | 1.x |
| Package Mgr | pnpm | 8+ |

## Reproducibility & Maintenance

### Fresh Clone

```bash
git clone https://github.com/JackSmack1971/desktop-ai-client.git
cd desktop-ai-client
pnpm install --frozen-lockfile
pnpm check
cargo run --manifest-path src-tauri/Cargo.toml --bin verify-command-inventory
pnpm dev
```

### Dependency Updates

```bash
pnpm update
cargo update --manifest-path src-tauri/Cargo.toml
pnpm check
```

### Platform Notes

- **Windows**: Visual Studio Build Tools or rustup component add windows-msvc
- **macOS**: xcode-select --install
- **Linux**: sudo apt install libssl-dev libgtk-3-dev libayatana-appindicator3-dev
- **WSL**: See Tauri WSL documentation

## Contributing

Before large changes:
1. Open an issue
2. Read AGENTS.md files
3. Read design blueprints in docs/
4. Update docs if boundaries change
5. Run pnpm check and verifier before PR

## Governance

| Area | Status |
|---|---|
| Code of Conduct | [TBD] |
| Security Policy | [TBD] |
| License | No license file found |
| Maintainers | [TBD] |
| Support | [TBD] |

## Roadmap

- **Phase 1-5**: Scaffold and document system
- **Phase 6 (current)**: Release baseline app
- **Phase 7+**: Multi-agent coordination

See .planning/ and docs/ for details.

## License

No license file found. Add before publishing or accepting contributions.

---

For agent access: Start with AGENTS.md, then docs/README.md and docs/agent-context.md.
