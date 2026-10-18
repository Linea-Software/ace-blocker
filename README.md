# Ace Blocker — Unified Workspace

This is a **new, independent source repository** created from the existing
`ace-engine`, `ace-app`, and `ace-installer` projects. Their original
directories and Git repositories are not modified or linked. This is an
initial snapshot, not a Git-history merge.

## Layout

- `crates/ace-engine/`: Rust blocking engine (library + Windows service executable).
- `crates/ace-watchdog/`: independent Rust watchdog service executable.
- `crates/ace-watchdog-protocol/`: shared heartbeat / maintenance protocol.
- `crates/ace-tun/`, `crates/mitm-proxy/`, `crates/ace-domain-blocklist/`: engine dependencies.
- `apps/ace-app/`: SvelteKit frontend + Rust Tauri crate in `src-tauri/`.
- `installer/`: existing WiX 4 MSI installer, including offline site packs.

**Runtime isolation is unchanged.** The engine and watchdog stay separate
LocalSystem Windows services; the Tauri app runs in the user session.
The app now uses typed remoc RPC over Windows Named Pipes through the
Tauri backend (Unix sockets on Linux). See docs/IPC_ARCHITECTURE.md for
security boundaries and release verification.

The public website (`ace-blocker-site`), Android client, host-list production
pipeline (`ace-hosts`), and resistance-script source (`boredom-engine`) are
intentionally **not** merged into this desktop runtime repository.

## Developer commands

```powershell
# From the repository root
cargo check --workspace --all-targets
cargo test -p ace-engine --lib remoc_tests
cargo test -p ace-watchdog-protocol
cargo build --release -p ace-engine -p ace-watchdog --bins

# Run frontend commands from apps/ace-app
cd apps/ace-app
bun install --frozen-lockfile
bun run check
bun run tauri dev

# Installer from repository root (Windows, elevated as required)
.\installer\build.ps1
```

The Windows x64 CRT and linker settings are in root `.cargo/config.toml`.
The shared Cargo build output is root `target/`. The app retains its
own `bun.lock`; there is one root `Cargo.lock`.

The WiX installer intentionally remains responsible for service ACLs,
recovery, lock-aware uninstall, certificates, and offline site packs.
Do not replace its service configuration with Tauri's basic bundler.

## Refactoring and code cleanup

AI agents should follow the [Refactoring Playbook](docs/ai/REFACTORING_PLAYBOOK.md)
for read-only audits, bounded implementation, independent review, and verification.
Track actionable findings in the [Technical Debt Backlog](docs/ai/TECH_DEBT_BACKLOG.md).

## Repository boundaries

This new repository has its own Git history and no configured remote.
Release workflow credentials, GitHub project configuration, and end-to-end
installation/anti-tampering validation must be reviewed before shipping.
