import { expect, test, type Page } from "@playwright/test";

/**
 * Landing page test with a mocked engine API.
 *
 * The dashboard at `/` is the default landing page and reads its data from
 * `https://127.0.0.1:10567` (the debug engine port). Every request is
 * intercepted with `page.route`, so this test needs neither a running engine
 * nor Tauri — same pattern as resistance-mode.spec.ts.
 */

const ENGINE = "https://127.0.0.1:10567";

/** Installs the minimal mock API the dashboard needs to render. */
async function mockEngine(page: Page) {
    await page.route(`${ENGINE}/ping`, (route) =>
        route.fulfill({ status: 200, body: "pong" }),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        route.fulfill({ json: { isPro: false, tosAccepted: true } }),
    );
    await page.route(`${ENGINE}/network-blocks/dashboard*`, (route) =>
        route.fulfill({
            json: {
                blocks: [],
                resistanceModeSettings: {
                    enabled: false,
                    isLocked: false,
                    lockState: { state: "inactive" },
                    lockEnd: null,
                    mode: { type: "static", intensity: "none" },
                    currentIntensity: "none",
                    staticIntensity: "none",
                    scriptsVersion: null,
                    scriptsLoaded: false,
                    scriptsTotal: 0,
                    scriptsEnabled: 0,
                    proEligible: true,
                    enforcementStatus: "disabled",
                },
                blockedAttempts: 0,
            },
        }),
    );
}

test("landing page shows the control heading", async ({ page }) => {
    // Skip the onboarding wizard/tour so it never covers the page.
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await mockEngine(page);
    await page.goto("/");
    // Generous timeout: the first request to a route compiles Vite modules,
    // which can take several seconds under parallel workers.
    await expect(
        page.getByRole("heading", { level: 1, name: "Control" }),
    ).toBeVisible({ timeout: 15_000 });
});
