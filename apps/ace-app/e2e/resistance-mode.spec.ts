import { expect, test, type Page } from "@playwright/test";

/**
 * Resistance Mode page tests with a fully mocked engine API.
 *
 * The app talks to `https://127.0.0.1:10567` (the debug engine port); every
 * request is intercepted with `page.route`, so these tests need neither a
 * running engine nor Tauri. The Vite dev server must be reachable at
 * `http://localhost:1420` (see playwright.config.ts webServer).
 *
 * Mock helpers mirror the engine's response shapes exactly (camelCase,
 * `{state}`-tagged LockState, `{type, intensity}` PunishmentMode, and the
 * `{error, code}` error envelope).
 */

const ENGINE = "https://127.0.0.1:10567";

const BASE_SETTINGS = {
    enabled: false,
    isLocked: false,
    lockState: { state: "inactive" },
    lockEnd: null,
    mode: { type: "static", intensity: "none" },
    currentIntensity: "none",
    staticIntensity: "none",
    dynamicIntensity: "none",
    probationProgress: 0,
    probationDaysTotal: 0,
    scriptsVersion: null,
    scriptsLoaded: false,
    scriptsTotal: 0,
    scriptsEnabled: 0,
    proEligible: true,
    enforcementStatus: "disabled",
} as const;

const SAMPLE_SCRIPTS = {
    scripts: [
        {
            id: "youtube-grayscale",
            domain: "youtube.com",
            name: "YouTube Grayscale",
            description: "Dulls colors on YouTube.",
            isEnabled: true,
        },
        {
            id: "youtube-shorts-remove",
            domain: "youtube.com",
            name: "Remove Shorts",
            description: "Hides the Shorts section.",
            isEnabled: true,
        },
        {
            id: "generic-dominant-color",
            domain: "global",
            name: "Dominant Color",
            description: "Runs everywhere.",
            isEnabled: true,
        },
    ],
} as const;

function settingsOverrides(overrides: Record<string, unknown>) {
    return { ...BASE_SETTINGS, ...overrides };
}

/** Installs the standard mock API: health, account, settings, scripts. */
async function mockEngine(
    page: Page,
    options: {
        settings?: Record<string, unknown>;
        scripts?: Record<string, unknown>;
        pro?: boolean;
        online?: boolean;
    } = {},
) {
    const { settings = {}, scripts = SAMPLE_SCRIPTS, pro = true, online = true } = options;

    await page.route(`${ENGINE}/ping`, (route) =>
        online
            ? route.fulfill({ status: 200, body: "pong" })
            : route.abort(),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        online
            ? route.fulfill({ json: { isPro: pro, tosAccepted: true } })
            : route.abort(),
    );
    await page.route(`${ENGINE}/resistance-mode/settings`, (route) =>
        online
            ? route.fulfill({ json: settingsOverrides(settings) })
            : route.abort(),
    );
    await page.route(`${ENGINE}/resistance-mode/scripts`, (route) =>
        online
            ? route.fulfill({ json: scripts })
            : route.abort(),
    );
}

/** Captures a single POST to the given path (fulfilling with 200). */
function capturePost(
    page: Page,
    path: string,
    options: { status?: number; json?: unknown } = {},
) {
    let captured: { body: unknown; headers: Record<string, string> } | null = null;
    void page.route(`${ENGINE}${path}`, async (route) => {
        captured = {
            body: route.request().postDataJSON(),
            headers: route.request().headers(),
        };
        if (options.status) {
            await route.fulfill({ status: options.status, json: options.json ?? { error: "mock", code: "mock" } });
        } else {
            await route.fulfill({ status: 200 });
        }
    });
    return () => captured;
}

async function gotoPage(page: Page) {
    // Skip the onboarding wizard/tour so it never covers the page.
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto("/resistance-mode");
    // Wait for the route shell to render; state-specific tests wait for their
    // own content because Free plans intentionally do not render the controls.
    await expect(
        page.getByRole("heading", { level: 1, name: "Resistance Mode" }),
    ).toBeVisible();
    return page;
}

