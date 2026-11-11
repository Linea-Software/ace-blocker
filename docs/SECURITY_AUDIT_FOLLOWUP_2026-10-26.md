<a id="ace-blocker--security-audit-di-follow-up"></a>
# Ace Blocker — follow-up security audit

**Date:** October 26, 2026 · **Repository:** `C:\Projects\ace-project\ace-blocker` · **Ref:** `ad1963ebaa8a63cb41fc07cea436e6a7d67ea53f` (`main`, clean at the start)

**Profile:** standard, whole-repository scope · **Result:** **INCOMPLETE — static review, not security attestation**.

## Executive summary

The review examined risk surfaces across `apps/ace-app`, `crates/ace-engine`, `ace-watchdog`, `ace-watchdog-protocol`, `ace-tun`, `mitm-proxy`, `ace-domain-blocklist`, `installer`, and CI.
The reviewers did not read every file or verify every execution path.
**Whole-repository scope does not mean complete coverage.**
The review reassessed three leads from the previous audit and identified two more paths for investigation.
**No vulnerability passed the independent validation required by the skill.**
Do not assign formal severity or treat these leads as demonstrated exploits.

The main trust boundary is between **LocalSystem components and unprivileged processes or users**.
The installer configures inherited read permissions in ProgramData.
The service stores the IPC capability, cloud API credential, and local CA private key in this area.
Test effective access by a second Windows user on an installed package.
Also examine **IPC operations without capability checks**, **browser-to-account callbacks**, and availability under untrusted list input.

<a id="registro-dei-lead-statici-non-indipendentemente-validati"></a>
## Register of static leads without independent validation

| ID | Verification priority | Boundary / component | Specific hypothesis | Status |
| --- | --- | --- | --- | --- |
| FUP-01 | Immediate, before release | MSI → SharedData → secrets / RPC | Effective DACLs might expose the IPC capability, cloud key, and CA to another local account | Not validated |
| FUP-02 | Immediate, before release | Multi-user pipe → read RPC → backup | A local client without capability might read backup/configuration data if those data are private | Not validated |
| FUP-03 | Immediate, before release | Browser → deep link → account association | `aceblocker://account/api-key` callback without app-bound state might replace a key without a pending request | Not validated |
| FUP-04 | High, before release | URL/file list → parser → LocalSystem memory | Lines as long as the download might expand `line_buffer` without a per-line limit | Not validated |
| FUP-05 | Measure before release | Local user → pipe → daemon sessions | A task per connection without an explicit limit might exhaust shared resources | Not validated |

FUP-01–03 revisit the previous report's `needs_validation` leads.
FUP-04 and FUP-05 extend the review.
No evidence established compromise, Lock-rule bypass, or exploitation of running services.

<a id="fup-01--isolamento-dei-segreti-acl-e-identità-ipc"></a>
### FUP-01 — Secret isolation (ACL and IPC identity)

**Code path:** `installer/installer.wxs:209-215` sets `(A;OICI;GRGX;;;BU)` on the data directory.
`crates/ace-engine/src/settings.rs:93-99,144-190` puts the database, certificates, and `capability.token` in SharedData.
It writes the token with `std::fs::write`.
`crates/mitm-proxy/src/ca.rs:199-216` stores the private PEM.
`crates/ace-engine/src/db/account_repo.rs:19-32` stores the API key in the database.
`crates/ace-engine/src/service/clients/ace_server_client.rs:9-20` uses it as a cloud credential.
`crates/ace-engine/src/service/api/rpc.rs:359-371` checks capability on methods that require it.

**Unverified decisive condition:** Inspect the effective DACLs of `capability.token`, the database, and `mitm_ca.key` on Windows.
Account for inheritance, ownership, and LocalSystem file creation.
No test proved that another user can read these files after installation.

**Remediation if confirmed:** Separate public data from secrets with file-specific ACLs and multi-user tests.
The legitimate Tauri app must still request operations.
Do not deny every user access to the token without an identity or broker design.
Restrict CA private-key access to SYSTEM/Administrators.
Rotate exposed material. Use exclusive creation and fail-closed writes for sensitive files.

