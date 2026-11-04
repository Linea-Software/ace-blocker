<script lang="ts">
    import { openAccountPage } from "$lib/utils/account-browser";
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
</script>

<button
    type="button"
    onclick={() => {
        if (!isPro) {
            void openAccountPage("pro");
            return;
        }
        onclick?.();
    }}
    class={cn(
        "inline-flex min-h-9 min-w-[200px] w-fit flex-wrap items-center justify-center gap-2 rounded-sm border border-border bg-transparent px-3 py-2 text-sm font-medium text-foreground transition-colors hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring motion-reduce:transition-none",
        !isPro && "cursor-pointer",
        className,
    )}
>
    <span class="inline-flex items-center justify-center gap-2">
        {#if Icon}<Icon class="h-4 w-4 shrink-0" aria-hidden="true" />{/if}
        {label}
    </span>

    {#if !isPro}
        <span class="inline-flex items-center gap-1.5 border-l border-border pl-2 text-xs text-muted-foreground">
            <Crown class="h-4 w-4 shrink-0" aria-hidden="true" />
            {m.get_pro()}
            <ExternalLink class="h-3.5 w-3.5 opacity-60" aria-hidden="true" />
        </span>
    {/if}
</button>
