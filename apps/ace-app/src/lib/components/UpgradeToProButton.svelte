<script lang="ts">
    import { openProPage } from "$engine-client";
    import { ArrowUpRight, Crown } from "@lucide/svelte";
    import { cn } from "$lib/utils";
    import * as m from "$lib/paraglide/messages.js";

    let {
        class: className = "",
        text = m.get_pro(),
        showIcon = true,
    }: {
        class?: string;
        text?: string;
        showIcon?: boolean;
    } = $props();

    let opening = $state(false);

    async function handleUpgradeClick() {
        if (opening) return;
        opening = true;
        try {
            await openProPage();
        } finally {
            opening = false;
        }
    }
</script>

<button
    type="button"
    class={cn("upgrade-action", className)}
    onclick={handleUpgradeClick}
    disabled={opening}
    aria-busy={opening}
>
    {#if showIcon}
        <Crown size={16} strokeWidth={2} aria-hidden="true" />
    {/if}
    <span class="upgrade-label">{text}</span>
    <span class="upgrade-mark" aria-hidden="true">
        <ArrowUpRight size={17} strokeWidth={2.2} />
    </span>
</button>

<style>
    .upgrade-action {
        min-height: 42px;
        max-width: 100%;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 9px;
        padding: 6px 7px 6px 14px;
        border: 1px solid var(--foreground);
        border-radius: 5px;
        background: var(--foreground);
        color: var(--background);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 700;
        line-height: 1.2;
        white-space: nowrap;
        cursor: pointer;
        transition: background-color 150ms ease, border-color 150ms ease;
    }

    .upgrade-action:hover:not(:disabled) {
        background: color-mix(in srgb, var(--foreground) 86%, var(--background));
        border-color: color-mix(in srgb, var(--foreground) 86%, var(--background));
    }

    .upgrade-action:active:not(:disabled) {
        background: color-mix(in srgb, var(--foreground) 78%, var(--background));
    }

    .upgrade-action:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 3px;
    }

    .upgrade-action:disabled {
        opacity: 0.7;
        cursor: wait;
    }

    .upgrade-label {
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .upgrade-mark {
        flex: 0 0 28px;
        width: 28px;
        height: 28px;
        display: grid;
        place-items: center;
        border-radius: 3px;
        background: color-mix(in srgb, var(--background) 14%, transparent);
    }

    @media (prefers-reduced-motion: reduce) {
        .upgrade-action {
            transition: none;
        }
    }
</style>
