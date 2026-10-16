import { expect, test } from "@playwright/test";

/**
 * Help page tests. The page is static apart from the Tauri `app_version`
 * invoke, which fails in a plain-browser run and falls back to "Unknown" —
 * that fallback path is exactly what we assert here.
 */

async function gotoPage(page: import("@playwright/test").Page) {
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto("/help");
    // Generous timeout: the first request to a route compiles Vite modules,
    // which can take several seconds under parallel workers.
    await expect(
        page.getByRole("heading", { level: 1, name: "Help & Support" }),
    ).toBeVisible({ timeout: 15_000 });
    return page;
}

test.describe("Help", () => {
    test("renders notice, contact card and support email", async ({ page }) => {
        await gotoPage(page);

        await expect(
            page.getByRole("heading", {
                name: "Important Notice Regarding Locks",
            }),
        ).toBeVisible();
        await expect(
            page.getByRole("heading", { name: "Contact the Team" }),
        ).toBeVisible();
        await expect(
            page.getByText("support@aceblocker.com", { exact: true }),
        ).toBeVisible();
        await expect(
            page.getByRole("link", { name: "Contact Support" }),
        ).toBeVisible();
    });

    test("shows the app version fallback (Unknown in a plain browser)", async ({
        page,
    }) => {
        await gotoPage(page);

        // The version box labels and the fallback value.
        await expect(page.getByText("Version", { exact: true })).toBeVisible();
        await expect(page.getByText("Unknown", { exact: true })).toBeVisible();
        // The OS box still resolves from the user agent (Linux on CI).
        await expect(page.getByText("System", { exact: true })).toBeVisible();
    });
});
