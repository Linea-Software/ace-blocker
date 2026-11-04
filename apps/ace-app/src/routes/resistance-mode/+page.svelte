<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import {
        getAllScripts,
        getResistanceModeSettings,
        setResistanceModeEnabled,
        setResistanceModeLock,
        setResistanceModeType,
        setScriptEnabled,
        downloadResistanceScripts,
        type ScriptInfo,
        type ResistanceModeSettings,
    } from "$engine-client";
    import { engineStore } from "$lib/stores/engine.svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import UpgradeToProButton from "$lib/components/UpgradeToProButton.svelte";
    import { LockKeyhole } from "@lucide/svelte";
    import { toast } from "svelte-sonner";
    import StatusCard from "./_components/StatusCard.svelte";
    import ResistanceMeter, { type ResistanceLevel } from "$lib/components/ace/domain/ResistanceMeter.svelte";
    import ScriptSections from "./_components/ScriptSections.svelte";
    import TimeLockDialog from "./_components/TimeLockDialog.svelte";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";
    import { onMount, untrack } from "svelte";

    // ─── State ───────────────────────────────────────────────────────
    let settings = $state<ResistanceModeSettings | null>(null);
    let scripts = $state<ScriptInfo[]>([]);
    let loading = $state(true);
    let loadError = $state(false);
    let mounted = $state(false);
    let isMutating = $state(false);
    let isSyncing = $state(false);
    let lockDialogOpen = $state(false);
    let pageVisible = $state(true);

    // Stale-response guard: each refresh bumps this; results from an older
    // request are dropped so a slow response can never overwrite newer state.
    // Deliberately NOT reactive — making it $state would make the mount
    // effect re-run on every increment (effect reads and writes the same
    // piece of state → effect_update_depth_exceeded).
    let refreshSeq = 0;

    const isPro = $derived(userStore.isPro);
    const engineOnline = $derived(engineStore.isAlive);
    const isLocked = $derived(settings?.isLocked ?? false);
    const controlsDisabled = $derived(!engineOnline || !isPro);

    // ─── Data loading ────────────────────────────────────────────────
    async function refresh() {
        const seq = ++refreshSeq;
        if (!engineOnline || !mounted) return;
        // Polling an already-loaded view should not flash a loading state.
        if (!settings) loading = true;
        loadError = false;
        try {
            const [scriptsRes, settingsRes] = await Promise.all([
                getAllScripts(),
                getResistanceModeSettings(),
            ]);
            if (seq !== refreshSeq) return;
            if (scriptsRes.error || settingsRes.error || !scriptsRes.data || !settingsRes.data) {
                throw new Error("Resistance data unavailable");
            }
            scripts = scriptsRes.data.scripts;
            settings = settingsRes.data;
        } catch {
            if (seq !== refreshSeq) return;
            scripts = [];
            settings = null;
            loadError = true;
        } finally {
            if (seq === refreshSeq) loading = false;
        }
    }

    // Reachability changes invalidate pending reads and reload on reconnect.
    $effect(() => {
        if (!mounted) return;
        if (engineOnline) {
            untrack(() => void refresh());
        } else {
            ++refreshSeq;
            loading = false;
            loadError = false;
            settings = null;
            scripts = [];
        }
    });

    // Poll while visible. The engine store owns reachability detection, so the
    // interval simply skips work while offline and resumes automatically.
    onMount(() => {
        mounted = true;
        pageVisible = document.visibilityState === "visible";
        const onVisibility = () => {
            pageVisible = document.visibilityState === "visible";
            if (pageVisible && engineStore.isAlive && !loadError) void refresh();
        };
        document.addEventListener("visibilitychange", onVisibility);

        const id = setInterval(() => {
            if (pageVisible && engineStore.isAlive && !loading && !loadError) void refresh();
        }, 5000);

        return () => {
            mounted = false;
            ++refreshSeq;
            document.removeEventListener("visibilitychange", onVisibility);
            clearInterval(id);
        };
    });

    // ─── Error mapping (stable engine codes, see engine docs) ────────
    function errorCode(error: unknown): string | undefined {
        if (error && typeof error === "object") {
            const candidate = (error as Record<string, unknown>).code;
            if (typeof candidate === "string") return candidate;
        }
        return undefined;
    }

    function showMutationError(error: { code: string; status: number }) {
        const code = error.code;
        if (code === "resistance_locked") {
            toast.error(m.locked_one_way_notice());
        } else if (code === "unsupported_lock_state") {
            toast.error(m.error_lock_state_unsupported());
        } else if (code === "script_download_failed") {
            toast.error(m.failed_to_download_scripts());
        } else if (code === "scripts_unavailable") {
            toast.error(m.error_scripts_unavailable());
        } else if (code === "pro_plan_required" || code === "pro_status_check_failed") {
            toast.error(m.status_pro_required());
        } else if (code === "invalid_request_body") {
            toast.error(m.error_invalid_request());
        } else if (error.status === 401 || code === "capability_required") {
            toast.error(m.error_unexpected());
        } else if (!engineOnline) {
            toast.error(m.error_engine_offline_action());
        } else {
            toast.error(m.error_unexpected());
        }
    }

    // ─── Mutations ───────────────────────────────────────────────────
    async function handleToggleEnabled(enabled: boolean) {
        if (isMutating) return; // prevent duplicated mutations from rapid clicks
        isMutating = true;
        const { error } = await setResistanceModeEnabled({
            body: { enabled },
        });
        isMutating = false;
        if (error) {
            showMutationError(error);
            return;
        }
        await refresh();
        toast.success(
            enabled ? m.resistance_mode_active() : m.resistance_mode_inactive(),
        );
    }

    async function handleSelectResistanceLevel(level: ResistanceLevel) {
        if (isMutating || isLocked || !settings?.enabled) return;
        isMutating = true;

        const { error } = await setResistanceModeType({
            body: { type: "static", intensity: level },
        });
        isMutating = false;
        if (error) {
            showMutationError(error);
            return;
        }

        await refresh();
        toast.success(m.domain_policies_updated({ domain: m.intensity_level() }));
    }

    async function handleToggleScript(script: ScriptInfo) {
        if (isLocked && script.isEnabled) {
            toast.error(m.locked_one_way_notice());
            return;
        }
        const next = !script.isEnabled;
        const original = script.isEnabled;
        // Optimistic update with rollback.
        scripts = scripts.map((s) =>
            s.id === script.id ? { ...s, isEnabled: next } : s,
        );
        const { error } = await setScriptEnabled({
            path: { script_id: script.id },
            body: { enabled: next },
        });
        if (error) {
            scripts = scripts.map((s) =>
                s.id === script.id ? { ...s, isEnabled: original } : s,
            );
            showMutationError(error);
            return;
        }
        toast.success(
            next
                ? m.script_activated({ name: script.name })
                : m.script_deactivated({ name: script.name }),
        );
        void refresh();
    }

    async function handleToggleDomain(domain: string, enabled: boolean) {
        if (isLocked) {
            toast.error(m.locked_one_way_notice());
            return;
        }
        const targets = scripts.filter(
            (s) => (s.domain || "Global") === domain && s.isEnabled !== enabled,
        );
        if (targets.length === 0) return;
        const original = new Map(targets.map((s) => [s.id, s.isEnabled]));
        scripts = scripts.map((s) =>
            (s.domain || "Global") === domain ? { ...s, isEnabled: enabled } : s,
        );
        const results = await Promise.all(
            targets.map(async (s) => {
                const { error } = await setScriptEnabled({
                    path: { script_id: s.id },
                    body: { enabled },
                });
                return { id: s.id, ok: !error };
            }),
        );
        const failed = results.filter((r) => !r.ok);
        if (failed.length > 0) {
            const failedIds = new Set(failed.map((f) => f.id));
            scripts = scripts.map((s) =>
                failedIds.has(s.id)
                    ? { ...s, isEnabled: original.get(s.id) ?? s.isEnabled }
                    : s,
            );
            toast.error(m.items_failed_to_update({ count: failed.length }));
        } else {
            toast.success(m.domain_policies_updated({ domain }));
        }
        void refresh();
    }

    async function handleSync() {
        if (isSyncing) return;
        isSyncing = true;
        const { error } = await downloadResistanceScripts();
        isSyncing = false;
        if (error) {
            showMutationError(error);
            return;
        }
        toast.success(m.scripts_sync_success());
        await refresh();
    }

    async function handleCommitLock(isoEnd: string) {
        if (isMutating) return;
        isMutating = true;
        const { error } = await setResistanceModeLock({
            body: { lockState: "active_until_lock_end", lockEnd: isoEnd },
        });
        isMutating = false;
        if (error) {
            showMutationError(error);
            return;
        }
        lockDialogOpen = false;
        toast.success(m.engine_locked_success());
        await refresh();
    }
