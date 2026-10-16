import { expect, test, type Page } from "@playwright/test";

/**
 * Network Blocks page tests with a fully mocked engine API.
 *
 * The live-engine suite (`network-blocks.spec.ts`) requires a running engine
 * on 127.0.0.1:10567. This suite intercepts every request with `page.route`
 * (same pattern as resistance-mode.spec.ts), so it runs without an engine.
 *
 * Mock shapes mirror the engine responses: camelCase, `{state}`-tagged
 * LockState, `{error, code}` error envelopes.
 */

const ENGINE = "https://127.0.0.1:10567";

type MockBlock = {
    block: {
        id: string;
        name: string;
        icon: string | null;
        enabled: boolean;
        isActive: boolean;
        isLocked: boolean;
        lockState: { state: string; end?: string | null };
        safeSearch: boolean;
    };
    lock: {
        isActive: boolean;
        lockEnd?: string | null;
        lockState: { state: string; end?: string | null };
        networkBlockId: string;
    } | null;
    scheduleEntries: unknown[];
    sitePackConnectionCount: number;
    userRulesCount: number;
};

/** Builds a flattened `NetworkBlockSnapshot` (the `/network-blocks/dashboard`
 *  response shape) from simple options. */
function mkBlock(
    options: Partial<MockBlock["block"]> & {
        id: string;
        name: string;
        sitePackConnectionCount?: number;
        userRulesCount?: number;
        scheduleEntries?: unknown[];
        lock?: MockBlock["lock"];
    },
): MockBlock {
    return {
        block: {
            id: options.id,
            name: options.name,
            icon: options.icon ?? null,
            enabled: options.enabled ?? true,
            isActive: options.isActive ?? true,
            isLocked: options.isLocked ?? false,
            lockState: options.lockState ?? { state: "inactive" },
            safeSearch: options.safeSearch ?? false,
        },
        lock: options.lock ?? null,
        scheduleEntries: options.scheduleEntries ?? [],
        sitePackConnectionCount: options.sitePackConnectionCount ?? 0,
        userRulesCount: options.userRulesCount ?? 0,
    };
}

const RESISTANCE_SETTINGS = {
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
} as const;

function snapshotFor(blocks: MockBlock[]) {
    return {
        blocks,
        resistanceModeSettings: RESISTANCE_SETTINGS,
        blockedAttempts: 0,
    };
}

const SAMPLE_PACKS = [
    {
        id: "pack-adult-free",
        name: "Adult Websites — Free",
        description: "Curated adult domains.",
        icon: "adult",
        entriesCount: 1250000,
        requiresPro: false,
        isSynced: true,
        // Installed with the installer — usable offline.
        isBundled: true,
        lastUpdatedAt: "2026-09-02T00:00:00Z",
        version: "2026.09.02",
    },
    {
        id: "pack-gambling-pro",
        name: "Gambling Websites — Pro",
        description: "Curated gambling domains.",
        icon: "gambling",
        entriesCount: 340000,
        requiresPro: true,
        isSynced: false,
        isBundled: false,
        lastUpdatedAt: null,
        version: null,
    },
    {
        id: "custom-focus-list",
        name: "My Focus List",
        description: "Imported focus domains.",
        icon: "list",
        entriesCount: 42,
        requiresPro: false,
        isSynced: true,
        isBundled: false,
        lastUpdatedAt: "2026-09-10T00:00:00Z",
        version: "custom",
    },
] as const;

const ALWAYS_ON_SCHEDULE = [
    {
        id: "sched-1",
        networkBlockId: "block-social",
        startTime: "00:00:00",
        endTime: "23:59:59",
        monday: true,
        tuesday: true,
        wednesday: true,
        thursday: true,
        friday: true,
        saturday: true,
        sunday: true,
    },
];

const EXACT_PERIOD_SCHEDULE = [
    {
        id: "sched-1",
        networkBlockId: "block-social",
        startTime: "09:15:00",
        endTime: "10:45:00",
        monday: true,
        tuesday: false,
        wednesday: false,
        thursday: false,
        friday: false,
        saturday: false,
        sunday: false,
    },
    {
        id: "sched-2",
        networkBlockId: "block-social",
        startTime: "14:30:00",
        endTime: "16:00:00",
        monday: false,
        tuesday: false,
        wednesday: true,
        thursday: false,
        friday: false,
        saturday: false,
        sunday: false,
    },
] as const;

/** Installs the mock API. `blocks` is read at request time, so tests can
 *  mutate it and the next dashboard poll/load reflects the change. */
