# Ace Blocker — unified desktop repository

This is an independent Cargo workspace, copied from the former desktop
repositories. **Do not modify the original sibling repositories** as part of
tasks on this workspace.

## Source of truth

- `Cargo.toml`: root Rust workspace, single `Cargo.lock`, root `target/`.
- `crates/ace-engine/`: engine library and service executable; original
  component instructions are in `crates/ace-engine/AGENTS.md`.
- `crates/ace-watchdog/`: small, separate LocalSystem service executable.
- `crates/ace-watchdog-protocol/`: shared heartbeat wire types and tests.
- `crates/ace-tun/`, `crates/mitm-proxy/`,
  `crates/ace-domain-blocklist/`: engine dependencies.
- `apps/ace-app/`: SvelteKit user interface and Tauri Rust crate; component
  instructions in `apps/ace-app/AGENTS.md`.
- `installer/`: WiX MSI, service configuration, bundled site packs.

The nested component AGENTS files predate this repository consolidation.
Their *behavioral and security requirements* still apply, but paths that
assume sibling repositories or an engine-local Cargo workspace are superseded
by this root layout.

## Non-negotiable runtime boundaries

- The engine and watchdog MUST remain independent LocalSystem services.
  Tauri is a normal user-session application. Closing the UI never stops
  enforcement.
- Preserve lock semantics, anti-tampering, Windows service recovery,
  watchdog heartbeat/maintenance, security ACLs, API auth, certificate
  handling, and installer uninstall gate. Do not introduce a UI-driven
  engine shutdown.
- Maintain the existing local HTTPS REST boundary for the app until an
  explicit architecture change is approved.
- The installer remains the production owner of service lifecycle and
  service security settings. Do not substitute ordinary Tauri bundling.
- Only browser traffic may be decrypted by the proxy.

## Commands

- `cargo check --workspace` (full Rust compilation)
- `cargo test -p ace-watchdog-protocol`
- `cargo check -p ace-engine --bin ace-engine`
- `cargo check -p ace-watchdog --bin ace-watchdog`
- `cd apps/ace-app; bun run gen:api; bun run check`
- `./installer/build.ps1` on Windows after installing Bun, Rust, and WiX 4

Always run Bun from `apps/ace-app/`. Frontend API code generation reads
`crates/ace-engine/docs/openapi.json`.

## Refactoring and technical debt

For AI-led audits, cleanup, and refactors, follow
[`docs/ai/REFACTORING_PLAYBOOK.md`](docs/ai/REFACTORING_PLAYBOOK.md).
Record findings and verification in
[`docs/ai/TECH_DEBT_BACKLOG.md`](docs/ai/TECH_DEBT_BACKLOG.md).
Do not interpret a broad cleanup request as permission for a wholesale rewrite.

## Changes

Keep edits scoped to this repository. Before editing, review Git status.
Never reset/rewrite other contributors' changes. Verify sensitive service,
lock, or proxy changes with specific tests. Commit finished verified changes
within this repository.
