LLMs should edit this file only after previous explicit consent from the user.

### Introduction

This is a set of things to verify when testing Ace Blocker. For each of the failed tests, the failure should be reported along with the evidence and pertinent logs.

# Testing

## How should you test Ace Blocker

1. **Create a VM compatibility matrix:** verify at least Windows 11, CachyOS, Ubuntu and Arch in separate disposable VMs. Record OS/build, kernel, browser versions, desktop environment, Ace package version, installation method and whether each tested feature is officially supported on that target. Windows MSI and Linux systemd integration have different contracts.
2. **Prepare reproducible snapshots:** keep a clean pre-install baseline, a freshly installed baseline, an enabled Always-block baseline, a Scheduled-block baseline and a Lock-active baseline. Restore the correct snapshot before each destructive test so one failure does not contaminate the next case.
3. **Use NovaGPT VM support where available:** install the actual release-candidate artifacts, interact with the guest UI, and collect screenshots, redacted service logs, network evidence and relevant Windows Event Log/systemd journal entries. If VM control is unavailable, mark native tests BLOCKED instead of inferring success from source code or mocks.
4. **Verify actual outcomes:** use one controlled blocked domain and one allowed domain. A successful process startup, running service, UI state, or passing API call is not proof of active filtering. For blocking and Resistance checks, observe browser traffic/content as well as engine state.
5. **Exercise different privileges and environments:** use a standard account for normal app behavior and a separate administrator/root account only where fault injection or installation requires elevation. Test connected and offline states; use disposable synthetic accounts and credentials.
6. **Automate safe repetitive cases:** write deterministic scripts for checks that can be repeated, such as service status, rule persistence, import parser fixtures, error/permission matrices and latency measurements. Scripts must have bounded timeouts, clean up after themselves and avoid weakening protections on the developer workstation.
7. **Capture traceable evidence:** store test ID, Git commit, package hashes, VM/snapshot ID, prerequisites, reproduction steps, expected/observed behavior, duration, screenshots, sanitized logs and a linked issue when a test fails. Exclude tokens, private keys and user data.
8. **Report findings without overstating them:** use PASS, FAIL, LIMITATION, BLOCKED or NOT RUN. Distinguish a code-level/unit test from installed-VM evidence. A P0 bypass that weakens active network enforcement or a Lock is a release blocker until triaged.

## What should we test

The original eleven verification goals are expanded into independent test suites below. Each test has an ID, a priority, a reproducible scenario and a measurable expected outcome. Use **P0** for blocking/release-critical failures, **P1** for important regression coverage and **P2** for compatibility or extended coverage. An unexecuted test is **NOT RUN**, not PASS.

### Shared test requirements

- Test released/release-candidate binaries installed through the supported packaging path. Record OS/browser versions, installer and binary hashes, Git commit, privilege level, VM snapshot and exact installed configuration. Debug builds are additional test targets, not substitutes for release verification.
- Maintain an explicitly enabled policy for one controlled blocked domain and a different allowed domain; verify real browser/network behavior before and after every operation. Use synthetic accounts, lists, tokens, clocks and local HTTP fixtures where possible.
- Run a baseline with no Lock, then repeat applicable security cases with a Network Block Hard/Time Lock and a Resistance Mode Lock. Separate expected denials from missing functionality or documented administrator/root limitations.
- Record **expected**, **observed**, evidence, recovery duration and PASS/FAIL/LIMITATION/NOT RUN. Do not treat a UI message, `Running` service, or successful compilation as proof of actual enforcement. See the detailed evidence template in the anti-tampering suite.
- Keep destructive installer, clock, storage, and service tests on disposable VMs. Revert snapshots after failures and never use production secrets, paid checkout, or real user data.

### 1. Installation and first-run engine connectivity

**Goal:** A clean install starts the engine and watchdog and lets Ace App connect immediately, without rebooting the machine. Windows MSI/WiX and Linux installation are distinct targets.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| IN-01 | P0 | Install the Windows x64 MSI in a clean Windows 11 VM with no prior Ace data | Installer finishes, registers both `AceEngine` and `AceEngineWatchdog` as separate auto-start LocalSystem services, and creates the required files |
| IN-02 | P0 | Launch Ace App as a non-elevated user immediately after installation, without restarting Windows | Typed remoc connection succeeds, initial account/dashboard state loads, and a permitted operation reaches the engine without authentication/transport errors |
| IN-03 | P0 | Enable a seeded Free Network Block while offline immediately after installation | Bundled Site Packs are present and usable; an actual blocked domain is denied while an unrelated domain remains available |
| IN-04 | P0 | Inspect Wintun, proxy, certificate trust and browser interception after installation | Tunnel/proxy operate and a supported HTTPS block produces the intended denial without a certificate warning on an allowed test site |
| IN-05 | P1 | Run as two separate standard-user accounts and inspect named-pipe, service, file and secret ACLs | Authorized UI access works; unauthorized service modification and protected file/credential access are denied according to the documented identity model |
| IN-06 | P1 | Check bundled executables, starter-pack manifest, Windows uninstall registration, advertised version and signing information | Installed payload matches the release candidate and does not silently omit a required component |
| IN-07 | P1 | Simulate installer failure or cancellation at staged, non-destructive points in separate snapshots | No false success; failure is diagnosable, rollback/repair follows the MSI contract, and previously working protection is not silently lost |
| IN-08 | P1 | Install the Linux package on CachyOS, Arch and Ubuntu as individually supported targets | Both systemd units are installed, started and recoverable; the user-session app connects to the intended canonical Unix socket and data path |
| IN-09 | P1 | On Linux, verify `/dev/net/tun`, routing/nftables, browser trust and service startup after installation | Running services correspond to functional interception; failure to initialize the network path is visible rather than counted as successful installation |
| IN-10 | P2 | Exercise launch after closing/reopening the app, and installer repair with no active Lock | Files and IPC survive normal app relaunch and approved maintenance without requiring unrelated system changes |

**Existing helpers:** `installer/tests/win11_smoke.ps1`, `installer/tests/windows_vm_harness.py`, `installer/verify-watchdog.ps1`, `installer/README.md` and `docs/IPC_ARCHITECTURE.md`. Smoke checks must be supplemented with an actual blocked-domain test.

