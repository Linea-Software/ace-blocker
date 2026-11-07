# Network Block activation modes (2026-11-07)

Network Blocks have two independent controls:

- `enabled`: the user's quick switch, subject to the existing Lock safeguards.
- `scheduleMode`: `always` or `scheduled`. The default for new blocks is
  `always`, which enforces at every time when enabled without schedule entries.
  `scheduled` enforces only in explicitly configured civil-time windows and
  **requires at least one window**. An empty Scheduled draft is rejected before
  any write.

A block without websites, site packs or Safe Search has no matching content:
the editor does not let users create a misleading empty policy.

The complete editor draft (including schedule mode) is saved through
`saveNetworkBlock` in one SQLite transaction; the live policy updates only
after COMMIT. Ordinary schedule-entry operations cannot remove the final
entry from a Scheduled policy. Mode is represented in the typed remoc schema,
dashboard snapshots, local backup exports/restores and SQLite persistence.

Under Lock, `Always -> Scheduled` is forbidden because it weakens enforcement.
`Scheduled -> Always` is allowed because it strengthens enforcement. Locked
Scheduled window edits must preserve all previously covered periods.
A schedule-bound lock on an Always block is active continuously; switching
to Always therefore does not create a shortcut around the Lock.

The pinned IANA timezone, DST rules, authenticated-time uncertainty,
independent watchdog service and privileged IPC boundary are unchanged.

## Pre-release migration

Migration `0027_network_block_schedule_mode` sets existing rows with at least
one schedule entry to `scheduled`; rows without entries become `always`.
That last case changes their former ineffective scheduling behavior and
must be noted when testing existing developer installations. Backward
compatibility with older pre-release API formats is not required.

## QA

- Engine: targeted Rust tests for atomic save, Lock, effective activity,
  scheduled validation, persistence and backup.
- App: `bun run check`, `bun run test:unit`, and
  `e2e/block-editor-redesign.spec.ts` (mocked browser-only inspector,
  no privileged IPC).
- Before release: native Tauri + Windows tests of Enabled/Always/Scheduled,
  locked edits and actual filtering. Browser visual fixtures do not prove
  enforcement.
