# Ace Blocker — full-product Windows updates (pre-release, 2026-11-06)

## Status and security boundary

**This is a bounded implementation, not a completed unattended installer.**
The following works in the current source tree (with production signing and
endpoint configuration still required):

- `ace-update-security` authenticates Ed25519 release envelopes using a
  public key pinned at compile time; compares strict stable SemVer numerically;
  validates Windows/x64, HTTPS package URL, compatibility and pinned signer
  certificate fingerprint; streams the MSI through SHA-256 verification.
- `ace-engine` discovers only **signed** product versions. It uses a separate
  six-hour single-flight cache and a Windows release-build background task with
  up to 30-minute jitter and 6/12/24-hour error backoff. Discovery finishes
  before a multi-minute download begins, and the cache exposes separate
  checking/downloading/verifying/completed/error states. It never consults the
  content updater's unsigned installer hint as download authority and does not
  share the TrustedClock polling/lock-manager path.
- When all build-time settings and a signed feed exist, a Windows service-mode
  engine downloads a signed-release MSI into
  `%ProgramData%\LineaSoftware\AceBlocker\updates`. It downloads without
  stopping filters, bounds the transfer, rejects staging-path reparse points,
  writes to an exclusive `.partial` cache file, syncs to disk, verifies the
  signed SHA-256, renames into the cache, checks Authenticode with the
  Windows trust provider and the exact pinned publisher certificate, then
  rehashes. A crash-interrupted partial file is discarded on next attempt.
- The app's Settings section exposes the installed version and authenticated
  discovery/download/verification status, polling during active transfers.
  It **does not** offer an unsafe Install button.
- WiX 4 remains the sole production package owner; the signing-verifier script
  is included in the MSI. Normal major upgrades no longer delete root CAs
  through the two explicit certificate-removal custom actions.
- The release workflow validates that Rust engine/watchdog/verifier, Tauri Rust,
  Tauri config and Svelte package versions all equal the supplied release
  version. Only a correctly Authenticode-signed and pinned MSI may be staged
  on the website. Unapproved releases remain GitHub drafts.

**Not implemented or not enabled:** stopping the services for installation,
one-time authorization of a locked upgrade, privileged MSI execution, durable
transaction journal, cross-process install mutex, database snapshot and
post-install enforcement health checks/recovery. The
`ace-update-security::UpdateCoordinator` models phases and single-flight in
memory, but is **not** yet a privileged authorization capability or reboot-
durable journal. No automatic MSI execution happens in the current build.
Do not publish or advertise the product as auto-installing at this stage.

No previously released product or legacy migration path exists. There is no
pre-release version-bridge requirement. This does **not** allow a first public
release to permit unsigned or arbitrary lock-breaking upgrades.

## Exact release manifest contract

A trusted HTTPS Ace Blocker release endpoint must serve an envelope with
these string fields. The inspected website checkout currently exposes
`GET /api/v1/releases/latest` in
`ace-blocker-site/src/lib/server/api/routers/releases.ts`, but that is
**unsigned installer-version metadata**, not this contract. No signed feed
endpoint was found or configured in the checked-out desktop source, and no
website repository was modified in this work:

```json
{
  "payload": "<base64 of exact UTF-8 JSON payload>",
  "signature": "<base64 64-byte detached Ed25519 signature>",
  "key_id": "ace-stable-v1"
}
```

The signature is Ed25519 over the exact bytes

```text
ASCII("ace-blocker-windows-msi-release-v1") || NUL || payload_json_utf8
```

The base64 payload is JSON with camelCase property names:

```json
{
  "schemaVersion": 1,
  "version": "0.2.0",
  "channel": "stable",
  "os": "windows",
  "architecture": "x64",
  "packageUrl": "https://aceblocker.com/approved/actual-package.msi",
  "sha256": "<64 lowercase hex digits of signed MSI>",
  "publisherCertificateSha256": "<64 hex digits of DER signer certificate SHA-256>",
  "minimumInstalledVersion": null
}
```

