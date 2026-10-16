<script lang="ts">
    import type { Snippet } from "svelte";
    import { cn } from "$lib/utils";

    let {
        title,
        class: className,
        children,
    }: {
        /** Optional uppercase section label at the top of the card. */
        title?: string;
        class?: string;
        children: Snippet;
    } = $props();
</script>

<div class={cn("rounded-2xl border border-border bg-card p-6", className)}>
    {#if title}
        <p
            class="text-xs font-bold uppercase tracking-widest text-muted-foreground"
        >
            {title}
        </p>
    {/if}
    {@render children()}
</div>
