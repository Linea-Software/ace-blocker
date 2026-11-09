<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { downloadResistanceScripts, getResistanceModeSettings } from "$lib/engine-client";
    import { engineStore } from "$lib/stores/engine.svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { ListFilter, RefreshCw } from "@lucide/svelte";
    import { toast } from "svelte-sonner";

    type VersionState = "loading" | "ready" | "missing" | "unavailable";
    let installedVersion = $state<string | null>(null);
    let versionState = $state<VersionState>("loading");
    let updating = $state(false);
    let requestSeq = 0;

    const engineOnline = $derived(engineStore.isAlive);
    const isPro = $derived(userStore.isPro);
    const canUpdate = $derived(engineOnline && isPro && !updating);

    async function loadInstalledVersion() {
        if (!engineStore.isAlive) {
            versionState = "unavailable";
            return;
        }
        const seq = ++requestSeq;
        versionState = "loading";
        try {
            const { data, error } = await getResistanceModeSettings();
            if (seq !== requestSeq) return;
            if (error || !data) {
                versionState = "unavailable";
                return;
            }
            installedVersion = data.scriptsVersion ?? null;
            versionState = data.enforcementStatus === "scripts_missing" ? "missing" : "ready";
        } catch {
            if (seq === requestSeq) versionState = "unavailable";
        }
    }

    $effect(() => {
        if (engineOnline) {
            void loadInstalledVersion();
        } else {
            ++requestSeq;
            installedVersion = null;
            versionState = "unavailable";
        }
        return () => { ++requestSeq; };
    });

    async function handleUpdate() {
        if (!canUpdate) return;
        updating = true;
        try {
            const { error } = await downloadResistanceScripts();
            if (error) {
                toast.error(m.failed_to_download_scripts());
                return;
            }
            toast.success(m.scripts_sync_success());
            await loadInstalledVersion();
        } catch {
            toast.error(m.failed_to_download_scripts());
        } finally {
            updating = false;
        }
    }
</script>

<section id="resistance-scripts-update" class="settings-section scroll-mt-6" aria-labelledby="resistance-scripts-update-title">
    <header>
        <span class="section-mark" aria-hidden="true"><ListFilter size={16}/></span>
        <div>
            <h2 id="resistance-scripts-update-title">{m.settings_resistance_scripts_title()}</h2>
            <p>{m.settings_resistance_scripts_description()}</p>
        </div>
    </header>

    <div class="update-row">
        <div class="update-details">
            <div class="version-label">{m.settings_resistance_scripts_installed()}</div>
            <strong class="version-number" data-testid="settings-resistance-scripts-version">
                {#if installedVersion}
                    {installedVersion}
                {:else if versionState === "missing"}
                    {m.settings_resistance_scripts_missing()}
                {:else}
                    {m.settings_resistance_scripts_unavailable()}
                {/if}
            </strong>
            {#if !engineOnline}
                <p>{m.settings_resistance_scripts_engine_offline()}</p>
            {:else if !isPro}
                <p>{m.settings_resistance_scripts_pro_required()}</p>
            {:else if versionState === "missing"}
                <p>{m.scripts_empty_description()}</p>
            {:else if versionState === "unavailable"}
                <p>{m.settings_resistance_scripts_status_unavailable()}</p>
            {/if}
        </div>
        <button
            type="button"
            class="refresh-button"
            data-testid="settings-resistance-scripts-update"
            disabled={!canUpdate}
            onclick={handleUpdate}
        >
            <RefreshCw size={15} aria-hidden="true" class={updating ? "spinning" : undefined}/>
            {updating ? m.settings_resistance_scripts_updating() : m.settings_resistance_scripts_action()}
        </button>
    </div>
</section>

<style>
    .settings-section { display: grid; gap: 12px; }
    header { display: grid; grid-template-columns: 28px minmax(0,1fr); gap: 8px; align-items: start; }
    .section-mark {
        display: grid; width: 24px; height: 24px; place-items: center;
        border-left: 3px solid var(--ace-ash); color: var(--muted-foreground);
    }
    h2 {
        margin: 0; font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px; font-weight: 600;
    }
    header p { margin: 3px 0 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.4; }
    .update-row {
        display: flex; justify-content: space-between; align-items: center;
        gap: 16px; border: 1px solid var(--border); border-radius: 5px;
        background: var(--card); padding: 16px;
    }
    .update-details { min-width: 0; }
    .version-label { color: var(--muted-foreground); font-size: 12px; }
    .version-number { font-family: "IBM Plex Mono", monospace; font-size: 18px; font-weight: 600; }
    .update-details p { margin: 8px 0 0; color: var(--muted-foreground); font-size: 12px; line-height: 1.4; }
    .refresh-button {
        display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center;
        gap: 8px; min-height: 40px; border: 1px solid var(--border); border-radius: 4px;
        background: transparent; padding: 9px 12px; color: var(--foreground);
        font-size: 12px; cursor: pointer;
    }
    .refresh-button:hover:not(:disabled) { background: var(--accent); }
    .refresh-button:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
    .refresh-button:disabled { cursor: not-allowed; opacity: .55; }
    :global(.spinning) { animation: spin 1.4s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg); } }
    @media (prefers-reduced-motion: reduce) {
        :global(.spinning) { animation: none; }
    }
    @media (max-width: 500px) { .update-row { flex-direction: column; align-items: stretch; } }
</style>