<a id="fup-02--esportazione-del-backup-senza-capability"></a>
### FUP-02 — Backup export without capability

**Code path:** `crates/ace-engine/src/service/api/transport.rs:17-36` grants `AU` access to the Windows pipe and sets `accept_remote(false)`.
Linux sets the socket to `0666` (`:66-80`).
`crates/ace-engine/src/service/api/rpc.rs:359-371,438-447` accepts `required=false` and exports the backup.
`crates/ace-engine/src/manager/network/backup.rs:48-67,111-163` includes names, rules, schedules, and lists in the result.

**Decisive question:** Does the multi-user privacy model require private settings or deliberately shared blocking state?
The review did not perform a dummy request from another account in an isolated VM.
This is not automatically an authentication bypass.
The code deliberately applies different checks to reads and writes.

**Conditional remediation:** If peers must not see settings, require capability or identity for `export_backup` and other sensitive reads.
Return only minimal public state from public RPCs.

<a id="fup-03--deep-link-non-associato-a-richiesta-pendente"></a>
### FUP-03 — Deep link without a pending request

**Code path:** `crates/ace-engine/src/protocol.rs:13-29,76-166` permits only `/account/api-key`.
`crates/ace-engine/src/bin/ace_redirect.rs:38-73` reads `apiKey` from the fragment or query.
It calls `setApiKey` with the local capability.
`crates/ace-engine/src/service/api/operations_account.rs:44-62` updates the credential.

**Existing protection:** The allowlist prevents a web URL from invoking other privileged actions.
**Unknown conditions:** Browser or OS consent, website login state, user intent, and actual account-replacement impact.
Do not claim that the callback can stop the service.
The route allowlist excludes that operation.

**Conditional remediation:** Use an app-generated one-time state/nonce with expiration.
Bind it to the account and initiated operation.
Reject unsolicited callbacks.
Test with two synthetic identities.

<a id="fup-04--righe-smisurate-nellimport-delle-liste"></a>
### FUP-04 — Excessive line length during list import

**New code path:** `crates/ace-engine/src/service/api/operations_site_pack.rs:42-79` accepts user-configured sources after a Pro entitlement check.
`crates/ace-engine/src/core/network_block/site_pack/importer/importer.rs:63-105` downloads a URL source or uses a file.
`crates/ace-engine/src/core/utils/file_download.rs:27-36,59-124` limits downloads, but the default maximum is **2 GiB**.
`importer.rs:140-161` uses `Vec<u8>` and `reader.read_until(b'\n', &mut line_buffer)`.
The reviewed code has no visible limit on one line.
`read_until` stores data before classifying the line.

**Possible effect:** A third-party source with a very long line can cause a large allocation during parsing.
This consumes service memory, not only download disk space.
No OOM test or service crash was observed.
Entitlement, capability, and download limits restrict exposure.
They do not independently prevent memory pressure from legitimate remote imports.

**Recommended remediation:** Define a format-based per-line limit, for example **8–64 KiB**.
Read in chunks. Reject long lines without retaining their full content.
Limit local-file content and add bounded synthetic regression tests.
Do not rely only on a global RAM limit after parsing.

<a id="fup-05--limiti-di-sessione-ipc-non-espliciti-nel-wrapper"></a>
### FUP-05 — No explicit IPC session limits in the wrapper

**Code path:** `crates/ace-engine/src/service/api/transport.rs:29-62` calls `accept()` and creates one `tokio::spawn` task per connection.
It then performs the remoc handshake.
The shown code has no explicit application-level session limit or timeout.
The Linux socket uses `0666` permissions (`:66-90`).
A local peer can begin a session before method-level capability verification.

**Unknown conditions:** Effective remoc, named-pipe, and OS limits; idle cleanup; cost per session; impact on enforcement.
No saturation or stress test ran on a shared host.