### 2. Restart and boot persistence

**Goal:** Blocking must work after a normal restart without requiring the user to open Ace App.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| RB-01 | P0 | Reboot with an enabled Always block and an active Hard/Time Lock | Engine and watchdog restart; persisted rules, Lock state and actual blocking remain effective before launching Ace App |
| RB-02 | P0 | Reboot while Ace App is not configured to auto-launch or while no user is signed in | Protection does not depend on the UI or an interactive session |
| RB-03 | P1 | Reboot with active Scheduled blocks and multiple site-pack connections | Schedule mode, pinned timezone, rules and pack membership survive; enforcement matches the expected post-boot window |
| RB-04 | P1 | Restart after network connectivity was absent at boot and appears later | Engine starts with locally available data and recovers remote/time-dependent operations without disabling existing blocking |
| RB-05 | P1 | Reboot after an abrupt power-off snapshot while a permitted policy save is in progress | Database state is transactionally consistent; either old or committed new state is loaded, never a partially applied policy |
| RB-06 | P1 | Reboot repeatedly and examine service recovery, logs, adapter and proxy state | No permanent startup race, duplicate service instance, resource leak, or false `healthy` status when enforcement is unavailable |
| RB-07 | P2 | Repeat reboot from a standard-user profile, another local profile and after a browser auto-restores tabs | Same machine-wide policy is enforced irrespective of which user or browser session starts first |

### 3. Suspend, resume and hibernation

**Goal:** Sleep and hibernation must not leave the engine unresponsive or bypass a scheduled/locked policy.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| PW-01 | P0 | Suspend Windows while a blocked page and a permitted page are open; resume | Network access returns on the allowed domain, blocked content remains denied, and the engine responds to IPC without manual restart |
| PW-02 | P0 | Hibernate then resume with a Network Block Lock active | Both privileged services, Lock state, Wintun/proxy and domain enforcement recover without disabling the Lock |
| PW-03 | P1 | Sleep across the start of a scheduled blocking interval | On resume, matching uses the intended schedule and authenticated-time state; no stale pre-sleep allow decision persists |
| PW-04 | P1 | Sleep across the end of an interval or timed Lock expiration | Unlock/allow transitions occur only when expiration can be established under trusted-time rules, not solely from local wall time |
| PW-05 | P1 | Resume with no internet, blocked Roughtime UDP, then connectivity restored | Cached rules still block; timed Locks are not prematurely released; time refresh retries later without blocking startup |
| PW-06 | P1 | Open a browser before sleep with an existing HTTPS connection/HTTP2 session and then change policy after resume | Reused connections cannot keep serving a newly blocked host through stale tunnel state |
| PW-07 | P1 | Repeat suspend/resume and hibernate/resume several times in succession | No accumulating duplicate adapters, unresponsive named pipes, heartbeat restart loop, or permanently degraded networking |
| PW-08 | P2 | Repeat representative cycles on Ubuntu/Arch/CachyOS with the supported Linux sleep modes | systemd, TUN, routing/nftables and the local socket recover; OS-specific unsupported sleep states are documented |
| PW-09 | P2 | Compare Windows restart, Fast Startup, suspend and hibernation | Distinguish cold boot from monotonic elapsed time over sleep; note different behavior in the report |

### 4. Network blocking, HTTP 404 and immediate policy changes

**Goal:** A policy change has an observable network effect promptly; supported browser HTTP/HTTPS requests receive the intended synthetic 404 when possible.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| NB-01 | P0 | Enable an Always block for a controlled domain with a fresh HTTP request | A blocked request is denied with the intended response; an allowed control domain is unaffected |
| NB-02 | P0 | Test a new HTTPS request for a blocked hostname on port 443 in a supported browser | Proxy returns the synthetic 404 when the original hostname is interceptable; failures in certificate/trust handling are separately reported |
| NB-03 | P0 | Enable/disable a permitted block while browser tabs, DNS cache and HTTP/2 connections already exist | New requests adopt the new policy; established tunnels are closed or re-evaluated, with the transition delay measured |
| NB-04 | P0 | Compare `Always` versus `Scheduled`, including disabled blocks and boundaries of a configured window | The effective decision matches enabled state, schedule mode and active period; no unintended always-on or fail-open behavior |
| NB-05 | P1 | Block a bare domain, subdomain and deep URL/path; try uppercase and equivalent normalized host forms | Domain/URL matching respects the configured rule semantics; subdomain or path scope is not silently broadened/narrowed |
| NB-06 | P1 | Test direct user rules, site-pack rules, exceptions and competing rules | Explicit policy precedence is consistent across proxy/tunnel paths; an exception does not disable unrelated blocked rules |
| NB-07 | P1 | Test redirects and navigation from an allowed page to a blocked page | The final blocked destination is denied even if the initial request was allowed |
| NB-08 | P1 | Test browser back/forward cache, reload, private windows, restored tabs and service/UI restarts | Stale cached content is distinguished from new network access; new traffic obeys policy without opening Ace App |
| NB-09 | P1 | Exercise HTTP/HTTPS, WebSocket upgrades, IPv4/IPv6 and QUIC/HTTP3 where supported | Record the actual response/connection-denial semantics for each transport; do not require an HTTP 404 when no HTTP response can be synthesized |
| NB-10 | P1 | Enable Safe Search and test supported search-engine requests, including HTTPS | Rewriting/enforcement applies to supported requests without affecting unrelated browsing |
| NB-11 | P1 | Exercise DNS-over-HTTPS, proxy/VPN browser extensions and alternate proxy settings with the optional browser policy guard both off and on | Document actual bypasses and opt-in guard coverage; do not claim that the installer alone defeats every encrypted-proxy extension |
| NB-12 | P2 | Repeatedly toggle unlocked rules and measure latency under multiple concurrent browser requests | No stale policy, crash or material regression; record distributions for activation and 404/denial latency instead of inventing a time target |

**Important distinction:** The engine has separate intercepted 404 and non-interceptable connection-denial paths. Verify network denial even when a browser cannot display an HTTP status. See `crates/ace-engine/src/service/proxy/mod.rs`, `docs/NETWORK_BLOCK_ACTIVATION.md`, `docs/BROWSER_PROXY_ENFORCEMENT.md` and `docs/LINUX_BROWSER_PROXY_ENFORCEMENT.md`.

