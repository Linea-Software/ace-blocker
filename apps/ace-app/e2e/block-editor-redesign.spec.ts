import { expect, test } from "@playwright/test";

// Browser-only visual fixture. No privileged Tauri IPC or real policy writes.
test("Network Block editor makes sites and activation visible", async ({ page }) => {
    await page.setViewportSize({ width: 1200, height: 820 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: BlockModal }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/routes/network-blocks/_BlockModal.svelte"),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "block-editor-fixture";
        document.body.append(host);
        mount(BlockModal, {
            target: host,
            props: { open: true, block: null, onSave: () => {} },
        });
    });

    const editor = page.locator(".inspector.wide");
    await expect(editor.getByRole("heading", { name: "What to block" })).toBeVisible();
    await expect(editor.getByRole("heading", { name: "When to block" })).toBeVisible();
    const sections = editor.getByRole("navigation", {
        name: "Block configuration sections",
    });
    await expect(sections).toBeVisible();
    await expect(sections.getByRole("button", { name: /When to block/ })).toBeVisible();
    await expect(editor.getByRole("button", { name: /Always/ })).toHaveAttribute("aria-pressed", "true");

    const website = editor.getByRole("textbox", { name: "Specific website" });
    await website.fill("https://instagram.com/reels");
    await editor.getByRole("button", { name: "Add specific website" }).click();
    await expect(editor.getByText("instagram.com", { exact: true })).toBeVisible();

    await page.screenshot({
        path: "docs/ux-audit-assets/network-block-editor-wide.png",
        fullPage: false,
    });

    await editor.getByRole("button", { name: /Custom hours/ }).click();
    await expect(editor.getByText("Choose at least one period before saving.").first()).toBeVisible();
    await expect(editor.getByRole("button", { name: "Create and enable block" })).toBeDisabled();

    await page.setViewportSize({ width: 800, height: 600 });
    await page.screenshot({
        path: "docs/ux-audit-assets/network-block-editor-compact.png",
        fullPage: false,
    });
    await expect(editor.getByRole("button", { name: "Create and enable block" })).toBeInViewport();
    const overflow = await editor.evaluate((element) => ({
        scrollWidth: element.scrollWidth,
        clientWidth: element.clientWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1);
});
