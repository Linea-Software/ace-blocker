import { expect, test } from "@playwright/test";

test("browser preview shows sample data without onboarding or an engine", async ({ page }) => {
    await page.goto("/network-blocks");
    await expect(page.getByText("Browser preview", { exact: true })).toBeVisible();
    await expect(page.getByText("Adult websites", { exact: true })).toBeVisible();
    await expect(page.getByText("Social media", { exact: true })).toBeVisible();

    await page.goto("/site-packs");
    await expect(page.getByText("Browser preview", { exact: true })).toBeVisible();
    await expect(page.getByText("My custom list", { exact: true })).toBeVisible();

    await page.goto("/resistance-mode");
    await expect(page.getByText("Browser preview", { exact: true })).toBeVisible();
    await expect(page.locator('[data-testid="rm-category-list"]')).toBeVisible();

    await page.goto("/");
    await expect(page.getByText("Browser preview", { exact: true })).toBeVisible();
    await expect(page.getByText("248", { exact: true }).first()).toBeVisible();
});

test("theme selection survives browser reload and stays selected", async ({ page }) => {
    await page.goto("/settings");
    const dark = page.getByRole("button", { name: "Dark", exact: true });
    await dark.click();
    await expect(page.locator("html.dark")).toHaveCount(1);
    await expect(dark).toHaveAttribute("aria-pressed", "true");
    await page.reload();
    await expect(page.locator("html.dark")).toHaveCount(1);
    await expect(page.getByRole("button", { name: "Dark", exact: true }))
        .toHaveAttribute("aria-pressed", "true");
});

test("Resistance master status spans the full content width", async ({ page }) => {
    await page.goto("/resistance-mode");
    const stack = page.locator(".resistance-stack");
    const statusCard = page.getByTestId("rm-status-card");
    await expect(statusCard).toBeVisible();

    for (const width of [1512, 700]) {
        await page.setViewportSize({ width, height: 850 });
        const parentBounds = await stack.boundingBox();
        const cardBounds = await statusCard.boundingBox();
        expect(parentBounds).not.toBeNull();
        expect(cardBounds).not.toBeNull();
        expect(Math.abs(cardBounds!.x - parentBounds!.x)).toBeLessThanOrEqual(2);
        expect(Math.abs(cardBounds!.width - parentBounds!.width)).toBeLessThanOrEqual(2);
    }
});