### 5. Resistance scripts and site-specific interventions

**Goal:** Test the actual intervention catalog and its effect in a real browser, at least on YouTube, Reddit and Twitch.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| RS-01 | P0 | With a valid Pro entitlement, enable Resistance Mode and supported interventions for YouTube | Each selected intervention produces its catalog-defined effect on a fresh page; other disabled interventions do not run |
| RS-02 | P0 | Repeat with the catalog-supported interventions for Reddit and Twitch | Verify each supported host/path independently and record which interactions and page layouts actually change |
| RS-03 | P0 | Disable the master switch, remove entitlement, or remove a valid script catalog in separate test fixtures | Injection does not continue as if operational; UI reports the appropriate `enforcementStatus` reason |
| RS-04 | P1 | Enable/disable individual scripts, including visual effects, delays, scrolling restrictions and optional CAPTCHA if present in the catalog | State persists by stable script ID; opt-in interventions remain off by default where specified; no obsolete global intensity setting is required |
| RS-05 | P1 | Toggle an intervention while an affected page is already open, then reload/navigate to a new document | Changes apply to new documents as designed; do not misreport the persistence of already injected code as a policy failure |
| RS-06 | P1 | Verify injection on supported HTTP/HTTPS browser navigations with CSP and dynamic application pages | Nonce-bearing injection and CSP changes are evaluated against actual page behavior; script-load or DOM errors are captured |
| RS-07 | P1 | Enable a Resistance Time Lock, then attempt to disable the master switch or an enabled intervention | Existing restrictions cannot be weakened while locked; adding an intervention remains possible |
| RS-08 | P1 | Reload/update the script catalog after saving explicit per-script overrides | Overrides remain keyed to stable IDs, including temporarily absent catalog IDs; status is accurate if the catalog becomes invalid |
| RS-09 | P1 | Export/import local backup and attempt an invalid/lossy cloud-backup operation | Explicit intervention states survive supported local backup; unsupported cloud serialization is rejected rather than silently losing settings |
| RS-10 | P2 | Test Free account read-only preview, unrelated websites, multiple supported browsers and language variants | No Pro-only injection for Free; no intervention leaks to unrelated hosts; compatibility gaps are documented per site/browser |

**Reference:** `crates/ace-engine/docs/resistance-mode-policies.md`. The catalog and support list are authoritative; do not assume a particular intervention exists on every site. Run the separate `boredom-engine` smoke tests when that repository is available.

### 6. Network Block and Resistance Lock behavior

**Goal:** Verify each persisted Lock state, allowed strengthening, forbidden weakening and recovery after restart.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| LK-01 | P0 | Create and edit an unlocked policy, then apply each available Lock type | Saved state matches the selected type and is enforced by the engine, not merely by disabled UI controls |
| LK-02 | P0 | For `soft_unlockable`, initiate Reflection unlock and attempt to bypass or accelerate the challenge | Challenge cannot complete before the 5-minute monotonic cooldown or without correct confirmation; invalid/reused challenges fail |
| LK-03 | P0 | For `active_until_lock_end`, try disabling, deleting, weakening, or shortening a locked block | Forbidden mutations are rejected; a lock end cannot be set in the past or beyond the supported 90-day maximum |
| LK-04 | P0 | Apply `active_during_schedule` and test both inside and outside scheduled windows | Effective editor guards match the documented window-bound semantics; the uninstall gate applies its intentionally stricter independent rule |
| LK-05 | P0 | Change a locked block from `Always` to `Scheduled`, or remove its final scheduled window | Weakening is rejected; `Scheduled` to `Always` is allowed when it strengthens the policy |
| LK-06 | P1 | Edit a locked Scheduled policy by moving, shortening, splitting or deleting windows | No previously protected period can be removed during active Lock enforcement; permitted extensions succeed atomically |
| LK-07 | P1 | Try lock-state downgrades (Time to Schedule/Soft/Inactive, Schedule to Soft/Inactive, Soft to Inactive) | The engine respects the monotonic strictness ladder; only supported authorized unlock/expiry paths remove a Lock |
| LK-08 | P1 | Change user rules, site-pack connections, Safe Search and Network Block enabled state through direct RPC while locked | Backend rejects every forbidden weakening even when bypassing the WebView |
| LK-09 | P1 | Lock a synced Site Pack through a Network Block, then attempt to delete, resync, or remove its sources | Canonical pack-lock guard prevents changes; busy/conflict outcomes are distinct from unauthorized modifications |
| LK-10 | P1 | Reboot and import a backup that would delete a locked block or disable an enabled locked Resistance intervention | Existing active Locks survive restart and are not silently replaced by weaker backup state |
| LK-11 | P2 | Test unavailable trusted time, malformed deadlines, overlapping locks and repeated unlock attempts | No early expiry or inconsistent guard result; capture typed error and authoritative post-restart state |

**References:** `crates/ace-engine/src/core/common/lock.rs`, `crates/ace-engine/src/core/common/unlock_challenge.rs`, `crates/ace-engine/src/lock_check.rs`, `docs/NETWORK_BLOCK_ACTIVATION.md` and `crates/ace-engine/docs/site-pack-lock-invariant.md`.

### 7. Schedules, timezone changes and trusted time

