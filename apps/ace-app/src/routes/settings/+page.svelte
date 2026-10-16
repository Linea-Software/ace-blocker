<script lang="ts">
    import { onMount } from "svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { load } from "@tauri-apps/plugin-store";
    import { setMode } from "mode-watcher";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";
    import AppearanceSection from "./_AppearanceSection.svelte";
    import BackupSection from "./_BackupSection.svelte";
    import LicenseSection from "./_LicenseSection.svelte";

    let selectedTheme = $state("system");

    onMount(async () => {
        try {
            const store = await load("settings.json");
            const savedTheme = await store.get<{ value: string }>("theme");
            if (savedTheme) {
                selectedTheme = savedTheme.value;
                setMode(savedTheme.value as "light" | "dark" | "system");
            }
        } catch (error) {
            console.error("Failed to load settings store:", error);
        }
    });

    $effect(() => {
        const currentSettings = {
            theme: { value: selectedTheme },
        };

        void (async () => {
            try {
                const store = await load("settings.json");
                for (const [key, value] of Object.entries(currentSettings)) {
                    await store.set(key, value);
                }
                await store.save();
            } catch (error) {
                console.error("Failed to save settings:", error);
            }
        })();
    });

    function handleThemeChange(value: string) {
        selectedTheme = value;
        setMode(value as "light" | "dark" | "system");
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
        <BackupSection />
        <div class="settings-rule" aria-hidden="true"></div>
        <LicenseSection />
    </div>
</PageLayout>

<style>
    .settings-rule {
        height: 1px;
        background: color-mix(in srgb, var(--border) 72%, transparent);
    }
</style>
