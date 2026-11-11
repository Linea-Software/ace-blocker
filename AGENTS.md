# Ace Blocker — unified desktop repository

This repository is an independent Cargo workspace.
Its code came from the former desktop repositories.
**Do not change the original sibling repositories** when you work in this workspace.

## Source of truth

- `Cargo.toml`: root Rust workspace with one `Cargo.lock` and one root `target/`.
- `crates/ace-engine/`: engine library and service executable. Read `crates/ace-engine/AGENTS.md` for component instructions.
- `crates/ace-watchdog/`: small, separate LocalSystem service executable.
- `crates/ace-watchdog-protocol/`: shared heartbeat wire types and tests.
- `crates/ace-update-security/`: signed Windows MSI release manifest, hashes, version policy, and update-phase checks. It has no privileged installer capability.
- `crates/ace-tun/`, `crates/mitm-proxy/`, and `crates/ace-domain-blocklist/`: engine dependencies.
- `apps/ace-app/`: SvelteKit UI and Tauri Rust crate. Read `apps/ace-app/AGENTS.md` for component instructions.
- `installer/`: WiX MSI, service configuration, and bundled Site Packs.

The nested component `AGENTS.md` files were written before repository consolidation.
Their behavior and security requirements still apply.
This root layout replaces old paths that refer to sibling repositories or an engine-local Cargo workspace.

## Mandatory runtime boundaries

- Keep the engine and watchdog as independent LocalSystem services.
  Tauri runs in a normal user session. Closing the UI must not stop enforcement.
- Preserve locks, anti-tampering, Windows service recovery, watchdog heartbeat and maintenance,
  security ACLs, API authentication, certificates, and the installer uninstall gate.
  Do not let the UI stop the engine.
- Local control uses typed remoc RPC over Windows Named Pipes or a Unix socket on Linux.
  The Tauri backend owns the IPC connection and capability credential.
  Do not add a local REST listener, API port, or HTTP fallback.
- The installer owns the production service lifecycle and security settings.
  Do not replace this process with ordinary Tauri bundling.
- Keep product update discovery and staging separate from content updates and TrustedClock.
  Read `docs/PRODUCT_UPDATES.md`.
  Signed metadata or a downloaded MSI does not permit stopping locked services.
  The privileged install path is currently disabled.
- The proxy may decrypt only browser traffic.

## Commands

- `cargo check --workspace` (compile all Rust workspace packages)
- `cargo test -p ace-watchdog-protocol`
- `cargo check -p ace-engine --bin ace-engine`
- `cargo check -p ace-watchdog --bin ace-watchdog`
- `cd apps/ace-app; bun run check; bun run test:unit; bun run build`
- `./installer/build.ps1` on Windows after you install Bun, Rust, and WiX 4

Always run Bun from `apps/ace-app/`.
The local engine API code is in `apps/ace-app/src/lib/engine-client/`.
The generated external Ace Server client is separate from this remoc control interface.

The Network Block editor sends the complete draft through one capability-protected
`saveNetworkBlock` remoc call.
`NetworkManager::save_network_block` validates the complete policy and its locks.
It saves metadata, rules, schedule, and connections in one SQLite transaction.
It publishes live state only after COMMIT.
The quick enable switch and Category download are separate actions.

Seed starter policies only with Free packs from the consolidated
`0021_seed_default_network_blocks` baseline.
Adult, Social, Gambling, and Streaming connect to Free Site Packs.
The compact Gambling and Streaming packs are bundled as offline SQLite artifacts.
Their manifest contains SHA-256 entries.
Keep release metadata consistent across `ace-hosts`, `ace-blocker-site`,
and `apps/ace-app/src/lib/data/site-packs.json`.
Never select a Pro pack during first-install seeding.

Network Blocks store `scheduleMode` (`always` or `scheduled`) separately from Enabled.
The default is `always`.
The `scheduled` mode needs at least one period.
A Lock must never permit `Always -> Scheduled`.
Read `docs/NETWORK_BLOCK_ACTIVATION.md` for migration, backup, and enforcement rules.

## AI refactoring: read only the applicable instructions

**Pre-release compatibility decision (2026-10-22):**
Ace Blocker has not been publicly released.
Old program formats, databases, backups, and internal protocol formats do **not** need backward compatibility.
Do not add migrations, compatibility shims, or fallbacks only to support that historical data.
Read [`docs/ai/ACE_ENGINE_REFACTOR_DECISIONS.md`](docs/ai/ACE_ENGINE_REFACTOR_DECISIONS.md)
for approved engine behavior and typed-error rules.
This decision does **not** weaken security, locks, watchdog, installer, or IPC requirements.
It does **not** permit arbitrary deletion of active development data.

Use [`docs/ai/REFACTORING_PLAYBOOK.md`](docs/ai/REFACTORING_PLAYBOOK.md)
for the audit, bounded implementation, independent review, and verification process.
Read the applicable guidance **before you propose an abstraction**:

- [`docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md`](docs/ai/BOUNDARIES_AND_ABSTRACTIONS.md):
  ownership, dependency direction, Rust crate boundaries, and Tauri/Svelte separation.
  It also explains when not to combine unrelated modules.
- [`docs/ai/DOMAIN_TYPES.md`](docs/ai/DOMAIN_TYPES.md):
  meaningful newtypes, fallible validation, and Rust/TypeScript trust boundaries.
  Avoid unnecessary primitive values and unnecessary wrappers.
- [`docs/ai/REVIEW_GATES.md`](docs/ai/REVIEW_GATES.md):
  architecture and domain-model review gates, independent diff checks,
  evidence requirements, and conflicting external skills.
- [`docs/ai/TECH_DEBT_BACKLOG.md`](docs/ai/TECH_DEBT_BACKLOG.md):
  verified findings, scope, risks, acceptance criteria, and check results.

A broad cleanup request does not permit a complete rewrite.
Before you add a dependency, shared or global state, a crate, a generic abstraction,
or a domain type, identify its owner, invariant, use case, alternatives, and verification.
Keep domains separate when they change independently, even if their code looks similar.
Prefer existing narrow interfaces.
Some code duplication is acceptable.
If a refactor changes product behavior, public contracts, or privileged runtime
boundaries, stop and propose a separate change for approval.

**Instruction priority:** Root security requirements and actual contracts take precedence.
Nested `AGENTS.md` files can add requirements, but cannot weaken root guarantees.
Third-party skills are optional guidance.
Before use, verify that a skill is present and read its contents.
Resolve conflicting guidance explicitly. See the review gates.
A skill name in a list does not prove that the skill is installed.
Do not reorganize code based only on line counts, import counts, or duplication metrics.

## Changes

Change only files in this repository.
Inspect Git status before you edit.
Do not reset or overwrite another contributor's changes.
Test changes to services, locks, or proxies with specific checks.
Commit completed and verified changes in this repository.
