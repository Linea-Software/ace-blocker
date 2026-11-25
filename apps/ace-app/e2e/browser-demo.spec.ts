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

test("Resistance Free enables free scripts while showing Pro upgrades", async ({ page }) => {
    await page.goto("/resistance-mode");
    const card = page.getByTestId("rm-status-card");
    await expect(card.getByTestId("rm-master-switch")).toBeVisible();

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
    await expect(card.getByTestId("rm-master-switch")).toBeEnabled();
    await expect(card.locator("button.pro-action")).toHaveCount(0);

    const video = page.getByTestId("rm-category-video");
    await video.getByTestId("rm-category-disclosure-video").click();
    await video.getByTestId("rm-site-row").first().getByRole("button").click();
    await expect(video.getByTestId("rm-script-switch")).toBeEnabled();

    const visual = page.getByTestId("rm-category-visual");
    await visual.getByTestId("rm-category-disclosure-visual").click();
    await visual.getByTestId("rm-site-row").first().getByRole("button").click();
    await expect(visual.locator(".rm-script-upgrade")).toBeVisible();
    await expect(visual.getByTestId("rm-script-switch")).toHaveCount(0);

    await page.setViewportSize({ width: 360, height: 800 });
    const [cardBox, switchBox] = await Promise.all([
        card.boundingBox(), card.getByTestId("rm-master-switch").boundingBox(),
    ]);
    expect(cardBox).not.toBeNull();
    expect(switchBox).not.toBeNull();
    expect(switchBox!.x + switchBox!.width).toBeLessThanOrEqual(cardBox!.x + cardBox!.width + 1);
});

test("Network Block rails distinguish active and scheduled states without Pro amber", async ({ page }) => {
    await page.goto("/network-blocks");
    const active = page.getByTestId("network-policy").filter({ hasText: "Adult websites" });
    const scheduled = page.getByTestId("network-policy").filter({ hasText: "Focus hours" });
    await expect(active).toBeVisible();
    await expect(scheduled).toBeVisible();
    for (const dark of [false, true]) {
        await page.evaluate((value) => document.documentElement.classList.toggle("dark", value), dark);
        await expect(active.locator(".state-rail span")).toHaveCSS("background-color", dark ? "rgb(74, 222, 128)" : "rgb(21, 128, 61)");
        await expect(scheduled.locator(".state-rail span")).toHaveCSS("background-color", dark ? "rgb(169, 166, 155)" : "rgb(139, 138, 130)");
        await expect(active.locator(".state-rail")).toHaveCSS("width", "5px");
        await expect(active.locator(".state-word")).toHaveCSS("color", dark ? "rgb(74, 222, 128)" : "rgb(21, 128, 61)");
        await expect(active.locator(".enable-switch")).toHaveCSS("background-color", dark ? "rgb(243, 240, 232)" : "rgb(16, 16, 14)");
        await page.screenshot({ path: `test-results/network-block-palette-${dark ? "dark" : "light"}.png` });
    }
});