async function mockEngine(
    page: Page,
    options: {
        blocks?: MockBlock[];
        online?: boolean;
        pro?: boolean;
        createStatus?: number;
    } = {},
) {
    const { blocks = [], online = true, pro = false, createStatus } = options;

    await page.route(`${ENGINE}/ping`, (route) =>
        online ? route.fulfill({ status: 200, body: "pong" }) : route.abort(),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        online
            ? route.fulfill({ json: { isPro: pro, tosAccepted: true } })
            : route.abort(),
    );
    await page.route(`${ENGINE}/network-blocks/dashboard*`, (route) =>
        online ? route.fulfill({ json: snapshotFor(blocks) }) : route.abort(),
    );
    await page.route(`${ENGINE}/site-lists`, (route) =>
        online
            ? route.fulfill({ json: { sitePacks: SAMPLE_PACKS } })
            : route.abort(),
    );

    // Per-block detail endpoints used by the edit modal.
    await page.route(`${ENGINE}/network-blocks/*/user-rules`, (route) =>
        online
            ? route.fulfill({ json: { userDefinedRules: [] } })
            : route.abort(),
    );
    await page.route(`${ENGINE}/network-blocks/*/schedule`, (route) =>
        online
            ? route.fulfill({ json: { scheduleEntries: ALWAYS_ON_SCHEDULE } })
            : route.abort(),
    );
    await page.route(`${ENGINE}/network-blocks/*/site-lists`, (route) =>
        online ? route.fulfill({ json: { connections: [] } }) : route.abort(),
    );

    // Creation endpoint (overridable per test for error cases).
    await page.route(`${ENGINE}/network-blocks`, async (route) => {
        if (route.request().method() !== "POST") {
            await route.continue();
            return;
        }
        if (createStatus) {
            await route.fulfill({
                status: createStatus,
                json: { error: "mock failure", code: "internal_error" },
            });
            return;
        }
        const body = route.request().postDataJSON() as { blockName?: string };
        await route.fulfill({
            json: {
                networkBlock: {
                    id: "block-created",
                    name: body.blockName ?? "New Block",
                    icon: null,
                    enabled: true,
                    isActive: true,
                    isLocked: false,
                    lockState: { state: "inactive" },
                    safeSearch: false,
                },
            },
        });
    });
    await page.route(`${ENGINE}/network-blocks/*/safe-search`, (route) =>
        online ? route.fulfill({ status: 200 }) : route.abort(),
    );
}

/** Captures a single POST to the given path (fulfilling with 200). */
function capturePost(
    page: Page,
    path: string,
    options: { status?: number; json?: unknown } = {},
) {
    let captured: { body: unknown } | null = null;
    void page.route(`${ENGINE}${path}`, async (route) => {
        captured = { body: route.request().postDataJSON() };
        if (options.status) {
            await route.fulfill({
                status: options.status,
                json: options.json ?? { error: "mock", code: "mock" },
            });
        } else {
            await route.fulfill({ status: 200 });
        }
    });
    return () => captured;
}

async function gotoPage(page: Page) {
    // Skip the onboarding wizard/tour so it never covers the page.
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto("/network-blocks");
    // Generous timeout: the first request to a route compiles the Vite
    // modules, which can take several seconds under parallel workers.
    await expect(
        page.getByRole("heading", { level: 1, name: "Network Blocks" }),
    ).toBeVisible({ timeout: 15_000 });
    return page;
}

function blockRow(page: Page, name: string) {
    return page.getByTestId("network-policy").filter({
        has: page.getByRole("heading", { name, exact: true }),
    });
}

function blockEditor(page: Page, title: "Create Block" | "Edit Block") {
    return page.getByRole("region", { name: title });
}

