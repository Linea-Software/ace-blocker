<script lang="ts">
    import { openAccountPage } from "$lib/utils/account-browser";
    import { LockKeyhole } from "@lucide/svelte";
    import { cn } from "$lib/utils";
    import * as m from "$lib/paraglide/messages.js";
    import type { Component } from "svelte";

    let {
        isPro,
        label,
        icon: Icon,
        class: className = "",
        onclick,
    }: {
        isPro: boolean;
        label: string;
        icon?: Component;
        class?: string;
        onclick?: () => void;
    } = $props();

    let opening = $state(false);
    const gated = $derived(!isPro);

    async function handleClick() {
        if (opening) return;
        if (!gated) {
            onclick?.();
            return;
        }

        opening = true;
        try {
            await openAccountPage("pro");
        } finally {
            opening = false;
        }
    }
</script>

<button
    type="button"
    onclick={handleClick}
    disabled={opening}
    aria-busy={opening}
    aria-label={gated ? `${label} · ${m.get_pro()}` : label}
    class={cn("pro-action", gated && "pro-action--gated", className)}
>
    {#if Icon}
        <Icon class="action-icon" size={16} aria-hidden="true" />
    {/if}
    <span class="action-label">
        <span class="current-label">{label}</span>
        {#if gated}
            <span class="upgrade-label" aria-hidden="true">{m.get_pro()}</span>
        {/if}
    </span>
    {#if gated}
        <span class="pro-flag" aria-hidden="true">
            <LockKeyhole size={13} strokeWidth={2} />
            <span>{m.pro_badge()}</span>
        </span>
    {/if}
</button>

<style>
    .pro-action {
        display: inline-flex;
        flex: 0 1 auto;
        min-height: 38px;
        min-width: 0;
        max-width: 100%;
        align-items: center;
        justify-content: center;
        gap: 8px;
        padding: 7px 12px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--card);
        color: var(--foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 14px;
        font-weight: 600;
        line-height: 1.2;
        white-space: nowrap;
        cursor: pointer;
        transition: background-color 150ms ease, border-color 150ms ease, color 150ms ease;
    }

    .pro-action:hover:not(:disabled) {
        border-color: var(--foreground);
        background: var(--muted);
    }

    .pro-action--gated {
        padding-left: 10px;
        border-left: 3px solid var(--ace-signal);
        background: color-mix(in srgb, var(--ace-signal) 5%, var(--card));
    }

    .pro-action--gated:hover:not(:disabled) {
        border-color: var(--foreground);
        border-left-color: var(--ace-signal);
        background: var(--foreground);
        color: var(--background);
    }

    .pro-action:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 3px;
    }

    .pro-action:disabled {
        opacity: 0.7;
        cursor: wait;
    }

    .action-icon {
        flex-shrink: 0;
    }

    .action-label {
        display: grid;
        min-width: 0;
        align-items: center;
    }

    .current-label,
    .upgrade-label {
        grid-area: 1 / 1;
        min-width: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        transition: opacity 120ms ease;
    }

    .upgrade-label {
        opacity: 0;
        pointer-events: none;
    }

    .pro-action--gated:hover .current-label,
    .pro-action--gated:focus-visible .current-label {
        opacity: 0;
    }

    .pro-action--gated:hover .upgrade-label,
    .pro-action--gated:focus-visible .upgrade-label {
        opacity: 1;
    }

    .pro-flag {
        display: inline-flex;
        flex-shrink: 0;
        align-items: center;
        gap: 4px;
        padding-left: 8px;
        border-left: 1px solid currentColor;
        font-family: "IBM Plex Mono", monospace;
        font-size: 10px;
        font-weight: 600;
        opacity: 0.72;
    }

    @media (prefers-reduced-motion: reduce) {
        .pro-action,
        .current-label,
        .upgrade-label {
            transition: none;
        }
    }
</style>
