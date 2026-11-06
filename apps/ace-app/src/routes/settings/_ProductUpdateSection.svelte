<script lang="ts">
    import { onMount } from "svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { checkUpdates, getVersion } from "$lib/engine-client";
    import { ArrowDownToLine, RefreshCw, ShieldCheck } from "@lucide/svelte";

    type CheckState = "checking" | "disabled" | "upToDate" | "available" | "downloading" | "verifying" | "downloaded" | "error";
    let updateState = $state<CheckState>("checking");
    let installedVersion = $state<string | null>(null);
    let releaseVersion = $state<string | null>(null);
    let busy = $state(false);

    async function refreshStatus(polling = false) {
        if (busy) return;
        busy = true;
        if (!polling) updateState = "checking";
        try {
            // Read installed version even when the remote feed is offline.
            const installed = await getVersion();
            if (installed.data) installedVersion = installed.data.version;

            const response = await checkUpdates();
            if (response.error || !response.data) {
                updateState = "error";
                return;
            }
            installedVersion = response.data.engine.currentVersion;
            releaseVersion = response.data.engine.latestVersion ?? null;
            updateState = response.data.engine.status;
        } catch (error) {
            console.error("Product update check failed", error);
            updateState = "error";
        } finally {
            busy = false;
        }
    }

    onMount(() => {
        void refreshStatus();
        // Poll only while a signed MSI is in progress; do not make the UI
        // block navigation or start a second downloader.
        const interval = setInterval(() => {
            if (["checking", "downloading", "verifying"].includes(updateState) && !busy) {
                void refreshStatus(true);
            }
        }, 15_000);
        return () => clearInterval(interval);
    });
</script>

<section class="settings-section" aria-labelledby="product-update-title">
    <header>
        <span class="section-mark" aria-hidden="true"><ArrowDownToLine size={16} /></span>
        <div>
            <h2 id="product-update-title">{m.product_updates_title()}</h2>
            <p>{m.product_updates_description()}</p>
        </div>
    </header>

    <div class="update-row">
        <div class="update-details">
            <div class="version-label">{m.product_updates_installed()}</div>
            <strong class="version-number">{installedVersion ?? "—"}</strong>
            <p role="status" aria-live="polite">
                {#if updateState === "checking"}
                    {m.product_updates_checking()}
                {:else if updateState === "available"}
                    <ShieldCheck size={14} aria-hidden="true" />
                    {m.product_updates_available({ version: releaseVersion ?? "—" })}
                {:else if updateState === "downloading"}
                    {m.product_updates_downloading({ version: releaseVersion ?? "—" })}
                {:else if updateState === "verifying"}
                    {m.product_updates_verifying({ version: releaseVersion ?? "—" })}
                {:else if updateState === "downloaded"}
                    <ShieldCheck size={14} aria-hidden="true" />
                    {m.product_updates_downloaded({ version: releaseVersion ?? "—" })}
                {:else if updateState === "upToDate"}
                    {m.product_updates_current()}
                {:else if updateState === "disabled"}
                    {m.product_updates_disabled()}
                {:else}
                    {m.product_updates_error()}
                {/if}
            </p>
            {#if updateState === "available" || updateState === "downloaded"}
                <p class="install-notice">{m.product_updates_not_installable()}</p>
            {/if}
        </div>

        <button
            class="refresh-button"
            type="button"
            disabled={busy}
            onclick={() => void refreshStatus()}
            aria-label={m.product_updates_retry()}
        >
            <RefreshCw size={15} aria-hidden="true" class={busy ? "spinning" : undefined} />
            <span>{m.product_updates_retry()}</span>
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
        display: flex; justify-content: space-between; align-items: flex-start;
        gap: 16px; border: 1px solid var(--border); border-radius: 5px;
        background: var(--card); padding: 16px;
    }
    .update-details { min-width: 0; }
    .version-label { color: var(--muted-foreground); font-size: 12px; }
    .version-number { font-family: "IBM Plex Mono", monospace; font-size: 18px; font-weight: 600; }
    .update-details p {
        display: flex; gap: 6px; align-items: center; margin: 8px 0 0;
        color: var(--muted-foreground); font-size: 12px; line-height: 1.4;
    }
    .update-details .install-notice { color: var(--foreground); }
    .refresh-button {
        display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center;
        gap: 8px; border: 1px solid var(--border); border-radius: 4px;
        background: transparent; padding: 9px 12px; color: var(--foreground);
        font-size: 12px; cursor: pointer;
    }
    .refresh-button:hover:not(:disabled) { background: var(--accent); }
    .refresh-button:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
    .refresh-button:disabled { cursor: wait; opacity: .65; }
    :global(.spinning) { animation: spin 1.4s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg); } }
    @media (prefers-reduced-motion: reduce) {
        :global(.spinning) { animation: none; }
    }
    @media (max-width: 500px) { .update-row { flex-direction: column; } }
</style>
