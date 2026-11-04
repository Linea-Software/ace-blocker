# Native UI inspection with WebdriverIO

ACE uses the WebdriverIO Tauri service to interact with its real desktop WebView, including native Tauri APIs and the normal, policy-checked engine IPC. A plain browser on `localhost:1420` does not provide that IPC.

## Run

From `ace-blocker/apps/ace-app`:

1. Install dependencies with `bun install --frozen-lockfile`.
2. Launch the usual debug app with `bun run tauri dev`. This also builds `target/debug/ace-app` and starts Vite on port 1420.
3. In another terminal, run `bun run test:native`. WDIO launches a **second** copy of the debug binary using its embedded WebDriver driver.
4. Review the smoke test in `e2e/native/read-only.smoke.spec.ts`. It navigates between screens and saves a screenshot as `ace-native-ui-smoke.png` in the system temp directory.

The wiring lives in `wdio.conf.ts`, `src/hooks.client.ts`, `src-tauri/src/lib.rs` and `src-tauri/capabilities/default.json`.

## Debug and release isolation

Rust registers `tauri-plugin-wdio` and `tauri-plugin-wdio-webdriver` only under `#[cfg(debug_assertions)]`; no release binary registers them. The guest JS adapter is imported only under Vite's `import.meta.env.DEV` and the Tauri environment, so a production bundle does not include it. No additional Cargo features, environment flags, Tauri configurations or HTTP engine interfaces are introduced.

The WebDriver server is started by the test runner, not by an ordinary debug session. It can execute code in the app WebView, so do not expose an active test session to remote clients. A test clicking a UI control can mutate real settings: use read-only smoke tests on the developer instance, disposable test profiles for mutation tests, and disposable VM validation for native security and lock behavior. WDIO does not bypass engine authorization, lock restrictions or watchdog protection.

If native tests fail after changing Rust code, rebuild the debug binary first. On Linux, WebKitGTK runtime dependencies may be needed.

References: https://tauri.app/develop/tests/webdriver/ and https://webdriver.io/docs/desktop-testing/tauri/plugin-setup .
