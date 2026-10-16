import { test, expect, type Page } from "@playwright/test";

/**
 * E2E Test Suite for Network Blocks.
 * This suite runs against a LIVE engine at 127.0.0.1:10567.
 *
 * CRITICAL: Since we are testing against a single local engine instance,
 * we must run tests SERIALY to avoid state conflicts.
 */

test.describe.configure({ mode: "serial" });

test.describe("Network Blocks Management", () => {
    test.beforeAll(async ({ request }) => {
        console.log("--- STARTING CLEANUP ---");
        try {
            const response = await request.get(
                "https://127.0.0.1:10567/network-blocks",
            );
            if (response.ok()) {
                const data = await response.json();
                const blocks = data.networkBlocks || [];
                for (const block of blocks) {
                    await request.delete(
                        `https://127.0.0.1:10567/network-blocks/${block.id}`,
                    );
                }
                console.log(`Cleaned up ${blocks.length} existing blocks.`);
            }
        } catch (e) {
            console.warn("Cleanup failed:", e);
        }
    });

    test.beforeEach(async ({ page }) => {
        // Connectivity check
        try {
            // We use the request context to ping the engine directly
            const pingCheck = await page.request.get(
                "https://127.0.0.1:10567/ping",
            );
            if (pingCheck.status() !== 200) {
                throw new Error();
            }
        } catch (error) {
            throw new Error(
                `CRITICAL ENGINE FAILURE: Local backend is not reachable.`,
            );
        }

        await page.goto("/network-blocks");
    });

    /**
     * Helper to create a block if it doesn't exist.
     */
    async function ensureBlockExists(page: Page, name: string) {
        let heading = page.getByRole("heading", { name, exact: true });
        if ((await heading.count()) === 0) {
            // Try one reload just in case
            await page.reload();
            if ((await heading.count()) === 0) {
                console.log(`Creating missing block for test: "${name}"`);
                await page
                    .getByRole("button", { name: /^Create Block$/i })
                    .first()
                    .click();
                await page.getByLabel(/Block Name/i).fill(name);
                await page.getByRole("button", { name: /^Save$/i }).click();
                await expect(page.getByRole("dialog")).toBeHidden();
            }
        }
    }

    test.describe("A. The Empty State", () => {
        test("should render empty state correctly", async ({ page }) => {
            await expect(
                page.getByRole("heading", { name: "No Network Blocks" }),
            ).toBeVisible();
        });
    });

    test.describe("B. Creation Scenarios", () => {
        test("should successfully create a new block", async ({ page }) => {
            const blockName = "Creation Scenario Block";
            await page
                .getByRole("button", { name: /^Create Block$/i })
                .first()
                .click();
            await page.getByLabel(/Block Name/i).fill(blockName);
            await page.getByRole("button", { name: /^Save$/i }).click();
            await expect(page.getByRole("dialog")).toBeHidden();
            await expect(
                page.getByRole("heading", { name: blockName, exact: true }),
            ).toBeVisible();
        });
    });

    test.describe("C. Editing Scenarios", () => {
        test("should rename a block", async ({ page }) => {
            const initialName = "Rename Scenario Block";
            const newName = "Renamed Successfully";

            await ensureBlockExists(page, initialName);

            // Click the card (role=button containing the heading)
            const card = page
                .locator('div[role="button"]')
                .filter({
                    has: page.getByRole("heading", {
                        name: initialName,
                        exact: true,
                    }),
                })
                .first();
            await card.click();

            await page.getByLabel(/Block Name/i).fill(newName);
            await page.getByRole("button", { name: /^Save$/i }).click();

            await expect(page.getByRole("dialog")).toBeHidden();
            await expect(
                page.getByRole("heading", { name: newName, exact: true }),
            ).toBeVisible({ timeout: 10000 });
            await expect(
                page.getByRole("heading", { name: initialName, exact: true }),
            ).toBeHidden();
        });
    });

    test.describe("D. Deletion Scenarios", () => {
        test("should delete a block", async ({ page }) => {
            const blockName = "Deletion Scenario Block";

            await ensureBlockExists(page, blockName);

            const card = page
                .locator('div[role="button"]')
                .filter({
                    has: page.getByRole("heading", {
                        name: blockName,
                        exact: true,
                    }),
                })
                .first();

            // Interaction with dropdown
            await card.getByRole("button").click(); // The menu dots button
            await page.getByRole("menuitem", { name: /Delete/i }).click();

            // Handle the Confirmation Dialog (might be an AlertDialog or Dialog)
            await page.getByRole("button", { name: /^Confirm$/i }).click();

            await expect(
                page.getByRole("heading", { name: blockName, exact: true }),
            ).toBeHidden();
        });
    });
});
