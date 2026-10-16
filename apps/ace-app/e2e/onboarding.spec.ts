import { expect, test, type Page } from "@playwright/test";

/**
 * Onboarding wizard tests with a fully mocked engine API.
 *
 * The wizard renders from the root layout whenever the engine is alive, the
 * ToS is accepted, and `ace:onboarding:v1` says the wizard is not completed.
 * It reads the seeded default blocks through `getDashboardSnapshot()`, so the
 * mocked snapshot here mirrors the engine's seeded state: four blocks with
 * zero user rules, backed by site-pack connections (commit 111ef62).
 */

const ENGINE = "https://127.0.0.1:10567";

const ADULT_BLOCK_ID = "01950000-0000-7000-8000-000000000001";
const SOCIAL_BLOCK_ID = "01950000-0000-7000-8000-000000000002";
const GAMBLING_BLOCK_ID = "01950000-0000-7000-8000-000000000003";
const STREAMING_BLOCK_ID = "01950000-0000-7000-8000-000000000004";

function seededBlock(
    id: string,
    name: string,
    sitePackConnectionCount: number,
    enabled = false,
    scheduleEntries = [{
        id: `sched-${id}`,
        startTime: "00:00:00",
        endTime: "00:00:00",
        monday: true, tuesday: true, wednesday: true, thursday: true,
        friday: true, saturday: true, sunday: true,
    }],
) {
    return {
        block: {
            id,
            name,
            icon: null,
            enabled,
            isActive: false,
            isLocked: false,
            lockState: { state: "inactive" },
            safeSearch: false,
        },
        lock: null,
        scheduleEntries,
        sitePackConnectionCount,
        userRulesCount: 0,
    };
}

const SNAPSHOT = {
    blocks: [
        seededBlock(ADULT_BLOCK_ID, "Adult Content", 2),
        seededBlock(SOCIAL_BLOCK_ID, "Social Media", 1),
        seededBlock(GAMBLING_BLOCK_ID, "Gambling & Betting", 0),
        seededBlock(STREAMING_BLOCK_ID, "Video & Streaming", 0),
    ],
    resistanceModeSettings: {
        enabled: false,
        isLocked: false,
        lockState: { state: "inactive" },
        lockEnd: null,
        mode: { type: "static", intensity: "none" },
        currentIntensity: "none",
        staticIntensity: "none",
        scriptsVersion: null,
        scriptsLoaded: false,
        scriptsTotal: 0,
        scriptsEnabled: 0,
        proEligible: true,
        enforcementStatus: "disabled",
    },
    blockedAttempts: 0,
} as const;

async function mockEngine(
    page: Page,
    blocks: ReturnType<typeof seededBlock>[] = SNAPSHOT.blocks.map((snapshot) => ({
        ...snapshot,
        block: { ...snapshot.block },
        scheduleEntries: [...snapshot.scheduleEntries],
    })),
) {
    const enabledCalls: Array<{ id: string; enabled: boolean }> = [];
    const scheduleCalls: string[] = [];
    await page.route(`${ENGINE}/ping`, (route) =>
        route.fulfill({ status: 200, body: "pong" }),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        route.fulfill({ json: { isPro: false, tosAccepted: true } }),
    );
    await page.route(`${ENGINE}/network-blocks/dashboard*`, (route) =>
        route.fulfill({ json: { ...SNAPSHOT, blocks } }),
    );
    // The wizard must NEVER mutate schedules.
    await page.route(`${ENGINE}/network-blocks/*/schedule`, (route) => {
        scheduleCalls.push(route.request().url());
        return route.fulfill({ status: 500 });
    });
    await page.route(`${ENGINE}/network-blocks/*/enabled`, (route) => {
        const id = route.request().url().split("/").at(-2)!;
        const { enabled } = route.request().postDataJSON() as { enabled: boolean };
        enabledCalls.push({ id, enabled });
        const block = blocks.find((item) => item.block.id === id);
        if (!block) return route.fulfill({ status: 404 });
        block.block.enabled = enabled;
        return route.fulfill({ status: 200 });
    });
    await page.route(`${ENGINE}/resistance-mode/set-enabled`, (route) =>
        route.fulfill({ status: 200 }),
    );
    return { enabledCalls, scheduleCalls };
}

