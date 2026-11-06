/** Expose only locales whose desktop message catalogs have full coverage. */
export type LanguagePreference = "system" | "en" | "it" | "de";
export type AppLocale = "en" | "it" | "de";

export function isLanguagePreference(value: unknown): value is LanguagePreference {
    return value === "system" || value === "en" || value === "it" || value === "de";
}

/** Never select an unfinished locale merely because Paraglide lists it. */
export function resolveAppLocale(preference: LanguagePreference, systemLocale: string | null): AppLocale {
    if (preference !== "system") return preference;
    const language = systemLocale?.trim().replace("_", "-").split("-")[0]?.toLowerCase();
    return language === "it" || language === "de" ? language : "en";
}

export interface LanguagePreferenceStore {
    get<T>(key: string): Promise<T | undefined>;
    set(key: string, value: { value: LanguagePreference }): Promise<void>;
    save(): Promise<void>;
}

export async function loadLanguagePreference(store: LanguagePreferenceStore): Promise<LanguagePreference> {
    const saved = await store.get<{ value?: unknown }>("language");
    return isLanguagePreference(saved?.value) ? saved.value : "system";
}

export async function saveLanguagePreference(store: LanguagePreferenceStore, language: LanguagePreference): Promise<void> {
    await store.set("language", { value: language });
    await store.save();
}
