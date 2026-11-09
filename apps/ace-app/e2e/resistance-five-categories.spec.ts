import { expect, test, type Page } from "@playwright/test";

async function mountResistanceFixture(page: Page, locked = false) {
    await page.goto("/getting-started");
    await page.evaluate(async (isLocked) => {
        const svelteModule = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: ScriptSections }] = await Promise.all([
            import(/* @vite-ignore */ svelteModule),
            import(/* @vite-ignore */ "/src/routes/resistance-mode/_components/ScriptSections.svelte"),
        ]);
        const fixture = document.createElement("div");
        fixture.id = "resistance-five-categories-fixture";
        document.body.append(fixture);
        const state = window as typeof window & {
            __resistanceBulkActions?: Array<{ ids: string[]; enabled: boolean }>;
        };
        state.__resistanceBulkActions = [];
        mount(ScriptSections, {
            target: fixture,
            props: {
                scripts: [
                    { id: "reddit_hide_recommendations", name: "Hide recommendations", domain: "reddit.com", description: "Recommendations", isEnabled: true },
                    { id: "reddit_hide_feed", name: "Hide feed", domain: "reddit.com", description: "Feed", isEnabled: false },
                    { id: "youtube_shorts_remove", name: "Remove Shorts", domain: "youtube.com", description: "Shorts", isEnabled: true },
                    { id: "youtube_grayscale", name: "Grayscale", domain: "youtube.com", description: "Colors", isEnabled: true },
                    { id: "youtube_audio_delay", name: "Audio delay", domain: "youtube.com", description: "Delay", isEnabled: false },
                    { id: "youtube_random_captcha", name: "Simulated CAPTCHA", domain: "youtube.com", description: "Verification", isEnabled: false },
                ],
                settings: { enabled: true, enforcementStatus: "operational", scriptsVersion: "1.0" },
                engineOnline: true,
                isPro: true,
                locked: isLocked,
                onToggleScript: () => {},
                onToggleGroup: (scripts: Array<{ id: string }>, enabled: boolean) => {
                    state.__resistanceBulkActions?.push({ ids: scripts.map((script) => script.id), enabled });
                },
            },
        });
    }, locked);
    return page.locator("#resistance-five-categories-fixture");
}

test("Resistance starts with all five categories collapsed and no extra grouping", async ({ page }) => {
    const fixture = await mountResistanceFixture(page);
    const categories = fixture.locator('[data-testid^="rm-category-"]:not([data-testid="rm-category-list"]):not([data-testid^="rm-category-disclosure-"])');
    await expect(categories).toHaveCount(5);
    for (const id of ["feeds", "video", "visual", "interruptions", "captcha"]) {
        const category = fixture.getByTestId("rm-category-" + id);
        await expect(category.getByTestId("rm-category-disclosure-" + id)).toHaveAttribute("aria-expanded", "false");
    }
    await expect(fixture.locator('[data-testid^="rm-select-all-"]')).toHaveCount(5);
    await expect(fixture.getByTestId("rm-select-all-captcha")).toHaveAttribute("aria-checked", "false");
    await expect(fixture.locator('[data-testid="rm-site-row"]')).toHaveCount(0);
    await fixture.screenshot({ path: "test-results/resistance-five-categories-closed.png" });
});

test("collapsed category checkbox toggles without opening details, including opt-in CAPTCHA", async ({ page }) => {
    const fixture = await mountResistanceFixture(page);
    const captcha = fixture.getByTestId("rm-category-captcha");
    const captchaCheckbox = captcha.getByTestId("rm-select-all-captcha");
    await expect(captchaCheckbox).toHaveAttribute("aria-checked", "false");
    await captchaCheckbox.click();
    await expect(captcha.getByTestId("rm-category-disclosure-captcha")).toHaveAttribute("aria-expanded", "false");
    await expect(captcha.getByText("Simulated CAPTCHA")).toHaveCount(0);

    const feeds = fixture.getByTestId("rm-category-feeds");
    const aggregate = feeds.getByTestId("rm-select-all-feeds");
    await expect(aggregate).toHaveAttribute("aria-checked", "mixed");
    await aggregate.click();
    await expect(feeds.getByTestId("rm-category-disclosure-feeds")).toHaveAttribute("aria-expanded", "false");
    await expect(fixture.locator('[data-testid="rm-site-row"]')).toHaveCount(0);
    const result = await page.evaluate(() => (window as any).__resistanceBulkActions);
    expect(result).toEqual([
        { ids: ["youtube_random_captcha"], enabled: true },
        { ids: ["reddit_hide_recommendations", "reddit_hide_feed"], enabled: true },
    ]);
    await fixture.screenshot({ path: "test-results/resistance-five-categories-open.png" });
});

test("a checked category checkbox clears the selected interventions when unlocked", async ({ page }) => {
    const fixture = await mountResistanceFixture(page);
    const all = fixture.getByTestId("rm-select-all-visual");
    await expect(all).toHaveAttribute("aria-checked", "true");
    await expect(all).toBeEnabled();
    await all.click();
    await expect(fixture.getByTestId("rm-category-disclosure-visual")).toHaveAttribute("aria-expanded", "false");
    const result = await page.evaluate(() => (window as any).__resistanceBulkActions);
    expect(result).toEqual([{ ids: ["youtube_grayscale"], enabled: false }]);
});

test("a Time Lock allows adding interventions but prevents clearing a complete category", async ({ page }) => {
    const fixture = await mountResistanceFixture(page, true);
    await expect(fixture.getByTestId("rm-select-all-visual")).toBeDisabled();

    const partial = fixture.getByTestId("rm-select-all-feeds");
    await expect(partial).toHaveAttribute("aria-checked", "mixed");
    await expect(partial).toBeEnabled();
    await partial.click();
    const result = await page.evaluate(() => (window as any).__resistanceBulkActions);
    expect(result).toEqual([{
        ids: ["reddit_hide_recommendations", "reddit_hide_feed"],
        enabled: true,
    }]);
});

test("script updates are in Settings, not the Resistance configuration", async ({ page }) => {
    const fixture = await mountResistanceFixture(page);
    await expect(fixture.getByTestId("rm-sync-scripts")).toHaveCount(0);
    await page.goto("/settings");
    const updateButton = page.getByTestId("settings-resistance-scripts-update");
    await expect(updateButton).toBeVisible();
    // The unprivileged browser fixture has no connected engine.
    await expect(updateButton).toBeDisabled();
    await expect(page.locator("#resistance-scripts-update")).toBeVisible();
});
