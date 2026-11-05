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

        // Gated features remain identifiable until hover/focus, without layout shifts.
        const defaultLabel = importAction.locator(".current-label");
        const upgradeLabel = importAction.locator(".upgrade-label");
        await expect(defaultLabel).toHaveCSS("opacity", "1");
        await expect(upgradeLabel).toHaveCSS("opacity", "0");
        const initialWidth = (await importAction.boundingBox())!.width;
        await importAction.hover();
        await expect(upgradeLabel).toHaveCSS("opacity", "1");
        await expect(defaultLabel).toHaveCSS("opacity", "0");
        expect((await importAction.boundingBox())!.width).toBe(initialWidth);
        await importAction.focus();
        await expect(upgradeLabel).toHaveCSS("opacity", "1");

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

    test("Control keeps its divider full height without stretching full width", async ({ page }) => {
        await page.setViewportSize({ width: 1200, height: 800 });
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Control" })).toBeVisible({
            timeout: 15_000,
        });

        // Browser-only layout fixture: never calls/mutates the real engine.
        await page.evaluate(async () => {
            const storeModules = [
                "/src/lib/stores/engine.svelte.ts",
                "/src/lib/stores/dashboard.svelte.ts",
                "/src/lib/stores/user-store.svelte.ts",
            ];
            const [{ engineStore }, { dashboardStore }, { userStore }] = await Promise.all(
                storeModules.map((modulePath) => import(/* @vite-ignore */ modulePath)),
            );
            userStore.accountInfoLoaded = true;
            userStore.tosAccepted = true;
            dashboardStore.loadError = false;
            dashboardStore.isLoading = false;
            dashboardStore.activeBlocksCount = 5;
            dashboardStore.totalBlocksCount = 5;
            dashboardStore.networkBlockSnapshots = Array.from({ length: 5 }, (_, index) => ({
                block: {
                    id: `test-${index}`,
                    name: `Policy ${index}`,
                    isActive: true,
                    lockState: { state: "inactive" },
                },
                scheduleEntries: [],
                sitePackConnectionCount: 1,
                userRulesCount: 2,
            })) as typeof dashboardStore.networkBlockSnapshots;
            engineStore.isAlive = true;
        });

        const board = page.locator(".state-board");
        await expect(board).toBeVisible();

        for (const width of [1200, 800]) {
            await page.setViewportSize({ width, height: 800 });
            const measurements = await board.evaluate((element) => {
                const left = element.querySelector(".now-board")!;
                const right = element.querySelector(".change-board")!;
                return {
                    boardWidth: element.getBoundingClientRect().width,
                    leftHeight: left.getBoundingClientRect().height,
                    rightHeight: right.getBoundingClientRect().height,
                    borderWidth: getComputedStyle(right).borderLeftWidth,
                };
            });
            expect(measurements.boardWidth).toBeLessThanOrEqual(920);
            expect(Math.abs(measurements.leftHeight - measurements.rightHeight)).toBeLessThan(1);
            expect(measurements.borderWidth).toBe("1px");
        }
    });
});
