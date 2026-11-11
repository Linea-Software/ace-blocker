<a id="ace-blocker--security-audit-statico-26-ottobre-2026"></a>
# Ace Blocker — static security audit (October 26, 2026)

> **Run result: INCOMPLETE (security-audit skill, `standard` profile).**
> This is a **static, risk-focused audit of the unified repository**.
> It is not a security certification or end-to-end verification.
> Reviewers examined the most sensitive surfaces and inventoried all 714 tracked files.
> They did **not** read every line of every file.
> The four independent reconnaissance delegations required by the skill failed because the NovaGPT backend was unavailable.
> Independent hunters, verifiers, and critics are therefore missing.
> No target executables, tests, installers, drivers, proxies, or services ran.
> The OS-enforced sandbox guarantees required by the skill were not demonstrated.
> No item below is `confirmed` or assigned formal severity without independent verification.

**Repository:** `C:\Projects\ace-project\ace-blocker`
**Reviewed commit:** `17c7e36a53374b3d07fb9eaef4f6120cdbbbe9e6` (`main`, working tree clean before the audit)
**Scope:** `apps/ace-app`, `crates/ace-engine`, `crates/ace-watchdog`, `crates/ace-watchdog-protocol`, `crates/ace-tun`, `crates/mitm-proxy`, `crates/ace-domain-blocklist`, `installer`, `.github/workflows`, and repository configuration/documentation.
**Excluded:** External website/account and cloud APIs (`ace-blocker-site`), Android app, `ace-hosts` pipeline, external `boredom-engine` script sources, and real production/provider configuration.
**Skill:** `ace-project/.agents/skills/security-audit/SKILL.md` with desktop/IPC, RPC, memory safety, client/browser, web, release/supply-chain, and availability companions.
**Execution:** Source inspection and inventory/search commands only. No real endpoints or user data were accessed. No offensive reproduction was attempted.

<a id="sintesi-esecutiva"></a>
## Executive summary

The highest observed risk is the **boundary between ordinary Windows processes and LocalSystem components**.
The MSI configuration might let **standard users read** the folder that contains the IPC token.
That folder also contains the cloud API credential in the database and the TLS interception CA private key.
A client with the token could send remoc requests normally restricted to the app.
The engine's separate Lock rules still apply. Do **not** assume they can be bypassed.
The reviewed code also exposes a **backup-export RPC without a token requirement**.

These leads require **priority verification before release** on a clean install with separate Windows identities.
Do not claim that installed file ACLs actually grant access until you inspect them.
Do not claim that a privilege escalation has been reproduced.

<a id="registro-delle-verifiche-prioritarie"></a>
### Priority verification register

| ID | Verification priority | Status | Surface | Question to resolve |
| --- | --- | --- | --- | --- |
| AB-01 | Before release | `needs_validation` | MSI / capability / IPC | Can other users read the installation token and cloud credential and reuse the token through the pipe? |
| AB-02 | Before release | `needs_validation` | MITM CA / filesystem | Can standard users read the trusted root CA private key? |
| AB-03 | Before release | `needs_validation` | remoc RPC / backup | Can another local identity export profile data and preferences without a token? |
| AB-04 | Before release | `needs_validation` | URI scheme / account | Can an `aceblocker://` callback replace the API key without a pending login? |
| AB-05 | Later, before release | `needs_validation` | Named Pipe / lifecycle | Can unlimited local connections consume service resources? |
| AB-06 | Later, before release | `needs_validation` | Site-pack import | Can LocalSystem process URL/file sources beyond the intended user authority? |

**Vulnerabilities formally confirmed through the complete skill workflow: 0.**
This number means **validation did not finish**. It does not mean the product has no vulnerabilities.

<a id="confini-di-fiducia-osservati"></a>
## Observed trust boundaries

