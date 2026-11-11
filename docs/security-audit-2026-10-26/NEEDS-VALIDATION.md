# Needs validation — defensive test plan

Do not assign a formal severity rating to these records. The skill requires independent evidence of a security violation before it permits a severity rating.

## AB-SA-01 — Secret data under ACLs readable by Users

- **Trace:** `installer/installer.wxs:210-214` → `crates/ace-engine/src/settings.rs:93-100,144-165` → `crates/ace-engine/src/db/account_repo.rs:19-32`, `crates/mitm-proxy/src/ca.rs:199-216`, `settings.rs:168-189`.
- **Source finding:** SharedData has an inheritable read permission for BUILTIN Users (BU). The source does not show separate ACLs for the stored secrets.
- **Validation blocker:** The final DACLs are unknown. The review did not verify protected ACE entries on real copies of the files. No isolated Windows VM was available.
- **Safe local test:** Use a disposable Windows VM with **dummy keys**. Install a test MSI. Use two temporary local accounts. Inspect only directory, database, capability-file, and CA-key DACL metadata. Do not read, copy, or export real keys. Check that Users cannot read the CA key or cloud credential. Check that the authorized UI still works.
- **Release-owner validation:** Add an automated ACL test before release. Test new files and upgrades from earlier installations. Confirm root CA trust and a key-rotation plan.

## AB-SA-02 — Read-only IPC without a capability

- **Trace:** `crates/ace-engine/src/service/api/transport.rs:25-80` → `rpc.rs:438-447,822-831` → `manager/network/backup.rs:72-163`.
- **Source finding:** `export_backup` does not require a capability token. The pipe permits AU connections. The Unix socket mode is `0666`.
- **Validation blocker:** The product has no documented confidentiality policy for settings across users of the same computer. The review did not verify RPC runtime behavior.
- **Safe local test:** In a VM, make a dummy block named `dummy-private-block`. Use two temporary local accounts. Check only whether a process without a capability receives that dummy string from `export_backup`. Do not include real user habits or lists.
- **Release-owner validation:** Approve a formal read-access model for each installation or user. Add tests that enforce the selected policy.

## AB-SA-03 — Account callback without request correlation

- **Trace:** `crates/ace-engine/src/protocol.rs:11-20,76-153` → `crates/ace-engine/src/bin/ace_redirect.rs:38-72` → `crates/ace-engine/src/service/api/operations_account.rs:44-62`.
- **Source finding:** The client allows a specific URL path. It does not check a pending login nonce or state. It forwards the key from the URL.
- **Validation blocker:** The external website and authentication flow are outside this repository. Browser and OS confirmation behavior is unverified. The review did not demonstrate actual harm.
- **Safe local test:** Use an isolated computer and browser with two synthetic accounts. Control and record app-opening consent. Check whether an unsolicited link changes the key binding when no login started in the app. Do not use real accounts.
- **Release-owner validation:** Test retries, replay, and old callbacks. If the risk is confirmed, design end-to-end, one-time request correlation.

## Other areas that need validation

The review did not validate watchdog recovery and races, non-browser proxy and tunnel traffic, FFI and WFP policies, update-manifest integrity, dependency CVEs, MSI rollback and upgrades, or service permissions.

Run runtime tests only in an OS-enforced sandbox. Deny external network access. Use an environment-variable allowlist, an isolated file system, and resource limits.
