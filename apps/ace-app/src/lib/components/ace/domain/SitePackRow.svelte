<script lang="ts">
    import type { SitePackSchema } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";
    import { SvelteDate } from "svelte/reactivity";
    import { getLocale } from "$lib/paraglide/runtime";
    import { getBlockingListDisplayName } from "$lib/utils/category-presentation";
    import {
        Check,
        ChevronRight,
        CircleAlert,
        LoaderCircle,
        LockKeyhole,
    } from "@lucide/svelte";

    let {
        pack,
        subtitle,
        selected = false,
        isPro,
        status,
        progress = 0,
        phase,
        onSelect,
    }: {
        pack: SitePackSchema;
        subtitle: string;
        selected?: boolean;
        isPro: boolean;
        status?: "queued" | "downloading" | "completed" | "failed";
        progress?: number;
        phase?: string | null;
        onSelect: (pack: SitePackSchema) => void;
    } = $props();

    const displayName = $derived(getBlockingListDisplayName(pack));
    const proRequired = $derived(pack.requiresPro && !isPro);

    const stateLabel = $derived.by(() => {
        if (proRequired) return m.status_pro_required();
        if (pack.isLocked) return m.locked();
        if (status === "failed") return m.pack_sync_failed_title();
        if (status === "completed") return m.pack_sync_completed();
        if (status === "queued") return m.pack_sync_queued();
        if (status === "downloading") return phase ?? m.syncing();
        return pack.isSynced ? m.downloaded() : m.not_downloaded();
    });
</script>

<button
    type="button"
    class:selected
    class:locked={pack.isLocked}
    class:pro-required={proRequired}
    class:failed={status === "failed"}
    class:syncing={status === "queued" || status === "downloading"}
    class="site-pack-row"
    data-testid="site-pack-row"
    data-sync-status={status ?? "idle"}
    aria-pressed={selected}
    title={pack.isLocked ? m.pack_locked_tooltip() : undefined}
    onclick={() => onSelect(pack)}
>
    <span class="state-rail" aria-hidden="true"></span>

    <span class="identity">
        <strong>{displayName}</strong>
        <span class="metadata">
            {#if pack.requiresPro}
                <span>{m.pro_badge()}</span>
            {:else}
                <span>{m.pack_access_free()}</span>
            {/if}
            {#if pack.isBundled}
                <span aria-hidden="true">·</span>
                <span>{m.pack_included_badge()}</span>
            {/if}
            {#if subtitle}
                <span aria-hidden="true">·</span>
                <span class="subtitle">{subtitle}</span>
            {/if}
        </span>
    </span>

    <span class="fact">
        <span>{m.domains_lowercase()}</span>
        <strong>
            {pack.isSynced
                ? (pack.entriesCount?.toLocaleString(getLocale()) ?? "—")
                : "—"}
        </strong>
    </span>

    <span class="fact">
        <span>{m.updated_label()}</span>
        <strong>
            {pack.lastUpdatedAt
                ? new SvelteDate(pack.lastUpdatedAt).toLocaleDateString(getLocale())
                : "—"}
        </strong>
    </span>

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
        {#if status === "downloading"}
            <span class="progress-copy">{Math.round(progress)}%</span>
        {/if}
    </span>

    <ChevronRight size={16} aria-hidden="true" />

    {#if status === "queued" || status === "downloading"}
        <span
            class="progress-track"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={Math.round(progress)}
            aria-label={m.pack_sync_progress_aria({ name: pack.name })}
        >
            <span style:width={String(Math.round(progress)) + "%"}></span>
        </span>
    {/if}
</button>

<style>
    .site-pack-row {
        position: relative;
        width: 100%;
        min-height: 68px;
        display: grid;
        grid-template-columns: 6px minmax(150px, 1.5fr) minmax(72px, 0.45fr) minmax(88px, 0.55fr) minmax(96px, 0.65fr) auto;
        align-items: center;
        gap: 10px;
        overflow: hidden;
        border: 0;
        border-bottom: 1px solid var(--border);
        padding: 0 14px 0 0;
        background: var(--card);
        color: var(--foreground);
        text-align: left;
        cursor: pointer;
        transition: background 140ms ease;
    }

    .site-pack-row:hover,
    .site-pack-row.selected {
        background: color-mix(in srgb, var(--muted) 52%, var(--card));
    }

    .site-pack-row.selected {
        box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--foreground) 22%, transparent);
    }

    .state-rail {
        align-self: stretch;
        background: var(--ace-ash);
    }

    .site-pack-row.locked .state-rail {
        background: var(--ace-signal);
    }

    .site-pack-row.failed .state-rail {
        background: var(--ace-danger);
    }

    .site-pack-row.syncing .state-rail {
        background: var(--ace-information);
    }

    .identity {
        min-width: 0;
        display: block;
    }

    .identity > strong {
        display: block;
        overflow: hidden;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 16px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .metadata {
        display: flex;
        min-width: 0;
        align-items: center;
        gap: 5px;
        margin-top: 4px;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.25;
    }

    .subtitle {
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .fact {
        min-width: 0;
        display: block;
    }

    .fact > span,
    .fact > strong {
        display: block;
    }

    .fact > span {
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .fact > strong {
        margin-top: 3px;
        overflow: hidden;
        font-family: "IBM Plex Mono", monospace;
        font-size: 12px;
        font-weight: 500;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .state {
        min-width: 0;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 6px;
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .locked .state,
    .pro-required .state {
        color: var(--ace-signal-deep);
    }

    .failed .state {
        color: var(--ace-danger);
    }

    .syncing .state {
        color: var(--ace-information);
    }

    .progress-copy {
        font-family: "IBM Plex Mono", monospace;
    }

    .progress-track {
        position: absolute;
        right: 0;
        bottom: 0;
        left: 6px;
        height: 2px;
        background: color-mix(in srgb, var(--ace-information) 15%, transparent);
    }

    .progress-track > span {
        display: block;
        height: 100%;
        background: var(--ace-information);
        transition: width 180ms ease;
    }

    .spinner {
        display: inline-flex;
        animation: spin 900ms linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @container page (max-width: 560px) {
        .site-pack-row {
            grid-template-columns: 6px minmax(0, 1fr) minmax(100px, auto) auto;
        }

        .fact {
            display: none;
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .spinner {
            animation: none;
        }
    }
</style>
