# Ace Engine — approved pre-release refactoring decisions

**Approved:** 2026-10-22. **Scope:** `crates/ace-engine/` and directly necessary engine integration boundaries.

## Product stage and compatibility policy

Ace Blocker **has not been released publicly**. The project does **not** need backward compatibility with old program data, internal releases, site-pack/version formats, databases, backups, IPC payloads, or local protocols. You can replace or remove old formats and code that exists only to support them. Validate new canonical formats when data enters the system. Do not keep old conversion branches without a current need.

These decisions do **not** permit weaker security or lock enforcement. Keep authenticated maintenance, independent engine and watchdog services, installer privileges, browser-only interception, and correct current behavior. Do not delete unrelated current development data or active policies without notice. Document persisted-format changes. Give disposable pre-release environments a clear reset procedure. Regenerate external Ace Server clients from their source contract. Do not edit generated clients by hand. Keep the typed remoc/Named Pipe architecture unless a separate explicit decision approves a change.

## Approved semantics (binding for the refactoring)

| Audit item | Decision | Required behavior |
| --- | --- | --- |
| AE-01 | B — availability | If `NetworkManager` cannot load after existing recovery, continue with an empty manager. Show a clear error or warning. This is an intentional **fail-open** startup policy, not a successful load. Do not invent a traffic-blocking fallback. |
| AE-03 | A — validated path components | Reject invalid identifiers and identifiers that allow path traversal at input and filesystem boundaries. Do not change untrusted path components into unexpected paths. Existing valid canonical IDs do not have to be UUIDs. Do not migrate historical data. |
| AE-04 | A — unchanged enforcement | Track and log site-pack database lookup failures separately from `NotBlocked`. Do not cache lookup errors. Allow recovery. **Keep the current fail-open enforcement result** when a lookup fails. |
| AE-05 | A — per-unit atomic restore | Restore each independent block or pack as one unit. Do not change locked blocks. Report each skipped locked block. Report errors instead of applying part of a unit. The complete backup does not have to be one atomic operation. |
| AE-11 | A — last-known-good | If script activation or persistence fails, restore the last working scripts and version. Plan and test recovery after interrupted activation. |
| AE-15 / versions | B — canonical only | Use one current site-pack version format and comparison rule. Reject old version formats. Remove compatibility code and historical migrations for these formats. Document the version grammar. Update all current producers and tests. |
| AE-15 / schedules | A — strict minutes (2026-10-22) | Accept times only when seconds and fractional seconds are zero. Reject sub-minute input with a typed validation error. For example, reject `08:30:45`. Do not truncate or round. Keep midnight, wraparound, and local weekday schedule behavior. |
| AE-16 | A — strict IDs | Empty script and site-pack identifiers are **always invalid**, including in stored input that is deserialized. Do not add exceptions for historical data. Validate values at trust boundaries with a fallible operation. Do not require UUIDs for canonical remote pack IDs. |
| AE-26 | A — typed errors | A function that can fail must return `Result<T, E>` or another explicit typed error carrier. **Never use a primitive sentinel** such as zero, `false`, an empty list, or `NotBlocked` for failure. Send `Err` to the correct boundary. Only a specifically approved fail-open policy can turn that error into an enforcement decision. Keep lock and uninstall rules unchanged. |

## Implementation workflow

Make small patches that reviewers can inspect separately. First identify consumers, validation, and failure modes. Then make the smallest correct change. Add failure, recovery, and security regression tests. Run applicable cargo, test, and VM checks. Record completed and pending work in `ACE_ENGINE_REFACTOR_AUDIT.md`. Use conventional English commit messages. These approvals do not prove that implementation is complete.

AE-13, AE-14, AE-17, AE-18, and AE-19 are delegated engineering decisions. Keep single-flight coordination, typed RPC contracts, stream and progress behavior, independent persistence domains, and test isolation. Do not add speculative abstractions.
