# Desktop CI and release candidates

ACE's unified desktop repository is `ace-blocker`.
This workflow does not include the website, Android app, or host-list pipeline.
Those projects have separate release policies.

## Continuous integration

`.github/workflows/ci.yml` runs on pushes to `main`, pull requests, and
manual dispatch:

- Bun locked install, Svelte/TypeScript checks, Vitest unit tests, and frontend build.
- Rust workspace compilation (including Tauri), service release checks,
  engine remoc RPC and schedule tests, and watchdog binary/protocol tests on
  Windows x64 and Linux x64. The full engine test suite also contains
  resource-intensive site-pack integration tests; run it separately before
  public releases rather than saturating every pull-request runner.

Linux runners install the native dependencies for Tauri 2 and WebKitGTK.
The pipeline does not include macOS.

Dependabot checks the root Cargo workspace, `apps/ace-app/bun.lock`, and
GitHub Actions weekly. Dependency updates remain pull requests subject to CI;
they are not auto-merged.

## Building release candidates

Run **Desktop release candidates (Windows and Linux)** manually from `main`.
Specify a three-part `version`, such as `0.3.0`. The workflow now rejects
versions not already aligned in the engine, watchdog, release verifier,
Tauri Rust/config and Svelte package manifests; the MSI packaging version
alone is not authoritative.

After **both** builds pass, the workflow checks artifact hashes and commit provenance.
It then creates or updates a GitHub **draft prerelease** named `v<version>`.
It never publishes the release automatically.

- **Windows x64:** `AceBlocker-<version>.msi` using WiX 4, plus the
  offline Burn bootstrapper `AceBlocker-Setup-<version>.exe` with custom icon,
  and one `release-candidate-windows.json` provenance manifest containing both
  hashes. The EXE embeds the original MSI and does not replace its lock-aware
  privileged custom actions.
- **Linux x64:** `AceBlocker-<version>-linux-x64.tar.gz`, plus
  `release-candidate-linux.json`. The archive contains the Tauri executable,
  engine, watchdog, URL callback helper, certificate tool, offline site packs,
  and this document.
- **Both:** `SHA256SUMS` covers the Windows MSI, branded Setup EXE, and Linux tarball.

The draft-assembly job also runs `installer/verify_release_candidates.py`.
It refuses to create a draft if a required platform file is absent, if the
version/commit/hash provenance differs, if the MSI header is not a valid
compound-file header, if the Setup EXE does not have an x64 PE header, or if the Linux archive is structurally unsafe or
missing required executables, scripts or bundled content. This gate checks release **candidates**.
It does not prove Authenticode signing or installation safety. Its offline regression tests run in ordinary desktop CI.

**GitHub Actions downloads each workflow artifact as a ZIP**, but the Linux
deliverable *inside* that ZIP is still `.tar.gz`. Keep the Linux tarball. Tar and gzip are more widely available than `unzip`.
Upload the inner tarball to the website, not GitHub's outer ZIP.

The individual GitHub Actions artifacts are retained for 14 days; the draft
release keeps a copy of all three binaries and the manifests. Neither workflow
builds or ships a macOS executable.

The optional `stage_on_site` switch works **only on Windows**.
It requires a valid signing certificate pinned by `ACE_RELEASE_PUBLISHER_CERT_SHA256`.
It sends the MSI to the website's staged ingestion endpoint. The workflow
cannot sign it by itself: configure an approved signing service first.
See `docs/PRODUCT_UPDATES.md` for the independently signed release envelope,
public build keys, updater status and remaining privileged installer gates. It requires the repository variable
`ACE_RELEASE_INGEST_URL` and secret `ACE_RELEASE_INGEST_TOKEN`; it does
not publish the staged MSI.

## Linux installation candidate — manual test systems only

The current Linux distribution is a **tarball**.
It is not a .deb, .rpm, AppImage, or unattended installer. Its archive is built on Ubuntu 24.04 for x86-64,
so other distributions and older glibc versions are not guaranteed compatible.
The binaries depend on the system's WebKitGTK 4.1 and related desktop libraries.

On a **fresh disposable** Linux system with systemd, review the certificate
and network privileges, unpack the release tarball and run its fresh-install
helper as root:

```sh
tar -xzf AceBlocker-VERSION-linux-x64.tar.gz
cd AceBlocker-VERSION-linux-x64
sudo ./install.sh
systemctl status ace-engine.service ace-watchdog.service
```

