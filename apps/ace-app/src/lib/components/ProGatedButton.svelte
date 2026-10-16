<script lang="ts">
    import { openProPage } from "$engine-client";
    import { Crown, ExternalLink } from "@lucide/svelte";
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

    let hover = $state(false);
</script>

<button
    type="button"
    onclick={() => {
        if (!isPro) {
            void openProPage();
            return;
        }
        onclick?.();
    }}
    onmouseenter={() => {
        if (!isPro) hover = true;
    }}
    onmouseleave={() => {
        if (!isPro) hover = false;
    }}
    class={cn(
        "relative h-9 min-w-[200px] w-fit overflow-hidden rounded-sm border border-border bg-transparent text-sm font-medium text-foreground transition-colors hover:bg-muted",
        !isPro && "cursor-pointer",
        className,
    )}
>
    <span
        class={cn(
            "absolute inset-0 flex items-center justify-center gap-2 px-4 whitespace-nowrap transition-opacity duration-150",
            !isPro && hover && "pointer-events-none opacity-0",
        )}
    >
        {#if Icon}<Icon class="h-4 w-4" aria-hidden="true" />{/if}
        {label}
    </span>

    {#if !isPro}
        <span
            class={cn(
                "absolute inset-0 flex items-center justify-center gap-2 bg-foreground px-4 font-semibold text-background whitespace-nowrap transition-opacity duration-150",
                !hover && "pointer-events-none opacity-0",
            )}
        >
            <Crown class="h-4 w-4" aria-hidden="true" />
            {m.get_pro()}
            <ExternalLink class="h-3.5 w-3.5 opacity-60" aria-hidden="true" />
        </span>
    {/if}
</button>