**Example only:** no publication path above is guaranteed to exist.
The envelope's `key_id` selects the *embedded* trust key, not a network-
supplied key; only `ace-stable-v1` is accepted. Unknown fields, unknown
channels, prereleases, incorrect OS/architecture, insecure URLs, absent/
invalid signatures, wrong hashes, wrong pinned publisher identities, and
downgrades are all rejected.

A replay of a *previously* authorized release can be prevented relative to
the installed version and a caller-supplied version floor. A privileged,
durable floor is required before install is enabled to block a revoked
version on devices that skipped newer releases. The current engine uses
the installed version as the floor: it **does not** persist a revocation
floor across reinstalls. Revocation/rotation policy must be designed before
unattended publication.

## Configuration, signing and publication

Only **public** trust material is embedded into Windows builds:

- GitHub Actions variable `ACE_RELEASE_MANIFEST_URL` — deployed HTTPS
  manifest endpoint at `aceblocker.com` or `www.aceblocker.com`.
  This code does not invent or deploy that endpoint.
- `ACE_RELEASE_MANIFEST_PUBLIC_KEY_HEX` — 32-byte Ed25519 public key
  encoded as 64 hex digits, bound to `ace-stable-v1`.
- `ACE_RELEASE_PUBLISHER_CERT_SHA256` — SHA-256 fingerprint of the
  **exact code-signing certificate** that signs the MSI.

If any are missing or invalid, discovery/install stays disabled. A publisher
certificate rotation or Ed25519 key rotation needs an explicitly reviewed
trust-root transition. The release pipeline does not obtain or store an
Ed25519 private key or code-signing PFX.

A production signing service/HSM must Authenticode-sign the final MSI before
any checksum/manifest is computed. At a trusted Windows signing stage, run:

```powershell
.\installer\verify-product-signature.ps1 -MsiPath "AceBlocker-VERSION.msi" -ExpectedPublisherCertSha256 "APPROVED_CERT_SHA256"
```

The workflow does **not currently** contain a configured external signing
service. Therefore `stage_on_site=true` fails safely for unsigned
candidates; candidates can still be built as drafts for private testing.
The staging ingestion variables remain `ACE_RELEASE_INGEST_URL` and
`ACE_RELEASE_INGEST_TOKEN`, and ingestion is not publication.

After an approved, signed MSI is hosted at its final HTTPS URL, prepare the
payload **on a trusted release host**, for example:

```sh
python3 installer/release_manifest.py prepare \
  --msi AceBlocker-VERSION.msi --version VERSION \
  --package-url https://aceblocker.com/actual-approved-url.msi \
  --publisher-cert-sha256 APPROVED_CERT_SHA256 --output-dir out
# Sign out/release-signing-input.bin with an offline Ed25519 private key/HSM.
# Never copy the private key to this repository or to GitHub Actions.
python3 installer/release_manifest.py assemble \
  --payload out/release-payload.json --signature out/signature.bin \
  --key-id ace-stable-v1 --output out/release-envelope.json
```

The `assemble` command checks the encoding/shape, **not the cryptographic
validity** of an externally supplied signature. Independently verify the
signature against the approved public key and the signed MSI's Authenticode
identity before manually publishing the envelope on the website. An
authenticated manifest is necessary but not enough for automatic
installation.

## MSI / lock policy and limitations

The existing MSI requires `CA_CheckLocks` for `REMOVE`, including the removal
half of `RemoveExistingProducts`, and uses the lock-aware authenticated
shutdown request. An active lock therefore still refuses the major upgrade.
**Do not** simply skip `CA_CheckLocks` on `UPGRADINGPRODUCTCODE`: a third-
party MSI could provide the same UpgradeCode and abuse that MSI property.