The helper installs all binaries and bundled packs under root-owned
`/opt/ace-blocker`, registers both systemd units, and installs the desktop
launcher/icon under `/usr/local/share`. It creates a dedicated `ace-blocker`
local group with group-readable (not world-readable) capability credentials in
the root-owned `/usr/local/share/AceBlocker` directory, and enrolls the user
identified by `SUDO_USER`. That user must log out and sign in again before the App can authenticate.
If `SUDO_USER` is absent, explicitly add the intended desktop account to the group. The installer also installs the allowlisted
`ace-redirect` desktop callback handler and attempts to make it the default
`aceblocker://` handler for `SUDO_USER`. Verify in the *desktop user's*
session with `xdg-mime query default x-scheme-handler/aceblocker` (expected:
`ace-blocker-handler.desktop`). If the installer could not set the association,
run `xdg-mime default ace-blocker-handler.desktop x-scheme-handler/aceblocker`
as that user. Never test a live callback using an invented API key on a machine
with real account data. Previously exposed credentials on older test
installations require rotation during authorized maintenance.
It **refuses existing installations**:
never overwrite live or locked engine/watchdog binaries. Upgrades/removal
must use authenticated engine shutdown and watchdog maintenance.

After a Linux kernel upgrade, `/lib/modules/$(uname -r)` may not match the
running kernel until reboot. `/dev/net/tun` can then exist but be unusable.
Verify an `ace-tun started` log entry; `systemctl is-active ace-engine`
alone does not prove traffic interception.

### Migrating pre-release Linux data

Some early test installations stored the active Engine database under the
first `XDG_DATA_DIRS` entry, which can be
`/var/lib/flatpak/exports/share/AceBlocker` rather than the new fixed
`/usr/local/share/AceBlocker`. Those locations may contain **different**
databases and tokens. Never upgrade by replacing the binaries alone.

Determine the live data path from the Engine journal. Before changing binaries,
obtain a successful authenticated, lock-aware shutdown and stop the Watchdog
within its confirmed maintenance lease. **If either protected service remains active or maintenance fails, stop.
Do not force-stop either service.**

With both services inactive, run the included helper, specifying the actual
old data path (the path below is an example):

```sh
sudo ./migrate-legacy-data.sh --from /var/lib/flatpak/exports/share/AceBlocker
```

The helper uses the **old XDG-aware Engine binary** to check both databases
for locks, rejects symlinks/running services, and atomically stages the copied
data. It keeps the legacy source and a backup of any previous canonical data.
It also restricts access to the database and private CA key. Run it before
replacing the Engine executable. Then complete the authorized upgrade and
start Watchdog and Engine. Do not run `install.sh` on existing installations.

## Before publishing any draft release

A successful build does **not** prove release safety.
Complete at least these checks:

1. Windows clean MSI install/upgrade/uninstall on a disposable VM, service
   permissions, named-pipe ACLs, capability-token and certificate ACLs across
   different local user accounts, watchdog crash recovery, lock-aware
   shutdown, Wintun and proxy operation.
2. Linux systemd service lifecycle, root-only Unix sockets, watchdog recovery,
   lock-aware shutdown/uninstall, certificate trust, DNS/TUN/proxy behavior,
   and user-session app connectivity on supported distributions.
3. Software signing, package authenticity, and the unresolved pre-release
   security review findings in `docs/SECURITY_AUDIT_2026-10-26.md` and
   `docs/SECURITY_AUDIT_FOLLOWUP_2026-10-26.md`.

Do **not** publish the draft to users based only on passing CI. Native
Windows/Linux integration and security tests are separate release gates.

## Signed Pro entitlements in development and release

Both development and release engines request the entitlement and its Ed25519
**public** verification key from the authenticated Ace server endpoint
`GET /api/v1/account/entitlement`. Production uses HTTPS and development uses
loopback HTTP. No entitlement key is required at compile time and
`crates/ace-engine/build.rs` does not read any server secrets or local
website checkout. The server-only signing private key never reaches the client.

The server configures `ACE_ENTITLEMENT_SIGNING_KEY_BASE64` (private PKCS#8)
and can optionally check the matching `ACE_ENTITLEMENT_PUBLIC_KEY_BASE64`.
The engine verifies each signed lease before granting Pro and stores the
public key alongside that lease in its service-owned SQLite database for
offline validation after restart. The public key is authenticated by HTTPS
at receipt time in production; local development trusts the loopback server.
A compromised server connection at the moment of first acquisition would
change the trust model compared with a compile-time pin; never use plaintext
remote HTTP in production.

Generate the server key pair using the website utility
`bun scripts/generate-entitlement-keys.ts <secure-directory-outside-repo>`.
Rotation is handled by the next authenticated entitlement response; no desktop
rebuild or release variable is necessary.

Pro leases expire after at most 24 hours, with another 72 hours allowed for
intermittent offline use. Previously active blocking protections remain enabled
when a subscription expires; new Pro operations need valid entitlements.

DodoPayments stays in **test_mode** for release builds.
Changing the billing environment requires an explicit product decision.
