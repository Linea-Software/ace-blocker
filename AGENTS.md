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
- `crates/ace-update-security/`: signed Windows MSI release manifest, hash, version-policy and update-phase verification. No privileged installer capability.
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
- Local control uses typed remoc RPC over Windows Named Pipes (Unix socket on
  Linux). The Tauri backend owns the IPC connection and capability credential.
  Never reintroduce a local REST listener, API port or HTTP fallback.
- The installer remains the production owner of service lifecycle and
  service security settings. Do not substitute ordinary Tauri bundling.
- Product update discovery/staging is separate from content updates and TrustedClock. See `docs/PRODUCT_UPDATES.md`. Never treat signed metadata or downloaded MSI as permission to stop locked services; the privileged install path is currently disabled.
- Only browser traffic may be decrypted by the proxy.

## Commands

- `cargo check --workspace` (full Rust compilation)
- `cargo test -p ace-watchdog-protocol`
- `cargo check -p ace-engine --bin ace-engine`
- `cargo check -p ace-watchdog --bin ace-watchdog`
- `cd apps/ace-app; bun run check; bun run test:unit; bun run build`
- `./installer/build.ps1` on Windows after installing Bun, Rust, and WiX 4

Always run Bun from `apps/ace-app/`. The local engine API is maintained in
`apps/ace-app/src/lib/engine-client/`; the generated external Ace Server
client is independent of this remoc control interface.

The Network Block editor saves the full draft through one capability-protected
`saveNetworkBlock` remoc call. `NetworkManager::save_network_block` validates
the entire policy and locks, persists its metadata/rules/schedule/connections
in one SQLite transaction, then publishes live state after COMMIT. The separate
quick enable switch and category download are independent explicit actions.
Network Blocks now persist an explicit `scheduleMode` (`always`/`scheduled`) independent of Enabled: `always` is the default, `scheduled` requires at least one period, and Lock must never permit `Always -> Scheduled`. See `docs/NETWORK_BLOCK_ACTIVATION.md` for migration, backup and enforcement semantics.

## AI refactoring: read only what applies

**Pre-release compatibility decision (2026-10-22):** Ace Blocker has not been publicly released. Legacy program formats, old databases/backups and older internal protocol shapes require **no backward compatibility**; do not introduce legacy migrations, compatibility shims or fallback paths for historical data solely to preserve them. See [`docs/ai/ACE_ENGINE_REFACTOR_DECISIONS.md`](docs/ai/ACE_ENGINE_REFACTOR_DECISIONS.md) for approved engine-specific behavior and typed-error policies. This does **not** relax current security/lock/watchdog/installer/IPC invariants or authorize arbitrary deletion of active development data.

Use [`docs/ai/REFACTORING_PLAYBOOK.md`](docs/ai/REFACTORING_PLAYBOOK.md)
for the audit → bounded implementation → independent review → verification workflow.
Route to the relevant specialized guidance **before proposing an abstraction**:

- [`docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`](docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md):
  ownership, dependency direction, Rust crate boundaries, Tauri/Svelte separation,
  and when *not* to deduplicate unrelated modules.
- [`docs/ai/DOMAIN_TYPES.md`](docs/ai/DOMAIN_TYPES.md):
  meaningful newtypes, fallible validation and Rust/TypeScript trust boundaries;
  avoid both primitive obsession **and** pointless wrappers.
- [`docs/ai/REVIEW_GATES.md`](docs/ai/REVIEW_GATES.md):
  architecture and domain-model review gates, independent diff assessment,
  evidence requirements, and handling contradictory external skills.
- [`docs/ai/TECH_DEBT_BACKLOG.md`](docs/ai/TECH_DEBT_BACKLOG.md):
  verified findings, scope, risks, acceptance criteria and check results.

Do not interpret a broad cleanup request as permission for a wholesale rewrite.
Before introducing a dependency between previously independent modules, new
shared/global state, a crate, a generic abstraction or a domain type, identify
the owning concept, specific invariant/use case, alternatives and verification.
Keep similar-looking but independently evolving domains separate. Prefer
existing narrow interfaces; a small amount of duplication can be correct.
If a refactor requires new product semantics, public contracts or privileged
runtime boundaries, stop and propose a separately approved change.

**Instruction priority:** root security/runtime constraints and actual contracts
always win. Nested `AGENTS.md` adds component-specific requirements, but
cannot weaken root guarantees. Third-party skills are optional guidance only:
verify they are actually present, examine their contents, and resolve conflicts
explicitly (see the review gates); do not assume a skills list is an install.
Never turn subjective metrics (line count, import count, duplication) into
automatic reasons to reorganize code.

## Changes

Keep edits scoped to this repository. Before editing, review Git status.
Never reset/rewrite other contributors' changes. Verify sensitive service,
lock, or proxy changes with specific tests. Commit finished verified changes
within this repository.
