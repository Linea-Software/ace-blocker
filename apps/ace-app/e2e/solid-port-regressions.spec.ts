import { expect, test } from "@playwright/test";

test("editor reacts to same-path query changes without reloading the WebView", async ({
    page,
}) => {
    await page.goto("/network-blocks/editor?block=alpha");
    await expect(page.locator('[data-requested-block="alpha"]')).toBeVisible();

    await page.evaluate(() => {
        (
            window as typeof window & { __aceRouteSentinel?: string }
        ).__aceRouteSentinel = "same-document";
        const link = document.createElement("a");
        link.href = "/network-blocks/editor?block=beta&section=schedule";
        document.body.append(link);
        link.click();
        link.remove();
    });

    await expect(page).toHaveURL(/block=beta/);
    await expect(page.locator('[data-requested-block="beta"]')).toBeVisible();
    expect(
        await page.evaluate(
            () =>
                (window as typeof window & { __aceRouteSentinel?: string })
                    .__aceRouteSentinel,
        ),
    ).toBe("same-document");

    // Revisit the first ID: the previous request must not resurrect beta.
    await page.evaluate(() => {
        const link = document.createElement("a");
        link.href = "/network-blocks/editor?block=alpha";
        document.body.append(link);
        link.click();
        link.remove();
    });
    await expect(page.locator('[data-requested-block="alpha"]')).toBeVisible();
});

test("onboarding is modal: the shell is inert, focus is trapped, and Escape is disabled", async ({
    page,
}) => {
    await page.goto("/getting-started");
    const guideLink = page
        .getByRole("navigation")
        .getByRole("link", { name: "Guide" });
    await guideLink.focus();

    await page.evaluate(async () => {
        // Module identity matters under Vite HMR: import the same reactive
        // singletons that the already-rendered app is using.
        const importLive = (path: string) => {
            const existing = performance
                .getEntriesByType("resource")
                .map((entry) => new URL(entry.name))
                .filter((url) => url.pathname === path)
                .at(-1);
            return import(
                /* @vite-ignore */ existing
                    ? existing.pathname + existing.search
                    : path
            );
        };
        const [{ engineStore }, { userStore }, { onboardingStore }] =
            await Promise.all([
                importLive("/src/lib/stores/engine.ts"),
                importLive("/src/lib/stores/user-store.ts"),
                importLive("/src/lib/stores/onboarding.ts"),
            ]);
        // Hold the live singleton for the dismissal assertion (a fresh
        // import URL can resolve to another HMR module instance).
        (
            window as typeof window & {
                __aceTestWizardStore?: { wizardCompleted: boolean };
            }
        ).__aceTestWizardStore = onboardingStore;
        engineStore.isReady = true;
        engineStore.isAlive = true;
        userStore.accountInfoLoaded = true;
        userStore.tosAccepted = true;
        onboardingStore.loaded = true;
        onboardingStore.tourRunning = false;
        onboardingStore.wizardCompleted = false;
    });

    const wizard = page.getByRole("dialog", { name: /welcome/i });
    await expect(wizard).toBeVisible();
    await expect(page.locator(".app-shell")).toHaveAttribute("inert", "");

    for (let index = 0; index < 12; index++) {
        await page.keyboard.press("Tab");
        expect(
            await page.evaluate(
                () =>
                    document.activeElement?.closest('[role="dialog"]') !== null,
            ),
        ).toBe(true);
    }
    for (let index = 0; index < 5; index++) {
        await page.keyboard.press("Shift+Tab");
        expect(
            await page.evaluate(
                () =>
                    document.activeElement?.closest('[role="dialog"]') !== null,
            ),
        ).toBe(true);
    }
    await page.keyboard.press("Escape");
    await expect(wizard).toBeVisible();

    await page.evaluate(() => {
        (
            window as typeof window & {
                __aceTestWizardStore?: { wizardCompleted: boolean };
            }
        ).__aceTestWizardStore!.wizardCompleted = true;
    });
    await expect(wizard).toHaveCount(0);
    await expect(page.locator(".app-shell")).not.toHaveAttribute("inert", "");
    await expect(guideLink).toBeFocused();
});

test("Inspector does not alter headings and paragraphs outside the panel", async ({
    page,
}) => {
    await page.goto("/getting-started");
    const heading = page.getByRole("heading", {
        level: 1,
        name: "Getting Started",
    });
    const paragraph = page.locator(".page-header p").first();
    await expect(heading).toBeVisible();
    const before = await page.evaluate(() => {
        const h = document.querySelector(".page-header h1")!;
        const p = document.querySelector(".page-header p")!;
        return {
            heading: getComputedStyle(h).fontSize,
            headingMargin: getComputedStyle(h).marginTop,
            paragraph: getComputedStyle(p).fontSize,
            paragraphMargin: getComputedStyle(p).marginTop,
        };
    });

    await page.evaluate(async () => {
        const [{ mount }, { default: Inspector }] = await Promise.all([
            import("/src/test-fixture-runtime.ts"),
            import("/src/lib/components/ace/structural/Inspector.tsx"),
        ]);
        const host = document.createElement("div");
        host.id = "test-inspector";
        document.body.append(host);
        (
            window as typeof window & { __disposeAceInspector?: () => void }
        ).__disposeAceInspector = mount(Inspector, {
            target: host,
            props: {
                title: "Pack details",
                description: "Your choices",
                closeLabel: "Close",
                onClose: () => {},
                children: "Inspector content",
            },
        });
    });
    await expect(page.locator(".inspector")).toBeVisible();
    const after = await page.evaluate(() => {
        const h = document.querySelector(".page-header h1")!;
        const p = document.querySelector(".page-header p")!;
        return {
            heading: getComputedStyle(h).fontSize,
            headingMargin: getComputedStyle(h).marginTop,
            paragraph: getComputedStyle(p).fontSize,
            paragraphMargin: getComputedStyle(p).marginTop,
        };
    });
    expect(after).toEqual(before);
    await page.evaluate(() =>
        (
            window as typeof window & { __disposeAceInspector?: () => void }
        ).__disposeAceInspector?.(),
    );
    await expect(page.locator(".inspector")).toHaveCount(0);
});

test("unmatched desktop routes have a keyboard-accessible fallback", async ({
    page,
}) => {
    await page.goto("/not-an-ace-route");
    await expect(
        page.getByRole("heading", { level: 1, name: "Page not found" }),
    ).toBeVisible();
    await page.getByRole("link", { name: "Back to Control" }).click();
    await expect(page).toHaveURL("/");
});

test("failed device linking retains its destructive error styling", async ({
    page,
}) => {
    await page.goto("/settings");
    // Browser-only deterministic failure: the mock has no native/engine access.
    await page.evaluate(() => {
        const tauri = window as typeof window & {
            isTauri?: boolean;
            __TAURI_INTERNALS__?: { invoke: (name: string) => Promise<never> };
        };
        tauri.isTauri = true;
        tauri.__TAURI_INTERNALS__ = {
            invoke: async () => {
                throw new Error("Simulated device link failure");
            },
        };
    });
    await page.locator(".ace-settings-license button.sign-in-action").click();
    await expect(
        page.locator(".ace-settings-license .device-link-error"),
    ).toBeVisible();
    await expect(
        page.locator(".ace-settings-license .device-link-panel"),
    ).toContainText(/link|account|code/i);
});
