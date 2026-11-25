import { expect, test } from "@playwright/test";

// Browser-only visual fixture. No privileged Tauri IPC or real policy writes.
test("Network Block wizard preserves the draft across steps and is usable at 800×600", async ({
    page,
}) => {
    await page.setViewportSize({ width: 1200, height: 820 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/src/test-fixture-runtime.ts", location.origin)
            .pathname;
        const [{ mount }, { default: BlockEditor }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(
                /* @vite-ignore */ "/src/routes/network-blocks/_BlockEditor.tsx"
            ),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "block-editor-fixture";
        host.style.cssText =
            "position:fixed;inset:0;overflow:auto;z-index:200;background:var(--background);";
        document.body.append(host);
        mount(BlockEditor, {
            target: host,
            props: { block: null, onSave: () => {} },
        });
    });

    const editor = page.locator(".block-editor-page");
    const steps = editor.locator(".editor-steps button");
    await expect(
        editor.getByRole("heading", { name: "What do you want to block?" }),
    ).toBeVisible();
    await expect(
        editor.getByRole("heading", { name: "Categories", exact: true }),
    ).toBeVisible();
    await expect(
        editor
            .getByTestId("block-safe-search-setting")
            .getByRole("switch", { name: "Force Safe Search" }),
    ).toBeVisible();
    await expect(
        editor.getByRole("textbox", {
            name: "Search categories and Custom Lists...",
        }),
    ).toBeVisible();
    await expect(steps).toHaveCount(3);
    await expect(steps.nth(0)).toHaveAttribute("aria-current", "step");
    for (const dark of [false, true]) {
        await page.evaluate((value) => document.documentElement.classList.toggle("dark", value), dark);
        const blue = dark ? "rgb(96, 165, 250)" : "rgb(37, 99, 235)";
        await expect(steps.nth(0).locator(".step-progress")).toHaveCSS("background-color", blue);
        await expect(steps.nth(0).locator(".step-label")).toHaveCSS("color", blue);
        await expect(steps.nth(0).locator(".step-number")).toHaveCSS("background-color", blue);
        await expect(editor.locator(".editor-primary-action")).toHaveCSS("background-color", blue);
        await page.screenshot({ path: `test-results/editor-interaction-${dark ? "dark" : "light"}.png` });
    }

    await expect(
        editor.getByRole("heading", { name: "When should this block run?" }),
    ).toHaveCount(0);

    await editor
        .getByRole("button", { name: /Manage individual websites/ })
        .click();
    await expect(
        editor.locator('[data-testid="website-manager"]'),
    ).toHaveAttribute("open", "");
    const website = editor.getByRole("textbox", { name: "Specific website" });
    await website.fill("https://instagram.com/reels");
    await editor.getByRole("button", { name: "Add specific website" }).click();
    await expect(
        editor.getByText("instagram.com", { exact: true }),
    ).toBeVisible();
    await page.screenshot({
        path: "test-results/network-block-wizard-wide.png",
        fullPage: false,
    });

    await steps.nth(1).click();
    await expect(
        editor.getByRole("heading", { name: "When should this block run?" }),
    ).toBeVisible();
    await expect(
        editor.getByRole("button", { name: /Always/ }),
    ).toHaveAttribute("aria-pressed", "true");
    await editor.getByRole("button", { name: /Custom hours/ }).click();
    await expect(
        editor.getByText("Choose at least one period before saving.").first(),
    ).toBeVisible();

    await steps.nth(2).click();
    await expect(
        editor.getByRole("heading", { name: "Review your block" }),
    ).toBeVisible();
    await expect(editor.getByText("Individual websites: 1")).toBeVisible();
    await expect(
        editor.getByRole("button", { name: "Create and enable block" }),
    ).toBeDisabled();
    await editor.getByRole("textbox", { name: "Block name" }).fill("Social");
    await steps.nth(1).click();
    await editor.locator(".mode-selector button").first().click();
    await steps.nth(2).click();
    await expect(
        editor.getByRole("textbox", { name: "Block name" }),
    ).toHaveValue("Social");
    await expect(
        editor.getByRole("button", { name: "Create and enable block" }),
    ).toBeEnabled();

    await page.setViewportSize({ width: 800, height: 600 });
    await page.screenshot({
        path: "test-results/network-block-wizard-compact.png",
        fullPage: false,
    });
    await expect(
        editor.getByRole("button", { name: "Create and enable block" }),
    ).toBeInViewport();
    const overflow = await editor.evaluate((element) => ({
        scrollWidth: element.scrollWidth,
        clientWidth: element.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1);

    // Many individual sites remain a searchable, paged list rather than a tag cloud.
    await steps.nth(0).click();
    await editor
        .getByRole("button", { name: /Manage individual websites/ })
        .click();
    await editor
        .getByRole("button", { name: "Paste multiple websites" })
        .click();
    await editor
        .getByRole("textbox", { name: "Paste multiple websites" })
        .fill(
            Array.from({ length: 25 }, (_, i) => `site${i}.example.org`).join(
                "\n",
            ),
        );
    await editor.getByRole("button", { name: "Add websites" }).click();
    await expect(editor.getByText("Page 1 of 2")).toBeVisible();
    await editor.getByRole("button", { name: "Next" }).click();
    await expect(editor.getByText("Page 2 of 2")).toBeVisible();
    await editor
        .getByRole("textbox", { name: "Search selected websites" })
        .fill("instagram");
    await expect(
        editor.getByText("instagram.com", { exact: true }),
    ).toBeVisible();
    await steps.nth(2).click();
    await expect(editor.getByText("Individual websites: 26")).toBeVisible();
    await expect(
        editor.getByRole("button", { name: "Create and enable block" }),
    ).toBeEnabled();
});

test("Safe Search-only blocks expose their coverage and keep the settings in the draft", async ({
    page,
}) => {
    await page.setViewportSize({ width: 800, height: 600 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/src/test-fixture-runtime.ts", location.origin)
            .pathname;
        const [{ mount }, { default: BlockEditor }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(
                /* @vite-ignore */ "/src/routes/network-blocks/_BlockEditor.tsx"
            ),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "safe-search-editor-fixture";
        // The IconPicker portal has z-index 80; the fixture must sit below it.
        host.style.cssText =
            "position:fixed;inset:0;overflow:auto;z-index:70;background:var(--background);";
        document.body.append(host);
        mount(BlockEditor, {
            target: host,
            props: { block: null, onSave: () => {} },
        });
    });

    const editor = page
        .getByTestId("safe-search-editor-fixture")
        .locator(".block-editor-page");
    const steps = editor.locator(".editor-steps button");
    const safeSearchRow = editor.getByTestId("block-safe-search-setting");
    const safeSearch = safeSearchRow.getByRole("switch", {
        name: "Force Safe Search",
    });
    await expect(safeSearch).toHaveAttribute("aria-checked", "false");

    // The icon and text are part of the clickable label, not only the small switch.
    await safeSearchRow.locator("label").click({ position: { x: 22, y: 22 } });
    await expect(safeSearch).toHaveAttribute("aria-checked", "true");
    await expect(editor.getByTestId("safe-search-only-hint")).toBeVisible();

    await steps.nth(2).click();
    const iconTrigger = editor.getByRole("button", { name: "Icon" });
    await expect(iconTrigger).toBeVisible();
    expect(
        await iconTrigger.evaluate((el) => el.getBoundingClientRect().width),
    ).toBeGreaterThanOrEqual(44);
    await expect(editor.getByText("More options")).toHaveCount(0);
    await editor
        .getByRole("textbox", { name: "Block name" })
        .fill("Safe searches");
    await expect(editor.getByTestId("review-safe-search")).toContainText(
        "Force Safe Search: Included",
    );
    await expect(
        editor.getByText("With only Safe Search selected", { exact: false }),
    ).toBeVisible();
    await expect(
        editor.getByRole("button", { name: "Create and enable block" }),
    ).toBeEnabled();

    await iconTrigger.click();
    await page.getByRole("textbox", { name: "Search icons…" }).fill("gaming");
    await page.getByRole("button", { name: "gaming", exact: true }).click();
    await expect(
        page.getByRole("textbox", { name: "Search icons…" }),
    ).not.toBeVisible();
    await expect(editor.locator("#general svg.lucide-gamepad-2")).toBeVisible();

    await page.setViewportSize({ width: 390, height: 750 });
    await page.screenshot({
        path: "test-results/network-block-safe-search-mobile.png",
    });
    const overflow = await editor.evaluate((el) => ({
        width: el.clientWidth,
        scroll: el.scrollWidth,
    }));
    expect(overflow.scroll).toBeLessThanOrEqual(overflow.width + 1);

    await steps.nth(0).click();
    await page.screenshot({
        path: "test-results/network-block-safe-search-content-mobile.png",
    });
    const contentOverflow = await editor.evaluate((el) => ({
        width: el.clientWidth,
        scroll: el.scrollWidth,
    }));
    expect(contentOverflow.scroll).toBeLessThanOrEqual(
        contentOverflow.width + 1,
    );
    await safeSearchRow.getByRole("switch").click();
    await expect(safeSearchRow.getByRole("switch")).toHaveAttribute(
        "aria-checked",
        "false",
    );
    await expect(editor.getByTestId("safe-search-only-hint")).toHaveCount(0);
    await steps.nth(2).click();
    await expect(editor.getByTestId("review-safe-search")).toContainText(
        "Not included",
    );
    await expect(
        editor.getByRole("button", { name: "Create and enable block" }),
    ).toBeDisabled();
});

test("Category preview keeps selections, expands, and never truncates search results", async ({
    page,
}) => {
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const svelte = new URL("/src/test-fixture-runtime.ts", location.origin)
            .pathname;
        const deltaModule = new URL(
            "/src/lib/deltas/site-packs-connection-delta.ts",
            location.origin,
        ).pathname;
        const userModule = new URL(
            "/src/lib/stores/user-store.ts",
            location.origin,
        ).pathname;
        const [
            { mount },
            { default: SectionSites },
            { SitePacksConnectionDelta },
            { userStore },
        ] = await Promise.all([
            import(/* @vite-ignore */ svelte),
            import(
                /* @vite-ignore */ "/src/routes/network-blocks/_components/SectionSites.tsx"
            ),
            import(/* @vite-ignore */ deltaModule),
            import(/* @vite-ignore */ userModule),
        ]);
        userStore.isPro = false;
        (
            window as unknown as { __sitePacksTestUserStore: typeof userStore }
        ).__sitePacksTestUserStore = userStore;
        const packs = [
            ...Array.from({ length: 9 }, (_, i) => ({
                id: `fixture-free-${i + 1}`,
                name: `Free category ${i + 1}`,
                isRemote: true,
                isSynced: true,
                requiresPro: false,
                entriesCount: 100,
                description: "Test category",
            })),
            ...Array.from({ length: 7 }, (_, i) => ({
                id: `fixture-pro-${i + 1}`,
                name: `Pro category ${i + 1}`,
                isRemote: true,
                isSynced: true,
                requiresPro: true,
                entriesCount: 100,
                description: "Test category",
            })),
        ];
        const host = document.createElement("div");
        host.dataset.testid = "categories-fixture";
        host.style.cssText =
            "position:fixed;inset:0;overflow:auto;z-index:200;background:var(--background);padding:25px;max-width:850px;";
        document.body.append(host);
        mount(SectionSites, {
            target: host,
            props: {
                delta: new SitePacksConnectionDelta(["fixture-free-9"]),
                availableLists: packs,
            },
        });
    });

    const root = page.getByTestId("categories-fixture");
    const preview = root.getByTestId("category-preview");
    const freeRows = preview.locator("button[aria-pressed]");
    const search = root.getByRole("textbox", {
        name: "Search categories and Custom Lists...",
    });
    await expect(freeRows).toHaveCount(6);
    await expect(
        preview.getByRole("button", { name: /Free category 9/ }),
    ).toHaveAttribute("aria-pressed", "true");
    await expect(
        preview.getByRole("button", { name: /Free category 6/ }),
    ).toHaveCount(0);
    await page.screenshot({
        path: "test-results/block-categories-collapsed.png",
    });
    await root.getByRole("button", { name: "Show all (9)" }).click();
    await expect(freeRows).toHaveCount(9);
    await page.screenshot({
        path: "test-results/block-categories-expanded.png",
    });
    await root.getByRole("button", { name: "Show less" }).click();
    await expect(freeRows).toHaveCount(6);
    await search.fill("Free category");
    await expect(freeRows).toHaveCount(9);
    await expect(
        root.getByRole("button", { name: "Show all (9)" }),
    ).toHaveCount(0);
    await search.clear();

    const pro = root.locator("details.pro-categories");
    await pro.locator("summary").click();
    const proRows = root
        .getByTestId("pro-category-preview")
        .locator("button[aria-pressed]");
    await expect(proRows).toHaveCount(5);
    await pro.getByRole("button", { name: "Show all (7)" }).click();
    await expect(proRows).toHaveCount(7);
    await pro.getByRole("button", { name: "Show less" }).click();
    await expect(proRows).toHaveCount(5);
    await search.fill("Pro category");
    await expect(
        root
            .getByTestId("pro-category-search-results")
            .locator("button[aria-pressed]"),
    ).toHaveCount(7);
    await expect(root.getByTestId("category-preview")).toHaveCount(0);
    await page.screenshot({ path: "test-results/block-category-search.png" });

    // The Free user's Pro-only catalog remains accessible behind its disclosure.
    await search.clear();
    await expect(root.locator("details.pro-categories")).toHaveCount(1);
});

test("Pro selects Extended directly and cannot remove locked category connections", async ({
    page,
}) => {
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/src/test-fixture-runtime.ts", location.origin)
            .pathname;
        const delta = new URL(
            "/src/lib/deltas/site-packs-connection-delta.ts",
            location.origin,
        ).pathname;
        const store = new URL("/src/lib/stores/user-store.ts", location.origin)
            .pathname;
        const [
            { mount },
            { default: SectionSites },
            { SitePacksConnectionDelta },
            { userStore },
        ] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(
                /* @vite-ignore */ "/src/routes/network-blocks/_components/SectionSites.tsx"
            ),
            import(/* @vite-ignore */ delta),
            import(/* @vite-ignore */ store),
        ]);
        userStore.isPro = true;
        const base = {
            isRemote: true,
            isSynced: true,
            isBundled: false,
            isLocked: false,
            entriesCount: 123,
            icon: null,
            description: "Test catalog",
        };
        const packs = [
            {
                ...base,
                id: "pack-gambling-free",
                name: "Gambling Websites — Free",
                requiresPro: false,
            },
            {
                ...base,
                id: "pack-gambling-pro",
                name: "Gambling Websites — Pro",
                requiresPro: true,
            },
        ];
        for (const [name, isLocked] of [
            ["switchable", false],
            ["locked", true],
        ] as const) {
            const host = document.createElement("div");
            host.dataset.testid = name + "-tier-fixture";
            host.style.cssText = "padding:20px;max-width:860px;";
            document.body.append(host);
            const draft = new SitePacksConnectionDelta(isLocked ? ["pack-gambling-free", "pack-gambling-pro"] : []);
            Object.assign(host, { draft });
            mount(SectionSites, {
                target: host,
                props: {
                    delta: draft,
                    availableLists: packs,
                    isLocked,
                },
            });
        }
    });
    const versions = await page.evaluate(async () => {
        const urls = [
            ...new Set(
                performance
                    .getEntriesByType("resource")
                    .map((entry) => new URL(entry.name))
                    .filter((url) =>
                        url.pathname.endsWith("/src/lib/stores/user-store.ts"),
                    )
                    .map((url) => url.pathname + url.search),
            ),
        ];
        for (const url of urls) {
            const { userStore } = await import(/* @vite-ignore */ url);
            userStore.isPro = true;
        }
        return urls;
    });
    expect(versions.length).toBeGreaterThan(0);
    const unlocked = page.getByTestId("switchable-tier-fixture").getByTestId("category-family-option");
    await expect(unlocked).toHaveCount(1);
    await expect(unlocked).toContainText("Pro · Extended");
    await unlocked.click();
    await expect(unlocked).toHaveAttribute("aria-pressed", "true");
    expect(await page.getByTestId("switchable-tier-fixture").evaluate((el: any) => el.draft.currentConnectedListIds)).toEqual(["pack-gambling-pro"]);
    const locked = page.getByTestId("locked-tier-fixture").getByTestId("category-family-option");
    await expect(locked).toHaveCount(1);
    await expect(locked).toHaveAttribute("aria-pressed", "true");
    await expect(locked).toHaveAttribute("aria-disabled", "true");
    expect(await page.getByTestId("locked-tier-fixture").evaluate((el: any) => el.draft.currentConnectedListIds)).toEqual(["pack-gambling-free", "pack-gambling-pro"]);
});