**Goal:** Verify both civil-time schedule correctness and conservative timed-Lock behavior without trusting the user-controlled system clock.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| TM-01 | P0 | Move the OS clock forward and backward while a timed Hard Lock and an Always block are active | Local clock manipulation cannot authorize early unlock or disable an Always block |
| TM-02 | P0 | Schedule a block to start/end at known boundaries and test just before, at and after each boundary | Enforcement matches the configured civil-time windows at the pinned IANA timezone, including consecutive and overnight windows |
| TM-03 | P0 | Change the Windows/Linux system timezone after saving a scheduled block | Existing schedules retain the saved IANA zone; displayed time and effective enforcement do not silently shift to the new host zone |
| TM-04 | P0 | Test both daylight-saving transitions, ambiguous repeated hours and nonexistent local times using synthetic clock fixtures | Behavior is deterministic under the stored timezone and does not accidentally shorten a protected window |
| TM-05 | P1 | Block Cloudflare Roughtime UDP/2003, then restore connectivity | Existing filtering continues; expiry cannot be established from unauthenticated local time; refresh recovers after service access returns |
| TM-06 | P1 | Feed forged, invalid-signature, replayed, stale or high-uncertainty Roughtime test packets in an isolated fixture | Invalid time samples are rejected; the authenticated floor and uncertainty prevent premature expiration |
| TM-07 | P1 | Observe authenticated-time refresh over extended runtime and simulated failures | Normal successful refresh cadence is approximately 60 minutes, not per flow; failures back off and do not create request storms |
| TM-08 | P1 | Hibernate/suspend across a deadline, then resume without and with valid authenticated time | Monotonic elapsed and trusted-time bounds behave as documented; release occurs only after demonstrably expired time |
| TM-09 | P1 | Reboot with a previously authenticated floor and no network access | Persisted floor is not treated as an elapsed clock; timed Locks remain conservative until revalidated |
| TM-10 | P1 | Call the installer's `--check-locks` in a VM while clock service is unreachable and a timed Lock exists | Lock check makes at most its allowed independent verification attempt and refuses uninstall if expiry is uncertain |
| TM-11 | P2 | Compare Windows, Ubuntu, Arch and CachyOS, including DST/timezone and schedule export/import | Local backup preserves schedule timezone, while unsupported cloud timezone conversions are explicitly rejected rather than guessed |

**Reference:** `docs/TRUSTED_TIME_FUTURE.md` and `crates/ace-engine/src/trusted_time.rs`. The current implementation uses signed Roughtime sampling; do not substitute local NTP or HTTP `Date` as evidence.

### 8. Custom Site Pack import (URLs and files)

**Goal:** Import and enforce real local/remote lists, with bounded resource use and correct failure handling.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| SP-01 | P0 | Import a valid hosts-format list from a local file using a Pro test account | The imported domains are counted, persisted, attached to a policy and actually blocked |
| SP-02 | P0 | Import equivalent content from a controlled HTTP/HTTPS URL | Download, parse, persistence and network blocking match the local-file result; redirects are checked against source policy |
| SP-03 | P0 | Import with `Auto`, standard hosts and wildcard-domain formats | Format handling matches the selected parser; no unexpected host/path expansion or silently ignored valid domains |
| SP-04 | P1 | Import duplicates, comments, blank lines, malformed domains, mixed case and internationalized/domain edge cases | Parser accepts valid input deterministically, rejects/skips malformed entries safely, and reports expected counts |
| SP-05 | P1 | Import multiple local files and URLs into the same pack | All valid sources contribute correctly without duplicate inflation; source list and provenance survive restart |
| SP-06 | P1 | Simulate DNS failure, timeouts, 404, truncated response, invalid format and source deletion during import | Terminal `FAILED` is reported; a dropped remoc stream/EOF never becomes false `COMPLETED` |
| SP-07 | P1 | Simulate interruption during initial import and resynchronization of an existing populated pack | No partially published live pack or loss of previously active blocking from an incomplete import |
| SP-08 | P1 | Test large files, oversized single lines, excessive domain counts and concurrency within a bounded fixture | Import remains within documented caps, does not exhaust LocalSystem memory, and fails with actionable errors when limits are exceeded |
| SP-09 | P1 | Attempt create/add-source/resync of Pro-only local/custom packs as Free, invalid-key and expired-Pro users | Engine enforces entitlement independently of UI; Free remote packs retain only their intended Free behavior |
| SP-10 | P1 | Delete, detach, resync, or modify a synced pack referenced by an active locked Network Block | Locked changes are denied; an in-use unlocked pack reports a conflict where applicable |
| SP-11 | P2 | Test temporary-file cleanup, permission-denied paths, URL schemes and malformed archives/list payloads | No partial database, unsafe filesystem side effect, misleading success, or unauthorized fetch; record source validation outcomes |

**References:** `crates/ace-engine/src/core/network_block/site_pack/importer/`, `crates/ace-engine/src/service/api/operations_site_pack.rs` and `apps/ace-app/src/lib/stores/syncing-store.svelte.ts`.

### 9. First-run Terms of Service and Privacy Policy

**Goal:** Show an unskippable legal consent gate until the engine has persisted the user's acceptance.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| LG-01 | P0 | Launch a clean install with no saved ToS acceptance | Terms and Privacy modal appears before normal app interaction, with both acknowledgments initially unchecked |
| LG-02 | P0 | Select only Terms, only Privacy, or neither | Continue remains disabled until both acknowledgments are selected |
| LG-03 | P0 | Try Escape, outside click, close icon, route navigation or application reload without accepting | Modal cannot be dismissed to reveal usable app controls; relaunch returns to the gate |
| LG-04 | P0 | Accept both and restart Ace App and the engine | Engine stores acceptance; the UI opens normally after reconnect/restart without prompting repeatedly |
| LG-05 | P1 | Simulate a failed or timed-out `acceptTos` remoc call | App reports failure, does not set a successful local accepted state, and allows retry when transport is restored |
| LG-06 | P1 | Make account-info lookup fail or become slow while offline | Loading/error/retry state is visible; an unverified acceptance state is not treated as consent and Pro-check latency does not permanently trap the UI |
| LG-07 | P1 | Open Terms and Privacy links in each supported OS/browser setup | Both links open the intended public legal documents; link failure never implies acceptance |
| LG-08 | P2 | Use the dev-only consent reset, restart, and switch between separate local user sessions | Repeatable first-run test behavior is correct and documented; clarify whether consent is per installation or per user rather than assuming |

**Reference:** `apps/ace-app/src/lib/components/TosModal.svelte`, `apps/ace-app/src/routes/+layout.svelte` and `crates/ace-engine/src/service/api/operations_account.rs`.

### 10. Pro entitlement and license verification

