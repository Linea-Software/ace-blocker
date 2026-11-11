# Network Block activation modes (2026-11-07)

Each Network Block has two independent controls:

- `enabled`: the user's quick switch, subject to the existing Lock safeguards.
- `scheduleMode`: `always` or `scheduled`. The default for new blocks is
  `always`. When enabled, this mode enforces continuously without schedule entries.
  `scheduled` enforces only during configured civil-time windows.
  It **requires at least one window**. The editor rejects an empty Scheduled draft before writing.

A block without websites, site packs, or Safe Search matches no content.
The editor prevents users from creating this empty policy.

`saveNetworkBlock` saves the complete draft, including schedule mode, in one SQLite transaction.
The live policy changes only after COMMIT. Ordinary schedule-entry operations cannot remove the final
entry from a Scheduled policy. Mode is represented in the typed remoc schema,
dashboard snapshots, local backup exports/restores and SQLite persistence.

Lock forbids `Always -> Scheduled` because the change weakens enforcement.
Lock permits `Scheduled -> Always` because the change strengthens enforcement. Locked
Scheduled window edits must preserve all previously covered periods.
A schedule-bound lock on an Always block stays active continuously.
Changing to Always does not bypass Lock.

The pinned IANA timezone, DST rules, authenticated-time uncertainty,
independent watchdog service and privileged IPC boundary are unchanged.

## Pre-release baseline

Ace Blocker has not shipped. The initial `network_blocks` schema includes
`enabled`, `scheduleMode`, and the pinned schedule timezone.
No follow-up ALTER migration is necessary.
Starter policies created by `0021_seed_default_network_blocks` are disabled
and use `always` without redundant all-day schedule entries. They attach
only Free categories, including compact Gambling and Streaming starter packs
that are shipped in the installer bundle. No starter has custom rules or
a preselected Pro pack. Back up and recreate development databases that used removed migrations.
Do not delete or rewrite live engine data as part of this source change.

## QA

- Engine: targeted Rust tests for atomic save, Lock, effective activity,
  scheduled validation, persistence and backup.
- App: `bun run check`, `bun run test:unit`, and
  `e2e/block-editor-redesign.spec.ts` (mocked browser-only inspector,
  no privileged IPC).
- Before release: native Tauri + Windows tests of Enabled/Always/Scheduled,
  locked edits and actual filtering. Browser visual fixtures do not prove
  enforcement.