1. **Windows user → Tauri WebView → Rust backend → remoc:** The frontend sends `engine_call(op,args)` to native code. Native code reads `capability.token` and attaches it to RPC requests. The WebView does not receive the token. The WebView cannot request `shutdown` (`apps/ace-app/src-tauri/src/lib.rs:25-68,107-113`).
2. **Other local processes → LocalSystem pipe:** The Windows pipe accepts local authenticated users with `accept_remote(false)`. The Linux engine socket uses `0666` (`crates/ace-engine/src/service/api/transport.rs:17-36,66-80`). Capability checks run per method, not during connection setup (`crates/ace-engine/src/service/api/rpc.rs:359-371`).
3. **LocalSystem Engine → database and filesystem:** Backups, settings, scripts, and certificates reside in app data. They remain sensitive when the desktop app is closed.
4. **Browser → TUN → HTTPS proxy:** Only browser traffic may be decrypted. Normal configurations enable upstream TLS verification. The proxy creates its CA and installs the root into the trust store (`crates/ace-engine/src/service/proxy/mod.rs:578-590,676-692,897-919`).
5. **MSI → SCM/watchdog:** Two LocalSystem services have recovery policies and service ACLs. Uninstall requires a Lock check and shutdown with a maintenance lease (`installer/installer.wxs:86-153,373-379,419-436`, `crates/ace-engine/src/service/api/operations_system.rs:105-149`).

<a id="ab-01--file-del-token-ipc-e-acl-ereditabili"></a>
## AB-01 — IPC token file and inherited ACLs

**Static evidence:** The MSI sets SDDL `D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;BU)` on `AceProgramDataDir`.
Here `BU` means Users, `GRGX` includes read access, and `OICI` enables inheritance (`installer/installer.wxs:209-215`).
`Settings::get_capability_token_path()` puts the token under `SharedData` (`crates/ace-engine/src/settings.rs:144-165`).
`load_or_create_capability_token` generates 32 random bytes and writes them with `std::fs::write`.
The reviewed path shows no ACL override (`settings.rs:168-190`).
In release, `AppState` uses the token with enforcement enabled (`crates/ace-engine/src/lib.rs:353-359`).
Tauri reads it from the same storage (`apps/ace-app/src-tauri/src/lib.rs:25-38`).
The server compares it in constant time for protected calls (`crates/ace-engine/src/service/api/rpc.rs:359-371`, `security.rs:3-14`).

**Hypothesized boundary:** Another `BU` local account might read the file and send modifying RPCs with the token.
This does **not** automatically bypass application Locks.
The engine handlers still enforce their own policies.

**Another secret in the same directory:** The main database resides under `SharedData` (`crates/ace-engine/src/settings.rs:93-95`).
`crates/ace-engine/src/db/account_repo.rs:19-32` writes `accounts.api_key` as a SQLite string.
`crates/ace-engine/src/service/clients/ace_server_client.rs:9-20` uses it as an API credential.
If `BU` can also read the database, the exposure includes a cloud credential separate from the IPC token.
Verify this risk with **ACLs and synthetic data**.
Do not extract real credentials.

**Validation blocker:** Inspect effective file DACLs after service creation.
Check explicit and inherited ACEs for the token and database, the owner, and installation profile.
Check whether a second account can connect to the pipe and authorize *one harmless operation* on dummy data.

**Safe verification:** Use a **disposable Windows VM** and test accounts.
Inspect **only ACLs** on the directory, token, and database with `icacls` or `Get-Acl`.
Compare access from a second standard account.
Use an isolated dummy token and resource for one nondestructive RPC.
Never copy, log, or expose real credentials.

**Suggested minimum fix:** Define explicit ACLs on the **file** and verify ACEs after creation.
Isolate authorized user SIDs.
Tauri runs as a normal user and must read the token.
Do **not** simply restrict the token to SYSTEM and break IPC.
Instead, design capability issuance/rotation for authorized identities or secure peer-identity binding in the engine.
Do not share one global credential across all users.
Add real ACL tests with multiple accounts.

<a id="ab-02--chiave-privata-della-ca-mitm-nello-stesso-albero-dati"></a>
## AB-02 — MITM CA private key in the same data tree

**Static evidence:** Certificates reside in `Settings::get_app_dir()/certs` (`crates/ace-engine/src/settings.rs:97-99`).
The service generates or manages `mitm_ca.pem` and `mitm_ca.key` (`crates/ace-engine/src/service/proxy/mod.rs:897-915`).
`CertAuthority::load_or_generate` writes the **private key** with `fs::write(key_path, &key_pem)`.
The reviewed path shows no file-specific DACL (`crates/mitm-proxy/src/ca.rs:199-223`).
The parent directory follows the AB-01 MSI policy.

