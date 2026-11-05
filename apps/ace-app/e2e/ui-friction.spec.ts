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
                storeModules.map((modulePath) => {
                    // HMR appends ?t=...; a bare import would create a second
                    // store instance and leave the actual Control view offline.
                    const loaded = performance.getEntriesByType("resource")
                        .map((entry) => new URL(entry.name))
                        .filter((url) => url.pathname === modulePath)
                        .at(-1);
                    return import(/* @vite-ignore */ (loaded ? loaded.pathname + loaded.search : modulePath));
                }),
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

test("official Pro packs remain identifiable for Free and Pro accounts", async ({
    page,
}) => {
    await page.goto("/site-packs");
    await page.evaluate(async () => {
        const svelteModule = new URL(
            "/node_modules/.vite/deps/svelte.js",
            location.origin,
        ).pathname;
        const [{ mount }, { default: SitePackRow }] = await Promise.all([
            import(/* @vite-ignore */ svelteModule),
            import(
                /* @vite-ignore */ "/src/lib/components/ace/domain/SitePackRow.svelte"
            ),
        ]);
        const row = {
            id: "server-managed-uuid",
            name: "Gaming Websites — Pro",
            description: "Official gaming category",
            icon: "gaming",
            isRemote: true,
            isSynced: true,
            isBundled: false,
            isLocked: false,
            entriesCount: 2345,
            requiresPro: true,
            lastUpdatedAt: null,
            version: "2026.11.1",
        };
        for (const isPro of [false, true]) {
            const fixture = document.createElement("div");
            fixture.dataset.testid = isPro ? "pro-pack" : "free-pack";
            document.body.append(fixture);
            mount(SitePackRow, {
                target: fixture,
                props: {
                    pack: row,
                    isPro,
                    subtitle: "Curated by Ace",
                    onSelect: () => {},
                },
            });
        }
    });
    await expect(page.locator('[data-testid="free-pack"]')).toContainText(
        "Pro required",
    );
    await expect(page.locator('[data-testid="pro-pack"]')).toContainText(
        "Downloaded",
    );
});

test("Resistance logos use the bundled brand colors", async ({ page }) => {
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const svelteModule = new URL(
            "/node_modules/.vite/deps/svelte.js",
            location.origin,
        ).pathname;
        const [{ mount }, { default: ScriptSections }] = await Promise.all([
            import(/* @vite-ignore */ svelteModule),
            import(
                /* @vite-ignore */ "/src/routes/resistance-mode/_components/ScriptSections.svelte"
            ),
        ]);
        const fixture = document.createElement("div");
        fixture.dataset.testid = "script-brands";
        document.body.append(fixture);
        mount(ScriptSections, {
            target: fixture,
            props: {
                scripts: [
                    {
                        id: "1",
                        name: "Reduce shorts",
                        domain: "youtube.com",
                        description: "",
                        isEnabled: true,
                    },
                    {
                        id: "2",
                        name: "Quiet feed",
                        domain: "reddit.com",
                        description: "",
                        isEnabled: true,
                    },
                ],
                settings: null,
                engineOnline: true,
                isPro: true,
                locked: false,
                onToggleScript: () => {},
                onToggleDomain: () => {},
                onSync: () => {},
            },
        });
    });
    await expect(page.locator('[data-site-brand-icon="FF0000"]')).toBeVisible();
    await expect(page.locator('[data-site-brand-icon="FF4500"]')).toBeVisible();
    await expect(page.locator('[data-site-brand-icon="FF0000"]')).toHaveCSS(
        "color",
        "rgb(255, 0, 0)",
    );
    await expect(page.locator('[data-site-brand-icon="FF0000"] svg')).toHaveCSS(
        "fill",
        "rgb(255, 0, 0)",
    );
});