test.describe("Onboarding wizard (mocked engine)", () => {
    test("wizard steps render and seeded blocks show pack coverage", async ({
        page,
    }) => {
        await mockEngine(page);
        // Do NOT mark the wizard completed — it must show on first run.
        await page.addInitScript(() => {
            localStorage.setItem(
                "ace:onboarding:v1",
                JSON.stringify({ wizardCompleted: false, tourCompleted: true }),
            );
        });
        await page.goto("/");

        // Step 1 — Welcome.
        const dialog = page.getByRole("dialog");
        await expect(
            dialog.getByRole("heading", { name: "Welcome to Ace Blocker" }),
        ).toBeVisible({ timeout: 15_000 });

        // Step 2 — How Ace works.
        await dialog.getByRole("button", { name: "Next" }).click();
        await expect(
            dialog.getByRole("heading", { name: "How Ace works" }),
        ).toBeVisible();

        // Step 3 — Choose protection: seeded blocks with category coverage.
        await dialog.getByRole("button", { name: "Next" }).click();
        await expect(
            dialog.getByRole("heading", { name: "Choose your protection" }),
        ).toBeVisible();
        await expect(dialog.getByRole("switch", { name: "Adult Content" }))
            .toHaveAttribute("data-state", "checked");
        await expect(dialog.getByRole("switch", { name: "Social Media" }))
            .toHaveAttribute("data-state", "unchecked");

        // 0 user rules + 2 connected categories → the category-coverage summary.
        await expect(dialog.getByText("Blocks 2 categories")).toBeVisible();
        await expect(dialog.getByText("Blocks 1 category")).toBeVisible();
        // Blocks with no rules and no packs fall back to "0 rules included".
        await expect(
            dialog.getByText("0 rules included", { exact: true }),
        ).toHaveCount(2);

        // Step 4 — Resistance mode.
        await dialog.getByRole("button", { name: "Next" }).click();
        await expect(
            dialog.getByRole("heading", {
                name: "Resistance mode",
                exact: true,
            }),
        ).toBeVisible();
    });

    test("finishing the wizard applies choices and starts the tour", async ({
        page,
    }) => {
        const { enabledCalls, scheduleCalls } = await mockEngine(page);
        let resistanceEnabledBody: unknown = null;
        await page.route(`${ENGINE}/resistance-mode/set-enabled`, (route) => {
            resistanceEnabledBody = route.request().postDataJSON();
            return route.fulfill({ status: 200 });
        });

        await page.addInitScript(() => {
            localStorage.setItem(
                "ace:onboarding:v1",
                JSON.stringify({ wizardCompleted: false, tourCompleted: true }),
            );
        });
        await page.goto("/");

        const dialog = page.getByRole("dialog");
        await expect(
            dialog.getByRole("heading", { name: "Welcome to Ace Blocker" }),
        ).toBeVisible({ timeout: 15_000 });

        // Walk through all four configuration steps.
        for (let step = 0; step < 4; step++) {
            await dialog.getByRole("button", { name: "Next" }).click();
        }

        // Step 5 — done. The recommended block (Adult Content) is applied.
        await expect(
            dialog.getByRole("heading", { name: "You are set up" }),
        ).toBeVisible();
        await expect(
            dialog.getByText("1 block(s) enabled."),
        ).toBeVisible();

        expect(enabledCalls).toEqual([{ id: ADULT_BLOCK_ID, enabled: true }]);
        expect(scheduleCalls).toHaveLength(0);

        // Resistance mode was enabled.
        expect(resistanceEnabledBody).toEqual({ enabled: true });

        // "Take the tour" closes the wizard and starts the spotlight tour.
        await dialog.getByRole("button", { name: "Take the tour" }).click();
        await expect(
            page.getByRole("dialog", { name: "Your blocked-attempt count" }),
        ).toBeVisible({ timeout: 10_000 });
    });

    test("reopened onboarding preserves enabled states and customized schedules", async ({ page }) => {
        const customSchedule = [{
            id: "custom-social",
            startTime: "09:00:00",
            endTime: "11:00:00",
            monday: true, tuesday: false, wednesday: false, thursday: false,
            friday: false, saturday: false, sunday: false,
        }];
        const blocks = [
            seededBlock(ADULT_BLOCK_ID, "Adult Content", 2, false),
            seededBlock(SOCIAL_BLOCK_ID, "Social Media", 1, true, customSchedule),
            seededBlock(GAMBLING_BLOCK_ID, "Gambling & Betting", 0),
            seededBlock(STREAMING_BLOCK_ID, "Video & Streaming", 0),
        ];
        const { enabledCalls, scheduleCalls } = await mockEngine(page, blocks);
        await page.addInitScript(() => {
            localStorage.setItem("ace:onboarding:v1", JSON.stringify({
                wizardCompleted: false,
                wizardPreviouslyCompleted: true,
                tourCompleted: true,
            }));
        });
        await page.goto("/");
        const dialog = page.getByRole("dialog");
        await expect(dialog.getByRole("heading", { name: "Welcome to Ace Blocker" })).toBeVisible();

        await dialog.getByRole("button", { name: "Next" }).click();
        await dialog.getByRole("button", { name: "Next" }).click();

        const adult = dialog.getByRole("switch", { name: "Adult Content" });
        const social = dialog.getByRole("switch", { name: "Social Media" });
        const streaming = dialog.getByRole("switch", { name: "Video & Streaming" });
        await expect(adult).toHaveAttribute("data-state", "unchecked");
        await expect(social).toHaveAttribute("data-state", "checked");
        await expect(social).toBeDisabled();
        await streaming.click();

        await dialog.getByRole("button", { name: "Next" }).click();
        await dialog.getByRole("button", { name: "Next" }).click();
        await expect(dialog.getByText("2 block(s) enabled.")).toBeVisible();
        expect(enabledCalls).toEqual([{ id: STREAMING_BLOCK_ID, enabled: true }]);
        expect(scheduleCalls).toEqual([]);
        expect(blocks[1].scheduleEntries).toEqual(customSchedule);

        // Opening setup again must not reselect the previously disabled adult block.
        await page.evaluate(() => {
            localStorage.setItem("ace:onboarding:v1", JSON.stringify({
                wizardCompleted: false, wizardPreviouslyCompleted: true, tourCompleted: true,
            }));
        });
        await page.reload();
        const reopened = page.getByRole("dialog");
        await expect(reopened.getByRole("heading", { name: "Welcome to Ace Blocker" })).toBeVisible();
        await reopened.getByRole("button", { name: "Next" }).click();
        await reopened.getByRole("button", { name: "Next" }).click();
        await expect(reopened.getByRole("switch", { name: "Adult Content" })).toHaveAttribute("data-state", "unchecked");
        await expect(reopened.getByRole("switch", { name: "Video & Streaming" })).toHaveAttribute("data-state", "checked");
        await expect(reopened.getByRole("switch", { name: "Social Media" })).toBeDisabled();
        expect(enabledCalls).toHaveLength(1);
        expect(scheduleCalls).toHaveLength(0);
    });

    test("disabled selection and failed enable do not appear in completion summary", async ({ page }) => {
        const { enabledCalls, scheduleCalls } = await mockEngine(page);
        await page.route(`${ENGINE}/network-blocks/${ADULT_BLOCK_ID}/enabled`, (route) =>
            route.fulfill({ status: 500, json: { code: "internal_error", error: "failed" } }),
        );
        await page.addInitScript(() => {
            localStorage.setItem("ace:onboarding:v1", JSON.stringify({
                wizardCompleted: false, tourCompleted: true,
            }));
        });
        await page.goto("/");
        const dialog = page.getByRole("dialog");
        await expect(dialog.getByRole("heading", { name: "Welcome to Ace Blocker" })).toBeVisible();
        for (let index = 0; index < 4; index++) {
            await dialog.getByRole("button", { name: "Next" }).click();
        }
        await expect(dialog.getByRole("heading", { name: "Resistance mode", exact: true })).toBeVisible();
        await expect(page.getByText("Some settings could not be saved. You can change them later in Network Blocks.")).toBeVisible();
        await expect(dialog.getByRole("heading", { name: "You are set up" })).toHaveCount(0);
        expect(enabledCalls).toHaveLength(0);
        expect(scheduleCalls).toHaveLength(0);
    });
});