Before enabling automatic install, introduce a separately vetted, minimal
LocalSystem installer orchestrator. It must re-open and re-verify a
non-replaceable package, acquire a machine-wide single-flight lock, bind a
short-lived one-shot maintenance authorization to the **specific MSI digest,
verified signer, transaction ID and installer process**, and preserve locked
policies. Watchdog maintenance must not depend on a UI-controlled shutdown
flag; it must expire/recover automatically. The MSI removal gate must require
that authorization for the exact upgrade transaction while keeping ordinary
uninstall blocked when locked. This design must be reviewed against Windows
Installer's nested `RemoveExistingProducts` session and PID model.

Back up the persisted SQLite database and CA keys without changing their ACLs;
plan migrations/rollback before any binary replacement. After restart, check
actual Wintun/WFP + proxy enforcement and both engine/watchdog liveness, not
only SCM `RUNNING`. On unrepairable post-install failure, keep the journal at
`RecoveryRequired`; do not claim success. Do not promise binary rollback
after irreversible data migrations.

The staged MSI directory is **cache only**. Hash/Authenticode success there is
not permission to execute it. An installer must re-verify after securely
opening the file immediately before execution; ordinary-user path writes,
file replacement and reparse-point races must be independently assessed on
Windows. Linux product auto-upgrades are not implemented in this Windows MSI
workstream.

## Automated checks run outside Windows

- `cargo test -p ace-update-security`: good/bad signed metadata, tampering,
  downgrade/floor, URL/publisher/compatibility, hash mismatch, stream failure,
  state transitions and single-flight.
- `python3 -m unittest discover -s tests -p 'test_release_manifest.py'`:
  signing-input format, payload SHA, malformed inputs, static MSI lock and
  certificate-removal conditions. **These are not Authenticode/SCM tests.**
- `cargo check --workspace --all-targets --offline` and
  `cargo check -p ace-engine --bin ace-engine --offline` (Linux),
  `cargo check --target x86_64-pc-windows-msvc -p ace-update-security`
  (cross-target pure Rust crate),
  `cd apps/ace-app && bun run check && bun run test:unit && bun run build`.
  All passed in this implementation session.
- `cargo test -p ace-engine --lib remoc_tests`: 7 passed;
  `cargo test -p ace-watchdog-protocol`: 4 passed.
- The whole-tree `cargo fmt --all --check` finds formatting differences
  in unrelated existing source; the changed Rust files passed a targeted
  `rustfmt --check --config skip_children=true`.
- Engine cross-compilation for MSVC was attempted but stopped at host C
  dependencies (`libsqlite3-sys`/`aws-lc-sys`, absent MSVC `lib.exe`).
  Native Windows MSI/SCM/Authenticode paths were **not** executed.
  This must not be reported as successful Windows compilation.

## Future Windows validation checklist — NOT executed in this task

- [ ] Clean install including service ACLs, MSI signature, UI, no local REST.
- [ ] Normal upgrade and persistence of all settings, root CAs, cert private
      keys, site packs, database, active enforcement and watchdog.
- [ ] Authentic locked upgrade **only after the grant system is implemented**.
- [ ] Locked ordinary uninstall and attempted unauthorized upgrade stay denied.
- [ ] Version rollback/replay and arbitrary/downgrade MSI are denied.
- [ ] Invalid/unsigned/incorrect-publisher MSI, replayed grant and grant
      expiration are denied.
- [ ] Kill updater/installer at each phase; resume/recover without a permanent
      watchdog maintenance gap.
- [ ] Reboot during install, restart both services, re-check lock expiry.
- [ ] Validate ACLs from standard and administrator accounts, cache symlinks,
      package-swap race and certificate preservation.
- [ ] Confirm Wintun, proxy, browser-only MITM, network rules and actual
      blocking after success and after recovery.
- [ ] Engine/watchdog recovery, concurrent requests, disk full, invalid DB
      migration, download interruption and clock jumps.

No Windows VM or interactive Windows test is performed here.
