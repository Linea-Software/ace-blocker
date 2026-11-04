import { expect, test } from "@playwright/test";

test.describe("Desktop UI friction", () => {
    test("Categories keeps its primary action visible in a small window", async ({ page }) => {
        await page.setViewportSize({ width: 800, height: 600 });
        await page.goto("/site-packs");

        const importAction = page.getByRole("button", { name: /Import Custom List/ });
        await expect(importAction).toBeVisible({ timeout: 15_000 });
        const bounds = await importAction.boundingBox();
        expect(bounds).not.toBeNull();
        expect(bounds!.width).toBeGreaterThan(200);
        expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(800);

        const search = page.getByRole("searchbox", {
            name: "Search categories and Custom Lists...",
        });
        await search.fill("social");
        await page.getByRole("button", { name: "Clear search" }).click();
        await expect(search).toHaveValue("");
    });

    test("Navigation remains reachable when window height is constrained", async ({ page }) => {
        await page.setViewportSize({ width: 800, height: 300 });
        await page.goto("/getting-started");
        const settingsLink = page.getByRole("navigation").getByRole("link", {
            name: "Settings",
        });
        await expect(settingsLink).toBeAttached({ timeout: 15_000 });
        await settingsLink.scrollIntoViewIfNeeded();
        await expect(settingsLink).toBeInViewport();
    });

    test("Offline Blocks does not offer an unusable create action", async ({ page }) => {
        await page.goto("/network-blocks");
        await expect(page.getByRole("heading", { name: "Network Blocks" })).toBeVisible({
            timeout: 15_000,
        });
        await expect(page.getByRole("button", { name: "Create Block" })).toBeDisabled();
        await expect(page.getByRole("link", { name: "Getting Started" })).toBeVisible();
    });
});
