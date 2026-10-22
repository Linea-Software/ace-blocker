# Architecture boundaries and abstraction policy

Use with [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md). This document guides **design choices during audits and scoped refactors**; it is not a license to reorganize the workspace or change runtime contracts.

## Start with the actual ownership map

Read the relevant code and call sites before enforcing any proposed dependency direction. The following identifies existing responsibilities, not a claim that every existing import already satisfies an ideal graph.

| Area | Owns | Boundary to protect |
| --- | --- | --- |
| `crates/ace-engine/src/core/` | Policy concepts, schedules, locks, domain data | Do not make policy semantics depend on UI or transport representation |
| `crates/ace-engine/src/manager/` | Orchestration of domain state and mutations | Keep mutation authorization, consistency, and persistence ordering explicit |
| `crates/ace-engine/src/db/` | Diesel/SQLite persistence details | Do not leak DB models into unrelated domain/UI layers without a reason |
| `crates/ace-engine/src/service/api/` | Typed remoc operations, authorization and transport | Delegates to policy-enforcing operations; never let UI shortcuts bypass them |
| `crates/ace-engine/src/service/proxy/`, `crates/mitm-proxy/`, `crates/ace-tun/` | Network enforcement, interception, tunnel/OS integration | Preserve non-browser opacity and enforcement decisions; do not turn the proxy into a global application service |
| `crates/ace-watchdog/`, `crates/ace-watchdog-protocol/` | Independent watchdog service and shared heartbeat wire contract | No lifecycle ownership by Tauri; shared types do not merge services |
| `apps/ace-app/src-tauri/` | Native user-session adapter, local IPC connection and capability | WebView must not acquire pipe credentials or privileged policy authority |
| `apps/ace-app/src/lib/engine-client/` | Stable UI-facing engine API facade | No ad hoc local REST, duplicated RPC transports, or direct pipe access from Svelte |
| `apps/ace-app/src/routes/`, `src/lib/components/`, `src/lib/stores/` | Route UI, presentational components and deliberately shared UI state | Avoid importing across unrelated feature internals or using global state as an integration bus |
| `installer/` | Production install, upgrade, service permissions and uninstall gate | UI bundling must not absorb privileged installer responsibilities |

See [IPC_ARCHITECTURE.md](../IPC_ARCHITECTURE.md) for the authoritative current local-control contract. External Ace Server HTTP client code is **separate** from local remoc IPC.

## Decision rule for a new dependency

Before introducing a cross-feature import, a shared helper, an additional crate, or a central store, answer:

1. **Who owns this behavior/data?** Identify the domain or feature whose semantics change when the requirement changes.
2. **Why must these units know each other?** Name the actual use case and the narrow contract required. A convenient import is not a design reason.
3. **Can the caller depend on a small public interface or value instead of implementation internals?** Keep validation, authorization and ownership at the authoritative boundary.
4. **Does sharing reflect one stable concept, or only similar-looking code?** If two features can evolve independently, prefer local implementations over premature coupling.
5. **What will this link cost?** Trace call sites, direction of imports, effect on tests, change propagation, and runtime privilege boundaries.
6. **How will the boundary be tested?** Prefer a focused contract/regression test and an enforceable dependency check where tooling permits.

Do not create a universal `common`, `utils`, `base`, `service`, generic repository, or event bus merely to remove duplication. Extract shared code only when semantics, lifetime, ownership, validation rules, and likely future changes **really are shared**. Some duplication is safer than the wrong abstraction. Equally, do not split every module into a separate crate: use a crate boundary only for a meaningful independently owned API, runtime isolation, dependency constraint, or build/testing benefit.

## Frontend-specific boundaries

- Route-specific components and state stay near the route by default. Share a component only when multiple consumers need the **same contract**, not merely similar markup.
- Prefer local `$state`/`$derived` and explicit props/callbacks for local presentation. Introduce shared stores only for truly shared application state with defined lifecycle and ownership; avoid `$effect` chains that synchronize duplicate state.
- The engine's authoritative policy state remains in Rust. UI snapshots and optimistic form state must not become a competing source of truth for enforcement.
- New engine operations go through the existing `engine-client` facade → Tauri adapter → typed remoc contract, with relevant authorization and error/stream behavior preserved.
- Keep generated Paraglide content, the external Ace Server client, and typed RPC contracts separate from application-specific business logic. Never patch generated outputs as a refactoring shortcut.

## Rust-specific boundaries

- Place domain rules near the relevant domain concept; keep infrastructure adaptors, persistence and transport mapping at their boundaries.
- Preserve existing newtypes and fallible conversions instead of passing raw strings/integers through unrelated layers (see [DOMAIN_TYPES.md](DOMAIN_TYPES.md)).
- Introduce traits when there is a concrete alternate implementation, test seam, or dependency inversion requirement; not for every struct or one-off helper.
- Do not casually change visibility to `pub`, move behavior into a global manager, or make unrelated crates depend on `ace-engine` implementation details.
- The independent engine/watchdog processes and capability-protected remoc contract outrank any suggested modularity cleanup.

## Architecture audit and escalation

For the **affected scope**, record the before/after import or call relationship and any newly crossed boundaries. Investigate dependency cycles, fan-in/fan-out, oversized coordinating modules, broad `pub` APIs, and changes requiring edits across unrelated areas. These are **signals**, not absolute thresholds: a long cohesive file can be healthier than five artificially coupled abstractions. Automated graph checks are useful only after the intended ownership/direction has been decided.

**Stop and propose a separate architectural decision** if the smallest fix requires reversing dependency direction, new cross-process communication, a new shared crate/protocol, moving enforcement authority, or a large public contract/data migration. Record the owner, options, rejected alternatives, contract, consequences and needed tests in the relevant backlog item (or a concise architectural decision note) before implementation. Never silently make these decisions inside a cleanup patch.
