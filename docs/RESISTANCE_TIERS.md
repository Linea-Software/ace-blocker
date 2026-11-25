# Resistance Free / Pro contract

Ace Blocker is pre-release; only the current contract is supported.

## Canonical ownership

- **Boredom Engine** owns script IDs and the tier designation. A source JSDoc
  `@tier free` makes that intervention free; every script without that tag is
  Pro. The build emits the required boolean `requiresPro` in
  `scripts-data.json` (catalog schemaVersion **2**).
- **Ace Blocker Site** distributes the same immutable, SHA-256-verified ZIP to
  anonymous and signed-in clients. Publishing is still controlled by the
  release admin. The ZIP is not an access-control boundary.
- **Ace Engine** is the only enforcement authority: it filters script
  candidates before interception and HTML injection, prevents enabling
  unauthorized Pro scripts through remoc, and preserves saved selections when
  the subscription changes. The master switch and the three Free scripts work
  without Pro; Resistance Time Lock remains Pro only.
- **Ace App** renders `ScriptInfo.requiresPro` from remoc and does not
  maintain a second allowlist or control engine entitlements. All scripts
  remain visible; Pro scripts display upgrade controls for Free accounts.

The three Free IDs are `youtube_shorts_remove`,
`reddit_block_recomandations` (existing catalog spelling), and `tiktok_desaturate`.

## Deployment order and verification

1. Release Boredom Engine schemaVersion 2 artifact with a new version.
2. Deploy the public Ace Server Resistance download route (and its OpenAPI
   contract); import and mark the new artifact downloadable.
3. Release Ace Engine with schema 2 support and eligibility filtering.
4. Release the desktop app consuming the new `requiresPro` remoc field.

Do not ship the new engine against an older catalog version. No legacy
catalog/backup migration is needed before the public release.

Required checks: Boredom Engine `bun run check`, `bun run build`,
`bun run test:artifact`, direct-injection smoke test; site API tests and
`bun run check`; Ace Engine Free/Pro, downgrade, Time Lock, checksum and
proxy injection tests; Ace App Playwright Free/Pro checks. A real proxy test
with the installed version is still required before shipping.
