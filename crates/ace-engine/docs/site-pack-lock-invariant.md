# Site-pack lock invariant (canonical rule)

This document defines the engine's canonical site-pack lock rule.
`NetworkManager::is_site_pack_locked` implements the rule in
`ace-engine/src/manager/network/site_pack.rs`.
The engine enforces this rule for every operation that could delete or weaken a site pack.
Do not duplicate the predicate.

## The rule

A site pack `P` is **locked** at time `t` only when all these conditions apply:

1. `P` exists locally (a row is loaded in memory), **and**
2. `P` is synchronized (`is_synced()` is true — its `pack.db` is loaded; a
   metadata-only row enforces nothing), **and**
3. at least one network block that references `P` is itself **locked at `t`**
   (per the block lock semantics in `core/common/lock.rs` — `ActiveUntilLockEnd`
   is active while `now <= end`; `ActiveDuringSchedule` is active only while the
   block's schedule matches).

All other packs are *unlocked*. This includes missing packs, unsynced packs, and packs referenced only by unlocked blocks.

## Why the `is_synced` conjunct

An unsynced remote pack has no data to protect, for example after a cache clear.
Keep this pack downloadable. Otherwise, the lock would reject `resync`.
The pack could not receive data, even when a locked block already references it.
Locking an unsynced pack would therefore *weaken* enforcement.

## Enforced operations

| Operation | Guard result when locked |
|---|---|
| `delete_site_pack` (manager + HTTP `DELETE /site-lists/{id}`) | `SitePackIsLocked` → HTTP 423, code `locked` |
| `resync_site_pack` (download / resync) | `SitePackIsLocked` → HTTP 423, code `locked` |
| `delete_site_pack_source` | `SitePackIsLocked` → HTTP 423, code `locked` |
| HTTP delete while pack is *in use* by an unlocked block | HTTP 409, code `conflict` (must disconnect from the block first) |
| Delete/resync/source-delete while a sync stream is running | `SitePackBusy` → HTTP 409, code `conflict` |

Re-evaluate lock state under the manager write lock before each mutation.
A stale frontend cannot bypass a lock change.
Return a stable machine-readable code for each forbidden operation. Never report silent success.

## API exposure

`GET /site-lists` returns each pack's `isLocked`, evaluated at request time.
The app uses this value to disable delete and resync controls.
The engine enforces the rule independently of UI state.
