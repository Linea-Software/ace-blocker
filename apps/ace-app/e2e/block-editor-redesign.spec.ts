import { expect, test } from "@playwright/test";

// Browser-only visual fixture. No privileged Tauri IPC or real policy writes.
test("Network Block wizard preserves the draft across steps and is usable at 800×600", async ({ page }) => {
    await page.setViewportSize({ width: 1200, height: 820 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: BlockEditor }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/routes/network-blocks/_BlockEditor.svelte"),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "block-editor-fixture";
        host.style.cssText = "position:fixed;inset:0;overflow:auto;z-index:200;background:var(--background);";
        document.body.append(host);
        mount(BlockEditor, {
            target: host,
            props: { block: null, onSave: () => {} },
        });
    });

    const editor = page.locator(".block-editor-page");
    const steps = editor.locator(".editor-steps button");
    await expect(editor.getByRole("heading", { name: "What do you want to block?" })).toBeVisible();
    await expect(editor.getByRole("heading", { name: "Categories", exact: true })).toBeVisible();
    await expect(editor.getByRole("textbox", { name: "Search categories and Custom Lists..." })).toBeVisible();
    await expect(steps).toHaveCount(3);
    await expect(steps.nth(0)).toHaveAttribute("aria-current", "step");
    await expect(editor.getByRole("heading", { name: "When should this block run?" })).toHaveCount(0);

    await editor.getByRole("button", { name: /Manage individual websites/ }).click();
    await expect(editor.locator('[data-testid="website-manager"]')).toHaveAttribute("open", "");
    const website = editor.getByRole("textbox", { name: "Specific website" });
    await website.fill("https://instagram.com/reels");
    await editor.getByRole("button", { name: "Add specific website" }).click();
    await expect(editor.getByText("instagram.com", { exact: true })).toBeVisible();
    await page.screenshot({ path: "test-results/network-block-wizard-wide.png", fullPage: false });

    await steps.nth(1).click();
    await expect(editor.getByRole("heading", { name: "When should this block run?" })).toBeVisible();
    await expect(editor.getByRole("button", { name: /Always/ })).toHaveAttribute("aria-pressed", "true");
    await editor.getByRole("button", { name: /Custom hours/ }).click();
    await expect(editor.getByText("Choose at least one period before saving.").first()).toBeVisible();

    await steps.nth(2).click();
    await expect(editor.getByRole("heading", { name: "Review your block" })).toBeVisible();
    await expect(editor.getByText("Individual websites: 1")).toBeVisible();
    await expect(editor.getByRole("button", { name: "Create and enable block" })).toBeDisabled();
    await editor.getByRole("textbox", { name: "Block name" }).fill("Social");
    await steps.nth(1).click();
    await editor.locator(".mode-selector button").first().click();
    await steps.nth(2).click();
    await expect(editor.getByRole("textbox", { name: "Block name" })).toHaveValue("Social");
    await expect(editor.getByRole("button", { name: "Create and enable block" })).toBeEnabled();

    await page.setViewportSize({ width: 800, height: 600 });
    await page.screenshot({ path: "test-results/network-block-wizard-compact.png", fullPage: false });
    await expect(editor.getByRole("button", { name: "Create and enable block" })).toBeInViewport();
    const overflow = await editor.evaluate((element) => ({
        scrollWidth: element.scrollWidth,
        clientWidth: element.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1);

    // Many individual sites remain a searchable, paged list rather than a tag cloud.
    await steps.nth(0).click();
    await editor.getByRole("button", { name: /Manage individual websites/ }).click();
    await editor.getByRole("button", { name: "Paste multiple websites" }).click();
    await editor.getByRole("textbox", { name: "Paste multiple websites" })
        .fill(Array.from({ length: 25 }, (_, i) => `site${i}.example.org`).join("\n"));
    await editor.getByRole("button", { name: "Add websites" }).click();
    await expect(editor.getByText("Page 1 of 2")).toBeVisible();
    await editor.getByRole("button", { name: "Next" }).click();
    await expect(editor.getByText("Page 2 of 2")).toBeVisible();
    await editor.getByRole("textbox", { name: "Search selected websites" }).fill("instagram");
    await expect(editor.getByText("instagram.com", { exact: true })).toBeVisible();
    await steps.nth(2).click();
    await expect(editor.getByText("Individual websites: 26")).toBeVisible();
    await expect(editor.getByRole("button", { name: "Create and enable block" })).toBeEnabled();
});

test("Category preview keeps selections, expands, and never truncates search results", async ({ page }) => {
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const svelte = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const deltaModule = new URL("/src/lib/deltas/site-packs-connection-delta.svelte.ts", location.origin).pathname;
        const userModule = new URL("/src/lib/stores/user-store.svelte.ts", location.origin).pathname;
        const [{ mount }, { default: SectionSites }, { SitePacksConnectionDelta }, { userStore }] =
            await Promise.all([
                import(/* @vite-ignore */ svelte),
                import(/* @vite-ignore */ "/src/routes/network-blocks/_components/SectionSites.svelte"),
                import(/* @vite-ignore */ deltaModule),
                import(/* @vite-ignore */ userModule),
            ]);
        userStore.isPro = false;
        const packs = [
            ...Array.from({ length: 9 }, (_, i) => ({
                id: `fixture-free-${i + 1}`, name: `Free category ${i + 1}`,
                isRemote: true, isSynced: true, requiresPro: false,
                entriesCount: 100, description: "Test category",
            })),
            ...Array.from({ length: 7 }, (_, i) => ({
                id: `fixture-pro-${i + 1}`, name: `Pro category ${i + 1}`,
                isRemote: true, isSynced: true, requiresPro: true,
                entriesCount: 100, description: "Test category",
            })),
        ];
        const host = document.createElement("div");
        host.dataset.testid = "categories-fixture";
        host.style.cssText = "position:fixed;inset:0;overflow:auto;z-index:200;background:var(--background);padding:25px;max-width:850px;";
        document.body.append(host);
        mount(SectionSites, { target: host, props: {
            delta: new SitePacksConnectionDelta(["fixture-free-9"]), availableLists: packs,
        } });
    });

    const root = page.getByTestId("categories-fixture");
    const preview = root.getByTestId("category-preview");
    const freeRows = preview.locator('button[aria-pressed]');
    const search = root.getByRole("textbox", { name: "Search categories and Custom Lists..." });
    await expect(freeRows).toHaveCount(6);
    await expect(preview.getByRole("button", { name: /Free category 9/ })).toHaveAttribute("aria-pressed", "true");
    await expect(preview.getByRole("button", { name: /Free category 6/ })).toHaveCount(0);
    await page.screenshot({ path: "test-results/block-categories-collapsed.png" });
    await root.getByRole("button", { name: "Show all (9)" }).click();
    await expect(freeRows).toHaveCount(9);
    await page.screenshot({ path: "test-results/block-categories-expanded.png" });
    await root.getByRole("button", { name: "Show less" }).click();
    await expect(freeRows).toHaveCount(6);
    await search.fill("Free category");
    await expect(freeRows).toHaveCount(9);
    await expect(root.getByRole("button", { name: "Show all (9)" })).toHaveCount(0);
    await search.clear();

    const pro = root.locator("details.pro-categories");
    await pro.locator("summary").click();
    const proRows = root.getByTestId("pro-category-preview").locator('button[aria-pressed]');
    await expect(proRows).toHaveCount(5);
    await pro.getByRole("button", { name: "Show all (7)" }).click();
    await expect(proRows).toHaveCount(7);
    await pro.getByRole("button", { name: "Show less" }).click();
    await expect(proRows).toHaveCount(5);
    await search.fill("Pro category");
    await expect(root.getByTestId("pro-category-search-results").locator('button[aria-pressed]')).toHaveCount(7);
    await expect(root.getByTestId("category-preview")).toHaveCount(0);
    await page.screenshot({ path: "test-results/block-category-search.png" });

    // A Pro account can reach every category with the same Show all control.
    await search.clear();
    await page.evaluate(async () => {
        const modulePath = new URL("/src/lib/stores/user-store.svelte.ts", location.origin).pathname;
        const { userStore } = await import(/* @vite-ignore */ modulePath);
        userStore.isPro = true;
    });
    await expect(root.locator("details.pro-categories")).toHaveCount(0);
    await expect(freeRows).toHaveCount(6);
    await root.getByRole("button", { name: "Show all (16)" }).click();
    await expect(freeRows).toHaveCount(16);
});

test("Editor hints at overflow, then hides the hint at the bottom", async ({ page }) => {
    await page.setViewportSize({ width: 800, height: 600 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: BlockEditor }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/routes/network-blocks/_BlockEditor.svelte"),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "editor-scroll-fixture";
        host.style.cssText = "position:fixed;inset:0;overflow:auto;z-index:200;background:var(--background);";
        document.body.append(host);
        mount(BlockEditor, { target: host, props: { block: null, onSave: () => {} } });
    });

    const fixture = page.getByTestId("editor-scroll-fixture");
    const editor = fixture.locator(".block-editor-page");
    await expect(editor.getByRole("heading", { name: "What do you want to block?" })).toBeVisible();
    await editor.locator(".editor-page-body").evaluate((body) => {
        const extra = document.createElement("div");
        extra.style.height = "1400px";
        body.append(extra);
    });
    await expect(editor.locator(".scroll-cue")).toBeVisible();
    await fixture.evaluate((host) => { host.scrollTop = host.scrollHeight; });
    await expect(editor.locator(".scroll-cue")).toHaveCount(0);
    await expect(editor.getByRole("button", { name: "Continue" })).toBeVisible();
});