test("Editor hints at overflow, then hides the hint at the bottom", async ({
    page,
}) => {
    await page.setViewportSize({ width: 800, height: 600 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/src/test-fixture-runtime.ts", location.origin)
            .pathname;
        const [{ mount }, { default: BlockEditor }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(
                /* @vite-ignore */ "/src/routes/network-blocks/_BlockEditor.tsx"
            ),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "editor-scroll-fixture";
        host.style.cssText =
            "position:fixed;inset:0;overflow:auto;z-index:200;background:var(--background);";
        document.body.append(host);
        mount(BlockEditor, {
            target: host,
            props: { block: null, onSave: () => {} },
        });
    });

    const fixture = page.getByTestId("editor-scroll-fixture");
    const editor = fixture.locator(".block-editor-page");
    await expect(
        editor.getByRole("heading", { name: "What do you want to block?" }),
    ).toBeVisible();
    await editor.locator(".editor-page-body").evaluate((body) => {
        const extra = document.createElement("div");
        extra.style.height = "1400px";
        body.append(extra);
    });
    await expect(editor.locator(".scroll-cue")).toBeVisible();
    await fixture.evaluate((host) => {
        host.scrollTop = host.scrollHeight;
    });
    await expect(editor.locator(".scroll-cue")).toHaveCount(0);
    await expect(
        editor.getByRole("button", { name: "Continue" }),
    ).toBeVisible();
});

test("Official Site Pack icons and Free/Pro coverage are visible in both selection surfaces", async ({
    page,
}) => {
    await page.setViewportSize({ width: 1000, height: 740 });
    await page.addInitScript(() => {
        localStorage.setItem("ace:onboarding:v1", JSON.stringify({ wizardCompleted: true, tourCompleted: true }));
        Object.assign(window, {
            isTauri: true,
            __TAURI_INTERNALS__: {
                invoke: async (cmd: string, args: { op?: string }) => {
                    if (cmd === "system_locale") return "en-US";
                    if (cmd === "engine_call" && ["getAccountInfo", "refreshAccount"].includes(args.op ?? "")) return { isPro: false, tosAccepted: true };
                    if (cmd === "engine_call" && args.op === "ping") return null;
                    throw new Error("Unsupported fixture operation");
                },
                transformCallback: () => 1, unregisterCallback: () => {},
            },
        });
    });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const svelte = new URL("/src/test-fixture-runtime.ts", location.origin)
            .pathname;
        const delta = new URL(
            "/src/lib/deltas/site-packs-connection-delta.ts",
            location.origin,
        ).pathname;
        const store = new URL("/src/lib/stores/user-store.ts", location.origin)
            .pathname;
        const [
            { mount },
            { default: SectionSites },
            { default: SitePackRow },
            { SitePacksConnectionDelta },
            { userStore },
        ] = await Promise.all([
            import(/* @vite-ignore */ svelte),
            import(
                /* @vite-ignore */ "/src/routes/network-blocks/_components/SectionSites.tsx"
            ),
            import(
                /* @vite-ignore */ "/src/lib/components/ace/domain/SitePackRow.tsx"
            ),
            import(/* @vite-ignore */ delta),
            import(/* @vite-ignore */ store),
        ]);
        userStore.isPro = false;
        const base = {
            description: "Test catalog",
            isBundled: false,
            isLocked: false,
            isRemote: true,
            isSynced: true,
            entriesCount: 0,
            icon: null,
        };
        const free = {
            ...base,
            id: "pack-gambling-free",
            name: "Gambling Websites — Free",
            requiresPro: false,
            entriesCount: 14,
        };
        const pro = {
            ...base,
            id: "pack-gambling-pro",
            name: "Gambling Websites — Pro",
            requiresPro: true,
            entriesCount: 454090,
        };
        const host = document.createElement("div");
        host.dataset.testid = "pack-family-fixture";
        host.style.cssText =
            "position:fixed;inset:0;overflow:auto;z-index:70;background:var(--background);padding:22px;container:page / inline-size;";
        document.body.append(host);
        const selection = document.createElement("section");
        host.append(selection);
        const draft = new SitePacksConnectionDelta();
        Object.assign(selection, { draft });
        mount(SectionSites, {
            target: selection,
            props: {
                delta: draft,
                availableLists: [free, pro],
            },
        });
        const ledger = document.createElement("section");
        ledger.style.cssText = "max-width:840px;margin-top:30px;";
        host.append(ledger);
        mount(SitePackRow, {
            target: ledger,
            props: { pack: free, extendedPack: pro, basicPack: free, isPro: false, onSelect: () => {} },
        });
    });

    const fixture = page.getByTestId("pack-family-fixture");
    const row = fixture.getByTestId("category-family-option");
    await expect(row).toHaveCount(1);
    await expect(row.getByTestId("category-icon").locator("svg.lucide-circle-dollar-sign")).toBeVisible();
    await expect(row.getByTestId("category-tier-comparison")).toHaveText("454,090 sites with Pro");
    await expect(row.getByTestId("category-coverage")).toContainText("14 sites");
    await expect(fixture.locator("details.pro-categories")).toHaveCount(0);
    await row.click();
    await expect(row).toHaveAttribute("aria-pressed", "true");
    expect(await fixture.locator("section").first().evaluate((el: any) => el.draft.currentConnectedListIds)).toEqual(["pack-gambling-free"]);
    const ledger = fixture.getByTestId("site-pack-row");
    await expect(ledger).toHaveCount(1);
    await expect(ledger.getByTestId("site-pack-row-icon")).toHaveCount(1);
    await expect(ledger.getByTestId("site-pack-tier-comparison")).toHaveText("454,090 sites with Pro");

    await page.setViewportSize({ width: 390, height: 750 });
    await page.screenshot({ path: "test-results/site-pack-tiers-mobile.png" });
    const overflow = await fixture.evaluate((el) => ({
        width: el.clientWidth,
        scroll: el.scrollWidth,
    }));
    expect(overflow.scroll).toBeLessThanOrEqual(overflow.width + 1);
});