</script>

<svelte:head>
    <title>{m.resistance_mode()} · {m.app_name()}</title>
</svelte:head>

<PageLayout>
    {#snippet header()}
        <PageHeader
            title={m.resistance_mode()}
            subtitle={m.resistance_mode_beta_description()}
        >
            <span data-testid="rm-beta-badge" class="beta-note">
                {m.beta_badge()}
            </span>
            {#if engineOnline && !loadError && isPro && settings?.enabled && !isLocked}
                <button
                    type="button"
                    class="lock-action"
                    data-testid="rm-open-lock-dialog"
                    onclick={() => (lockDialogOpen = true)}
                    title={m.time_lock_description()}
                >
                    <LockKeyhole size={16} aria-hidden="true" />
                    {m.time_lock_title()}
                </button>
            {/if}
        </PageHeader>
    {/snippet}

    {#if !engineOnline}
        <div class="resistance-stack">
            <StatusCard
                {settings}
                {engineOnline}
                {isPro}
                enabled={settings?.enabled ?? false}
                locked={isLocked}
                controlsDisabled={true}
                inFlight={isMutating}
                onToggle={handleToggleEnabled}
            />
        </div>
    {:else if loadError || !settings}
        <section class="load-state" aria-busy={loading} data-testid="rm-load-state">
            <div role="status" aria-live="polite">
                <h2>{loadError ? m.resistance_load_error_title() : m.resistance_loading()}</h2>
                {#if loadError}
                    <p>{m.resistance_load_error_description()}</p>
                {/if}
            </div>
            {#if loadError}
                <button type="button" class="retry-action" onclick={() => void refresh()} disabled={loading}>
                    {m.retry()}
                </button>
            {/if}
        </section>
    {:else}
        <div class="resistance-stack" aria-busy={loading}>
            {#if loading}
                <p class="refresh-status" role="status">{m.resistance_loading()}</p>
            {/if}
            {#if !isPro}
                <div class="pro-gate" data-testid="rm-pro-gate">
                    <div class="pro-gate-copy">
                        <h2>
                            <LockKeyhole size={17} aria-hidden="true" />
                            {m.resistance_pro_preview_title()}
                        </h2>
                        <p>{m.resistance_pro_preview_description()}</p>
                    </div>
                    <UpgradeToProButton showIcon={false} />
                </div>
            {/if}

            <StatusCard
                {settings}
                {engineOnline}
                {isPro}
                enabled={isPro && (settings?.enabled ?? false)}
                locked={isPro && isLocked}
                controlsDisabled={controlsDisabled}
                inFlight={isMutating}
                onToggle={handleToggleEnabled}
            />

            <ResistanceMeter
                enabled={isPro && (settings?.enabled ?? false)}
                currentIntensity={isPro ? (settings?.currentIntensity ?? "none") : "none"}
                locked={isPro && isLocked}
                disabled={controlsDisabled || !settings || !settings.enabled}
                proRequired={!isPro}
                inFlight={isMutating}
                onSelect={handleSelectResistanceLevel}
            />

            <div class="coverage-ledger">
                {#if !isPro}
                    <section class="coverage-locked" data-testid="rm-coverage-locked">
                        <h2>{m.covered_sites_title()}</h2>
                        <p>{m.covered_sites_hint()}</p>
                        <div class="coverage-locked-note">
                            <LockKeyhole size={14} aria-hidden="true" />
                            <span>{m.resistance_coverage_pro_locked()}</span>
                        </div>
                    </section>
                {:else}
                    <ScriptSections
                        {scripts}
                        {settings}
                        {engineOnline}
                        {isPro}
                        locked={isLocked}
                        syncing={isSyncing}
                        onToggleScript={handleToggleScript}
                        onToggleDomain={handleToggleDomain}
                        onSync={handleSync}
                    />
                {/if}
            </div>
        </div>
    {/if}

    <TimeLockDialog
        bind:open={lockDialogOpen}
        isLocked={isLocked}
        lockEndRaw={settings?.lockEnd ?? null}
        disabled={!engineOnline || loadError || !settings}
        committing={isMutating}
        onCommit={handleCommitLock}
    />
</PageLayout>

<style>
    .load-state {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        padding: 18px;
        border: 1px solid var(--border);
        border-radius: 6px;
        background: var(--card);
    }

    .load-state h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 19px;
        font-weight: 600;
    }

    .load-state p,
    .refresh-status {
        margin: 5px 0 0;
        color: var(--muted-foreground);
        font-size: 13px;
        line-height: 1.5;
    }

    .retry-action {
        min-height: 40px;
        padding: 0 14px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--background);
        color: var(--foreground);
        font-size: 13px;
        cursor: pointer;
    }

    .retry-action:hover:not(:disabled) {
        background: var(--muted);
    }

    .retry-action:focus-visible {
        outline: 2px solid var(--foreground);
        outline-offset: 3px;
    }

    .beta-note {
        color: var(--muted-foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 12px;
        font-weight: 500;
    }

    .lock-action {
        min-height: 40px;
        display: inline-flex;
        align-items: center;
        gap: 8px;
        border: 1px solid var(--ace-signal-deep);
        border-radius: 4px;
        padding: 0 14px;
        background: var(--ace-signal);
        color: var(--ace-ink);
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        transition:
            background 140ms ease,
            border-color 140ms ease;
    }

    .lock-action:hover:not(:disabled) {
        background: var(--ace-signal-bright);
        border-color: var(--ace-signal);
    }

    .lock-action:disabled {
        cursor: not-allowed;
        opacity: 0.4;
    }

    .resistance-stack {
        display: grid;
        gap: 16px;
    }

    .pro-gate {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 18px;
        padding: 1px 2px 16px;
        border-bottom: 1px solid var(--border);
    }

    .pro-gate-copy {
        min-width: 0;
    }

    .pro-gate h2,
    .coverage-locked h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 19px;
        font-weight: 600;
        line-height: 1.2;
    }

    .pro-gate h2 {
        display: flex;
        align-items: center;
        gap: 9px;
    }

    .pro-gate h2 :global(svg) {
        flex: 0 0 auto;
        color: var(--muted-foreground);
    }

    .pro-gate p,
    .coverage-locked p {
        margin: 5px 0 0;
        color: var(--muted-foreground);
        font-size: 13px;
        line-height: 1.5;
    }

    .coverage-ledger {
        padding-top: 7px;
        border-top: 1px solid var(--border);
    }

    .coverage-locked-note {
        display: flex;
        align-items: center;
        gap: 9px;
        margin-top: 13px;
        color: var(--muted-foreground);
        font-size: 13px;
    }

    @container page (max-width: 560px) {
        .lock-action {
            align-self: flex-start;
        }

        .pro-gate {
            align-items: flex-start;
            flex-direction: column;
            gap: 12px;
        }
    }
</style>