**Hypothesized boundary:** If `BU` inherits read access to `mitm_ca.key` and the public root is trusted on the device, an unauthorized local user can read the signing key.
Actual traffic interception also requires traffic control and the relevant client trust configuration.

**Validation blocker:** Inspect actual DACLs on `certs` and `mitm_ca.key`.
Verify root-store installation, other ACL management, and differences between trust stores.

**Safe verification:** Use `Get-Acl` or `icacls` to inspect only permission metadata in a fresh VM.
Use a **test CA**. Inspect trust stores without exporting keys.
Do not intercept real traffic.

**Suggested minimum fix:** Create the CA key file atomically with explicit **SYSTEM + Administrators** permissions.
Allow only other strictly necessary service SIDs.
Verify access before installing the trusted root.
Migrate or regenerate safely if an older key is readable.
The Tauri client **must not** access the private key.
Automate ACL and trust-store tests on Windows VMs.

<a id="ab-03--rpc-di-lettura-sensibile-privo-di-capability"></a>
## AB-03 — Sensitive read RPC without capability

**Static evidence:** On Windows, the pipe grants read/write to `AU` (`crates/ace-engine/src/service/api/transport.rs:17-36`).
The Linux socket uses `0666` (`:66-80`).
`check_capability(presented, required)` succeeds immediately when `required == false` (`crates/ace-engine/src/service/api/rpc.rs:359-371`).
`export_backup` passes `false` and exports without additional identity checks in the reviewed path (`rpc.rs:438-447`; `operations_backup.rs:6-12`).
`EngineBackup` includes `network_blocks`, `custom_site_packs`, and `deactivated_resistance_scripts`.
Each Network Block includes its name, schedule, user rules, and attached site packs (`crates/ace-engine/src/manager/network/backup.rs:48-67,71-165`).
`get_all_network_blocks`, `get_user_rules`, and `get_account_info` also pass `false` (`rpc.rs:376-386,822-830,1360-1371`).

**Hypothesized boundary:** Another local account admitted by the OS might read another user's Ace rules and preferences without a token.
The source alone does not establish whether the product intentionally shares this information across machine users.

**Validation blocker:** Establish the multi-user privacy model and actual Windows pipe ACLs.
Verify `remoc` broker behavior on each platform with dummy data.

**Safe verification:** Use an isolated VM with two dummy accounts and a synthetic backup.
Call `export_backup(None)` through a local remoc client.
Check only for a synthetic marker in the result.
Do not inspect real users' backups.

**Suggested minimum fix:** Authorize reads of personal data, including backups, Network Blocks, user rules, account information, and dashboard data.
If necessary, keep only `ping` and version anonymous.
Distinguish pipe connection access from data authorization.
Test `None`, an incorrect token, a valid token, and a different SID.

<a id="ab-04--account-key-deep-link-non-associato-a-una-richiesta-di-login"></a>
## AB-04 — Account-key deep link without a pending login

**Static evidence:** The `aceblocker://` protocol allows only `/account/api-key` (`crates/ace-engine/src/protocol.rs:23-33,91-176`).
The receiver reads `apiKey` from the URL fragment or query and calls authenticated `setApiKey` (`crates/ace-engine/src/bin/ace_redirect.rs:23-72`).
The operation updates credential storage and verifies Pro status (`crates/ace-engine/src/service/api/operations_account.rs:44-63`).
The reviewed paths show no binding to an app-generated login nonce or state.

**Hypothesized boundary:** An untrusted website could persuade a user to open a URI that binds Ace to another account credential.
This can cause identity confusion and affect backups or synchronization.
Browser confirmation, link origin, and server token behavior are outside this repository.

**Validation blocker:** Review website login, browser UX, API-key properties, and actual ability to replace a session without a valid login challenge.

**Safe verification:** Use two synthetic identities and a test browser in a VM.
Observe whether the engine accepts an unexpected callback.
Do not use real cloud accounts.

**Proposed fix if confirmed:** Bind callbacks to an app-generated, one-time state/nonce.
Set expiry and bind the initiated operation.
Require explicit confirmation for account changes.
The current route allowlist is a useful defense, but it does not bind a session.

