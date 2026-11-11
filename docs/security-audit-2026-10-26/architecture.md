# Architecture and trust boundaries — Ace Blocker

Local static review of commit `17c7e36a` on October 26, 2026. The review did not connect to public endpoints or active installations.

## Main components

- **Engine** (`crates/ace-engine`): Runs as a privileged LocalSystem service on Windows and a systemd service on Linux. It manages SQLite, blocking rules, locks, schedules, remoc RPC, content updates, and TLS interception.
- **Watchdog** (`crates/ace-watchdog`, `ace-watchdog-protocol`): Runs as an independent service. It monitors the engine, checks PIDs through SCM or systemd, and controls maintenance.
- **App** (`apps/ace-app`): Uses SvelteKit 5 in a Tauri 2 WebView. Its Rust bridge connects to IPC. JavaScript components must not receive the capability token.
- **Proxy/TUN** (`crates/mitm-proxy`, `ace-tun`, `ace-domain-blocklist`): Handles TCP forwarding, interception of browser traffic only, a locally generated CA, network policy, and Windows/Linux FFI.
- **Installer/CI** (`installer`, `.github/workflows`): Handles WiX/MSI packages, service startup and ACLs, the ProgramData directory, trusted certificates, offline packs, and releases.
- **Outside this repository**: The Ace account website, Android app, boredom-engine script sources, and ace-hosts pipeline. This review does not establish their security.

## Main trust boundaries

1. **Other local process → LocalSystem engine**: Windows pipe with a DACL or a Unix socket, remoc RPC, and capability checks for changes. Some read operations intentionally do not require a capability. Sources: `crates/ace-engine/src/service/api/{transport,rpc}.rs`.
2. **WebView → Tauri backend → engine**: Tauri Rust reads the token. The WebView cannot call `shutdown`. The bridge can forward other operation names. Source: `apps/ace-app/src-tauri/src/lib.rs`.
3. **Standard user → privileged data**: The MSI creates a shared directory with inherited read access for BUILTIN Users. Credentials, the database, and the CA key are in SharedData. Sources: `installer/installer.wxs`, `crates/ace-engine/src/settings.rs`.
4. **External web page → protocol handler**: The handler accepts `aceblocker://account/api-key`. It can call `setApiKey` with the local token. Request-intent and account-linking checks need validation. Sources: `crates/ace-engine/src/{protocol.rs,bin/ace_redirect.rs}`.
5. **Internet → proxy**: The proxy forwards connections and intercepts TLS conditionally. Only debug builds disable upstream certificate verification. Some content downloads use SHA-256 checks and bounded ZIP extraction.
6. **Installer privileges → services**: Custom actions use `Impersonate=no`. The installer has lock and maintenance gates. The review needs a Windows VM to verify their runtime behavior.

## Method and limits

The review inventoried all 714 tracked Git files. It searched `apps/`, `crates/`, `installer/`, `.github/`, and the documentation. It directly examined high-risk paths. This work was not a semantic review of every line.

The reviewers found no compatible prior-run ledger in `docs/`. They did not treat existing refactoring audits as earlier security audits.

The reviewers loaded the `security-audit` skill. They did not use subagents because the user explicitly requested this. They did not have a verified OS-enforced sandbox. They did not run target-controlled tests.

Therefore, **no record meets the skill's `confirmed` criteria**. All candidates remain `needs_validation`.
