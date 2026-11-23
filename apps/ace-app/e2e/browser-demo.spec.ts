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