**Goal:** A locally edited flag, database row, client backup or fabricated license must not unlock production Pro features.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| PR-01 | P0 | Compare valid Free, active Pro, revoked, expired and invalid test API keys | Only appropriately signed and currently eligible Pro accounts can perform gated operations |
| PR-02 | P0 | Alter payload/signature bytes, `kid`, `iss`, account binding or API-key fingerprint in an entitlement token | Engine rejects modified, mismatched or incorrectly signed entitlements |
| PR-03 | P0 | Replace the signed lease/public verifier in service-owned storage using a disposable offline fixture | Arbitrary edited DB values, unauthenticated key substitution and fake local Pro booleans cannot establish Pro |
| PR-04 | P0 | Attempt premium pack download/resync, script download and cloud backup with no token, wrong token or a Free account | Both engine gating and server-hosted authorization reject unauthorized resources; do not rely on a disabled UI control |
| PR-05 | P1 | Simulate login, logout, account switching, checkout return and server-side entitlement change using sandbox accounts | Entitlement refreshes correctly; logout/account switch invalidates the previous cached lease |
| PR-06 | P1 | Test token `iat`, `exp`, feature grants and the bounded offline window with controlled signed fixtures | Invalid temporal claims and missing grants deny Pro; offline behavior never extends beyond the documented limit |
| PR-07 | P1 | Disconnect the entitlement endpoint, restart engine and reconnect | Only a valid cached signed lease survives within its allowed offline period; explicit revocation/invalid key takes precedence |
| PR-08 | P1 | Move local wall time forward/backward, change timezone and reboot with a cached entitlement | Clock rollback cannot indefinitely prolong Pro; schedule/Lock protection remains independent of Pro cache validity |
| PR-09 | P1 | Expire or revoke Pro while protected Network Blocks and downloaded Site Packs are active | Existing blocking is not weakened merely because premium access expires; new Pro-only operations are denied |
| PR-10 | P1 | Attempt backup import, false `requiresPro` metadata, forged premium pack IDs and direct remoc requests | Server/engine authorization remains authoritative after bypassing UI-level guards |
| PR-11 | P1 | Test builds with the optional `no-license` Cargo feature versus ordinary release binaries | Bypass exists only in explicitly designated test builds; release packaging checks must detect and refuse accidentally shipping such a build |
| PR-12 | P2 | In a sandbox billing environment, simulate payment success, refund, cancellation and webhook duplicates/out-of-order events | Backend entitlement ultimately reflects the authorized subscription state without replaying paid access |
| PR-13 | P2 | Exercise malformed server responses, rate limits and public-key rotation with controlled test infrastructure | Errors are observable, invalid trust material is rejected, and the documented authenticated key-change policy is respected |

**References:** `crates/ace-engine/docs/licence-architecture.md`, `docs/RELEASING.md` and `crates/ace-engine/src/manager/account.rs`. Test the `no-license` feature in an isolated build only; it is not a production defense.

### 11. Product release discovery, verification and upgrades

**Goal:** Current builds discover/download authenticated releases safely; distinguish that capability from automatic installation, which is **not implemented**.

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| UP-01 | P0 | Serve a correctly signed stable Windows/x64 release envelope from an isolated HTTPS fixture | Embedded-key Ed25519 verification, schema checks, SemVer and platform compatibility accept only the intended manifest |
| UP-02 | P0 | Tamper with signed payload, signature, key ID, package URL, publisher fingerprint, channel, platform or version | Invalid, unsigned, insecure, unsupported, downgraded and incompatible metadata are rejected |
| UP-03 | P0 | Supply an unsigned `/api/v1/releases/latest` response or content-updater version hint | These are not accepted as authority to stage or install a binary; no trust-path confusion occurs |
| UP-04 | P0 | Download a valid signed MSI fixture with the correct digest and approved Authenticode publisher | Bounded streaming download, SHA-256, exact signer identity, rehash and secure cache staging succeed before UI reports verified state |
| UP-05 | P0 | Replace the MSI with incorrect hash, bad signature, wrong publisher or a package at an unsafe path | Verification fails without treating the file as installable; staged/replaced files cannot bypass final integrity checks |
| UP-06 | P1 | Interrupt download, write a truncated `.partial` file, exhaust disk, and retry | Failed transfer is reported, partial data is discarded safely, and retry can recover without leaving a trusted corrupt artifact |
| UP-07 | P1 | Trigger concurrent manual/periodic checks and inspect repeated status polling | Single-flight/cache/backoff prevent duplicate work; discovery and transfer stages show accurate UI status without blocking normal engine service |
| UP-08 | P1 | Run with missing/invalid build-time trust settings or an unavailable signed feed | Discovery safely reports disabled/error, not a fabricated available update; normal filtering continues |
| UP-09 | P1 | Try to request installation directly from the current UI/API after successful verification | Current release builds **do not** execute an MSI automatically, create a hidden privileged install path or stop protected services |
| UP-10 | P0 | Perform a **manual** major MSI upgrade while an ordinary Network Block or Resistance Mode Lock is active | Existing `CA_CheckLocks` / authenticated shutdown maintenance gate refuses an unauthorized upgrade or uninstall |
| UP-11 | P1 | Perform a legitimate manual unlocked major upgrade on a disposable Windows VM | Both services restart and reconnect; persisted data, site packs, certificates and actual network blocking survive upgrade |
| UP-12 | P1 | Try an interrupted manual upgrade/repair, restart during maintenance and inspect SCM/watchdog recovery | Partial replacement is detectable; no silent permanent enforcement gap; restore snapshot before repeating |
| UP-13 | P2 | Check pinned trust configuration, signed release-candidate publication policy and Windows version consistency | Release draft/staging conditions reject unsigned candidates; user-visible version matches the packaged binary and website metadata |
| UP-14 | P2 | Examine platform differences and planned future automatic installation scenarios | Mark Linux auto-update and privileged unattended Windows installation **NOT IMPLEMENTED**, not PASS or an unexpected regression |

**Important:** Do not test future one-shot locked-upgrade grants, an automatic MSI installer, reboot-durable update journal, or binary rollback as if they exist. These are future release gates in `docs/PRODUCT_UPDATES.md`, not shipped behavior. Native Windows Authenticode and upgrade tests cannot be replaced by pure-Rust tests.

### Coverage and release decision

