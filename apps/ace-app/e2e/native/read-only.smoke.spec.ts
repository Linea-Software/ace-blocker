/// <reference types="mocha" />
/// <reference types="webdriverio" />
import { browser, $, expect } from "@wdio/globals";
import type {} from "@wdio/tauri-service";
import { tmpdir } from "node:os";
import { join } from "node:path";

describe("ACE native UI (read-only)", () => {
    it("renders its real Tauri workspace and captures a screenshot", async () => {
        const navigation = await $("nav[aria-label]");
        await navigation.waitForDisplayed();
        const controlLink = await $('nav a[href="/"]');
        await expect(controlLink).toExist();

        // Use a temporary file, not the repository. No engine settings change.
        const path = join(tmpdir(), "ace-native-ui-smoke.png");
        await browser.saveScreenshot(path);
        console.log("Native ACE screenshot: " + path);
    });

    it("reads a harmless native Tauri command through WDIO", async () => {
        const version = await browser.tauri.execute(({ core }) => core.invoke("app_version"));
        if (typeof version !== "string" || !/^\d+\.\d+\.\d+/.test(version)) {
            throw new Error(`Unexpected native ACE version: ${String(version)}`);
        }
    });

    it("navigates to Blocks without mutating enforcement", async () => {
        const link = await $('nav a[href="/network-blocks"]');
        await link.click();
        const heading = await $("main h1");
        await heading.waitForDisplayed();
        await expect(heading).toHaveText(/.+/);
        await expect(link).toHaveAttribute("aria-current", "page");
    });

    it("navigates to Guide and survives a deep-route WebView reload", async () => {
        const guide = await $('nav a[href="/getting-started"]');
        await guide.click();
        const heading = await $("main h1");
        await heading.waitForDisplayed();
        await expect(heading).toHaveText(/.+/);
        await expect(guide).toHaveAttribute("aria-current", "page");

        // A packaged Tauri SPA must resolve a route directly after reload,
        // including after a Paraglide locale change or application restart.
        await browser.execute(() => window.location.reload());
        await heading.waitForDisplayed();
        await expect(guide).toHaveAttribute("aria-current", "page");
        await browser.saveScreenshot(join(tmpdir(), "ace-native-guide-reloaded.png"));
    });
});
