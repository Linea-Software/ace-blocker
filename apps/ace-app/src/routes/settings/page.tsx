import { onSettled } from "solid-js";
import { invoke, isTauri } from "@tauri-apps/api/core";
import * as m from "$lib/paraglide/messages.js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import { setMode, themeMode, type ThemeMode } from "$lib/utils/theme";
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

function isTheme(value: string): value is ThemeMode {
    return value === "light" || value === "dark" || value === "system";
}

export default function SettingsPage() {
    const state = {
        showDebugSettings: import.meta.env.DEV,
    };
    reactiveFields(state, ["showDebugSettings"]);

    onSettled(() => {
        document.title = `${m.settings()} · ${m.app_name()}`;
        if (!import.meta.env.DEV && isTauri()) {
            void invoke<boolean>("is_debug_mode")
                .then((isDebug) => {
                    state.showDebugSettings = isDebug;
                })
                .catch(() => {
                    state.showDebugSettings = false;
                });
        }
    });

    function handleThemeChange(value: string) {
        if (isTheme(value)) setMode(value);
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
                    selectedTheme={themeMode()}
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
