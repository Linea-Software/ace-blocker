import { expect, test, type Page } from "@playwright/test";

/**
 * Settings page tests with a mocked engine API.
 *
 * The settings page renders client-side state (theme select) and reads Pro
 * status from `userStore` (fed by `/account/info`). The Tauri settings store
 * (`load("settings.json")`) is unavailable in a plain-browser run — the page
 * catches that and keeps the defaults, which is exactly the fallback path we
 * assert here.
 */

const ENGINE = "https://127.0.0.1:10567";

async function mockEngine(page: Page, options: { pro?: boolean } = {}) {
    const { pro = false } = options;
    await page.route(`${ENGINE}/ping`, (route) =>
        route.fulfill({ status: 200, body: "pong" }),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        route.fulfill({ json: { isPro: pro, tosAccepted: true } }),
    );
}

async function gotoPage(page: Page) {
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto("/settings");
    // Generous timeout: the first request to a route compiles Vite modules,
    // which can take several seconds under parallel workers.
    await expect(
        page.getByRole("heading", { level: 1, name: "Settings" }),
    ).toBeVisible({ timeout: 15_000 });
    return page;
}

test.describe("Settings (mocked engine)", () => {
    test("renders appearance, backup and license sections", async ({
        page,
    }) => {
        await mockEngine(page, { pro: false });
        await gotoPage(page);

        await expect(
            page.getByRole("heading", { name: "Appearance", exact: true }),
        ).toBeVisible();
        await expect(
            page.getByRole("heading", { name: "Backup & Restore", exact: true }),
        ).toBeVisible();
        await expect(
            page.getByRole("heading", { name: "License", exact: true }),
        ).toBeVisible();

        // Local backup actions render.
        await expect(
            page.getByRole("button", { name: "Export" }),
        ).toBeVisible();
        await expect(
            page.getByRole("button", { name: "Import" }),
        ).toBeVisible();
    });

    test("theme toggle switches the document mode", async ({ page }) => {
        await mockEngine(page);
        await gotoPage(page);

        // Default is "system" → light under the default Playwright color
        // scheme, so the `dark` class is absent from <html>.
        const html = page.locator("html");
        await expect(html).not.toHaveClass(/dark/);

        await page.getByRole("button", { name: "Dark", exact: true }).click();

        // mode-watcher applies the `dark` class when the mode flips.
        await expect(html).toHaveClass(/dark/, { timeout: 5_000 });
    });

    test("backup cloud section is Pro-gated", async ({ page }) => {
        await mockEngine(page, { pro: false });
        await gotoPage(page);

        // Non-Pro: the cloud backup card pushes an upgrade instead of the
        // cloud actions (both backup + license cards carry the button).
        await expect(
            page.getByRole("button", { name: "Upgrade to Pro" }).first(),
        ).toBeVisible();
        await expect(
            page.getByRole("button", { name: "Upload", exact: true }),
        ).toHaveCount(0);
    });

    test("Pro user sees cloud backup actions and active license", async ({
        page,
    }) => {
        await mockEngine(page, { pro: true });
        await gotoPage(page);

        await expect(
            page.getByRole("button", { name: "Upload", exact: true }),
        ).toBeVisible();
        await expect(
            page.getByRole("button", { name: "Restore", exact: true }),
        ).toBeVisible();

        // License section reflects the Pro state.
        await expect(
            page.getByRole("heading", { name: "Pro Status Active" }),
        ).toBeVisible();
        await expect(
            page.getByText("Thank you for supporting Ace Blocker!"),
        ).toBeVisible();
    });
});
