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

test("Resistance shows a Pro upgrade action instead of the switch for Free users", async ({ page }) => {
    await page.goto("/resistance-mode");
    const card = page.getByTestId("rm-status-card");

    // The browser fixture starts with Pro: retain the existing master switch.
    await expect(card.getByTestId("rm-master-switch")).toBeVisible();
    await expect(card.locator(".pro-action")).toHaveCount(0);

    // Toggle the live reactive account store without mocking native IPC.
    await page.evaluate(async () => {
        const path = "/src/lib/stores/user-store.ts";
        const existing = performance.getEntriesByType("resource")
            .map((entry) => new URL(entry.name))
            .filter((url) => url.pathname === path)
            .at(-1);
        const { userStore } = await import(
            /* @vite-ignore */ existing ? existing.pathname + existing.search : path
        );
        userStore.isPro = false;
    });

    await expect(page.getByTestId("rm-pro-gate")).toHaveCount(0);
    await expect(card.getByTestId("rm-master-switch")).toHaveCount(0);
    const proButton = card.locator("button.pro-action");
    await expect(proButton).toBeVisible();
    await expect(proButton).toBeEnabled();
    await expect(proButton).toHaveAttribute("aria-label", /Resistance.*Pro/);
    await expect(page.locator('[data-testid="rm-category-list"]')).toBeVisible();

    // The CTA remains within the card even on a narrow viewport.
    await page.setViewportSize({ width: 360, height: 800 });
    const [cardBox, buttonBox] = await Promise.all([card.boundingBox(), proButton.boundingBox()]);
    expect(cardBox).not.toBeNull();
    expect(buttonBox).not.toBeNull();
    expect(buttonBox!.x).toBeGreaterThanOrEqual(cardBox!.x);
    expect(buttonBox!.x + buttonBox!.width).toBeLessThanOrEqual(cardBox!.x + cardBox!.width + 1);
});
