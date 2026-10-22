# Domain types, validation, and trust boundaries

Use during audits and refactors touching Rust entities, persisted data, IPC payloads, or TypeScript state. Refer to [REFACTORING_PLAYBOOK.md](REFACTORING_PLAYBOOK.md) and [BOUNDARIES_AND_ABSTRACTIONS.md](BOUNDARIES_AND_ABSTRACTIONS.md).

## The question is semantic, not syntactic

A `String`, `u32`, or `boolean` is appropriate when it is genuinely unconstrained data. A dedicated type is justified when it represents a **domain identity**, a value with important invariants, a unit/clock interpretation, a secret, or a pair of values that would be easy to mix up. Never introduce a branded/newtype wrapper only to increase the number of types.

Before proposing a new type, establish:
- The specific invalid states or mix-ups that currently remain representable.
- Who owns the meaning and validation rule; whether one existing standard/library type already models it.
- Which boundaries accept untrusted/raw input (UI forms, storage, remote APIs, RPC, files).
- Which operations should remain fallible, and which invariants must survive serialization, persistence, and round trips.
- Whether migration would change the public wire format, stored representation, normalization, error behavior, or compatibility. Those changes require their own approval and tests.

### Rust

The existing example is `crates/ace-engine/src/core/common/domain_types.rs`: it uses `nutype` for `RemoteSitePackId`, `Sha256Digest`, `BundleSchemaVersion`, and `ApiKey`, with explicit validation. `ApiKey` deliberately avoids ordinary `Debug`/`Display` implementations so secrets are harder to log. Use existing patterns instead of adding a competing validation framework.

- Keep constructors/private fields controlled. Parse or `TryFrom` untrusted values **once at the owning boundary**, then pass validated types internally.
- Reuse established types such as `url::Url`, `Uuid`, `chrono` types, and existing project IDs **where appropriate**. A web URL does not automatically need a custom wrapper around a string; a `SitePackSourceUrl` might if it has stricter product rules.
- Never weaken a valid domain type to a raw primitive for convenience; likewise avoid redundant wrappers around values whose invariants already live in a suitable type.
- Model meaningful states with enums and associated data when this prevents invalid combinations; avoid booleans and magic integers with ambiguous meaning.
- Validate at ingress even if the sender had a TypeScript type. Rust's authorization and business rules remain authoritative.
- Treat (de)serialization as a trust boundary: prove invalid data is rejected and that the on-wire/DB representation stays compatible. A typed field is not proof of secure deserialization or authorization.

### Svelte/TypeScript

- Static TypeScript types and `as` casts are **not runtime validation**. Use existing Zod patterns at genuinely untrusted boundaries where runtime validation is required; derive TypeScript types from the validated schema when appropriate.
- A branded TypeScript type distinguishes identities at compile time but does not validate raw input automatically. Constructors/parsers must enforce invariants when data first enters the trusted part of the app.
- Do not validate every prop, local variable or already-validated in-process value with Zod. Avoid duplicate schemas that can drift away from the existing Rust/remoc contracts.
- Keep UI-editing strings as strings while incomplete (for example, a URL being typed). Convert to the domain type only at submit/commit, producing a visible validation error without changing the meaning of partially entered input.
- Keep domain types close to their owning feature. A shared types folder is not automatically the right place for every new type.

## Practical design examples

| Symptom | Prefer | Avoid |
| --- | --- | --- |
| An arbitrary string stands for a validated external URL | A standard URL parser, or a specific validated URL type only if business invariants demand one | Regex-only parsing or an unchecked `as Url` cast |
| Two different IDs are interchangeable `string`/`Uuid` | Existing ID wrappers/newtypes scoped to the owning domain | Exposing all IDs as raw strings throughout internal code |
| Unclear schedule duration, timezone or units | Explicit duration/timezone semantics and tests around boundaries | Unlabelled `number`/`u64` or silently changing stored time interpretation |
| A secret gets logged in common debugging output | A secret-owning type whose formatting/serialization is intentionally constrained | `Debug`/`Display` of credential values |
| Similar fields in two unrelated subsystems | Independent types until a **shared meaning** is established | One generic object with optional fields and branching behavior |

## Review / regression requirements

Check both valid and invalid inputs, normalization, round-trip serialization, persisted historical values, error compatibility, and authorization/secret exposure. For any proposal to change a type used in remoc, Tauri, DB migrations, or the external API, enumerate consumers and require an explicitly approved contract change rather than folding it into a behavior-preserving refactor.

The review question is: **what real class of mistakes becomes impossible, at what concrete cost?** Reject the new type if the answer is only "stronger typing looks cleaner."
