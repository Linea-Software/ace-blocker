# Architecture boundaries and abstraction policy

Read [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) with this guide. Use these rules for design decisions during audits and small refactors. Do not use them to reorganize the workspace or change runtime contracts.

## Start with the actual ownership map

Read the code and its call sites before you propose a dependency direction. The table identifies current responsibilities. It does not claim that all imports follow an ideal dependency graph.

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

Answer these questions before you add a cross-feature import, shared helper, crate, or central store:

1. **Who owns this behavior or data?** Name the domain or feature that controls its meaning.
2. **Why must these units depend on each other?** Name the use case and the smallest necessary contract. Convenience alone is not a reason.
3. **Can the caller use a small public interface or value?** Do not expose implementation details. Keep validation, authorization, and ownership at the correct boundary.
4. **Is this one stable concept?** Similar code is not enough. Keep separate implementations when features can change independently.
5. **What does the dependency cost?** Check call sites, import direction, test impact, change propagation, and privilege boundaries.
6. **How will tests protect the boundary?** Prefer focused contract and regression tests. Add dependency checks where tools permit.

Do not add a universal `common`, `utils`, `base`, `service`, generic repository, or event bus only to remove repeated code. Share code only when semantics, lifetime, ownership, validation, and expected changes are shared. Some duplication is safer than incorrect coupling. Do not put every module in its own crate. Add a crate boundary only when it gives a clear API, isolation, dependency, or build/test benefit.

## Frontend-specific boundaries

- Keep route-specific components and state near their route by default. Share a component only when multiple callers need the **same contract**, not merely similar markup.
- Use local `$state` and `$derived` with explicit props and callbacks for local presentation. Use a shared store only for shared application state with clear ownership and lifetime. Avoid `$effect` chains that copy state between stores.
- Keep policy state authoritative in Rust. Do not let UI snapshots or optimistic forms control enforcement.
- Route new engine operations through `engine-client` → Tauri adapter → typed remoc. Keep authorization, error handling, and stream behavior unchanged.
- Keep generated Paraglide content, the external Ace Server client, and typed RPC contracts separate from business logic. Do not edit generated output to simplify a refactor.

## Rust-specific boundaries

- Keep domain rules near the domain concept. Keep infrastructure adapters, persistence, and transport mapping at their boundaries.
- Use existing newtypes and fallible conversions. Do not pass raw strings or integers through unrelated layers. See [DOMAIN_TYPES.md](DOMAIN_TYPES.md).
- Add a trait only for a real alternate implementation, test boundary, or dependency inversion. Do not add one for each struct or helper.
- Do not make code `pub` without a reason. Do not move unrelated behavior into a global manager. Do not make other crates depend on internal `ace-engine` details.
- Keep independent engine and watchdog processes. Keep the capability-protected remoc boundary. These requirements take priority over module cleanup.

## Architecture audit and escalation

For the **affected scope**, record dependency and call relationships before and after the change. Identify each new boundary crossing. Examine cycles, fan-in and fan-out, large coordinating modules, broad `pub` APIs, and changes that affect unrelated areas. Treat these as signals, not automatic limits. A long, cohesive file can be better than several coupled abstractions. Use automated graph checks only after you define intended ownership and import direction.

**Stop and request a separate architecture decision** if the smallest fix reverses dependency direction, adds cross-process communication, adds a shared crate or protocol, moves enforcement authority, or needs a large contract or data migration. Record the owner, options, rejected alternatives, contract, effects, and tests in the backlog or a short decision note. Do not make this decision as part of a cleanup patch.
