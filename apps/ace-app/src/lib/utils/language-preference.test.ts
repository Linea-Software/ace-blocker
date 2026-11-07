import { describe, expect, it, vi } from "vitest";
import {
    isLanguagePreference,
    loadLanguagePreference,
    resolveAppLocale,
    saveLanguagePreference,
    type LanguagePreferenceStore,
} from "./language-preference";

describe("desktop language preference", () => {
    it("starts in system mode and detects Italian, German and Spanish", () => {
        expect(isLanguagePreference(undefined)).toBe(false);
        expect(isLanguagePreference("system")).toBe(true);
        expect(resolveAppLocale("system", "it-IT")).toBe("it");
        expect(resolveAppLocale("system", "it_IT")).toBe("it");
        expect(resolveAppLocale("system", "en-US")).toBe("en");
        for (const locale of [
            "es",
            "es-ES",
            "es-MX",
            "es-AR",
            "es-CO",
            "es-419",
            "ES_mx",
        ]) {
            expect(resolveAppLocale("system", locale)).toBe("es");
        }
        for (const locale of ["de", "de-DE", "de-AT", "de-CH", "DE_de"]) {
            expect(resolveAppLocale("system", locale)).toBe("de");
        }
    });

    it("falls back to English for configured but untranslated languages", () => {
        for (const locale of ["fr", "pt-BR", "zh-Hans", null, ""]) {
            expect(resolveAppLocale("system", locale)).toBe("en");
        }
    });

    it("honors the manual override and supports returning to system mode", () => {
        expect(resolveAppLocale("it", "en-US")).toBe("it");
        expect(resolveAppLocale("de", "it-IT")).toBe("de");
        expect(resolveAppLocale("es", "de-DE")).toBe("es");
        expect(resolveAppLocale("de", "en-US")).toBe("de");
        expect(resolveAppLocale("system", "de-CH")).toBe("de");
        expect(resolveAppLocale("en", "it-IT")).toBe("en");
        expect(resolveAppLocale("system", "it-IT")).toBe("it");
    });

    it("restores a saved preference and ignores invalid values", async () => {
        const get = vi.fn().mockResolvedValue({ value: "it" });
        const store = { get } as unknown as LanguagePreferenceStore;
        expect(await loadLanguagePreference(store)).toBe("it");
        expect(get).toHaveBeenCalledWith("language");
        get.mockResolvedValue({ value: "de" });
        expect(await loadLanguagePreference(store)).toBe("de");
        get.mockResolvedValue({ value: "es" });
        expect(await loadLanguagePreference(store)).toBe("es");
        get.mockResolvedValue({ value: "xx" });
        expect(await loadLanguagePreference(store)).toBe("system");
        get.mockResolvedValue(undefined);
        expect(await loadLanguagePreference(store)).toBe("system");
    });

    it("persists the override and returns to system preference", async () => {
        const set = vi.fn().mockResolvedValue(undefined);
        const save = vi.fn().mockResolvedValue(undefined);
        const store = { set, save } as unknown as LanguagePreferenceStore;
        await saveLanguagePreference(store, "it");
        await saveLanguagePreference(store, "de");
        await saveLanguagePreference(store, "es");
        await saveLanguagePreference(store, "system");
        expect(set).toHaveBeenNthCalledWith(1, "language", { value: "it" });
        expect(set).toHaveBeenNthCalledWith(2, "language", { value: "de" });
        expect(set).toHaveBeenNthCalledWith(3, "language", { value: "es" });
        expect(set).toHaveBeenNthCalledWith(4, "language", { value: "system" });
        expect(save).toHaveBeenCalledTimes(4);
    });
});
