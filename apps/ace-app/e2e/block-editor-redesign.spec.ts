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