test.describe("Network Blocks (mocked engine)", () => {
    test("loaded: rows render with lock icons, counts and tooltips", async ({
        page,
    }) => {
        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                scheduleEntries: ALWAYS_ON_SCHEDULE,
                sitePackConnectionCount: 2,
                userRulesCount: 0,
            }),
            mkBlock({
                id: "block-gaming",
                name: "Gaming",
                isActive: false,
                userRulesCount: 4,
            }),
        ];
        await mockEngine(page, { blocks });
        await gotoPage(page);

        const socialRow = blockRow(page, "Social Media");
        await expect(socialRow).toBeVisible();
        await expect(socialRow).toContainText("Enforcing");
        await expect(socialRow).toContainText("Categories / lists 2");
        await expect(socialRow).toContainText("Specific websites 0");

        const gamingRow = blockRow(page, "Gaming");
        await expect(gamingRow).toBeVisible();
        await expect(gamingRow).toContainText("Not scheduled");
        await expect(gamingRow).toContainText("Specific websites 4");
        await expect(gamingRow).toContainText(
            "This block never activates until you add a schedule.",
        );

        // Lock icons: both blocks are unlocked → "Lock this block" buttons.
        const lockButtons = page.getByLabel("Lock this block");
        await expect(lockButtons).toHaveCount(2);

        // The action keeps an explicit accessible name even without the
        // legacy hover-tooltip wrapper.
        await expect(lockButtons.first()).toHaveAttribute(
            "aria-label",
            "Lock this block",
        );
    });

    test("enabled, scheduled, unscheduled and disabled states remain distinct at 800×600", async ({ page }) => {
        await page.setViewportSize({ width: 800, height: 600 });
        await mockEngine(page, {
            blocks: [
                mkBlock({ id: "active", name: "Active block", enabled: true, isActive: true, scheduleEntries: ALWAYS_ON_SCHEDULE }),
                mkBlock({ id: "scheduled", name: "Scheduled block", enabled: true, isActive: false, scheduleEntries: EXACT_PERIOD_SCHEDULE }),
                mkBlock({ id: "unscheduled", name: "Unscheduled block", enabled: true, isActive: false }),
                mkBlock({ id: "disabled", name: "Disabled block", enabled: false, isActive: true, scheduleEntries: EXACT_PERIOD_SCHEDULE }),
            ],
        });
        await gotoPage(page);

        const enforcing = blockRow(page, "Active block");
        const scheduled = blockRow(page, "Scheduled block");
        const unscheduled = blockRow(page, "Unscheduled block");
        const disabled = blockRow(page, "Disabled block");

        await expect(enforcing.locator(".state-word")).toHaveText("Enforcing");
        await expect(scheduled.locator(".state-word")).toHaveText("Scheduled");
        await expect(unscheduled.locator(".state-word")).toHaveText("Not scheduled");
        await expect(disabled.locator(".state-word")).toHaveText("Disabled");
        await expect(enforcing.getByRole("switch")).toHaveAttribute("aria-checked", "true");
        await expect(disabled.getByRole("switch")).toHaveAttribute("aria-checked", "false");
        await expect(disabled.locator(".schedule-zone")).toBeVisible();
        await expect(disabled.locator(".schedule-copy .transition-copy")).toHaveCount(0);
        await expect(disabled).toHaveClass(/disabled/);
        await expect(disabled).not.toHaveClass(/active/);

        const enabledControl = enforcing.locator(".actions > .enable-action");
        await expect(enabledControl).toContainText("Enabled");
        await expect(enabledControl.locator("button")).toHaveAttribute("role", "switch");
        await expect(enforcing.locator(".actions > :nth-child(2)")).toContainText("Edit");
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
        await page.screenshot({ path: "test-results/network-blocks-enabled-800.png", fullPage: true });
    });

    test("switch waits for the engine and refreshes enabled state without editing schedules", async ({ page }) => {
        const blocks = [mkBlock({
            id: "block-social",
            name: "Social Media",
            enabled: true,
            isActive: true,
            scheduleEntries: ALWAYS_ON_SCHEDULE,
        })];
        await mockEngine(page, { blocks });
        let requestBody: { enabled: boolean } | null = null;
        let finishRequest: (() => void) | undefined;
        const inFlight = new Promise<void>((resolve) => { finishRequest = resolve; });
        await page.route(`${ENGINE}/network-blocks/block-social/enabled`, async (route) => {
            requestBody = route.request().postDataJSON();
            await inFlight;
            blocks[0].block.enabled = requestBody!.enabled;
            blocks[0].block.isActive = false;
            await route.fulfill({ status: 200 });
        });
        let scheduleCalls = 0;
        await page.route(`${ENGINE}/network-blocks/block-social/schedule`, async (route) => {
            if (route.request().method() !== "GET") scheduleCalls++;
            await route.fulfill({ json: { scheduleEntries: ALWAYS_ON_SCHEDULE } });
        });
        await gotoPage(page);

        const row = blockRow(page, "Social Media");
        const toggle = row.getByRole("switch");
        await expect(row.locator(".state-word")).toHaveText("Enforcing");
        await toggle.click();
        await expect(toggle).toBeDisabled();
        await expect(toggle).toHaveAttribute("aria-checked", "true");
        await expect(row.locator(".state-word")).toHaveText("Enforcing");
        finishRequest?.();
        await expect(toggle).toHaveAttribute("aria-checked", "false");
        await expect(toggle).toBeEnabled();
        await expect(row.locator(".state-word")).toHaveText("Disabled");
        await expect(row.locator(".schedule-zone")).toBeVisible();
        expect(requestBody).toEqual({ enabled: false });
        expect(scheduleCalls).toBe(0);

        await toggle.click();
        await expect(toggle).toHaveAttribute("aria-checked", "true");
        await expect(row.locator(".state-word")).toHaveText("Scheduled");
        expect(requestBody).toEqual({ enabled: true });
        expect(scheduleCalls).toBe(0);
    });

    test("locked blocks cannot be disabled, and locked rail takes precedence", async ({ page }) => {
        const blocks = [
            mkBlock({
                id: "locked",
                name: "Locked block",
                enabled: true,
                isActive: true,
                isLocked: true,
                lockState: { state: "active_until_lock_end" },
                scheduleEntries: ALWAYS_ON_SCHEDULE,
            }),
            mkBlock({
                id: "outside",
                name: "Outside lock window",
                enabled: true,
                isActive: false,
                isLocked: false,
                lockState: { state: "active_during_schedule" },
                scheduleEntries: EXACT_PERIOD_SCHEDULE,
            }),
        ];
        await mockEngine(page, { blocks });
        let toggleRequests = 0;
        await page.route(`${ENGINE}/network-blocks/locked/enabled`, async (route) => {
            toggleRequests++;
            await route.fulfill({ status: 200 });
        });
        await gotoPage(page);

        const row = blockRow(page, "Locked block");
        await expect(row).toHaveClass(/locked/);
        await expect(row.getByRole("switch")).toBeDisabled();
        await expect(row.getByRole("switch")).toHaveAttribute("aria-checked", "true");
        expect(toggleRequests).toBe(0);

        // A schedule-bound lock remains visible outside its active window,
        // but the engine permits disabling while isLocked is false.
        await page.route(`${ENGINE}/network-blocks/outside/enabled`, async (route) => {
            blocks[1].block.enabled = (route.request().postDataJSON() as { enabled: boolean }).enabled;
            await route.fulfill({ status: 200 });
        });
        const outside = blockRow(page, "Outside lock window");
        await expect(outside).toHaveClass(/locked/);
        await expect(outside.getByRole("switch")).toBeEnabled();
        await outside.getByRole("switch").click();
        await expect(outside.getByRole("switch")).toHaveAttribute("aria-checked", "false");
        await expect(outside.locator(".state-word")).toHaveText("Disabled");
    });

    test("403 lock conflict and API errors preserve the enabled state", async ({ page }) => {
        const blocks = [
            mkBlock({ id: "conflict", name: "Conflict block", enabled: true, isActive: true, scheduleEntries: ALWAYS_ON_SCHEDULE }),
            mkBlock({ id: "failure", name: "Failure block", enabled: true, isActive: true, scheduleEntries: ALWAYS_ON_SCHEDULE }),
        ];
        await mockEngine(page, { blocks });
        await page.route(`${ENGINE}/network-blocks/conflict/enabled`, async (route) => {
            await route.fulfill({ status: 403, json: { code: "locked", error: "Locked" } });
        });
        await page.route(`${ENGINE}/network-blocks/failure/enabled`, async (route) => {
            await route.fulfill({ status: 500, json: { code: "internal_error", error: "Failed" } });
        });
        await gotoPage(page);

        for (const name of ["Conflict block", "Failure block"]) {
            const row = blockRow(page, name);
            const toggle = row.getByRole("switch");
            await toggle.click();
            await expect(toggle).toBeEnabled();
            await expect(toggle).toHaveAttribute("aria-checked", "true");
            await expect(row.locator(".state-word")).toHaveText("Enforcing");
            await expect(
                page.getByText(
                    name === "Conflict block"
                        ? "This block is locked"
                        : "Failed to synchronize settings with the engine.",
                    { exact: true },
                ),
            ).toBeVisible();
        }
    });

    test("empty state shows starter paths", async ({ page }) => {
        await mockEngine(page, { blocks: [] });
        await gotoPage(page);

        await expect(
            page.getByRole("heading", { name: "No Network Blocks" }),
        ).toBeVisible();
        // The compact starter paths remain directly navigable without
        // reintroducing the old onboarding-card stack.
        await expect(
            page.getByRole("link").filter({ hasText: "Start with categories" }),
        ).toBeVisible();
        await expect(
            page.getByRole("link").filter({ hasText: "Import a Custom List" }),
        ).toBeVisible();
        await expect(
            page.getByRole("link").filter({ hasText: "Add a time-lock" }),
        ).toBeVisible();
    });

    test("create block flow: modal opens, creates and the row appears", async ({
        page,
    }) => {
        const blocks: MockBlock[] = [];
        const createdBlocks: MockBlock[] = [];
        await mockEngine(page, { blocks });
        // Register AFTER mockEngine so these handlers win (routes are LIFO).
        let createdBody: unknown = null;
        await page.route(`${ENGINE}/network-blocks`, async (route) => {
            if (route.request().method() !== "POST") {
                await route.continue();
                return;
            }
            createdBody = route.request().postDataJSON();
            const name =
                (createdBody as { blockName?: string }).blockName ??
                "My New Block";
            createdBlocks.push(
                mkBlock({
                    id: "block-created",
                    name,
                }),
            );
            await route.fulfill({
                json: {
                    networkBlock: {
                        id: "block-created",
                        name,
                        icon: null,
                        isActive: true,
                        isLocked: false,
                        lockState: { state: "inactive" },
                        safeSearch: false,
                    },
                },
            });
        });
        // Dashboard loads (invalidateAll) must include the created block.
        await page.route(`${ENGINE}/network-blocks/dashboard*`, (route) =>
            route.fulfill({ json: snapshotFor([...blocks, ...createdBlocks]) }),
        );
        await gotoPage(page);

        await page
            .getByRole("button", { name: /^Create Block$/i })
            .first()
            .click();
        const inspector = blockEditor(page, "Create Block");
        await expect(
            inspector.getByRole("heading", { name: "Create Block" }),
        ).toBeVisible();

        // Save is disabled until a name is entered.
        const saveButton = inspector.getByRole("button", { name: "Save" });
        await expect(saveButton).toBeDisabled();
        await inspector.getByLabel(/Block Name/i).fill("My New Block");
        await expect(saveButton).toBeEnabled();
        await saveButton.click();

        const captured = createdBody;
        expect(captured).not.toBeNull();
        expect(captured).toMatchObject({ blockName: "My New Block" });

        // invalidateAll re-runs the load; the new block now shows.
        await expect(blockRow(page, "My New Block")).toBeVisible();
    });

    test("block detail modal renders rules, schedule and sites sections", async ({
        page,
    }) => {
        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                scheduleEntries: ALWAYS_ON_SCHEDULE,
                sitePackConnectionCount: 2,
                userRulesCount: 3,
            }),
        ];
        await mockEngine(page, { blocks });
        await gotoPage(page);

        await blockRow(page, "Social Media")
            .getByRole("button", { name: "Edit" })
            .click();
        const inspector = blockEditor(page, "Edit Block");
        await expect(
            inspector.getByRole("heading", { name: "Edit Block" }),
        ).toBeVisible();

        // Blocking combines name, categories, websites and secondary list controls.
        await expect(
            inspector.getByRole("button", { name: "Blocking" }),
        ).toHaveAttribute("aria-pressed", "true");
        await expect(
            inspector.getByRole("heading", { name: "Categories" }),
        ).toBeVisible();
        await expect(
            inspector.getByRole("heading", { name: "Specific websites" }),
        ).toBeVisible();
        await expect(
            inspector.getByText("No specific websites added."),
        ).toBeVisible();
        await expect(
            inspector.getByRole("heading", { name: "Custom Lists" }),
        ).toBeVisible();
        await expect(
            inspector.getByText("Adult Content", { exact: true }),
        ).toBeVisible();
        await expect(
            inspector.getByText("Gambling", { exact: true }),
        ).toBeVisible();
        await expect(
            inspector.getByText("My Focus List", { exact: true }),
        ).toBeVisible();
        await expect(inspector.getByRole("button", { name: "General" })).toHaveCount(0);
        await expect(
            inspector.getByRole("button", { name: "Categories", exact: true }),
        ).toHaveCount(0);
        await expect(
            inspector.getByRole("button", { name: "Specific websites" }),
        ).toHaveCount(0);

        // Schedule is the only other top-level section and stays spatial.
        await inspector.getByRole("button", { name: "Schedule" }).click();
        await expect(
            inspector.getByText("Weekly Coverage", { exact: true }),
        ).toHaveCount(0);
        await expect(
            inspector.getByRole("button", { name: "Always" }),
        ).toBeVisible();
        await expect(
            inspector.getByText("Periods", { exact: true }),
        ).toBeVisible();
        await expect(
            inspector.getByRole("button", { name: "Add period" }),
        ).toBeVisible();
        await expect(
            inspector.locator(
                '[role="img"][aria-label^="Weekly schedule preview"]',
            ),
        ).toBeVisible();
        await expect(inspector.locator("[data-schedule-cell]")).toHaveCount(0);
    });

    test("schedule periods edit inline and persist only after Apply + Save", async ({
        page,
    }) => {
        await page.setViewportSize({ width: 800, height: 600 });

        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                scheduleEntries: [...EXACT_PERIOD_SCHEDULE],
            }),
        ];

        let addedBody: unknown = null;
        let editedBody: unknown = null;
        let removed = false;

        await mockEngine(page, { blocks });
        await page.route(
            `${ENGINE}/network-blocks/block-social/schedule`,
            async (route) => {
                if (route.request().method() === "GET") {
                    await route.fulfill({
                        json: { scheduleEntries: EXACT_PERIOD_SCHEDULE },
                    });
                    return;
                }
                if (route.request().method() === "POST") {
                    addedBody = route.request().postDataJSON();
                    await route.fulfill({ status: 200 });
                    return;
                }
                await route.continue();
            },
        );
        await page.route(`${ENGINE}/schedule/sched-1`, async (route) => {
            if (route.request().method() === "PUT") {
                editedBody = route.request().postDataJSON();
                await route.fulfill({ status: 200 });
                return;
            }
            await route.continue();
        });
        await page.route(`${ENGINE}/schedule/sched-2`, async (route) => {
            if (route.request().method() === "DELETE") {
                removed = true;
                await route.fulfill({ status: 200 });
                return;
            }
            await route.continue();
        });

        await gotoPage(page);
        await blockRow(page, "Social Media")
            .getByRole("button", { name: "Edit" })
            .click();
        const inspector = blockEditor(page, "Edit Block");
        await inspector.getByRole("button", { name: "Schedule" }).click();

        const preview = inspector.locator("[data-weekly-preview]");
        await expect(preview).toBeVisible();
        expect(
            await preview.evaluate(
                (element) => element.scrollWidth <= element.clientWidth,
            ),
        ).toBe(true);

        const mondayTrack = inspector.locator(
            '[data-preview-track="monday"]',
        );
        const exactRange = mondayTrack.locator(
            '[data-coverage-range="applied"][data-start-minute="555"][data-end-minute="645"]',
        );
        await expect(exactRange).toHaveCount(1);
        const [trackBox, rangeBox] = await Promise.all([
            mondayTrack.boundingBox(),
            exactRange.boundingBox(),
        ]);
        expect(trackBox).not.toBeNull();
        expect(rangeBox).not.toBeNull();
        expect(rangeBox!.width / trackBox!.width).toBeCloseTo(
            90 / (24 * 60),
            2,
        );

        const saveButton = inspector.getByRole("button", {
            name: "Save",
            exact: true,
        });
        const addPeriod = inspector.getByRole("button", { name: "Add period" });
        await addPeriod.click();

        await expect(page.getByRole("dialog")).toHaveCount(0);
        const addEditor = inspector.locator("[data-add-period-editor]");
        await expect(addEditor.locator("[data-period-editor]")).toBeVisible();
        await expect(saveButton).toBeDisabled();
        await expect(
            inspector.getByRole("button", { name: "Blocking" }),
        ).toBeDisabled();
        await expect(addPeriod).toBeDisabled();
        await expect(
            inspector
                .locator('[data-period-row="sched-1"]')
                .getByRole("button", { name: "Edit" }),
        ).toBeDisabled();
        await expect(inspector.locator("[data-period-row]")).toHaveCount(2);

        await addEditor.getByRole("button", { name: "Weekends" }).click();
        await addEditor.getByLabel("Start").fill("11:15");
        await addEditor.getByLabel("End").fill("12:45");
        await expect(
            inspector.locator(
                '[data-preview-track="saturday"] [data-coverage-range="editing"][data-start-minute="675"][data-end-minute="765"]',
            ),
        ).toHaveCount(1);

        await addEditor.getByRole("button", { name: "Cancel" }).click();
        await expect(inspector.locator("[data-period-editor]")).toHaveCount(0);
        await expect(inspector.locator("[data-period-row]")).toHaveCount(2);
        await expect(
            inspector.locator('[data-coverage-range="editing"]'),
        ).toHaveCount(0);

        await addPeriod.click();
        const stagedAddEditor = inspector.locator("[data-add-period-editor]");
        await stagedAddEditor.getByRole("button", { name: "Weekends" }).click();
        await stagedAddEditor.getByLabel("Start").fill("11:15");
        await stagedAddEditor.getByLabel("End").fill("12:45");
        await stagedAddEditor.getByRole("button", { name: "Apply" }).click();
        await expect(inspector.locator("[data-period-row]")).toHaveCount(3);
        await expect(saveButton).toBeEnabled();

        const firstPeriod = inspector.locator('[data-period-row="sched-1"]');
        await firstPeriod.getByRole("button", { name: "Edit" }).click();
        const editEditor = firstPeriod.locator("[data-period-editor]");
        await expect(editEditor).toBeVisible();
        await expect(saveButton).toBeDisabled();
        await editEditor.getByLabel("Start").fill("09:30");
        await editEditor.getByLabel("End").fill("10:15");
        await expect(
            inspector.locator(
                '[data-preview-track="monday"] [data-coverage-range="editing"][data-start-minute="570"][data-end-minute="615"]',
            ),
        ).toHaveCount(1);
        const pinnedPreviewBox = await preview.boundingBox();
        expect(pinnedPreviewBox).not.toBeNull();
        expect(pinnedPreviewBox!.y).toBeGreaterThanOrEqual(0);
        expect(
            pinnedPreviewBox!.y + pinnedPreviewBox!.height,
        ).toBeLessThanOrEqual(page.viewportSize()!.height);

        if (process.env.CAPTURE_SCHEDULE_UI === "1") {
            await page.screenshot({
                path: "test-results/schedule-inline-light.png",
            });
            await page.emulateMedia({ colorScheme: "dark" });
            await expect(page.locator("html")).toHaveClass(/dark/);
            await page.screenshot({
                path: "test-results/schedule-inline-dark.png",
            });
        }

        await editEditor.getByRole("button", { name: "Apply" }).click();
        await expect(saveButton).toBeEnabled();

        const secondPeriod = inspector.locator('[data-period-row="sched-2"]');
        await secondPeriod.getByRole("button", { name: "Remove" }).click();
        await expect(secondPeriod).toHaveCount(0);

        await saveButton.click();
        await expect(inspector).toHaveCount(0);
        expect(editedBody).toMatchObject({
            monday: true,
            startTime: "09:30",
            endTime: "10:15",
        });
        expect(removed).toBe(true);
        expect(addedBody).toMatchObject({
            scheduleEntries: [
                {
                    saturday: true,
                    sunday: true,
                    startTime: "11:15",
                    endTime: "12:45",
                },
            ],
        });
    });

    test("locked schedule keeps existing periods protected while allowing stricter additions", async ({
        page,
    }) => {
        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                isLocked: true,
                lockState: {
                    state: "active_until_lock_end",
                    end: "2026-10-20T18:00:00",
                },
                scheduleEntries: [...EXACT_PERIOD_SCHEDULE],
            }),
        ];

        let addedBody: unknown = null;
        await mockEngine(page, { blocks });
        await page.route(
            `${ENGINE}/network-blocks/block-social/schedule`,
            async (route) => {
                if (route.request().method() === "GET") {
                    await route.fulfill({
                        json: { scheduleEntries: EXACT_PERIOD_SCHEDULE },
                    });
                    return;
                }
                if (route.request().method() === "POST") {
                    addedBody = route.request().postDataJSON();
                    await route.fulfill({ status: 200 });
                    return;
                }
                await route.continue();
            },
        );

        await gotoPage(page);
        await blockRow(page, "Social Media")
            .getByRole("button", { name: "View", exact: true })
            .click();
        const inspector = blockEditor(page, "Edit Block");
        await inspector.getByRole("button", { name: "Schedule" }).click();

        const persistentRows = inspector.locator('[data-period-row^="sched-"]');
        await expect(persistentRows).toHaveCount(2);
        for (const row of await persistentRows.all()) {
            await expect(
                row.getByRole("button", { name: "Edit" }),
            ).toBeDisabled();
            await expect(
                row.getByRole("button", { name: "Remove" }),
            ).toBeDisabled();
        }
        await expect(
            inspector.getByRole("button", { name: "Clear" }),
        ).toBeDisabled();

        await inspector.getByRole("button", { name: "Add period" }).click();
        const editor = inspector.locator("[data-add-period-editor]");
        await editor.getByRole("button", { name: "Weekends" }).click();
        await editor.getByLabel("Start").fill("18:15");
        await editor.getByLabel("End").fill("19:45");
        await editor.getByRole("button", { name: "Apply" }).click();

        const rows = inspector.locator("[data-period-row]");
        await expect(rows).toHaveCount(3);
        const temporaryRow = rows.last();
        await expect(
            temporaryRow.getByRole("button", { name: "Edit" }),
        ).toBeEnabled();
        await expect(
            temporaryRow.getByRole("button", { name: "Remove" }),
        ).toBeEnabled();

        const saveButton = inspector.getByRole("button", {
            name: "Save",
            exact: true,
        });
        await expect(saveButton).toBeEnabled();
        await saveButton.click();
        await expect(inspector).toHaveCount(0);
        expect(addedBody).toMatchObject({
            scheduleEntries: [
                {
                    saturday: true,
                    sunday: true,
                    startTime: "18:15",
                    endTime: "19:45",
                },
            ],
        });
    });

    test("specific websites preserve domain and advanced regex rule types", async ({
        page,
    }) => {
        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                scheduleEntries: ALWAYS_ON_SCHEDULE,
            }),
        ];
        await mockEngine(page, { blocks });
        const getCapturedRules = capturePost(
            page,
            "/network-blocks/block-social/user-rules",
        );
        await gotoPage(page);

        await blockRow(page, "Social Media")
            .getByRole("button", { name: "Edit" })
            .click();
        const inspector = blockEditor(page, "Edit Block");

        await inspector
            .getByRole("textbox", { name: "Specific website", exact: true })
            .fill("instagram.com");
        await inspector
            .getByRole("button", { name: "Add specific website" })
            .click();

        await inspector.getByText("Advanced matching", { exact: true }).click();
        await inspector
            .getByLabel("Advanced matching pattern")
            .fill("/reels(?:/|$)");
        await inspector
            .getByRole("button", { name: "Add advanced match" })
            .click();

        await inspector
            .getByRole("button", { name: "Save", exact: true })
            .click();

        expect(getCapturedRules()?.body).toEqual({
            rules: [
                { rule: "instagram.com", ruleType: "domain" },
                { rule: "/reels(?:/|$)", ruleType: "regex" },
            ],
        });
    });

    test("soft unlock modal: word-count validation then cooldown confirm", async ({
        page,
    }) => {
        await page.clock.install({ time: new Date("2026-09-10T10:00:00") });
        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                isLocked: true,
                lockState: {
                    state: "soft_unlockable",
                    end: "2026-09-30T18:00:00",
                },
                lock: {
                    isActive: true,
                    lockEnd: "2026-09-30T18:00:00",
                    lockState: {
                        state: "soft_unlockable",
                        end: "2026-09-30T18:00:00",
                    },
                    networkBlockId: "block-social",
                },
                scheduleEntries: ALWAYS_ON_SCHEDULE,
                sitePackConnectionCount: 2,
            }),
        ];

        const confirmCaptured = capturePost(
            page,
            "/network-blocks/block-social/unlock/confirm",
        );
        let initiateCalls = 0;
        await page.route(
            `${ENGINE}/network-blocks/block-social/unlock/initiate`,
            (route) => {
                initiateCalls += 1;
                return route.fulfill({
                    json: {
                        reflectionPrompt: "Why do you want to unlock?",
                        minWords: 20,
                        challengeId: "challenge-1",
                        cooldownSeconds: 5 * 60,
                        initiatedAt: "2026-09-10T10:00:00",
                    },
                });
            },
        );
        await mockEngine(page, { blocks });
        await gotoPage(page);

        // Reflection lock → the unlock action opens the reflection modal.
        await page.getByRole("button", { name: "Unlock block" }).click();
        const dialog = page.getByRole("dialog");
        await expect(
            dialog.getByRole("heading", { name: "Take a moment" }),
        ).toBeVisible();

        // Opening the reflection step must not start the engine cooldown.
        expect(initiateCalls).toBe(0);

        // Word-count validation: too few words keeps the commit disabled.
        const answer = dialog.getByPlaceholder("Write your answer here…");
        await answer.fill("not enough words here");
        await expect(dialog.getByText("4 / 20 words")).toBeVisible();
        await expect(
            dialog.getByRole("button", { name: "Start 5-minute cooldown" }),
        ).toBeDisabled();

        const longAnswer = Array.from(
            { length: 25 },
            (_, i) => `word${i}`,
        ).join(" ");
        await answer.fill(longAnswer);
        await expect(dialog.getByText("25 / 20 words")).toBeVisible();
        const commit = dialog.getByRole("button", {
            name: "Start 5-minute cooldown",
        });
        await expect(commit).toBeEnabled();
        // The modal content scrolls with the long answer; scroll the footer
        // into view and click through (the entrance animation can otherwise
        // leave the dialog overlay on top of the hit target).
        await commit.scrollIntoViewIfNeeded();
        await commit.click({ force: true });
        expect(initiateCalls).toBe(1);

        // The full five-minute cooldown starts only after the reflection is
        // submitted, and the button remains disabled until it expires.
        await expect(
            dialog.getByRole("heading", { name: "Cooldown in progress" }),
        ).toBeVisible();
        const waiting = dialog.getByRole("button", {
            name: /Unlock available in 0[45]:\d{2}/,
        });
        await expect(waiting).toBeDisabled();

        await page.clock.fastForward(5 * 60 * 1000);
        const unlockNow = dialog.getByRole("button", { name: "Unlock now" });
        await expect(unlockNow).toBeEnabled();
        await unlockNow.click();

        const captured = confirmCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toEqual({ challengeId: "challenge-1" });
        await expect(page.getByText("Block unlocked.")).toBeVisible();
    });

    test("lock modal: soft and hard modes with exact expiration", async ({
        page,
    }) => {
        // Pin "today" so the calendar's current month always contains an
        // enabled future day (the 15th) and the default 12:00 time is future.
        await page.clock.install({ time: new Date("2026-09-10T10:00:00") });

        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                scheduleEntries: ALWAYS_ON_SCHEDULE,
                sitePackConnectionCount: 2,
            }),
        ];
        const lockCaptured = capturePost(
            page,
            "/network-blocks/block-social/lock",
        );
        await mockEngine(page, { blocks });
        await gotoPage(page);

        // Open the lock modal from the unlocked row.
        await page.getByLabel("Lock this block").first().click();
        const dialog = page.getByRole("dialog");
        await expect(
            dialog.getByRole("heading", { name: "Lock Block" }),
        ).toBeVisible();

        // Soft mode (default): body sends soft_unlockable with no end.
        await dialog.getByRole("switch").click();
        await dialog.getByRole("button", { name: "Save" }).click();
        let captured = lockCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toEqual({ lockState: "soft_unlockable" });

        // Hard mode with a specific date.
        await page.getByLabel("Lock this block").first().click();
        await dialog.getByRole("switch").click();
        await dialog
            .getByRole("button", { name: "Soft Unlock", exact: true })
            .click();
        await page
            .getByRole("option", { name: "Locked until given date" })
            .click();
        await dialog.getByRole("button", { name: /pick date/i }).click();
        await page
            .getByRole("gridcell")
            .filter({ hasText: "15" })
            .first()
            .click();
        await dialog.getByRole("button", { name: "Save" }).click();

        captured = lockCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toMatchObject({
            lockState: "active_until_lock_end",
        });
        expect((captured!.body as { lockEnd: string }).lockEnd).toBe(
            "2026-09-15T12:00:00",
        );
    });

    test("connect a category to a block from the edit modal", async ({
        page,
    }) => {
        const blocks: MockBlock[] = [
            mkBlock({
                id: "block-social",
                name: "Social Media",
                scheduleEntries: ALWAYS_ON_SCHEDULE,
            }),
        ];
        await mockEngine(page, { blocks });
        // Register AFTER mockEngine so this handler wins (routes are LIFO).
        const connectCaptured = capturePost(
            page,
            "/network-blocks/block-social/site-lists",
        );
        await gotoPage(page);

        await blockRow(page, "Social Media")
            .getByRole("button", { name: "Edit" })
            .click();
        const inspector = blockEditor(page, "Edit Block");

        // Categories are on the default Blocking section; the synced catalog
        // entry toggles the connection directly.
        await inspector.getByRole("button", { name: /Adult Content/ }).click();
        await inspector.getByRole("button", { name: "Save" }).click();

        const captured = connectCaptured();
        expect(captured).not.toBeNull();
        expect(captured!.body).toEqual({ sitesListId: "pack-adult-free" });
    });

    test("engine offline shows the offline notice", async ({ page }) => {
        await mockEngine(page, { online: false });
        await gotoPage(page);

        const offline = page
            .locator("#ace-workspace")
            .getByRole("status")
            .filter({ hasText: "Engine offline" });
        await expect(offline).toBeVisible({ timeout: 15_000 });
        await expect(offline).toContainText("The engine is not reachable");
        // The block list and empty state are replaced by the notice.
        await expect(
            page.getByRole("heading", { name: "No Network Blocks" }),
        ).toHaveCount(0);
    });

    test("API failure surfaces an error toast and keeps the modal open", async ({
        page,
    }) => {
        await mockEngine(page, { blocks: [], createStatus: 500 });
        await gotoPage(page);

        await page
            .getByRole("button", { name: /^Create Block$/i })
            .first()
            .click();
        const inspector = blockEditor(page, "Create Block");
        await inspector.getByLabel(/Block Name/i).fill("Doomed Block");
        await inspector.getByRole("button", { name: "Save" }).click();

        await expect(
            page.getByText("Failed to synchronize settings with the engine."),
        ).toBeVisible();
        // The inspector stays open so the user can retry.
        await expect(
            inspector.getByRole("heading", { name: "Create Block" }),
        ).toBeVisible();
    });
});
