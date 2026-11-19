<script lang="ts">
    import type { SitePackSchema } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";
    import { getLocale } from "$lib/paraglide/runtime";
    import { getBlockingListDisplayName } from "$lib/utils/category-presentation";
    import { getPackIcon } from "$lib/utils/pack-icons";
    import { getSitePackCoverage, getSitePackIconSlug } from "$lib/utils/site-pack-presentation";
    import { Check, ChevronRight, CircleAlert, LoaderCircle, LockKeyhole } from "@lucide/svelte";

    let {
        pack,
        subtitle = "",
        selected = false,
        variantOnly = false,
        isPro,
        status,
        progress = 0,
        phase,
        onSelect,
    }: {
        pack: SitePackSchema;
        subtitle?: string;
        selected?: boolean;
        variantOnly?: boolean;
        isPro: boolean;
        status?: "queued" | "downloading" | "completed" | "failed";
        progress?: number;
        phase?: string | null;
        onSelect: (pack: SitePackSchema) => void;
    } = $props();

    const PackIcon = $derived(getPackIcon(getSitePackIconSlug(pack)));
    const coverage = $derived(getSitePackCoverage(pack));
    const proRequired = $derived(pack.requiresPro && !isPro);
    const name = $derived(
        variantOnly
            ? (pack.requiresPro ? m.catalog_pro_extended() : m.catalog_free_essential())
            : getBlockingListDisplayName(pack),
    );
    const stateLabel = $derived.by(() => {
        if (proRequired) return m.status_pro_required();
        if (pack.isLocked) return m.locked();
        if (status === "failed") return m.pack_sync_failed_title();
        if (status === "completed") return m.pack_sync_completed();
        if (status === "queued") return m.pack_sync_queued();
        if (status === "downloading") return phase ?? m.syncing();
        return null;
    });
</script>

<button
    type="button"
    class:selected
    class:locked={pack.isLocked}
    class:pro-required={proRequired}
    class:failed={status === "failed"}
    class:syncing={status === "queued" || status === "downloading"}
    class:variant-only={variantOnly}
    class="site-pack-row"
    data-testid="site-pack-row"
    data-sync-status={status ?? "idle"}
    aria-pressed={selected}
    title={pack.isLocked ? m.pack_locked_tooltip() : undefined}
    onclick={() => onSelect(pack)}
>
    <span class="state-rail" aria-hidden="true"></span>
    {#if !variantOnly}
        <span class="pack-icon" data-testid="site-pack-row-icon" aria-hidden="true">
            <PackIcon size={18} strokeWidth={1.75} />
        </span>
    {/if}
    <span class="identity">
        <span class="identity-heading">
            <strong>{name}</strong>
            {#if !variantOnly && !proRequired}
                <span class="tier-label">{pack.requiresPro ? m.pro_badge() : m.pack_access_free()}</span>
            {/if}
        </span>
        {#if subtitle && !variantOnly && !pack.isRemote}
            <span class="subtitle">{subtitle}</span>
        {/if}
    </span>
    <span class="coverage" data-testid="site-pack-row-coverage">
        {#if coverage.count !== null}
            {coverage.count.toLocaleString(getLocale())} {m.domains_lowercase()}
        {:else}
            —
        {/if}
    </span>
    {#if stateLabel}
        <span class="state">
            {#if proRequired || pack.isLocked}
                <LockKeyhole size={14} aria-hidden="true" />
            {:else if status === "failed"}
                <CircleAlert size={14} aria-hidden="true" />
            {:else if status === "completed"}
                <Check size={14} aria-hidden="true" />
            {:else if status === "queued" || status === "downloading"}
                <span class="spinner"><LoaderCircle size={14} aria-hidden="true" /></span>
            {/if}
            <span>{stateLabel}</span>
            {#if status === "downloading"}<span>{Math.round(progress)}%</span>{/if}
        </span>
    {/if}
    <span class="chevron"><ChevronRight size={16} aria-hidden="true" /></span>
    {#if status === "queued" || status === "downloading"}
        <span
            class="progress-track"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(progress)}
            aria-label={m.pack_sync_progress_aria({ name: pack.name })}
        ><span style:width={Math.round(progress) + "%"}></span></span>
    {/if}
</button>

<style>
    .site-pack-row {
        position: relative;
        display: flex;
        width: 100%;
        min-height: 58px;
        align-items: center;
        gap: 12px;
        border: 0;
        border-bottom: 1px solid var(--border);
        padding: 9px 14px 9px 0;
        background: var(--card);
        color: var(--foreground);
        text-align: left;
        cursor: pointer;
        transition: background 140ms ease;
    }
    .site-pack-row:hover, .site-pack-row.selected {
        background: color-mix(in srgb, var(--muted) 52%, var(--card));
    }
    .site-pack-row:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: -2px;
    }
    .state-rail {
        align-self: stretch;
        flex: 0 0 4px;
        margin-block: -9px;
        background: var(--ace-ash);
    }
    .locked .state-rail, .pro-required .state-rail { background: var(--ace-signal); }
    .failed .state-rail { background: var(--ace-danger); }
    .syncing .state-rail { background: var(--ace-information); }
    .pack-icon {
        display: grid;
        flex: 0 0 34px;
        width: 34px;
        height: 34px;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--muted);
    }
    .identity { display: flex; flex: 1; min-width: 0; flex-direction: column; gap: 3px; }
    .identity-heading { display: flex; align-items: center; gap: 9px; min-width: 0; }
    .identity-heading strong {
        overflow: hidden;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .tier-label {
        flex: 0 0 auto;
        color: var(--muted-foreground);
        font: 500 11px "IBM Plex Mono", monospace;
    }
    .subtitle { overflow: hidden; color: var(--muted-foreground); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
    .coverage { flex: 0 0 auto; color: var(--muted-foreground); font: 500 12px "IBM Plex Mono", monospace; white-space: nowrap; }
    .state { display: flex; align-items: center; gap: 5px; color: var(--muted-foreground); font-size: 11px; white-space: nowrap; }
    .locked .state, .pro-required .state { color: var(--ace-signal-deep); }
    .failed .state { color: var(--ace-danger); }
    .chevron { flex: 0 0 auto; color: var(--muted-foreground); }
    .variant-only { min-height: 52px; padding-left: 18px; }
    .variant-only .state-rail { background: transparent; }
    .progress-track { position: absolute; bottom: 0; left: 4px; right: 0; height: 2px; background: var(--muted); }
    .progress-track > span { display: block; height: 100%; background: var(--ace-information); }
    .spinner { animation: spin 900ms linear infinite; }
    @keyframes spin { to { transform: rotate(360deg); } }
    @media (max-width: 650px) {
        .site-pack-row { flex-wrap: wrap; gap: 6px 10px; }
        .identity { flex-basis: calc(100% - 105px); }
        .coverage { margin-left: 48px; }
        .variant-only .coverage { margin-left: 0; }
        .state { margin-left: auto; }
        .chevron { margin-left: auto; }
    }
    @media (prefers-reduced-motion: reduce) {
        .site-pack-row { transition: none; }
        .spinner { animation: none; }
    }
</style>
