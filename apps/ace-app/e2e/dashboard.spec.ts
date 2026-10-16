import { expect, test, type Page } from "@playwright/test";

/**
 * Dashboard (`/`) tests with a fully mocked engine API.
 *
 * Same pattern as resistance-mode.spec.ts: the app talks to
 * `https://127.0.0.1:10567` and every request is intercepted with
 * `page.route`, so these tests need neither a running engine nor Tauri.
 *
 * Regression coverage: the dashboard used to render zeroed/misleading stats
 * (or a blank shell) when the engine was unreachable — it must show the same
 * WifiOff offline notice as the other routes, driven by `engineStore.isAlive`.
 */

const ENGINE = "https://127.0.0.1:10567";

const SNAPSHOT = {
    blocks: [
        {
            block: {
                id: "01950000-0000-7000-8000-000000000001",
                name: "Adult Content",
                icon: null,
                isActive: true,
                isLocked: false,
                lockState: { state: "inactive" },
                safeSearch: false,
            },
            lock: null,
            scheduleEntries: [
                {
                    id: "sched-1",
                    networkBlockId: "01950000-0000-7000-8000-000000000001",
                    startTime: "00:00:00",
                    endTime: "23:59:59",
                    monday: true,
                    tuesday: true,
                    wednesday: true,
                    thursday: true,
                    friday: true,
                    saturday: true,
                    sunday: true,
                },
            ],
            sitePackConnectionCount: 2,
            userRulesCount: 0,
        },
    ],
    resistanceModeSettings: {
        enabled: true,
        isLocked: true,
        lockState: { state: "active_until_lock_end", end: "2026-09-30T18:00:00" },
        lockEnd: "2026-09-30T18:00:00",
        mode: { type: "static", intensity: "medium" },
        currentIntensity: "medium",
        staticIntensity: "medium",
        scriptsVersion: null,
        scriptsLoaded: true,
        scriptsTotal: 3,
        scriptsEnabled: 3,
        proEligible: true,
        enforcementStatus: "operational",
    },
    blockedAttempts: 3,
} as const;

/** Installs the standard mock API the dashboard needs to render. */
async function mockEngine(
    page: Page,
    options: { online?: boolean } = {},
) {
    const { online = true } = options;
    await page.route(`${ENGINE}/ping`, (route) =>
        online
            ? route.fulfill({ status: 200, body: "pong" })
            : route.abort(),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        online
            ? route.fulfill({ json: { isPro: false, tosAccepted: true } })
            : route.abort(),
    );
    await page.route(`${ENGINE}/network-blocks/dashboard*`, (route) =>
        online
            ? route.fulfill({ json: SNAPSHOT })
            : route.abort(),
    );
}

async function gotoDashboard(page: Page) {
    // Skip the onboarding wizard/tour so it never covers the page.
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto("/");
    return page;
}

test.describe("Dashboard (mocked engine)", () => {
    test("renders the offline card when the engine is unreachable", async ({
        page,
    }) => {
        await mockEngine(page, { online: false });
        await gotoDashboard(page);

        // The engine store polls /ping; the offline card appears once the
        // first ping fails.
        const offline = page
            .locator("#ace-workspace")
            .getByRole("status")
            .filter({ hasText: "Engine offline" });
        await expect(offline).toBeVisible({ timeout: 15_000 });
        await expect(offline).toContainText(
            "The engine is not reachable",
        );
        // No misleading stats while offline.
        await expect(
            page.getByText("Local activity", { exact: true }),
        ).toHaveCount(0);
    });

    test("recovers to the full dashboard when the engine comes back", async ({
        page,
    }) => {
        await mockEngine(page, { online: false });
        await gotoDashboard(page);

        const offline = page
            .locator("#ace-workspace")
            .getByRole("status")
            .filter({ hasText: "Engine offline" });
        await expect(offline).toBeVisible({ timeout: 15_000 });

        // Bring the engine back; the next ping poll flips the store and the
        // dashboard grid renders with real data.
        await mockEngine(page, { online: true });
        await expect(
            page.getByRole("heading", { level: 1, name: "Control" }),
        ).toBeVisible();
        // The blocked-attempts card shows the local aggregate from the mock.
        await expect(
            page.getByText("Local activity", { exact: true }),
        ).toBeVisible({ timeout: 20_000 });
        await expect(
            page.getByText("3 blocked attempts", { exact: true }),
        ).toBeVisible();
        await expect(offline).toHaveCount(0);
    });

    test("renders fully when the engine is up: heading, lock status, blocked attempts", async ({
        page,
    }) => {
        await mockEngine(page);
        await gotoDashboard(page);

        await expect(
            page.getByRole("heading", { level: 1, name: "Control" }),
        ).toBeVisible();

        // Blocked-attempts card with today's local aggregate.
        await expect(
            page.getByText("Local activity", { exact: true }),
        ).toBeVisible();
        await expect(
            page.getByText("3 blocked attempts", { exact: true }),
        ).toBeVisible();

        // Resistance is reported as current enforcement state, including
        // the active lock returned by the same dashboard snapshot.
        const nextChange = page.getByLabel("Next change");
        await expect(nextChange.getByText("Resistance", { exact: true })).toBeVisible();
        await expect(nextChange.getByText("Medium", { exact: true })).toBeVisible();
        await expect(page.getByText(/Until .*9\/30\/2026/)).toBeVisible();

        // The active block is the primary state-board content.
        await expect(page.getByText("1 of 1 blocks active", { exact: true })).toBeVisible();
        await expect(page.getByText("Adult Content", { exact: true })).toBeVisible();
    });
});
