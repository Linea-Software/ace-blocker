<script lang="ts">
    import { onMount } from "svelte";
    import { dev } from "$app/env";
    import { invoke, isTauri } from "@tauri-apps/api/core";
    import * as m from "$lib/paraglide/messages.js";
    import { load } from "@tauri-apps/plugin-store";
    import { setMode } from "mode-watcher";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";
    import AppearanceSection from "./_AppearanceSection.svelte";
    import LanguageSection from "./_LanguageSection.svelte";
    import BackupSection from "./_BackupSection.svelte";
    import ProductUpdateSection from "./_ProductUpdateSection.svelte";
    import LicenseSection from "./_LicenseSection.svelte";
    import DebugSection from "./_DebugSection.svelte";

    let selectedTheme = $state<"light" | "dark" | "system">("system");
    let showDebugSettings = $state(dev);

    let userChangedTheme = false;
    let pendingSave: Promise<void> = Promise.resolve();
    let settingsStore: ReturnType<typeof load> | undefined;

    function isTheme(value: unknown): value is "light" | "dark" | "system" {
        return value === "light" || value === "dark" || value === "system";
    }

    onMount(() => {
        // Vite dev and native debug builds can expose these controls. A
        // release Tauri binary always reports false, regardless of URL.
        if (!dev && isTauri()) {
            void invoke<boolean>("is_debug_mode")
                .then((isDebug) => (showDebugSettings = isDebug))
                .catch(() => (showDebugSettings = false));
        }
        settingsStore = load("settings.json");
        void (async () => {
            try {
                const store = await settingsStore;
                const savedTheme = await store.get<{ value: string }>("theme");
                if (!userChangedTheme && isTheme(savedTheme?.value)) {
                    selectedTheme = savedTheme.value;
                    setMode(savedTheme.value);
                }
            } catch (error) {
                console.error("Failed to load settings store:", error);
            }
        })();
    });

    function handleThemeChange(value: string) {
        if (!isTheme(value)) return;
        userChangedTheme = true;
        selectedTheme = value;
        setMode(value);

        pendingSave = pendingSave.then(async () => {
            const store = await (settingsStore ?? load("settings.json"));
            await store.set("theme", { value });
            await store.save();
        }).catch((error: unknown) => {
            console.error("Failed to save settings:", error);
        });
    }
</script>

<svelte:head>
    <title>{m.settings()} · {m.app_name()}</title>
</svelte:head>

<PageLayout width="narrow">
    {#snippet header()}
        <PageHeader
            title={m.settings()}
            subtitle={m.settings_description()}
        />
    {/snippet}

    <div class="grid gap-7">
        <AppearanceSection {selectedTheme} onThemeChange={handleThemeChange} />
        <div class="settings-rule" aria-hidden="true"></div>
        <LanguageSection />
        <div class="settings-rule" aria-hidden="true"></div>
        <BackupSection />
        <div class="settings-rule" aria-hidden="true"></div>
        <LicenseSection />
        <div class="settings-rule" aria-hidden="true"></div>
        <ProductUpdateSection />
        {#if showDebugSettings}
            <div class="settings-rule" aria-hidden="true"></div>
            <DebugSection />
        {/if}
    </div>
</PageLayout>

<style>
    .settings-rule {
        height: 1px;
        background: color-mix(in srgb, var(--border) 72%, transparent);
    }
</style>
