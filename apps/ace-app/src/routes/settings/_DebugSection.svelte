<script lang="ts">
    import { onMount } from "svelte";
    import { Bug, RefreshCcw, ShieldCheck } from "@lucide/svelte";
    import { toast } from "svelte-sonner";
    import * as m from "$lib/paraglide/messages.js";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { getDebugProPolicy, resetTos, setDebugProPolicy, type DebugProPolicy } from "$engine-client";

    let selectedPolicy = $state<DebugProPolicy>("Default");
    let loadingPolicy = $state(true);
    let savingPolicy = $state(false);
    let resettingTos = $state(false);

    function isPolicy(value: string): value is DebugProPolicy {
        return value === "Default" || value === "Free" || value === "Pro";
    }

    onMount(() => {
        void (async () => {
            try {
                const { data, error } = await getDebugProPolicy();
                if (error || !data) throw error ?? new Error("Missing debug policy");
                selectedPolicy = data;
            } catch {
                toast.error(m.debug_license_load_failed());
            } finally {
                loadingPolicy = false;
            }
        })();
    });

    async function changePolicy(value: string) {
        if (!isPolicy(value) || savingPolicy || loadingPolicy || value === selectedPolicy) return;

        const previous = selectedPolicy;
        savingPolicy = true;
        try {
            const { error } = await setDebugProPolicy({ path: { policy: value } });
            if (error) throw error;
            selectedPolicy = value;
            await userStore.fetch();
            toast.success(m.debug_license_changed());
        } catch {
            selectedPolicy = previous;
            toast.error(m.debug_license_change_failed());
        } finally {
            savingPolicy = false;
        }
    }

    async function replayTos() {
        if (resettingTos) return;
        resettingTos = true;
        try {
            const { error } = await resetTos();
            if (error) throw error;
            toast.success(m.debug_legal_reset_success());
            // The engine has persisted the reset. Immediately close the legal
            // gate so the non-dismissible modal appears without an app restart.
            userStore.tosAccepted = false;
        } catch {
            toast.error(m.debug_legal_reset_failed());
        } finally {
            resettingTos = false;
        }
    }
</script>

<section class="settings-section">
    <header>
        <span class="section-mark" aria-hidden="true"><Bug size={16} /></span>
        <div>
            <h2>{m.debug_settings_title()}</h2>
            <p>{m.debug_settings_description()}</p>
        </div>
    </header>

    <div class="setting-row">
        <div class="setting-copy">
            <label for="debug-license-policy">{m.debug_license_label()}</label>
            <p>{m.debug_license_description()}</p>
        </div>
        <select
            id="debug-license-policy"
            value={selectedPolicy}
            disabled={loadingPolicy || savingPolicy}
            onchange={(event) => void changePolicy(event.currentTarget.value)}
        >
            <option value="Default">{m.debug_license_default()}</option>
            <option value="Free">{m.debug_license_free()}</option>
            <option value="Pro">{m.debug_license_pro()}</option>
        </select>
    </div>

    <div class="setting-row">
        <div class="setting-copy">
            <strong>{m.debug_legal_label()}</strong>
            <p>{m.debug_legal_description()}</p>
        </div>
        <button type="button" disabled={resettingTos} onclick={replayTos}>
            {#if resettingTos}
                <RefreshCcw size={15} class="animate-spin" aria-hidden="true" />
            {:else}
                <ShieldCheck size={15} aria-hidden="true" />
            {/if}
            {m.debug_legal_reset_button()}
        </button>
    </div>
</section>

<style>
    .settings-section {
        display: grid;
        gap: 12px;
    }

    header {
        display: grid;
        grid-template-columns: 28px minmax(0, 1fr);
        gap: 8px;
        align-items: start;
    }

    .section-mark {
        width: 24px;
        height: 24px;
        display: grid;
        place-items: center;
        border-left: 3px solid var(--ace-signal);
        color: var(--muted-foreground);
    }

    h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
        font-weight: 600;
    }

    header p,
    .setting-copy p {
        margin: 3px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .setting-row {
        min-height: 72px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 24px;
        padding: 13px 14px;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .setting-copy {
        min-width: 0;
    }

    .setting-copy label,
    .setting-copy strong {
        display: block;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 14px;
        font-weight: 600;
        color: var(--foreground);
    }

    select,
    button {
        min-height: 38px;
        flex-shrink: 0;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--background);
        color: var(--foreground);
        padding: 0 12px;
        font-size: 12px;
        font-weight: 600;
    }

    button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        cursor: pointer;
    }

    select:focus-visible,
    button:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 2px;
    }

    button:hover:not(:disabled) {
        background: var(--muted);
    }

    select:disabled,
    button:disabled {
        cursor: wait;
        opacity: 0.55;
    }

    @container page (max-width: 560px) {
        .setting-row {
            align-items: stretch;
            flex-direction: column;
            gap: 12px;
        }

        select,
        button {
            width: 100%;
        }
    }
</style>
