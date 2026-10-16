# Site-pack lock invariant (canonical rule)

This is the single source of truth for the engine's site-pack lock rule. It is
implemented by `NetworkManager::is_site_pack_locked`
(`ace-engine/src/manager/network/site_pack.rs`) and enforced server-side on
every operation that could delete or weaken a site pack. Do not inline your
own copy of the predicate.

## The rule

A site pack `P` is **locked** at a point in time `t` iff:

1. `P` exists locally (a row is loaded in memory), **and**
2. `P` is synchronized (`is_synced()` is true — its `pack.db` is loaded; a
   metadata-only row enforces nothing), **and**
3. at least one network block that references `P` is itself **locked at `t`**
   (per the block lock semantics in `core/common/lock.rs` — `ActiveUntilLockEnd`
   is active while `now <= end`; `ActiveDuringSchedule` is active only while the
   block's schedule matches).

Everything else is *unlocked*: missing packs, unsynced packs, and packs that
only unlocked blocks reference.

## Why the `is_synced` conjunct

An unsynced remote pack (e.g. after a cache clear) has no data to protect and
must stay downloadable. If it were locked, the `resync` action would be refused
and the pack could never acquire data — including for a locked block that
already references it. Locking an unsynced pack would therefore *weaken*
enforcement instead of protecting it.

## Enforced operations

| Operation | Guard result when locked |
|---|---|
| `delete_site_pack` (manager + HTTP `DELETE /site-lists/{id}`) | `SitePackIsLocked` → HTTP 423, code `locked` |
| `resync_site_pack` (download / resync) | `SitePackIsLocked` → HTTP 423, code `locked` |
| `delete_site_pack_source` | `SitePackIsLocked` → HTTP 423, code `locked` |
| HTTP delete while pack is *in use* by an unlocked block | HTTP 409, code `conflict` (must disconnect from the block first) |
| Delete/resync/source-delete while a sync stream is running | `SitePackBusy` → HTTP 409, code `conflict` |

The lock state is re-evaluated under the manager write lock at mutation time,
so a stale frontend cannot race a lock change: a forbidden attempt always
receives a stable machine-readable code, never a silent success.

## API exposure

`GET /site-lists` returns each pack's `isLocked` (evaluated at request time) so
the app can disable the delete/resync controls. The engine enforces the rule
regardless of UI state.