<a id="ab-05--accettazione-concorrente-di-client-remoc-senza-quota-evidente"></a>
## AB-05 — Concurrent remoc clients without a visible quota

**Static evidence:** Each accepted connection gets a `tokio::spawn` task with `remoc::Cfg::default()`.
The reviewed file shows no global connection limit (`crates/ace-engine/src/service/api/transport.rs:29-63`).
Local clients can open the pipe before capability verification.

**Hypothesized boundary:** An unauthorized process might consume privileged service resources through idle connections.
The review did not measure resource use, internal remoc mitigations, or actual service impact.

**Safe verification:** Run a **small, bounded, monitored** number of synthetic clients on a disposable VM.
Apply OS limits for CPU, memory, processes, and time.
Stop at the first adverse effect.
Never test against a shared service.
Possible controls include a connection semaphore, handshake/idle timeouts, payload limits, and metrics.

<a id="ab-06--sorgenti-url-e-file-usate-da-un-engine-privilegiato"></a>
## AB-06 — URL and file sources handled by a privileged engine

**Static evidence:** `create_site_pack` accepts user sources and checks Pro entitlement (`crates/ace-engine/src/service/api/operations_site_pack.rs:43-80`).
The RPC creation call requires capability (`rpc.rs:1124-1132`).
The importer passes a URL to `download_file_with_limit` or opens a configured `File` path (`crates/ace-engine/src/core/network_block/site_pack/importer/importer.rs:58-105,124-135`).
The reqwest downloader uses 10/60-second timeouts and a 2 GiB limit (`crates/ace-engine/src/core/utils/file_download.rs:27-49,59-124`).
The production engine runs as a privileged service.

**Hypothesized boundary:** Legitimate import might access local resources or network addresses unavailable to the calling user.
No data exposure, cross-identity SSRF consequence, or Pro/capability bypass was demonstrated.
Do not treat an intentional user file import as a vulnerability by itself.

**Validation blocker:** Determine permitted destinations, protocols, redirects, process privileges, data visibility, and infrastructure limits outside this repository.
Test only with dummy files and an isolated loopback server.
Do not target internal or production services.
If the boundary is real, restrict `File` to authorized origins.
Restrict URLs to permitted protocols and destinations.
Recheck redirects.

<a id="hardening-senza-vulnerabilità-dimostrata"></a>
## Hardening without a demonstrated vulnerability

- **WebView:** `apps/ace-app/src-tauri/tauri.conf.json` uses `app.security.csp: null` and `withGlobalTauri: true`. The frontend can call native functions with app capabilities, except shutdown. Consider a restrictive CSP, command/operation allowlist, and HTML input review. **No XSS path was demonstrated.** Do not classify these settings alone as a vulnerability.
- **Secret generation:** The token is written with `std::fs::write`. Consider fail-closed persistence and exclusive/atomic creation to prevent unintended initial permissions after reinstall or with existing files. No exploitable race was demonstrated.
- **Content delivery:** Script SHA-256 verification is useful, but hashes and URLs come from server metadata. Independent signatures, key pinning, or signed manifests are threat-model decisions, not proven incidents (`crates/ace-engine/src/service/content_updater.rs:320-409`).
- **Dependencies and supply chain:** `Cargo.lock` and `apps/ace-app/bun.lock` exist. **No current CVE check** or advisory database scan was performed. The release pipeline uses some tagged GitHub Actions (`.github/workflows/release.yml`) and creates an SHA-256 manifest. Consider commit-pinned actions, attestations, and real MSI signing. Hashes alone do not prove authenticity.
- **Unsafe/FFI:** WinTun, Windows networking, Linux/macOS TUN, and packet/process functions contain `unsafe` code. The inventory located these areas. **No memory corruption proof** or fuzzing was performed. Run parser fuzzing, instrumentation, and ABI tests in a dedicated sandbox.
- **Installer policy:** At `installer/installer.wxs:419-436`, the Lock check precedes maintenance preparation. An adjacent comment states the reverse. Align the comment, design, and actual MSI test results. Do not assume a working bypass.

<a id="controlli-positivi-osservati-nel-sorgente"></a>
## Positive controls observed in source

