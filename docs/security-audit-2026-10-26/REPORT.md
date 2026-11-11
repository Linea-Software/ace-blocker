# Security audit — Ace Blocker

**Date:** October 26, 2026 · **Commit:** `17c7e36a53374b3d07fb9eaef4f6120cdbbbe9e6` (`main` was clean before the changes).

**Scope:** The full unified repository (`apps`, `crates`, `installer`, and CI). External repositories were out of scope.

**Requested profile:** `standard` · **Status:** **INCOMPLETE — first static review** · **Subagents:** 0 (explicit user request).

**Work performed:** Reviewed source files, manifests, and documents. Did not run tests or builds. Did not connect to real services, perform network attacks, or use a Windows VM.

**Previous reviews:** No compatible prior security-audit ledger was found. No independent skill validator ran.

## Summary of results

The review does not establish that the app is secure or ready for release. Static analysis identified **three hypotheses supported by source code**. One hypothesis concerns the same access-control boundary for **cloud credentials, control tokens, and the TLS CA private key**.

The review did not establish runtime behavior, the intended authorization policy, or independent verification. No finding meets the skill's `confirmed` criteria. The descriptions below do not prove an exploit on an installed system.

| ID | Status | Affected boundary | Possible result | Next action |
| --- | --- | --- | --- | --- |
| AB-SA-01 | Needs validation | Privileged files → local users | Read API key, capability token, or CA private key | Check the ACLs on all three files after an MSI installation in a VM |
| AB-SA-02 | Needs validation | Local client without capability → read-only RPC | Export personal block settings | Define the confidentiality boundary and check a dummy RPC call |
| AB-SA-03 | Needs validation | External site → deep link → account linking | Unintended account change after URL opening | Test with synthetic accounts, an isolated browser, and known consent |

## AB-SA-01 — Possible exposure of secrets through ProgramData ACLs

**Primary source:** `installer/installer.wxs:210-214` creates the data directory with `(A;OICI;GRGX;;;BU)`. This ACE gives BUILTIN Users inheritable read and execute access. `crates/ace-engine/src/settings.rs:93-100,144-165` places the database, `certs/`, and capability under SharedData. `crates/mitm-proxy/src/ca.rs:199-216` writes the **CA private key** PEM. `crates/ace-engine/src/db/account_repo.rs:19-32` stores `api_key` as text in SQLite. `crates/ace-engine/src/service/clients/ace_server_client.rs:9-20` sends this value as `x-api-key`. `crates/ace-engine/src/settings.rs:168-189` writes the capability token as text.

**Hypothesis:** Other standard local users might read sensitive data if the files inherit the BU ACE. The capability token permits changes through RPC. The API key authenticates to the website. The CA private key signs certificates used for TLS interception. These are secrets, not ordinary debug data.

Source review alone does not establish the effective ACLs on an installed system. It also does not prove that another process can use these secrets in practice.

**Proposed minimum fix:** Store secrets separately from data that users can read. Set an explicit DACL on the secret directory and new files. Grant SYSTEM and Administrators access. Give the Tauri client only the access it needs through a strictly authorized broker or a per-user capability. Do not make the token inaccessible to the authorized client. Apply exclusive ACLs to the CA key. Create files without a temporary window of broad access. Migrate or rotate existing keys and tokens after the fix. Test ACLs and multiuser behavior in a dedicated VM.

## AB-SA-02 — RPC export without explicit caller authentication

`crates/ace-engine/src/service/api/rpc.rs:438-447` permits `export_backup` with `check_capability(..., false)`. The response contains network blocks, source lists, schedules, and deactivated scripts (`crates/ace-engine/src/manager/network/backup.rs:72-163`).

The same rule applies to `get_all_network_blocks` (`rpc.rs:822-831`). The Windows named pipe includes `AU` (`transport.rs:25-42`). Linux sets the socket mode to `0o666` (`transport.rs:66-80`). These read operations **intentionally** do not require a capability. The review did not prove a bypass.