- All eleven original topics have independent suites; the anti-tampering suite below remains an additional cross-cutting security gate.
- A P0 failure that causes silent loss of blocking, early Lock expiration, unauthorized premium access, unsafe binary trust, or lock-breaking uninstall/upgrade blocks release until resolved or explicitly re-scoped.
- Passing unit, browser-mock or static checks does **not** establish installed Windows/Linux behavior. Native VM evidence is required for service, Wintun/TUN, IPC, lock, browser and MSI tests.
- Keep a result ledger with IDs, environment, steps, expected/observed outcome, links to screenshots/logs, and an issue for every failure. Mark missing infrastructure as BLOCKED/NOT RUN rather than treating it as a functional failure or success.

## Anti-tampering, data integrity and recovery testing

### Testing methodology

Run destructive tests only in isolated, disposable VMs using a test installer, synthetic data, and a snapshot before each destructive scenario. Do not test on a developer workstation.

For every scenario:

- Repeat with an active Hard Lock, an active Resistance Mode Lock, and no active locks (where applicable).
- Test as both a standard user and an elevated administrator; classify administrator/SYSTEM-level bypasses against the documented user-mode threat model.
- Use a known blocked domain and a known allowed domain to verify actual enforcement, not just a running process or a successful IPC connection.
- Measure the recovery time and any interval during which blocked content becomes accessible.
- Collect redacted engine/watchdog logs, Windows Event Log or systemd journal entries, service state/PIDs, effective file permissions, database integrity status, and screenshots.
- Restore the VM snapshot between destructive scenarios.
- Record PASS when prevention or recovery and enforcement are verified, FAIL when the specified protection is bypassed, and LIMITATION when behavior is outside the documented threat model. Do not mark an untested or unverifiable case as PASS.

A running engine does not prove that rules and locks are still enforced. Service recovery with an empty policy or lost locks is a failed enforcement recovery.

### P0 — Critical tests (release blockers)

| ID | Scenario | Expected verification |
| --- | --- | --- |
| AT-01 | Force-kill `AceEngine` while an active Hard Lock is enforcing a block | SCM/watchdog restore the engine; the original Lock and blocked-domain behavior survive recovery |
| AT-02 | Force-kill `AceEngineWatchdog` | SCM or the engine restores watchdog supervision without interrupting enforcement |
| AT-03 | Terminate engine and watchdog in rapid succession | Measure the enforcement gap and verify recovery while service registrations remain intact; document elevated-administrator limits |
| AT-04 | Attempt to stop, disable, reconfigure, or delete the two Windows services | Standard-user operations are denied; privileged actions and reboot persistence are measured and classified separately |
| AT-05 | Delete, truncate, replace, or corrupt the main SQLite database before engine startup | Existing Locks must not silently disappear; specifically test automatic DB recreation and empty-policy initialization |
| AT-06 | Corrupt individual persisted Lock states, deadlines, schedules, and Resistance Mode settings | Invalid or unverifiable security state cannot silently turn into an unlocked policy |
| AT-07 | Delete, truncate, replace, or corrupt an active Site Pack SQLite database | Verify actual domain blocking and report any missing/corrupt-pack fail-open path |
| AT-08 | Attempt MSI uninstall, repair, and major upgrade while locked, including direct service removal | Protected installer paths refuse unauthorized removal; direct privileged bypasses are explicitly classified |
| AT-09 | Attempt unauthorized engine shutdown or watchdog maintenance through local remoc IPC | Invalid capabilities and unauthorized requests never bypass Lock or maintenance checks |

### P1 — Recovery and persistent tampering

| ID | Scenario | Expected verification |
| --- | --- | --- |
| AT-10 | Simulate an engine hang by preventing heartbeat replies | Watchdog observes startup grace/missed-heartbeat thresholds, targets the current SCM-owned PID, and avoids premature restarts |
| AT-11 | Modify a service's startup type, recovery actions, executable path, or registration; then reboot | Verify effective ACLs and whether persistent changes defeat automatic startup; separate standard-user from administrator results |
| AT-12 | On Linux, attempt `stop`, `disable`, `mask`, and removal for `ace-engine.service` and `ace-watchdog.service` | Verify systemd permissions and recovery, including explicit root-level limitations |
| AT-13 | Simulate power loss or process termination during a SQLite transaction | No half-committed Lock update or unexpected policy loss; integrity and recovery are reported |
| AT-14 | Replace persisted state with an old backup or tamper with trusted-clock files/system time | A stale snapshot or clock change must not silently shorten or remove an active Lock |
| AT-15 | Simulate a full disk, inaccessible database, or permission failures | Detect and report failures; investigate any transition to unrestricted network access |
| AT-16 | Attempt to replace, rename, or remove the engine, watchdog, Wintun, and protected configuration files | Standard-user writes are denied; elevated-administrator limits and lack of binary-integrity monitoring are documented |
| AT-17 | Test unauthorized Named Pipe/Unix socket clients, invalid capabilities, and forged/replayed watchdog messages | Privileged RPCs and maintenance are rejected without valid authorization and watchdog peer identity |
| AT-18 | Close/kill Ace App, log out, switch accounts, or restart only the UI | Enforcement continues independently of Tauri and the interactive session |
| AT-19 | Interrupt an uninstall or major upgrade at different stages | Services, Lock gates, and installed files do not silently end up in an unprotected partial state |
| AT-20 | Test rejected, expired, or interrupted watchdog maintenance leases | No unauthorized or unbounded maintenance; legitimate lock-free maintenance completes without restart races |

### P2 — Additional hardening tests

| ID | Scenario | Expected verification |
| --- | --- | --- |
| AT-21 | Inspect effective directory/file permissions from two distinct standard-user accounts | Protected credentials, IPC capability, and CA private key have appropriate ACLs without breaking authorized application access |
| AT-22 | Test symlinks, junctions, and path replacement on paths writable by ordinary users | A standard user cannot redirect privileged reads/writes to unauthorized targets |
| AT-23 | Try DNS changes, VPNs, alternative proxies, and browser-specific networking settings | Document which settings preserve enforcement and which are known, accepted bypass limitations |
| AT-24 | Attempt to activate debug-only watchdog heartbeat controls in release builds | Release recovery cannot be disabled by test-only flags or sentinel files |
| AT-25 | Inspect engine, watchdog, OS service, and installer diagnostics | Failures are diagnosable without logging secrets, private keys, or capability tokens |

### Acceptance criteria and known architectural risks

