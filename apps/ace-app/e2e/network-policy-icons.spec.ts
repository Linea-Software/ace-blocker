import { expect, test } from "@playwright/test";

// Browser-only fixture: exercise the actual policy card without privileged Tauri IPC.
test("Network Block cards display custom and fallback icons without changing their controls", async ({ page }) => {
    await page.setViewportSize({ width: 1200, height: 800 });
    await page.goto("/getting-started");
    await page.evaluate(async () => {
        const runtime = new URL("/node_modules/.vite/deps/svelte.js", location.origin).pathname;
        const [{ mount }, { default: NetworkPolicy }] = await Promise.all([
            import(/* @vite-ignore */ runtime),
            import(/* @vite-ignore */ "/src/lib/components/ace/domain/NetworkPolicy.svelte"),
        ]);
        const host = document.createElement("div");
        host.dataset.testid = "policy-icon-fixture";
        host.style.cssText = [
            "position:fixed", "inset:0", "overflow:auto", "z-index:200",
            "background:var(--background)", "padding:22px",
            "display:grid", "gap:14px", "align-content:start",
        ].join(";");
        document.body.append(host);
        const base = {
            id: "fixture-policy",
            name: "Example",
            icon: null,
            enabled: true,
            isActive: true,
            isLocked: false,
            lockState: { state: "inactive" },
            scheduleMode: "always",
            safeSearch: false,
            sitesListCount: 2,
            rulesCount: 5,
            schedules: [],
        };
        for (const [index, icon, name, enabled] of [
            [0, "gaming", "Gaming", true],
            [1, null, "Without a custom icon", true],
            [2, "ti-messages", "Social (legacy icon)", false],
        ] as const) {
            const section = document.createElement("section");
            host.append(section);
            mount(NetworkPolicy, {
                target: section,
                props: {
                    policy: { ...base, id: String(index), icon, name, enabled, isActive: enabled },
                    onEdit: () => {},
                    onDelete: () => {},
                    onLock: () => {},
                    onEnabledChange: async () => {},
                },
            });
        }
    });

    const cards = page.getByTestId("network-policy");
    await expect(cards).toHaveCount(3);
    await expect(cards.nth(0).getByTestId("network-policy-icon").locator("svg.lucide-gamepad-2")).toBeVisible();
    await expect(cards.nth(1).getByTestId("network-policy-icon").locator("svg.lucide-globe")).toBeVisible();
    await expect(cards.nth(2).getByTestId("network-policy-icon").locator("svg.lucide-message-square")).toBeVisible();
    for (const card of await cards.all()) {
        await expect(card.getByTestId("network-policy-icon")).toHaveAttribute("aria-hidden", "true");
        await expect(card.getByRole("switch")).toBeVisible();
        await expect(card.getByRole("button", { name: "Edit" })).toBeVisible();
    }
    await expect(cards.nth(2)).toHaveClass(/disabled/);
    await page.screenshot({ path: "test-results/network-policy-icons-wide.png" });

    await page.setViewportSize({ width: 800, height: 600 });
    await page.screenshot({ path: "test-results/network-policy-icons-compact.png" });
    const measurements = await cards.evaluateAll(nodes =>
        nodes.map(node => ({
            scrollWidth: node.scrollWidth,
            clientWidth: node.clientWidth,
            iconWidth: node.querySelector("[data-testid=network-policy-icon]")?.getBoundingClientRect().width,
        })),
    );
    for (const card of measurements) {
        expect(card.scrollWidth).toBeLessThanOrEqual(card.clientWidth + 1);
        expect(card.iconWidth).toBeGreaterThanOrEqual(35);
    }
});
