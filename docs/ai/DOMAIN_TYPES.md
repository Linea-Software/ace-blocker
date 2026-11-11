# Domain types, validation, and trust boundaries

Use this guide for audits and refactors that affect Rust entities, stored data, IPC payloads, or TypeScript state. Also read [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) and [BOUNDARIES_AND_ABSTRACTIONS.md](BOUNDARIES_AND_ABSTRACTIONS.md).

## The question is semantic, not syntactic

Use `String`, `u32`, or `boolean` for data without special constraints. Use a domain type for an identity, a value with important rules, a unit or clock meaning, a secret, or values that can be confused. Do not add a branded type or newtype only to increase type count.

Before you propose a new type, identify:
- Invalid states or mistaken value combinations that the current code permits.
- The owner of the meaning and validation rule. Check whether a standard or library type already models it.
- Boundaries that accept untrusted input: UI forms, storage, remote APIs, RPC, and files.
- Operations that can fail. Identify rules that must survive serialization, persistence, and round trips.
- Any change to wire formats, stored values, normalization, errors, or compatibility. Get separate approval and tests for these changes.

### Rust

See `crates/ace-engine/src/core/common/domain_types.rs`. It uses `nutype` and explicit validation for `RemoteSitePackId`, `Sha256Digest`, `BundleSchemaVersion`, and `ApiKey`. `ApiKey` does not implement ordinary `Debug` or `Display` because these could expose secrets. Use these patterns. Do not add a competing validation framework.

- Control constructors and private fields. Parse or `TryFrom` untrusted values **once at the boundary that owns them**. Then use validated types inside the system.
- Use established types such as `url::Url`, `Uuid`, `chrono` types, and existing project IDs when they fit. Do not wrap every URL string. Use a specific `SitePackSourceUrl` only if product rules require more checks.
- Do not replace a valid domain type with a raw primitive for convenience. Do not wrap a value again when an existing type already enforces its rules.
- Use enums and associated data for states that must not be combined. Avoid unclear booleans and magic integers.
- Validate data when it enters Rust, even if TypeScript already assigned a type. Rust controls authorization and business rules.
- Treat serialization and deserialization as trust boundaries. Test rejection of invalid data. Test compatibility of the wire and DB forms. A typed field alone does not prove secure deserialization or authorization.

### Svelte/TypeScript

- TypeScript types and `as` casts do **not validate input at runtime**. Use existing Zod patterns for untrusted input that needs runtime checks. Derive TypeScript types from the schema when useful.
- A TypeScript brand separates identities during compilation. It does not validate input. Validate through constructors or parsers when data first enters trusted app code.
- Do not validate every prop, local variable, or already validated value with Zod. Do not duplicate Rust/remoc schemas that can become inconsistent.
- Keep unfinished form input as strings. For example, let the user type a partial URL. Convert to a domain value only at submit or commit. Show a validation error without changing the partial input.
- Put domain types near their owning feature. Do not move every type to a shared types folder.

## Practical design examples

| Symptom | Prefer | Avoid |
| --- | --- | --- |
| An arbitrary string stands for a validated external URL | A standard URL parser, or a specific validated URL type only if business invariants demand one | Regex-only parsing or an unchecked `as Url` cast |
| Two different IDs are interchangeable `string`/`Uuid` | Existing ID wrappers/newtypes scoped to the owning domain | Exposing all IDs as raw strings throughout internal code |
| Unclear schedule duration, timezone or units | Explicit duration/timezone semantics and tests around boundaries | Unlabelled `number`/`u64` or silently changing stored time interpretation |
| A secret gets logged in common debugging output | A secret-owning type whose formatting/serialization is intentionally constrained | `Debug`/`Display` of credential values |
| Similar fields in two unrelated subsystems | Independent types until a **shared meaning** is established | One generic object with optional fields and branching behavior |

## Review / regression requirements

Check valid and invalid inputs, normalization, round-trip serialization, old stored values, error compatibility, and authorization or secret exposure. List consumers before you change a type used by remoc, Tauri, DB migrations, or an external API. Require explicit approval for a contract change. Do not hide it in a behavior-preserving refactor.

Ask: **Which real mistakes will this type prevent, and what will it cost?** Reject the type if the only benefit is cleaner-looking type declarations.
