# Desktop CI and release candidates

ACE's unified desktop repository is `ace-blocker`. The website, Android app,
and host-list pipeline have their own release policies and are not part of this
workflow.

## Continuous integration

`.github/workflows/ci.yml` runs on pushes to `main`, pull requests, and
manual dispatch:

- Bun locked install, Svelte/TypeScript checks, Vitest unit tests, and frontend build.
- Rust workspace compilation (including Tauri), service release checks,
  engine remoc RPC and schedule tests, and watchdog binary/protocol tests on
  Windows x64 and Linux x64. The full engine test suite also contains
  resource-intensive site-pack integration tests; run it separately before
  public releases rather than saturating every pull-request runner.

The Linux runners install the native dependencies required by Tauri 2/WebKitGTK.
The pipeline intentionally has no macOS job.

Dependabot checks the root Cargo workspace, `apps/ace-app/bun.lock`, and
GitHub Actions weekly. Dependency updates remain pull requests subject to CI;
they are not auto-merged.

## Building release candidates

Manually run **Desktop release candidates (Windows and Linux)** from
`main` with a three-part `version` (e.g. `0.3.0`).

After **both** builds succeed, the workflow checks the artifact hashes and
commit provenance and creates (or updates) a **draft prerelease** on GitHub
named `v<version>`. It never publishes that release automatically.

- **Windows x64:** `AceBlocker-<version>.msi` using WiX 4, plus a
  `release-candidate-windows.json` provenance manifest.
- **Linux x64:** `AceBlocker-<version>-linux-x64.tar.gz`, plus
  `release-candidate-linux.json`. The archive contains the Tauri executable,
  engine, watchdog, URL callback helper, certificate tool, offline site packs,
  and this document.
- **Both:** `SHA256SUMS` covers the MSI and Linux tarball.

The individual GitHub Actions artifacts are retained for 14 days; the draft
release keeps a copy of both assets and the manifests. Neither workflow
builds or ships a macOS executable.

The optional `stage_on_site` switch is **Windows only** and sends the MSI
to the website's staged ingestion endpoint. It requires the repository variable
`ACE_RELEASE_INGEST_URL` and secret `ACE_RELEASE_INGEST_TOKEN`; it does
not publish the staged MSI.

## Linux installation candidate — manual test systems only

Linux is currently distributed as a **tarball**, not a .deb, .rpm, AppImage,
or an unattended installer. Its archive is built on Ubuntu 24.04 for x86-64,
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
identified by `SUDO_USER`. That user must log out and back in before the App
can authenticate; without `SUDO_USER`, enroll the intended desktop account
in that group explicitly. Previously exposed credentials on older test
installations require rotation during authorized maintenance.
It **refuses existing installations**:
never overwrite live or locked engine/watchdog binaries. Upgrades/removal
must use authenticated engine shutdown and watchdog maintenance.

After a Linux kernel upgrade, `/lib/modules/$(uname -r)` may not match the
running kernel until reboot. `/dev/net/tun` can then exist but be unusable.
Verify an `ace-tun started` log entry; `systemctl is-active ace-engine`
alone does not prove traffic interception.

## Before publishing any draft release

A successful build is **not** proof of release safety. Verify at least:

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

Do **not** publish the draft to end users solely because CI passes. Native
Windows/Linux integration and security tests are separate release gates.
