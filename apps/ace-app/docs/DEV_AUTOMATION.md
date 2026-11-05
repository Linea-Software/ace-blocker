# Native UI inspection with WebdriverIO

ACE uses the WebdriverIO Tauri service to interact with its real desktop WebView, including native Tauri APIs and the normal, policy-checked engine IPC. A plain browser on `localhost:1420` does not provide that IPC.

## Run

From `ace-blocker/apps/ace-app`:

1. Install dependencies with `bun install --frozen-lockfile`.
2. Launch the usual debug app with `bun run tauri dev`. On Linux it also builds and registers the account callback receiver before starting Vite on port 1420. This builds `target/debug/ace-app` as usual.
3. In another terminal, run `bun run test:native`. WDIO launches a **second** copy of the debug binary using its embedded WebDriver driver.
4. Review the smoke test in `e2e/native/read-only.smoke.spec.ts`. It navigates between screens and saves a screenshot as `ace-native-ui-smoke.png` in the system temp directory.

The wiring lives in `wdio.conf.ts`, `src/hooks.client.ts`, `src-tauri/src/lib.rs` and `src-tauri/capabilities/default.json`.

## Linux browser login callback during development

The public website returns sign-in to ACE with an `aceblocker://account/api-key`
URL. The Linux release installer registers a handler under `/opt/ace-blocker`,
but the development Vite startup now builds the debug receiver and registers
it automatically as the **desktop user** when started via `bun run tauri dev`.
If registration fails, Vite stops with an explicit error instead of leaving the
browser sign-in callback broken. No root privileges are requested.

To re-register a checkout manually, run these commands as the **desktop user**,
not as root:

```sh
# From the ace-blocker repository root:
cargo build --locked -p ace-engine --bin ace-redirect
cd apps/ace-app
bun run auth:register-linux
xdg-mime query default x-scheme-handler/aceblocker
# Expected: ace-blocker-dev-handler.desktop
```

The helper registers a user-scoped `.desktop` entry that invokes the debug
`ace-redirect` binary. It intentionally uses an **unquoted, safe absolute
path** so that both GIO and the generic `xdg-open` fallback used by
Niri/Wayland resolve it correctly. It never overwrites a working production
handler, starts/stops services, or edits the engine database.

For a callback dispatch smoke test (safe: no key and no account mutation):

```sh
xdg-open 'aceblocker://account/invalid-test-route'
```

An exit status of 0 with a message that the route is *not in the forward
allowlist* is expected: the launcher was found and the engine callback
receiver correctly rejected an unsafe URL. Do **not** test with a made-up
`apiKey`; using the allowed endpoint with a key can change the actual
account.

**Separate authorization requirement:** the unprivileged callback receiver
must be allowed to read the installation capability from
`/usr/local/share/AceBlocker/capability.token`. If the helper warns that
this file is unreadable, apply the same restricted `ace-blocker` group
ownership/membership model as the release installer (with administrator
approval); do **not** make the token world-readable. The following is the
intended group setup for a disposable development host:

```sh
sudo groupadd --system --force ace-blocker
sudo chgrp ace-blocker /usr/local/share/AceBlocker /usr/local/share/AceBlocker/capability.token
sudo chmod 2750 /usr/local/share/AceBlocker
sudo chmod 0640 /usr/local/share/AceBlocker/capability.token
sudo usermod -aG ace-blocker "$USER"
# Log out of your desktop session and back in, then verify:
test -r /usr/local/share/AceBlocker/capability.token && echo 'ACE callback access OK'
```

Changing protected installation permissions must be a deliberate
administrator action; the registration helper will only diagnose this
condition. An auth callback still requires a running Engine and its existing
capability enforcement; registration alone does not prove a full login.

## Debug and release isolation

Rust registers `tauri-plugin-wdio` and `tauri-plugin-wdio-webdriver` only under `#[cfg(debug_assertions)]`; no release binary registers them. The guest JS adapter is imported only under Vite's `import.meta.env.DEV` and the Tauri environment, so a production bundle does not include it. No additional Cargo features, environment flags, Tauri configurations or HTTP engine interfaces are introduced.

The WebDriver server is started by the test runner, not by an ordinary debug session. It can execute code in the app WebView, so do not expose an active test session to remote clients. A test clicking a UI control can mutate real settings: use read-only smoke tests on the developer instance, disposable test profiles for mutation tests, and disposable VM validation for native security and lock behavior. WDIO does not bypass engine authorization, lock restrictions or watchdog protection.

If native tests fail after changing Rust code, rebuild the debug binary first. On Linux, WebKitGTK runtime dependencies may be needed.

References: https://tauri.app/develop/tests/webdriver/ and https://webdriver.io/docs/desktop-testing/tauri/plugin-setup .
