import { onSettled } from "solid-js";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import * as m from "$lib/paraglide/messages.js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import { setMode, themeMode } from "$lib/utils/theme";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";
import AppearanceSection from "./_AppearanceSection";
import LanguageSection from "./_LanguageSection";
import BackupSection from "./_BackupSection";
import ProductUpdateSection from "./_ProductUpdateSection";
import ResistanceScriptsUpdateSection from "./_ResistanceScriptsUpdateSection";
import LicenseSection from "./_LicenseSection";
import DebugSection from "./_DebugSection";
import "./page.css";

type Theme = "light" | "dark" | "system";
function isTheme(value: unknown): value is Theme {
    return value === "light" || value === "dark" || value === "system";
}

export default function SettingsPage() {
    const state = {
        selectedTheme: "system" as Theme,
        showDebugSettings: import.meta.env.DEV,
    };
    reactiveFields(state, ["selectedTheme", "showDebugSettings"]);
    let userChangedTheme = false;
    let pendingSave: Promise<void> = Promise.resolve();
    let settingsStore: ReturnType<typeof load> | undefined;

    const applyTheme = (theme: Theme) => setMode(theme);
    onSettled(() => {
        document.title = `${m.settings()} · ${m.app_name()}`;
        state.selectedTheme = themeMode();
        if (!import.meta.env.DEV && isTauri()) {
            void invoke<boolean>("is_debug_mode")
                .then((isDebug) => {
                    state.showDebugSettings = isDebug;
                })
                .catch(() => {
                    state.showDebugSettings = false;
                });
        }
        settingsStore = load("settings.json");
        void (async () => {
            try {
                const store = await settingsStore;
                const saved = await store.get<{ value: string }>("theme");
                if (!userChangedTheme && isTheme(saved?.value)) {
                    state.selectedTheme = saved.value;
                    applyTheme(saved.value);
                }
            } catch (error) {
                console.error("Failed to load settings store:", error);
            }
        })();
    });

    function handleThemeChange(value: string) {
        if (!isTheme(value)) return;
        userChangedTheme = true;
        state.selectedTheme = value;
        applyTheme(value);
        pendingSave = pendingSave
            .then(async () => {
                const store = await (settingsStore ?? load("settings.json"));
                await store.set("theme", { value });
                await store.save();
            })
            .catch((error: unknown) => {
                console.error("Failed to save settings:", error);
            });
    }

    return (
        <PageLayout
            width="narrow"
            header={
                <PageHeader
                    title={m.settings()}
                    subtitle={m.settings_description()}
                />
            }
        >
            <div class="grid gap-7">
                <AppearanceSection
                    selectedTheme={state.selectedTheme}
                    onThemeChange={handleThemeChange}
                />
                <div class="settings-rule" aria-hidden="true"></div>
                <LanguageSection />
                <div class="settings-rule" aria-hidden="true"></div>
                <BackupSection />
                <div class="settings-rule" aria-hidden="true"></div>
                <LicenseSection />
                <div class="settings-rule" aria-hidden="true"></div>
                <ProductUpdateSection />
                <div class="settings-rule" aria-hidden="true"></div>
                <ResistanceScriptsUpdateSection />
                {state.showDebugSettings ? (
                    <>
                        <div class="settings-rule" aria-hidden="true"></div>
                        <DebugSection />
                    </>
                ) : null}
            </div>
        </PageLayout>
    );
}
