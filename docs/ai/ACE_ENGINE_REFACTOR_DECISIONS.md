# Ace Engine — approved pre-release refactoring decisions

**Approved:** 2026-10-22. **Scope:** `crates/ace-engine/` and directly necessary engine integration boundaries.

## Product stage and compatibility policy

Ace Blocker **has not been released publicly**. **No backward compatibility with legacy program data, previous internal releases, old site-pack/version formats, historical databases, old backups, prior IPC payload shapes or obsolete local protocols is required at this stage.** It is explicitly acceptable to simplify, replace or remove historical data formats and the code paths that existed solely to support them. New canonical formats should be validated at ingress rather than carrying legacy conversion branches indefinitely.

This is **not** a license to weaken security, lock enforcement, authenticated maintenance, engine/watchdog independence, installer privileges, browser-only interception or correct behavior in the current build. Do not silently destroy unrelated current development data or existing active policies during refactoring. Document changes to persisted formats and provide a clear reset path for disposable pre-release environments. Generated external Ace Server clients must still be regenerated from their source contract rather than patched manually. Preserve current production-facing typed remoc/Named Pipe architecture, unless a separate explicit decision authorizes a change.

## Approved semantics (binding for the refactoring)

| Audit item | Decision | Required behavior |
| --- | --- | --- |
| AE-01 | B — availability | If `NetworkManager` cannot be loaded after existing recovery, continue with an empty manager and a conspicuous error/warning. This is an intentional, explicit **fail-open** startup policy, not a successful policy load. No invented traffic-blocking fallback. |
| AE-03 | A — validated path components | Reject invalid/traversal-capable identifiers at entry and filesystem boundaries; never normalize untrusted path components into surprising paths. Existing legitimate canonical IDs need not be UUIDs. No historical-data migration. |
| AE-04 | A — unchanged enforcement | Represent and log site-pack database lookup failure distinctly from `NotBlocked` internally, avoid caching errors, allow recovery, but **retain the current fail-open enforcement result** for a failed lookup. |
| AE-05 | A — per-unit atomic restore | A restorable independent block/pack must not be half-restored; locked blocks remain untouched and skipped explicitly. Report errors instead of silently applying a partial unit. No requirement that an entire backup be globally atomic. |
| AE-11 | A — last-known-good | On failed script bundle activation/persistence, restore the previously working scripts and version; plan/test recovery after interrupted activation. |
| AE-15 / versions | B — canonical only | Standardize current site-pack version representation/comparison; reject deprecated version formats, remove compatibility code and historical-data migration for these formats. Document chosen grammar and update all current producers/tests. |
| AE-15 / schedules | A — minutes | Keep the user-facing minute-level schedule semantics, including midnight/wrap boundary cases; do not silently introduce second-level behavior. |
| AE-16 | A — strict IDs | Empty script/site-pack identifiers are **always invalid**, including deserialization of persisted input. No exceptions for historical data. Choose appropriate fallible validation at trust boundaries; do not require UUID for canonical remote pack IDs. |
| AE-26 | A — typed errors | Functions which can fail must return `Result<T, E>` or another explicit typed error carrier; **never encode failure as a primitive sentinel** (such as zero, `false`, an empty list, or `NotBlocked`). Propagate `Err` to the appropriate boundary, where a specifically approved fail-open policy may translate it into an enforcement decision. Keep lock/uninstall safety rules unchanged. |

## Implementation workflow

Proceed in independently reviewable, scoped patches. First characterize current consumers, existing validation and failure modes, then implement minimal correct changes, add failure/recovery and security regression tests, run relevant cargo/test/VM gates, record completed versus pending work in `ACE_ENGINE_REFACTOR_AUDIT.md`, and use conventional English commit messages. Do not treat the decisions above as evidence that their implementations have already been completed.

AE-13, AE-14, AE-17, AE-18 and AE-19 are delegated engineering decisions: preserve single-flight coordination, typed RPC contracts, stream/progress semantics, independent persistence domains and test isolation without speculative abstraction.