1. Check the blocking result during downtime and after recovery, without opening Ace App. A service reported as Running is insufficient.
2. Previously persisted Locks must not silently disappear due to DB corruption, schema errors, missing Site Packs, or failed policy loading.
3. Explicitly verify the behavior of `initialize_db()` and `initialize_network_manager()` in `crates/ace-engine/src/lib.rs`: database recreation and `NetworkManager::new_empty()` are possible recovery paths and **must not be mistaken for secure Lock recovery**. Report the observed result before deciding whether a fail-closed design change is required.
4. Investigate whether Site Pack read/open errors degrade into an unblocked lookup. A logging-only result is not sufficient if the active blocking policy has disappeared.
5. Watchdog recovery must not interrupt authorized lock-free maintenance or cause an uncontrolled restart loop.
6. Do not claim that this userspace design is resistant to an administrator with full control of service configuration, ACLs, processes, or offline storage. Record such attempts as limitations or defects according to the agreed threat model.
7. Define and measure explicit limits for maximum restart time and maximum enforcement interruption before declaring the recovery suite complete.
8. A P0 test that demonstrates a Lock or blocking bypass is a release blocker until resolved or explicitly re-scoped by the owner.

### Existing test helpers and documentation

- Windows service configuration and single-process recovery: `installer/verify-watchdog.ps1`, optionally with `-ExerciseRecovery`. This script does **not** prove end-to-end blocking continuity.
- Installer service configuration, ACLs, and maintenance lifecycle: `installer/README.md`.
- Current shutdown mechanism: `ace-engine.exe --request-shutdown` over typed remoc IPC, **not** the obsolete HTTP `POST /system/shutdown` path.
- Native IPC/security checks: `docs/ai/ACE_APP_NATIVE_TEST_GATE.md` and `docs/IPC_ARCHITECTURE.md`.
- Related threat models requiring native verification: `docs/VPN_BROWSER_EXTENSION_THREAT_MODEL.md` and `docs/TRUSTED_TIME_FUTURE.md`.

### Evidence template

For each scenario, record:

- Test ID, OS/version, build configuration, Git commit, executable hashes, and VM/snapshot ID.
- Active Lock configuration and known blocked/allowed test domains.
- Test-account privilege level, exact reproduction steps, and any test-only fault injection.
- Expected versus observed behavior, error/exit codes, service/PID state before and after.
- Recovery duration, blocked-domain outcomes during and after the fault, and database/Lock state after recovery.
- Redacted logs and screenshots, plus PASS/FAIL/LIMITATION and a linked issue for failures.

Never include real credentials, capability tokens, CA private keys, or production user data in logs or test artifacts.

## 12. VPN, TOR, I2P and encrypted-proxy bypass testing

**Goal:** Active Network Blocks and Locks continue to protect supported browsers when a desktop VPN, browser VPN/proxy extension, TOR transport, local SOCKS/HTTP relay or anonymity-network helper is present. Verify the *actual blocked origin*, not only that a tunnel is connected or an engine service is Running.

**Separate threat mechanisms:**

- **Desktop VPN routing:** WireGuard/OpenVPN/Proton clients modify physical/default routes, tunnel interfaces and DNS. On Windows verify WFP route enforcement; on Linux verify ACE's TUN, policy routing and nftables guard.
- **Browser extensions and encrypted relays:** the proxy CONNECT may hide the origin hostname from ACE. Known relay-domain rules and TCP/4443 guard are defense in depth, not universal protection against HTTPS relays on TCP/443.
- **Local SOCKS/HTTP helpers:** browsers may send to a loopback proxy; the helper is a separate process making the outbound connection. Verify TOR/Tor Browser, `torsocks`, process aliases and different ports. Loopback exceptions can preserve this bypass boundary.
- **Anonymity networks and bridges:** TOR bridges (obfs4, Snowflake, WebTunnel) and I2P outproxies are separate cases. An I2P tunnel alone is not evidence that a public HTTPS origin was reached.

### Reusable snapshot topology and automation

1. Maintain **one disposable libvirt VM per OS**: Windows 11, Ubuntu, Arch and CachyOS. Do not install clients or change routing on the development host.
2. For each VM maintain `base` and `ace-ready` states, then **environment snapshots** such as `ubuntu-wireguard-ready`, `ubuntu-openvpn-ready`, `ubuntu-tor-ready`, `ubuntu-tor-browser-ready`, `ubuntu-socks-ready`, `ubuntu-i2p-ready` and `ubuntu-proton-ready`. A snapshot is per provider/configuration, **not per test case**.
3. Use an environment snapshot for many scenarios. For tests that change persistent state or damage the guest, shut down normally and restore it before continuing. Do not use `virsh destroy` or force snapshot-revert to defeat an active Lock. The initial Python backend accepts only offline single-disk QCOW2 internal snapshots of disposable `ace-lab-*` domains and verifies domain UUID, memory/disk snapshot metadata and identity. It deliberately rejects external/multi-disk layouts and uncaptured UEFI NVRAM/virtual TPM; use a separately validated full-VM backend for Windows 11 OVMF/TPM snapshots. Snapshot operations require exact `--confirm-uuid` in addition to the domain confirmation.
4. Create a new test Network Block (Always, known blocked HTTPS hostname, Hard/Time Lock) **after** restoring a snapshot. Do not preserve a live timed Lock inside a long-lived snapshot: trusted-clock rollback and expired wall-clock state can contaminate results.
5. Provision official free distro packages through the QEMU Guest Agent. Use a controlled WireGuard/OpenVPN gateway and SOCKS/HTTPS relay where possible, with disposable keys. Keep real Proton/Windscribe Free sessions in **private** snapshots prepared by a human; never automate account creation, CAPTCHA, login or paid subscriptions.
6. Run the VM orchestration in `installer/tests/network_lab/` using **TOML**, not YAML. `lab.toml` declares environment snapshots and scenarios; `run.py` supports validation, dry-run planning, provisioning, provider connect/disconnect, snapshot save/restore and read-only probes. All mutating operations require `--execute --confirm-domain`; snapshot restore additionally requires `--accept-data-loss`.
7. Distinguish `INFRA_OK` (service, routes, proxy listener) from **PASS** (supported browser cannot reach the blocked origin while an allowed origin still works). Current Python probes do **not** prove end-to-end blocking; native browser evidence must be added separately and missing checks remain NOT RUN. A suite with zero executed checks exits nonzero; use `--require-coverage` to reject any remaining NOT RUN cases.
8. Record `ace-tun` decisions, Linux `ip rule`/`nft` and service state (or Windows WFP), browser process identity and NetLog, remote relay address, and blocked/allowed origin-server logs, plus any packet captures. Keep screenshots and network metadata private, and sanitize all tokens and credentials.

