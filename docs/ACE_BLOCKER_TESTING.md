LLMs should edit this file only after previous explicit consent from the user.

### Introduction

This is a set of things to verify when testing Ace Blocker. For each of the failed tests, the failure should be reported along with the evidence and pertinent logs.

# Testing

## How should you test Ace Blocker

1. Ace Blocker should be tested on various VMs, one for each operating system we want to support, at least Windows 11, CachyOS, Ubuntu and Arch.
2. Use NovaGPT integrated VM support to setup the testing environment and use screenshot as well as logs to gather evidence
3. If there's a repetitive automatable action, a script can be made to automate the task.

## What should we test

1. The Ace Blocker installer should install properly Ace Blocker in sucha a way: (a) after installation, Ace Blocker should be active and Ace App should be able to talk with the engine without needing a system restart.
2. When restarting the machine, Ace Blocker should work properly
3. Neither ibernating nor suspending the machine should make Ace Blocker unresponsive
4. When blocking a site, the 404 page should appear as soon as possible
5. Resistance scripts should be tested at least for youtube, reddit and twitch
6. Each lock type functionality should be verified
7. System time should be changed to: (a) verify schedule functionality and (b) verify that the remote clock properly works
8. Importing custom site packs requires testing with both URLs and files
9. When starting for the first time, Ace Blocker should require ToS and Privacy Policy acceptance
10. The Pro checks should be stress tested, by editing time, editing database settings, verifying that the change in system time won't break anything here, faking the license, trying to download the packs with a e token as well as without a token
11. Product updates should be tested thoroughly

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
- Existing security leads requiring native verification: `docs/SECURITY_AUDIT_FOLLOWUP_2026-10-26.md`.

### Evidence template

For each scenario, record:

- Test ID, OS/version, build configuration, Git commit, executable hashes, and VM/snapshot ID.
- Active Lock configuration and known blocked/allowed test domains.
- Test-account privilege level, exact reproduction steps, and any test-only fault injection.
- Expected versus observed behavior, error/exit codes, service/PID state before and after.
- Recovery duration, blocked-domain outcomes during and after the fault, and database/Lock state after recovery.
- Redacted logs and screenshots, plus PASS/FAIL/LIMITATION and a linked issue for failures.

Never include real credentials, capability tokens, CA private keys, or production user data in logs or test artifacts.