test.describe("Resistance Mode (mocked engine)", () => {
    test("disabled state", async ({ page }) => {
        await mockEngine(page, { settings: settingsOverrides({}) });
        await gotoPage(page);

        await expect(page.getByTestId("rm-status")).toHaveAttribute("data-status", "off");
        await expect(page.getByTestId("rm-master-switch")).not.toBeChecked();
        await expect(page.getByTestId("rm-beta-badge")).toHaveText("Beta");
    });

    test("enabled and operational", async ({ page }) => {
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "medium" },
                currentIntensity: "medium",
                staticIntensity: "medium",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
                enforcementStatus: "operational",
            }),
        });
        await gotoPage(page);

        await expect(page.getByTestId("rm-status")).toHaveAttribute("data-status", "operational");
        await expect(page.getByTestId("rm-status")).toContainText("Active on supported sites.");
    });

    test("enabled but scripts missing shows degraded state with reason", async ({ page }) => {
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "medium" },
                currentIntensity: "medium",
                staticIntensity: "medium",
                scriptsLoaded: false,
                scriptsTotal: 0,
                scriptsEnabled: 0,
                enforcementStatus: "scripts_missing",
            }),
            scripts: { scripts: [] },
        });
        await gotoPage(page);

        await expect(page.getByTestId("rm-status")).toHaveAttribute("data-status", "degraded");
        await expect(page.getByTestId("rm-status")).toContainText("On");
        await expect(page.getByTestId("rm-status")).toContainText("Scripts need to be installed before Resistance can operate.");
        await expect(page.getByTestId("rm-scripts-empty")).toBeVisible();
        await expect(page.getByTestId("rm-scripts-empty").getByTestId("rm-sync-scripts")).toBeVisible();
        await expect(page.getByTestId("rm-sync-scripts")).toHaveCount(1);
    });

    test("engine offline shows offline state and recovers", async ({ page }) => {
        await mockEngine(page, { online: false });
        await gotoPage(page);

        // engineStore polls /ping every 10s; the offline card appears once
        // the first poll fails.
        await expect(page.getByTestId("rm-status")).toHaveAttribute("data-status", "offline", {
            timeout: 15_000,
        });

        // Bring the engine back; the next ping poll flips the store and the
        // page resumes polling.
        await mockEngine(page, { online: true });
        await expect(page.getByTestId("rm-status")).not.toHaveAttribute("data-status", "offline", {
            timeout: 20_000,
        });
    });

    test("lock creation requires confirmation and sends the exact expiration", async ({ page }) => {
        // Pin "today" so the calendar's current month always contains an
        // enabled future day (the 15th) and the default 12:00 time is in the
        // future.
        await page.clock.install({ time: new Date("2026-09-10T10:00:00") });

        const lockCaptured = capturePost(page, "/resistance-mode/set-lock");
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "low" },
                currentIntensity: "low",
                staticIntensity: "low",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
                enforcementStatus: "operational",
            }),
        });
        await gotoPage(page);

        await page.getByTestId("rm-open-lock-dialog").click();
        await expect(
            page.getByRole("dialog").getByRole("heading", { name: "Time lock" }),
        ).toBeVisible();

        // Commit must be disabled until a future date/time is picked.
        const commit = page.getByTestId("rm-lock-commit");
        await expect(commit).toBeDisabled();

        // Pick the 15th of the current month.
        await page.getByRole("button", { name: /pick date/i }).click();
        await page.getByRole("gridcell").filter({ hasText: "15" }).first().click();

        await expect(commit).toBeEnabled();
        // The exact resulting expiration is shown before committing.
        await expect(page.getByTestId("rm-lock-resulting-expiration")).toBeVisible();

        await commit.click();
        // One-way nature is spelled out in the confirmation dialog.
        await expect(
            page.getByRole("alertdialog").getByText(/cannot be undone/i),
        ).toBeVisible();
        await page.getByTestId("rm-lock-confirm").click();

        const captured = lockCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toMatchObject({ lockState: "active_until_lock_end" });
        const end = (captured!.body as { lockEnd: string }).lockEnd;
        expect(end).toBe("2026-09-15T12:00:00");
    });

    test("locked state disables controls and shows exact expiration", async ({ page }) => {
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                isLocked: true,
                lockState: { state: "active_until_lock_end", end: "2026-09-30T18:00:00" },
                lockEnd: "2026-09-30T18:00:00",
                mode: { type: "static", intensity: "high" },
                currentIntensity: "high",
                staticIntensity: "high",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
                enforcementStatus: "operational",
            }),
        });
        await gotoPage(page);

        await expect(page.getByTestId("rm-lock-expiration")).toContainText("9/30/2026");
        await expect(page.getByTestId("rm-master-switch")).toBeDisabled();
        await expect(page.getByTestId("rm-intensity-high")).toBeDisabled();
        await expect(page.getByTestId("rm-locked-note")).toBeVisible();

        // Expand the youtube.com group: its script switches must be disabled
        // while the lock is active (weakening requires an unlocked state).
        await page.getByRole("button", { name: "youtube.com" }).click();
        const scriptSwitches = page.getByTestId("rm-script-switch");
        await expect(scriptSwitches.first()).toBeDisabled();
        await expect(scriptSwitches.nth(1)).toBeDisabled();
    });

    test("static intensity mutation sends the selected tier", async ({ page }) => {
        const typeCaptured = capturePost(page, "/resistance-mode/type");
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "low" },
                currentIntensity: "low",
                staticIntensity: "low",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
                enforcementStatus: "operational",
            }),
        });
        await gotoPage(page);

        await page.getByTestId("rm-intensity-high").click();

        const captured = typeCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toEqual({ type: "static", intensity: "high" });
    });

    test("script toggle mutation", async ({ page }) => {
        const toggleCaptured = capturePost(page, "/resistance-mode/scripts/youtube-grayscale/set-enabled");
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "medium" },
                currentIntensity: "medium",
                staticIntensity: "medium",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
                enforcementStatus: "operational",
            }),
        });
        await gotoPage(page);

        // Expand the youtube.com group and toggle the first script off.
        await page.getByRole("button", { name: "youtube.com" }).click();
        const firstSwitch = page.getByTestId("rm-script-switch").first();
        await firstSwitch.click();

        const captured = toggleCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toEqual({ enabled: false });
    });

    test("pro-required state", async ({ page }) => {
        await mockEngine(page, { pro: false, settings: settingsOverrides({ proEligible: false }) });
        await gotoPage(page);

        await expect(
            page.getByRole("heading", { name: "Resistance is a Pro feature" }),
        ).toBeVisible();
        await expect(
            page.getByText(
                "Pro adds configurable friction to supported sites that you still need to access.",
            ),
        ).toBeVisible();
        await expect(page.getByTestId("rm-master-switch")).toHaveCount(0);
    });

    test("failed script download shows an honest error", async ({ page }) => {
        await page.route(`${ENGINE}/resistance-mode/download-scripts`, (route) =>
            route.fulfill({
                status: 502,
                json: { error: "download failed", code: "script_download_failed" },
            }),
        );
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "medium" },
                currentIntensity: "medium",
                staticIntensity: "medium",
                scriptsLoaded: false,
                scriptsTotal: 0,
                scriptsEnabled: 0,
                enforcementStatus: "scripts_missing",
            }),
            scripts: { scripts: [] },
        });
        await gotoPage(page);

        await page.getByTestId("rm-sync-scripts").first().click();
        await expect(page.getByText("Failed to download Resistance scripts.")).toBeVisible();
    });

    test("dynamic mode is absent from the UI", async ({ page }) => {
        await mockEngine(page, { settings: settingsOverrides({}) });
        await gotoPage(page);

        await expect(page.getByText("Dynamic mode", { exact: true })).toHaveCount(0);
        await expect(page.getByText(/escalat/i)).toHaveCount(0);
        await expect(page.getByText(/usage is tracked/i)).toHaveCount(0);
    });

    test("locked action is refused without a generic error", async ({ page }) => {
        // Engine rejects a script toggle with the typed locked error.
        await page.route(`${ENGINE}/resistance-mode/scripts/youtube-grayscale/set-enabled`, (route) =>
            route.fulfill({
                status: 403,
                json: { error: "Resistance engine is locked", code: "resistance_locked" },
            }),
        );
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "medium" },
                currentIntensity: "medium",
                staticIntensity: "medium",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
                enforcementStatus: "operational",
            }),
        });
        await gotoPage(page);

        await page.getByRole("button", { name: "youtube.com" }).click();
        const firstSwitch = page.getByTestId("rm-script-switch").first();
        await firstSwitch.click();

        // Rollback + the one-way lock notice (not a generic sync error).
        await expect(firstSwitch).toBeChecked();
        await expect(page.getByText("This setting is locked and cannot be disabled.")).toBeVisible();
    });

    test("stale responses cannot overwrite newer state", async ({ page }) => {
        await mockEngine(page, {
            settings: settingsOverrides({
                enabled: true,
                mode: { type: "static", intensity: "low" },
                currentIntensity: "low",
                staticIntensity: "low",
                scriptsLoaded: true,
                scriptsTotal: 3,
                scriptsEnabled: 3,
            }),
        });

        // Override the settings route AFTER mockEngine so it wins: the first
        // request is parked (stale, disabled), later ones answer immediately
        // with the fresh operational state.
        let firstRequestSeen = false;
        let releaseFirst: (() => void) | null = null;
        await page.route(`${ENGINE}/resistance-mode/settings`, async (route) => {
            if (!firstRequestSeen) {
                firstRequestSeen = true;
                await new Promise<void>((resolve) => {
                    releaseFirst = () => {
                        void route.fulfill({
                            json: settingsOverrides({
                                enabled: false,
                                enforcementStatus: "disabled",
                            }),
                        });
                        resolve();
                    };
                });
                return;
            }
            await route.fulfill({
                json: settingsOverrides({
                    enabled: true,
                    mode: { type: "static", intensity: "medium" },
                    currentIntensity: "medium",
                    staticIntensity: "medium",
                    scriptsLoaded: true,
                    scriptsTotal: 3,
                    scriptsEnabled: 3,
                    enforcementStatus: "operational",
                }),
            });
        });

        await gotoPage(page);
        // The first (parked) request was the initial load; the page's 5s
        // poll then delivers the fresh operational state.
        await expect(page.getByTestId("rm-status")).toHaveAttribute("data-status", "operational", {
            timeout: 10_000,
        });
        // Now release the stale response — it must NOT overwrite the newer
        // state, so the card stays operational.
        releaseFirst?.();
        await page.waitForTimeout(1500);
        await expect(page.getByTestId("rm-status")).toHaveAttribute("data-status", "operational");
        await expect(page.getByTestId("rm-status")).not.toContainText("Off");
    });
});
