import { expect, test } from "@playwright/test";

test.describe("Desktop UI friction", () => {
    test("Developer settings expose local license and consent controls in dev", async ({ page }) => {
        await page.setViewportSize({ width: 800, height: 600 });
        await page.goto("/settings");

        // Browser-only smoke: controls are rendered, but no privileged RPC is
        // called because the real engine is only available inside Tauri.
        await expect(page.getByText("Developer diagnostics", { exact: true })).toBeAttached();
        await expect(page.locator("#debug-license-policy option")).toHaveText([
            "Default (saved license)",
            "Free (simulated)",
            "Pro (simulated)",
        ]);
        await expect(page.getByRole("button", {
            name: "Replay consent dialog",
            includeHidden: true,
        })).toHaveCount(1);
    });

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

    test("Control fills large screens without overflowing compact windows", async ({ page }) => {
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
                    enabled: true,
                    lockState: { state: "inactive" },
                },
                scheduleEntries: [],
                sitePackConnectionCount: 1,
                userRulesCount: 2,
            })) as typeof dashboardStore.networkBlockSnapshots;
            engineStore.isAlive = true;
        });

        const board = page.locator(".control-dashboard");
        await expect(board).toBeVisible();
        await expect(board.locator(".protection-overview")).toBeVisible();
        await expect(board.getByRole("heading", { name: "What does locking a block do?" })).toBeVisible();

        for (const width of [1600, 1200, 800, 580]) {
            await page.setViewportSize({ width, height: 800 });
            const measurements = await board.evaluate((element) => {
                const grid = element.querySelector(".dashboard-grid")!;
                const pageLayout = element.closest(".page-layout")!;
                return {
                    layoutWidth: pageLayout.getBoundingClientRect().width,
                    columnCount: getComputedStyle(grid).gridTemplateColumns.split(" ").length,
                    scrollWidth: element.scrollWidth,
                    clientWidth: element.clientWidth,
                };
            });
            expect(measurements.layoutWidth).toBeLessThanOrEqual(1280);
            expect(measurements.scrollWidth).toBeLessThanOrEqual(measurements.clientWidth + 1);
            expect(measurements.columnCount).toBe(width >= 1200 ? 2 : 1);
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

test("Custom List icon picker stays inside the window and above dialog footer", async ({ page }) => {
    await page.setViewportSize({ width: 880, height: 720 });
    await page.goto("/site-packs");
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: ImportPackModal }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/routes/site-packs/_ImportPackModal.svelte"),
        ]);
        const host = document.createElement("div");
        document.body.append(host);
        mount(ImportPackModal, { target: host, props: { open: true } });
    });
    const dialog = page.getByRole("dialog").first();
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "Choose icon" }).click();
    const iconPanel = page.locator('[data-slot="popover-content"]');
    await expect(iconPanel).toBeVisible();

    const desktop = await iconPanel.evaluate((el) => {
        const box = el.getBoundingClientRect();
        const trigger = document.querySelector('button[aria-label="Choose icon"]')!.getBoundingClientRect();
        return { left: box.left, right: box.right, top: box.top, bottom: box.bottom,
            triggerLeft: trigger.left, width: innerWidth, height: innerHeight };
    });
    expect(desktop.left).toBeGreaterThanOrEqual(10);
    expect(desktop.right).toBeLessThanOrEqual(desktop.width - 10);
    expect(desktop.top).toBeGreaterThanOrEqual(10);
    expect(desktop.bottom).toBeLessThanOrEqual(desktop.height - 10);
    expect(desktop.right).toBeLessThan(desktop.triggerLeft);
    await page.screenshot({ path: "test-results/ui-audit/32-picker-fixed-desktop.png" });

    await page.setViewportSize({ width: 430, height: 680 });
    await expect(iconPanel).toBeVisible();
    const compact = await iconPanel.evaluate((el) => {
        const box = el.getBoundingClientRect();
        return { left: box.left, right: box.right, top: box.top, bottom: box.bottom,
            width: innerWidth, height: innerHeight };
    });
    expect(compact.left).toBeGreaterThanOrEqual(10);
    expect(compact.right).toBeLessThanOrEqual(compact.width - 10);
    expect(compact.top).toBeGreaterThanOrEqual(10);
    expect(compact.bottom).toBeLessThanOrEqual(compact.height - 10);
    await page.screenshot({ path: "test-results/ui-audit/33-picker-fixed-compact.png" });
});

test("Lock setup explains each mode and rejects an unscheduled or undated hard lock", async ({ page }) => {
    await page.setViewportSize({ width: 840, height: 720 });
    await page.goto("/network-blocks");
    // A local Svelte component fixture; no privileged engine request is sent.
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: LockModal }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/routes/network-blocks/_components/_LockModal.svelte"),
        ]);
        const fixture = document.createElement("div");
        document.body.append(fixture);
        mount(LockModal, {
            target: fixture,
            props: {
                open: true,
                block: {
                    id: "lock-fixture",
                    name: "Focus",
                    enabled: true,
                    isActive: true,
                    isLocked: false,
                    lockState: { state: "inactive" },
                    schedules: [],
                },
            },
        });
    });
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByRole("heading", { name: "Protect this Network Block" })).toBeVisible();
    await expect(dialog.getByRole("button", { name: /Reflection lock/ })).toHaveAttribute("aria-pressed", "true");
    await dialog.getByRole("button", { name: /During scheduled hours/ }).click();
    await expect(dialog.getByRole("alert")).toContainText("Add a schedule");
    await expect(dialog.getByRole("button", { name: "Review lock" })).toBeDisabled();
    await dialog.getByRole("button", { name: /Until a date and time/ }).click();
    await expect(dialog.getByRole("alert")).toContainText("Choose a date and time");
    await expect(dialog.getByRole("button", { name: "Review lock" })).toBeDisabled();
    await dialog.getByRole("button", { name: /Reflection lock/ }).click();
    await expect(dialog.getByRole("button", { name: "Enable reflection lock" })).toBeEnabled();
});

test("Reviewing a scheduled lock only offers removal outside active hours", async ({ page }) => {
    await page.goto("/network-blocks");
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: LockModal }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/routes/network-blocks/_components/_LockModal.svelte"),
        ]);
        mount(LockModal, {
            target: document.body,
            props: {
                open: true,
                block: {
                    id: "lock-fixture",
                    name: "Work hours",
                    enabled: true,
                    isActive: false,
                    isLocked: false,
                    lockState: { state: "active_during_schedule" },
                    schedules: [{ monday: true, startTime: "09:00", endTime: "17:00" }],
                },
            },
        });
    });
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByRole("heading", { name: "Review protection" })).toBeVisible();
    await dialog.getByRole("button", { name: "Remove scheduled lock" }).click();
    await expect(dialog.getByText("The scheduled lock will be removed.", { exact: false })).toBeVisible();
    await dialog.getByRole("button", { name: "Remove scheduled lock" }).last().click();
    await expect(dialog.getByText("Remove the scheduled lock?", { exact: false })).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Confirm removal" })).toBeVisible();
});
