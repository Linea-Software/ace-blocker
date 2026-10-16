import { expect, test } from "@playwright/test";

const ENGINE = "https://127.0.0.1:10567";

test("legal gate cannot be dismissed until both policies are accepted", async ({
    page,
}) => {
    let tosAccepted = false;
    let acceptRequests = 0;

    await page.route(`${ENGINE}/ping`, (route) =>
        route.fulfill({ status: 200, body: "pong" }),
    );
    await page.route(`${ENGINE}/account/info`, (route) =>
        route.fulfill({ json: { isPro: false, tosAccepted } }),
    );
    await page.route(`${ENGINE}/account/refresh`, (route) =>
        route.fulfill({ json: { isPro: false, tosAccepted } }),
    );
    await page.route(`${ENGINE}/account/accept-tos`, (route) => {
        acceptRequests += 1;
        tosAccepted = true;
        return route.fulfill({ status: 200 });
    });

    await page.addInitScript(() => {
        localStorage.setItem(
            "ace:onboarding:v1",
            JSON.stringify({ wizardCompleted: true, tourCompleted: true }),
        );
    });

    await page.goto("/");

    const dialog = page.getByRole("dialog");
    await expect(
        dialog.getByRole("heading", {
            name: "Terms of Service & Privacy Policy",
        }),
    ).toBeVisible({ timeout: 15_000 });

    await expect(dialog.getByRole("button", { name: "Close" })).toHaveCount(0);
    await expect(page.locator(".app-shell")).toHaveAttribute("inert", "");

    const continueButton = dialog.getByRole("button", {
        name: "Accept & Continue",
    });
    await expect(continueButton).toBeDisabled();

    await page.keyboard.press("Escape");
    await expect(dialog).toBeVisible();

    await page.mouse.click(4, 4);
    await expect(dialog).toBeVisible();

    const checkboxes = dialog.getByRole("checkbox");
    await expect(checkboxes).toHaveCount(2);

    await checkboxes.nth(0).click();
    await expect(continueButton).toBeDisabled();

    await checkboxes.nth(1).click();
    await expect(continueButton).toBeEnabled();

    await continueButton.click();

    await expect(dialog).toBeHidden();
    expect(acceptRequests).toBe(1);
});
