# Inspect the native UI with WebdriverIO

ACE uses the WebdriverIO Tauri service to test its actual desktop WebView.
Tests can use native Tauri APIs and the normal engine IPC that checks policies.
A browser at `localhost:1420` does not provide this IPC.

## Run the native smoke test

Run these steps from `ace-blocker/apps/ace-app`:

1. Install dependencies with `bun install --frozen-lockfile`.
2. Start the debug app with `bun run tauri dev`. On Linux, startup also builds and registers the account callback receiver before Vite starts on port 1420. The command builds `target/debug/ace-app`.
3. Open another terminal and run `bun run test:native`. WDIO starts a **second** instance of the debug binary with its embedded WebDriver driver.
4. Read `e2e/native/read-only.smoke.spec.ts`. This test opens different screens and saves `ace-native-ui-smoke.png` in the system temporary directory.

The integration uses `wdio.conf.ts`, `src/hooks.client.ts`,
`src-tauri/src/lib.rs`, and `src-tauri/capabilities/default.json`.

## Linux browser login callback during development

The public website returns users to ACE through an `aceblocker://account/api-key` URL.
The Linux release installer registers its handler under `/opt/ace-blocker`.
During development, `bun run tauri dev` builds the debug receiver.
It also registers the receiver automatically for the **desktop user**.
If registration fails, Vite stops and shows an error.
This prevents a silent failure of the browser login callback.
The registration process does not request root privileges.

To register the checkout callback again, run these commands as the **desktop user**.
Do not run them as root.

```sh
# From the ace-blocker repository root:
cargo build --locked -p ace-engine --bin ace-redirect
cd apps/ace-app
bun run auth:register-linux
xdg-mime query default x-scheme-handler/aceblocker
# Expected: ace-blocker-dev-handler.desktop
```

The helper registers a user-scoped `.desktop` entry.
This entry starts the debug `ace-redirect` binary.
The helper uses an **unquoted, safe absolute path**.
This lets GIO and the `xdg-open` fallback on Niri/Wayland find the receiver.
The helper does not overwrite a working production handler.
It does not start or stop services.
It does not edit the engine database.

Use this command to test callback dispatch without an API key or account change:

```sh
xdg-open 'aceblocker://account/invalid-test-route'
```

The expected result is exit status 0 and a message that the route is *not in the forward allowlist*.
This result means that the launcher was found and the engine callback receiver rejected an unsafe URL.
Do **not** test with an invented `apiKey`.
A key on the allowed endpoint can change the real account.

### Authorization for the callback receiver

The unprivileged callback receiver must read the installation capability from
`/usr/local/share/AceBlocker/capability.token`.

If the helper reports that this file is not readable, use the restricted
`ace-blocker` group ownership and membership model from the release installer.
Get administrator approval before you change protected permissions.
Do **not** make the capability token readable by all users.

Use these commands only for the intended group setup on a disposable development host:

```sh
sudo groupadd --system --force ace-blocker
sudo chgrp ace-blocker /usr/local/share/AceBlocker /usr/local/share/AceBlocker/capability.token
sudo chmod 2750 /usr/local/share/AceBlocker
sudo chmod 0640 /usr/local/share/AceBlocker/capability.token
sudo usermod -aG ace-blocker "$USER"
# Log out of your desktop session and back in, then verify:
test -r /usr/local/share/AceBlocker/capability.token && echo 'ACE callback access OK'
```

Only an administrator may approve changes to protected installation permissions.
The registration helper only reports permission problems.
A callback also needs a running Engine and its existing capability checks.
Successful registration does not prove that a complete login works.

## Keep debug and release separate

Rust registers `tauri-plugin-wdio` and `tauri-plugin-wdio-webdriver`
only with `#[cfg(debug_assertions)]`.
Release binaries do not register these plugins.
The guest JavaScript adapter loads only when Vite has `import.meta.env.DEV`
and the app runs in Tauri.
Production bundles do not include this adapter.
This integration does not add Cargo features, environment flags, Tauri
configurations, or an HTTP engine interface.

The test runner starts the WebDriver server.
An ordinary debug session does not start it.
WebDriver can execute code in the app WebView.
Do not give remote clients access to an active test session.

A test that clicks a UI control can change real settings.
Use read-only smoke tests on the developer instance.
Use disposable test profiles for tests that change state.
Use disposable VM validation for native security and lock behavior.
WDIO does not bypass engine authorization, lock restrictions, or watchdog protection.

If native tests fail after a Rust change, rebuild the debug binary.
Linux can also need WebKitGTK runtime dependencies.

References: https://tauri.app/develop/tests/webdriver/ and https://webdriver.io/docs/desktop-testing/tauri/plugin-setup .
