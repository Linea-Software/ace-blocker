import { expect, test } from "@playwright/test";

/**
 * Getting Started guide tests. The page is fully static (no engine calls),
 * so these run without mocks.
 */

async function gotoPage(page: import("@playwright/test").Page) {
    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });
    await page.goto("/getting-started");
    // Generous timeout: the first request to a route compiles Vite modules,
    // which can take several seconds under parallel workers.
    await expect(
        page.getByRole("heading", { level: 1, name: "Getting Started" }),
    ).toBeVisible({ timeout: 15_000 });
    return page;
}

test.describe("Getting Started", () => {
    test("renders replay, concepts, setup and FAQ sections", async ({
        page,
    }) => {
        await gotoPage(page);

        // Replay card (Card.Title renders a div, not a heading).
        await expect(
            page.getByText("Setup tools", { exact: true }),
        ).toBeVisible();
        await expect(
            page.getByRole("button", { name: "Start the tour" }),
        ).toBeVisible();
        await expect(
            page.getByRole("button", { name: "Run setup again" }),
        ).toBeVisible();

        // Core concepts (card titles are divs; scope to the section since
        // some titles collide with sidebar links).
        await expect(
            page.getByRole("heading", { name: "Core concepts", exact: true }),
        ).toBeVisible();
        const conceptsSection = page.locator("section").filter({
            has: page.getByRole("heading", {
                name: "Core concepts",
                exact: true,
            }),
        });
        for (const concept of [
            "Network Blocks",
            "Categories",
            "Schedules",
            "Locks",
            "Resistance Mode",
        ]) {
            await expect(
                conceptsSection.getByText(concept, { exact: true }),
            ).toBeVisible();
        }

        // Setup checklist.
        await expect(
            page.getByRole("heading", { name: "Setting up", exact: true }),
        ).toBeVisible();

        // FAQ: all questions render inside the accordion.
        await expect(
            page.getByRole("heading", {
                name: "Common questions",
                exact: true,
            }),
        ).toBeVisible();
        await expect(
            page.getByRole("button", {
                name: "Why is a site I need being blocked?",
            }),
        ).toBeVisible();
        await expect(
            page.getByRole("button", {
                name: "I locked myself out. Now what?",
            }),
        ).toBeVisible();

        // "Still stuck?" card links to help (title is a div).
        await expect(
            page.getByText("Still stuck?", { exact: true }),
        ).toBeVisible();
    });

    test("FAQ answers expand", async ({ page }) => {
        await gotoPage(page);

        const question = page.getByRole("button", {
            name: "Why is a site I need being blocked?",
        });
        await question.click();
        await expect(
            page.getByText(/A category or Custom List probably matches it/i),
        ).toBeVisible();
    });
});
