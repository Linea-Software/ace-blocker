import { defineConfig } from "@playwright/test";
import { existsSync } from "node:fs";

/**
 * Browser-only smoke tests for static Ace pages. They do not mock an HTTP
 * engine: actual desktop/control integration must run through Tauri/remoc.
 */
export default defineConfig({
    testDir: "e2e",
    testMatch: /(getting-started|help|ui-friction|block-editor-redesign|network-policy-icons|resistance-five-categories|solid-port-regressions)\.spec\.ts/,
    use: {
        baseURL: process.env.PW_E2E_BASE_URL ?? "http://localhost:1420",
        launchOptions: {
            // Use an installed Chromium on Linux CI/dev when the bundled
            // Playwright browser is unavailable; other platforms use the default.
            executablePath: process.env.PW_CHROMIUM_EXECUTABLE ??
                (existsSync("/usr/bin/google-chrome-stable") ? "/usr/bin/google-chrome-stable" : undefined),
            args: process.platform === "linux" ? ["--no-sandbox"] : [],
        },
    },
    webServer: {
        command: process.env.PW_E2E_WEBSERVER_COMMAND ?? "bun run dev",
        url: process.env.PW_E2E_BASE_URL ?? "http://localhost:1420",
        reuseExistingServer: true,
        timeout: 120_000,
    },
});
