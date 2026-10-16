import { expect, test, type Page } from "@playwright/test";

/**
 * Categories page tests with a fully mocked engine API.
 *
 * Same pattern as resistance-mode.spec.ts: every request to
 * `https://127.0.0.1:10567` is intercepted with `page.route`, so these tests
 * need neither a running engine nor Tauri.
 *
 * Note: the site-packs page itself manages pack download/resync/delete — it
 * has no connect-to-block surface. Connecting a pack to a block happens in the
 * network-blocks edit modal ("Categories" section), covered by
 * `network-blocks-mocked.spec.ts`.
 */

const ENGINE = "https://127.0.0.1:10567";

const CATALOG = {
    sitePacks: [
        {
            id: "pack-adult-free",
            name: "Adult Websites — Free",
            description: "Curated adult domains.",
            icon: "adult",
            entriesCount: 1250000,
            requiresPro: false,
            isSynced: true,
            // Data installed from the installer bundle (works offline).
            isBundled: true,
            lastUpdatedAt: "2026-09-02T00:00:00Z",
            version: "2026.09.02",
        },
        {
            id: "pack-social-free",
            name: "Social Media Websites — Free",
            description: "Curated social domains.",
            icon: "social",
            entriesCount: 890000,
            requiresPro: false,
            isSynced: false,
            isBundled: false,
            lastUpdatedAt: null,
            version: null,
        },
        {
            id: "pack-gambling-pro",
            name: "Gambling Websites — Pro",
            description: "Curated gambling domains.",
            icon: "gambling",
            entriesCount: 340000,
            requiresPro: true,
            isSynced: false,
            isBundled: false,
            lastUpdatedAt: null,
            version: null,
        },
        {
            id: "custom-focus-list",
            name: "My Focus List",
            description: "Imported domains.",
            icon: "list",
            entriesCount: 42,
            requiresPro: true,
            isSynced: true,
            isBundled: false,
            lastUpdatedAt: "2026-09-10T00:00:00Z",
            version: "custom",
        },
    ],
} as const;

async function mockEngine(page: Page, options: { pro?: boolean } = {}) {
    const { pro = false } = options;
    await page.route(`${ENGINE}/ping`, (route) =>
        route.fulfill({ status: 200, body: "pong" }),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        route.fulfill({ json: { isPro: pro, tosAccepted: true } }),
    );
    await page.route(`${ENGINE}/site-lists`, (route) =>
        route.fulfill({ json: CATALOG }),
    );
    await page.route(`${ENGINE}/open-pro`, (route) =>
        route.fulfill({ status: 200 }),
    );
}

async function gotoPage(page: Page, query = "") {
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto(`/site-packs${query}`);
    // Generous timeout: the first request to a route compiles Vite modules,
    // which can take several seconds under parallel workers.
    await expect(
        page.getByRole("heading", { level: 1, name: "Categories" }),
    ).toBeVisible({ timeout: 15_000 });
    return page;
}

test.describe("Categories (mocked engine)", () => {
    test("loaded: categories and Custom Lists render from the same engine catalog", async ({
        page,
    }) => {
        await mockEngine(page);
        await gotoPage(page);

        await expect(
            page.getByRole("heading", { level: 2, name: "Categories" }),
        ).toBeVisible();
        await expect(
            page.getByText("Adult Content", { exact: true }),
        ).toBeVisible();
        await expect(
            page.getByText("1,250,000", { exact: true }),
        ).toBeVisible();
        await expect(
            page.getByText("Social Media", { exact: true }),
        ).toBeVisible();

        // Row actions live in the contextual inspector rather than on every
        // catalog row.
        await page
            .getByTestId("site-pack-row")
            .filter({ hasText: "Adult Content" })
            .click();
        const adultInspector = page.getByRole("region", {
            name: "Adult Content",
        });
        await expect(
            adultInspector.getByRole("button", { name: "Resync" }),
        ).toBeVisible();

        await adultInspector.getByRole("button", { name: "Close" }).click();
        await page
            .getByTestId("site-pack-row")
            .filter({ hasText: "Social Media" })
            .click();
        const socialInspector = page.getByRole("region", {
            name: "Social Media",
        });
        await expect(
            socialInspector.getByRole("button", { name: "Download" }),
        ).toBeVisible();
        await socialInspector.getByRole("button", { name: "Close" }).click();

        // The synced free pack was installed with the installer (bundled):
        // it must be visibly "ready offline", not require a download.
        await expect(page.getByText("Included", { exact: true })).toHaveCount(
            1,
        );
        // Metadata-only packs (never downloaded) must not show a domain
        // count as if their rules existed — show "—" instead.
        await expect(page.getByText("—", { exact: true })).toHaveCount(4);

        // Pro gating remains a category property rather than a separate group.
        const gamblingRow = page
            .getByTestId("site-pack-row")
            .filter({ hasText: "Gambling" });
        await expect(
            gamblingRow.getByText("Gambling", { exact: true }),
        ).toBeVisible();
        await expect(
            gamblingRow.getByText("PRO", { exact: true }),
        ).toBeVisible();

        await expect(
            page.getByText("Custom Lists", { exact: true }),
        ).toBeVisible();
        await expect(
            page.getByText("My Focus List", { exact: true }),
        ).toBeVisible();
    });

    test("non-Pro user: Import Custom List button is Pro-gated", async ({
        page,
    }) => {
        await mockEngine(page, { pro: false });
        // Register AFTER mockEngine so this handler wins (routes are LIFO).
        let openProCalled = false;
        await page.route(`${ENGINE}/open-pro`, (route) => {
            openProCalled = true;
            return route.fulfill({ status: 200 });
        });
        await gotoPage(page);

        await page.getByRole("button", { name: "Import Custom List" }).click();
        // No import modal for a non-Pro user; the pro page was requested.
        await expect(page.getByRole("dialog")).toHaveCount(0);
        await expect.poll(() => openProCalled, { timeout: 5_000 }).toBe(true);
    });

    test("Pro user: ?import=1 deep-link opens the import modal", async ({
        page,
    }) => {
        await mockEngine(page, { pro: true });
        await gotoPage(page, "?import=1");

        const dialog = page.getByRole("dialog");
        await expect(
            dialog.getByRole("heading", { name: "Import Custom List" }),
        ).toBeVisible({ timeout: 10_000 });
        // Modal form fields render.
        await expect(dialog.getByLabel("List Name")).toBeVisible();
        await expect(dialog.getByLabel("List Description")).toBeVisible();
        await expect(dialog.getByText("No sources added yet.")).toBeVisible();
    });
});
