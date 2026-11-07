import { invoke, isTauri } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import { getLocale, setLocale } from "$lib/paraglide/runtime";
import { localizationStore } from "$lib/stores/localization.svelte";
import {
    loadLanguagePreference,
    resolveAppLocale,
    saveLanguagePreference,
    type LanguagePreference,
} from "./language-preference";

let initPromise: Promise<void> | undefined;
let storePromise: ReturnType<typeof load> | undefined;
let preferenceChanged = false;
let saving: Promise<void> = Promise.resolve();
const browserPreferenceKey = "ace:app-language";

function browserLanguagePreference(): LanguagePreference {
    try {
        const saved = localStorage.getItem(browserPreferenceKey);
        return saved === "it" ||
            saved === "en" ||
            saved === "de" ||
            saved === "es"
            ? saved
            : "system";
    } catch {
        return "system";
    }
}

function settingsStore() {
    return (storePromise ??= load("settings.json"));
}

/** Use Tauri's native OS locale; browser locale is for non-Tauri previews only. */
async function systemLocale(): Promise<string | null> {
    if (isTauri()) {
        try {
            return await invoke<string | null>("system_locale");
        } catch (error) {
            console.error("Failed to detect operating-system language:", error);
            return null; // Unknown native locale: safe English fallback.
        }
    }
    return navigator.languages?.[0] ?? navigator.language ?? null;
}

async function applyPreference(preference: LanguagePreference) {
    const locale = resolveAppLocale(preference, await systemLocale());
    if (getLocale() !== locale) {
        // Paraglide's cookie-backed strategy reloads to update the entire
        // hydrated app, including text initialized outside Svelte components.
        await setLocale(locale);
    }
}

/** Called once from the root layout, including when settings is never opened. */
export function initializeAppLanguage(): Promise<void> {
    return (initPromise ??= (async () => {
        if (isTauri()) {
            try {
                const saved = await loadLanguagePreference(
                    await settingsStore(),
                );
                if (!preferenceChanged) localizationStore.language = saved;
            } catch (error) {
                console.error("Failed to restore language preference:", error);
            }
        } else if (!preferenceChanged) {
            // The browser-only development preview has no Tauri store.
            localizationStore.language = browserLanguagePreference();
        }
        await applyPreference(localizationStore.language);
    })());
}

export async function changeAppLanguage(
    preference: LanguagePreference,
): Promise<void> {
    await initializeAppLanguage();
    preferenceChanged = true;
    const previous = localizationStore.language;
    localizationStore.language = preference;
    try {
        // Persist before changing the locale, because Paraglide navigates.
        if (isTauri()) {
            saving = saving.then(async () => {
                await saveLanguagePreference(await settingsStore(), preference);
            });
            await saving;
        } else {
            // Make the non-Tauri preview usable for testing locale changes.
            localStorage.setItem(browserPreferenceKey, preference);
        }
        await applyPreference(preference);
    } catch (error) {
        localizationStore.language = previous;
        saving = Promise.resolve();
        throw error;
    }
}