**Hardening and tests:** Measure a small number of dummy clients in an offline VM under strict limits.
Add session limits, handshake/idle timeouts, and prompt cleanup if measurements show unwanted sustained growth.

<a id="controlli-difensivi-osservati"></a>
## Observed defensive controls

- The Tauri wrapper denies `shutdown` before reading capability (`apps/ace-app/src-tauri/src/lib.rs:41-69`). The engine retains Lock gates and the independent watchdog.
- RPC compares digests in constant time (`crates/ace-engine/src/service/api/security.rs:3-14`). Production initializes `Some(Settings::load_or_create_capability_token())` with enforcement enabled (`crates/ace-engine/src/lib.rs:350-359`). The `expected=None` case in `rpc.rs:364-366` fails open in code, but the reviewed release constructor does not establish reachability. **This is hardening, not a confirmed finding.**
- The database ZIP importer rejects path traversal and enforces a cap (`crates/ace-engine/src/core/network_block/site_pack/mod.rs:342-409`). Script updates check SHA-256 and extraction limits (`crates/ace-engine/src/service/content_updater.rs:364-411`).
- The Windows Named Pipe sets `accept_remote(false)` (`transport.rs:29-36`). The installer contains maintenance/uninstall gates (`installer/installer.wxs:353-379,419-437`).

<a id="hardening-non-classificato-come-vulnerabilità"></a>
## Hardening not classified as a vulnerability

Tauri config sets `csp: null` and `withGlobalTauri: true` (`apps/ace-app/src-tauri/tauri.conf.json:3-17`).
`apps/ace-app/src-tauri/capabilities/default.json` documents the filesystem scope.
Review XSS and origins before release, but no reachable XSS was demonstrated.
Consider per-operation restrictions in the generic bridge, CI action pinning, independent bundle authenticity, and lockfile CVE analysis.
Review `std::env::temp_dir()/AceBlocker/temp_sites` (`settings.rs:116-127`) for Windows ownership and reparse points.
No exploitable path squatting was demonstrated.

<a id="coverage-esecuzione-e-limiti"></a>
## Coverage, execution, and limits

- **Compared source refs:** Previous audit `17c7e36a53374b3d07fb9eaef4f6120cdbbbe9e6` and current ref `ad1963ebaa8a63cb41fc07cea436e6a7d67ea53f`. Only documentation changes were observed between them. The previous application code paths did not change.
- **Coverage ledger:** 21 major units/surfaces; **0 covered, 0 independently verified candidates, 0 blocked, 21 deferred**. Priority and manual static review do not qualify a unit as `covered`.
- **Delegations:** Four independent NovaGPT reconnaissance attempts were blocked by a manual browser inspection request. No acceptable results were received: 0 hunter, 0 critic, 0 verifier. The complete standard workflow did not finish.
- **Execution:** No target process, build, test, Windows service, real endpoint, network attack input, runtime MSI/ACL check, or current advisory CVE search. No Windows VM was proven isolated under the skill requirements.
- **Structural validation:** The official `validate-coverage-ledger.cjs` validator fails in the available Windows runtime because safe JSON opening lacks no-follow/nonblocking primitives. An in-memory `validateDocument` check is semantic only. It cannot replace the official validator.
- **Out of scope:** The cloud website/account `ace-blocker-site`, Android app, external `ace-hosts` pipeline, remote content sources, production infrastructure, and other sibling repositories.

**Operational decision:** Do not approve go-live based on this run.
First verify FUP-01/02/03 on a disposable multi-user Windows VM.
Then investigate FUP-04/05 in a confined harness.
Complete independent audits of every RPC entrypoint and FFI/TUN, TLS, installer, and supply-chain modules.
Use synthetic data. Do not contact production.

**Run files:** `C:\Projects\temp\ace-blocker-security-audit-run2\` (technical artifacts outside the repository, as required by skill isolation).
This document is the copy intended for `ace-blocker/docs`.
