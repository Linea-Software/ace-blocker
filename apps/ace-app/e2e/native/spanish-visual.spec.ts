/// <reference types="mocha" />
import { browser, $, expect } from "@wdio/globals";
import { join } from "node:path";

const root = "/home/cap/Projects/ace-project/.localization-work/screenshots";

describe("Spanish native screenshot and language switch (temporary QA)", () => {
    it("selects Spanish in the real Tauri app and inspects representative pages", async () => {
        const settingsLink = await $('nav a[href$="/settings"]');
        await settingsLink.waitForExist();
        await settingsLink.click();
        const language = await $("#app-language");
        await language.waitForDisplayed();
        const initial = await language.getValue();
        console.log("Native selected language before QA:", initial);
        try {
            await language.selectByAttribute("value", "es");
            await browser.waitUntil(async () => (await $("h2").getText()).includes("Idioma") || (await $("body").getText()).includes("Idioma de la aplicación"), {timeout:20000});
            await browser.saveScreenshot(join(root, "app-es-settings-native.png"));
            const pages = [
                ["/", "control"], ["/network-blocks","blocks"], ["/resistance-mode","resistance"],
                ["/site-packs","categories"], ["/getting-started","guide"]
            ];
            for(const [route,name] of pages) {
                const link=await $('nav a[href$="'+route+'"]');
                if (await link.isExisting()) {
                    await link.click(); await browser.pause(500);
                    await browser.saveScreenshot(join(root, "app-es-"+name+"-native.png"));
                    console.log("Native screenshot:",name, (await $("body").getText()).slice(0,80).replace(/\n/g," "));
                }
            }
        } finally {
            const settingsAgain=await $('nav a[href$="/settings"]');
            if(await settingsAgain.isExisting()) {
                await settingsAgain.click();
                const select=await $("#app-language");
                await select.waitForDisplayed();
                await select.selectByAttribute("value", initial as string);
            }
        }
    });
});