**Unresolved policy:** Does the product let *all* local users read every block setting on the computer? Or are block names, rules, and schedules private to the user who configured them?

**Conditional fix:** If the settings are private, require a capability or caller-identity check for sensitive reads. Alternatively, separate public status data from backups and rules.

## AB-SA-03 — Account-linking deep link without visible request correlation

`crates/ace-engine/src/protocol.rs:11-20,27-35,76-153` notes that any website can present an `aceblocker://` link. The browser may ask for confirmation. The handler permits only `/account/api-key`. It does not check a nonce or state for a pending login request.

`crates/ace-engine/src/bin/ace_redirect.rs:38-72` extracts `apiKey`. It uses the capability to call `setApiKey`. `crates/ace-engine/src/service/api/operations_account.rs:44-62` replaces the local key.

This could be **account-linking CSRF or account swapping**. The review did not confirm this.

**Missing evidence:** The review did not check browser behavior or whether users must confirm the action. It did not assess the quality of that consent. It did not inspect website-side correlation because the website is in another repository. It did not prove an impact with a key from another account.

**Conditional fix:** Start linking from the app. Use a short-lived, one-time nonce tied to the user's request. Reject unsolicited callbacks.

## Positive controls observed in source code

- RPC change operations check the capability (`rpc.rs:359-371`). Digest comparisons take constant time (`service/api/security.rs:3-13`). Note: The `None` expected-token branch fails open and should fail closed. The visible runtime constructor passes `Some` (`lib.rs:350-359`). Therefore, the review **did not classify this as a reachable vulnerability**.
- Tauri rejects `shutdown` from the WebView before it reads the capability (`apps/ace-app/src-tauri/src/lib.rs:41-63`).
- Lock checks protect block disabling (`manager/network/network_blocks.rs:88-118`) and deletion (`:143-159`). Backup restore skips locked blocks (`manager/network/backup.rs:273-278`).
- The Windows pipe SDDL uses `accept_remote(false)` (`service/api/transport.rs:16-46`). The separate watchdog has a more restrictive ACL.
- Only debug builds disable upstream TLS verification (`service/proxy/mod.rs:686-695`).
- Pack ZIP import limits size and checks path traversal (`core/network_block/site_pack/mod.rs:345-409`). Script updates check SHA-256 and limit download size (`service/content_updater.rs:364-401`).
- The installer checks locks before removal. It also requests authorized maintenance (`installer/installer.wxs:353-379,419-437`).

## Coverage gaps and validation plan

**Not performed:** Rust, Bun, or WiX builds or tests; a current CVE advisory audit; Windows/Linux IPC, ACL, or SCM runtime tests; real browser or certificate tests; TUN, parser, or proxy fuzzing; independent critic/verifier review; or skill-schema validation.

The review included the entire codebase in its inventory and plan. It **did not review all code exhaustively**. FFI, networking, update behavior, and simultaneous operations remain coverage gaps.

Recommended validation order:

1. Check ACLs and secrets in a disposable MSI installation.
2. Decide the multiuser IPC confidentiality policy.
3. Test account linking with two synthetic accounts.
4. Run isolated tests of the update pipeline, proxy/TUN, watchdog, and installer.

Do not test third-party systems or production installations.

### Recorded coverage summary

The ledger contains **12 units** across engine/RPC, secrets, locks/backup, deep links, Tauri, watchdog, proxy, TUN/FFI, updates, releases, blocklists, and frontend.

Final status: **0 covered, 0 verified candidates, 0 blocked, 12 deferred, and 0 out_of_scope within the repository**. External repositories were excluded from the target definition. No independent coverage critic ran because the review did not use subagents.

These numbers do not represent an exhaustive check. They describe the incomplete audit program.

See `NEEDS-VALIDATION.md` for noninvasive test procedures. See `FINDINGS-DETAIL.md` for the absence of confirmed findings. See `coverage-ledger.json` for the incomplete coverage units.