- No alternate HTTP listener exists for the declared control plane. It uses native remoc IPC and Windows `accept_remote(false)` (`docs/IPC_ARCHITECTURE.md`, `transport.rs:29-36`).
- Modifying RPC methods check capability per method. `tokens_match` uses SHA-256 digests and `subtle::ConstantTimeEq` (`rpc.rs:359-371`, `security.rs:3-14`).
- The UI cannot request `shutdown` through `engine_call`. The engine rechecks Locks and requires a watchdog lease before shutdown (`apps/ace-app/src-tauri/src/lib.rs:44-58`, `operations_system.rs:105-149`).
- Sensitive handlers verify Lock and Pro authorization independently of the frontend (`operations_site_pack.rs:43-80,83-128`).
- The downloader has timeouts and size limits. ZIP extraction validates path components and total size (`crates/ace-engine/src/core/utils/file_download.rs:27-49,219-297`).
- Dangerous `debug` operations are compile-gated, and release enables capability enforcement (`crates/ace-engine/src/service/api/rpc.rs:510-560`, `crates/ace-engine/src/lib.rs:353-359`).
- The separate Linux watchdog socket has restrictive permissions (`crates/ace-watchdog/src/linux.rs:350-369`).

<a id="copertura-e-lacune"></a>
## Coverage and gaps

| Component | Static analysis performed | Critical gap |
| --- | --- | --- |
| `ace-engine` API/IPC/authorization | Tauri → remoc → handler paths, token, backup, and system operations | Real multi-user clients and permission matrix for 63 calls |
| `ace-engine` Lock/schedule/backup | Lock, shutdown, and import contracts/flows; existing tests inventoried | State-machine tests, fuzzing, differential restore, and races |
| `ace-watchdog` and protocol | Architecture, permissions, and installer integration reviewed | Real VM heartbeat/recovery/SCM tests with service failures |
| `mitm-proxy` and CA | Key generation/storage, TLS interception, and trust policy | Root-key ACLs, TLS/certificate chain, and browsers with client certificates |
| `ace-tun`, networking, `ace-domain-blocklist` | `unsafe`/FFI inventory, routing, and input trust boundaries | Packet-parser fuzzing, crash containment, VPN enforcement, and drivers |
| `ace-app` Svelte/Tauri | IPC bridge, capability, CSP, native commands, and capabilities | Full XSS review of all routes and native WebView tests |
| `installer` WiX | DACL, SCM, recovery, root certificate, uninstall gates, custom actions | Real MSI repair/upgrade/uninstall and clean-install file permissions |
| `.github/workflows`, dependencies | Release pipeline, permissions, lockfiles, and manifests | CVE database, provenance attestation, and supply-chain verification |
| External cloud website/account | Only desktop call sites and contracts are visible | **Outside repository:** separate audit needed |

**Run metrics:** 714 tracked files inventoried; 4 reconnaissance delegations attempted and failed; 0 operational verifiers/critics/hunters; 0 target-code tests; 0 Windows/MSI runtime checks; 0 online advisory validations; 0 formally confirmed vulnerabilities.
No compatible earlier `coverage-ledger.json` was found for this run.
The required `standard` skill profile **did not finish**.
Coverage remains partial.

<a id="piano-di-chiusura-suggerito"></a>
### Suggested closure plan

1. **Block a public release** until AB-01/AB-02/AB-03 are resolved or disproved. Use ACL checks on a clean Windows VM with two separate local identities.
2. Add regression tests for ACLs, RPC reads without credentials, CA state, service restart, and uninstall gates. Use disposable offline VMs, synthetic datasets, and nonsensitive output.
3. Rerun the skill when independent NovaGPT reviewers and an OS-enforced sandbox are available. Complete reconnaissance, hunter waves, verifier, critic, `findings.json`, and both skill validators.
4. Plan separate reviews of the full frontend, `unsafe` modules, and cloud website. Do not automatically classify these gaps as confirmed bugs.

<a id="integrità-del-report"></a>
### Report integrity

This document is a **repository artifact**.
No application code changed, and no successful verification was simulated.
All `file:line` references point to the reviewed commit.
Unknowns remain explicitly marked `needs_validation`.
No tests ran against real users, services, or hosts.
