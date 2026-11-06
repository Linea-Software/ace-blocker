/** Only English and Italian have full message coverage. */
export type LanguagePreference = "system" | "en" | "it";
export type AppLocale = "en" | "it";

export function isLanguagePreference(value: unknown): value is LanguagePreference {
    return value === "system" || value === "en" || value === "it";
}

/** Never select an unfinished locale merely because Paraglide lists it. */
export function resolveAppLocale(preference: LanguagePreference, systemLocale: string | null): AppLocale {
    if (preference !== "system") return preference;
    const language = systemLocale?.trim().replace("_", "-").split("-")[0]?.toLowerCase();
    return language === "it" ? "it" : "en";
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
