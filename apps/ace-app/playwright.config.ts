import { defineConfig } from "@playwright/test";

/**
 * Specs that mock the engine API (page.route against 127.0.0.1:10567) and
 * therefore run without a live engine: dashboard, network-blocks (mocked),
 * site-packs, settings, getting-started, help, onboarding, legal-gate,
 * resistance-mode and the landing-page demo test.
 */
const MOCKED_SPECS =
    /(demo\.test|resistance-mode\.spec|network-blocks-mocked\.spec|site-packs\.spec|settings\.spec|getting-started\.spec|help\.spec|dashboard\.spec|onboarding\.spec|legal-gate\.spec)\.ts/;

/** The live-engine suite (serial, non-destructive) — requires a running
 *  engine on 127.0.0.1:10567. */
const LIVE_ENGINE_SPECS = /network-blocks\.spec\.ts/;

export default defineConfig({
    testDir: "e2e",
    projects: [
        {
            name: "mocked",
            testMatch: MOCKED_SPECS,
        },
        {
            name: "live-engine",
            testMatch: LIVE_ENGINE_SPECS,
        },
    ],
    use: {
        baseURL: "http://localhost:1420",
        ignoreHTTPSErrors: true,
    },
    // The Vite dev server is started automatically for the mocked specs
    // (which intercept the engine API) and reused when one is already
    // running. The live-engine suite additionally requires the engine to be
    // up on 127.0.0.1:10567.
    //
    // `bun run dev` regenerates the API client from the committed ace-engine
    // spec first. CI (which checks out only the ace-app repo) has no such
    // spec, so the mocked-e2e job overrides the command with plain
    // `bunx vite dev` via PW_E2E_WEBSERVER_COMMAND.
    webServer: {
        command: process.env.PW_E2E_WEBSERVER_COMMAND ?? "bun run dev",
        url: "http://localhost:1420",
        reuseExistingServer: true,
        timeout: 120_000,
    },
});
