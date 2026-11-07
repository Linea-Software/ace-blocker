import { defineConfig } from "@playwright/test";

/**
 * Browser-only smoke tests for static Ace pages. They do not mock an HTTP
 * engine: actual desktop/control integration must run through Tauri/remoc.
 */
export default defineConfig({
    testDir: "e2e",
    testMatch: /(getting-started|help|ui-friction|block-editor-redesign)\.spec\.ts/,
    use: {
        baseURL: "http://localhost:1420",
    },
    webServer: {
        command: process.env.PW_E2E_WEBSERVER_COMMAND ?? "bun run dev",
        url: "http://localhost:1420",
        reuseExistingServer: true,
        timeout: 120_000,
    },
});