### VPN / proxy test cases

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| VPN-01 | P0 | Supported browser with no VPN, baseline | Blocked origin denied and allowed origin reachable before enabling a VPN |
| VPN-02 | P0 | WireGuard **IPv4-only** test tunnel, including route changes and startup order | Protected browser traffic remains filtered on IPv4; IPv6 is a separate VPN-09 test and cannot be assumed secure from this lab tunnel |
| VPN-03 | P0 | OpenVPN and WireGuard split tunneling, include/exclude browser or destination | A route exclusion cannot silently permit a locked blocked origin |
| VPN-04 | P0 | Proton Free browser extension connected/disconnected, guard OFF | Measure actual proxy bypass if present; record it as FAIL/limitation, not a successful test |
| VPN-05 | P0 | Same extension with **explicitly opted-in** managed browser guard ON | Proxy permission is denied on supported native browsers; unrelated allowed browsing remains functional |
| VPN-06 | P1 | Desktop VPN and browser proxy extension active simultaneously | Layered routing/proxy behavior is visible and a blocked origin does not silently escape |
| VPN-07 | P0 | Start VPN before/after ACE, reboot, restore browser tabs and switch servers | No permanent stale unrestricted session or service-routing race |
| VPN-08 | P1 | Disconnect/reconnect, tunnel failure, kill switch and server rotation | Active blocks remain enforced or protection is clearly marked degraded |
| VPN-09 | P1 | DoH, IPv4/IPv6, QUIC/HTTP3, DNS switch, split DNS | No unnoticed route/hostname attribution bypass on supported browsers |
| VPN-10 | P1 | HTTPS proxy relay on TCP 443, 4443 and 8443 | Identify and document hidden-origin paths; 4443-specific deny alone is insufficient |
| VPN-11 | P0 | Local HTTP/SOCKS5/HTTPS proxy helper or browser-to-loopback connection | An unprotected helper does not silently act as an untracked relay for a locked blocked origin |
| VPN-12 | P1 | Chrome/Chromium, Firefox, Edge/Brave as available, private windows and new profiles | Process attribution and browser-managed policy remain effective on every claimed target |
| VPN-13 | P2 | Real free providers (Proton, Windscribe) and any additional voluntarily available clients | Log client/protocol, exact free-tier limitations and coverage; **do not** buy paid plans |
| VPN-14 | P1 | Ubuntu, Arch, CachyOS and Windows network routes and enforcement boundary | Linux RPDB/nftables vs Windows WFP are validated separately, not inferred from one another |

### TOR and related anonymity-network test cases

| ID | Priority | Scenario | Expected verification |
| --- | --- | --- | --- |
| TOR-01 | P0 | Tor Browser with no bridge | Compare blocked and allowed HTTPS origins, including the Tor Browser process identity |
| TOR-02 | P0 | Chrome/Firefox configured with TOR SOCKS5 on localhost | Local proxy routing cannot deliver an origin blocked by an active Network Block |
| TOR-03 | P1 | TOR daemon and `torsocks` as separate processes | Verify listener, process attribution and real browser behavior; a listening socket alone is only INFRA_OK |
| TOR-04 | P1 | Tor Browser with obfs4 bridge | Verify blocked origin while distinguishing bridge bootstrap failure from enforcement failure |
| TOR-05 | P1 | Tor Browser with Snowflake or WebTunnel | Test encrypted/obfuscated helper relay behavior without treating transport success as filter success |
| TOR-06 | P0 | TOR starts before/after ACE and browser | No startup-order bypass or stale pre-existing proxy session |
| TOR-07 | P1 | Kill/restart TOR process with active Hard Lock | Engine stays responsive and blocking persists after helper recovery |
| TOR-08 | P1 | Nondefault SOCKS port, process path/binary alias, separate proxy owner | Test the real process-matching rule, not solely the known `tor.exe` name |
| TOR-09 | P0 | Loopback/localhost, remote DNS over SOCKS, IPv6 and new browser profile | Observe whether trusted/untrusted helper boundaries create an origin bypass |
| TOR-10 | P1 | Repeat Windows 11, Ubuntu, Arch and CachyOS | Record OS-specific browser process aliases, policy differences and known limits |
| I2P-01 | P2 | I2P local HTTP proxy and a *controlled* external outproxy where available | Test hostname visibility and origin reachability; no outproxy means NOT RUN, not PASS |

### Release criteria and known limits

- **P0 FAIL:** a standard-user supported browser reaches an origin blocked by an active protected Network Block through VPN/TOR/proxy, without explicit documented exemption. A P0 issue blocks release pending triage.
- Do not confuse "TOR blocked completely" with secure behavior unless the product's intended compatibility policy explicitly permits blanket denial. Check allowed browsing and non-browser applications to detect harmful false positives.
- Do not claim protection from every VPN relay, bridge or root/administrator-controlled network route. If the policy does not support a configuration, document the **LIMITATION** and verify no false "Protected" UI state.
- Never claim a live provider test passed from a local simulation. Free test tunnels exercise routing and proxy *classes*; paid/proprietary client differences remain unverified until tested with the actual client.
- The executable names in `LOCAL_PROXY_PROCESSES` and the existing localhost exception require **native process and route tests**, especially for Tor Browser's bundled helper processes on Linux.

**References:** `installer/tests/network_lab/README.md`, `installer/tests/network_lab/lab.toml`, `docs/VPN_BROWSER_EXTENSION_THREAT_MODEL.md`, `docs/BROWSER_PROXY_ENFORCEMENT.md`, `docs/LINUX_BROWSER_PROXY_ENFORCEMENT.md`, `crates/ace-engine/src/service/proxy/constants.rs`, and `crates/ace-tun/src/platform/mod.rs`.
