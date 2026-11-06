<script lang="ts">
    import { Languages } from "@lucide/svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { localizationStore } from "$lib/stores/localization.svelte";
    import { changeAppLanguage } from "$lib/utils/app-language";
    import { isLanguagePreference } from "$lib/utils/language-preference";

    let error = $state(false);
    let busy = $state(false);

    async function changeLanguage(value: string) {
        if (!isLanguagePreference(value) || busy) return;
        error = false;
        busy = true;
        try {
            await changeAppLanguage(value);
        } catch (cause) {
            console.error("Failed to save app language:", cause);
            error = true;
        } finally {
            busy = false;
        }
    }
</script>

<section class="settings-section">
    <header>
        <span class="section-mark" aria-hidden="true"><Languages size={16} /></span>
        <div>
            <h2>{m.settings_language_title()}</h2>
            <p>{m.settings_language_description()}</p>
        </div>
    </header>
    <div class="setting-row">
        <label for="app-language">{m.settings_language_label()}</label>
        <select
            id="app-language"
            value={localizationStore.language}
            disabled={busy}
            onchange={(event) => void changeLanguage(event.currentTarget.value)}
        >
            <option value="system">{m.settings_language_system()}</option>
            <option value="en">English</option>
            <option value="it">Italiano</option>
            <option value="de">Deutsch</option>
        </select>
    </div>
    {#if error}
        <p class="error" role="alert">{m.settings_language_save_failed()}</p>
    {/if}
</section>

<style>
    .settings-section { display: grid; gap: 12px; }
    header { display: grid; grid-template-columns: 28px minmax(0, 1fr); gap: 8px; align-items: start; }
    .section-mark { width: 24px; height: 24px; display: grid; place-items: center; border-left: 3px solid var(--ace-ash); color: var(--muted-foreground); }
    h2 { margin: 0; font: 600 18px "IBM Plex Sans Condensed", sans-serif; }
    header p { margin: 3px 0 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.4; }
    .setting-row {
        min-height: 72px; padding: 13px 14px; display: flex; align-items: center;
        justify-content: space-between; gap: 16px;
        border: 1px solid var(--border); border-radius: 5px; background: var(--card);
    }
    label { font: 600 14px "IBM Plex Sans Condensed", sans-serif; }
    select {
        width: min(100%, 215px); min-height: 38px; padding: 7px 10px;
        border: 1px solid var(--border); border-radius: 4px;
        background: var(--background); color: var(--foreground); font: 500 13px "IBM Plex Sans", sans-serif;
        cursor: pointer;
    }
    select:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
    .error { color: var(--destructive); font-size: 12px; }
    @container page (max-width: 560px) {
        .setting-row { align-items: stretch; flex-direction: column; }
        select { width: 100%; }
    }
</style>
