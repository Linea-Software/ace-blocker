<script lang="ts">
    import { openProPage } from "$engine-client";
    import { Button } from "$lib/components/ui/button";
    import { Crown, ExternalLink } from "@lucide/svelte";
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

    async function handleUpgradeClick() {
        await openProPage();
    }
</script>

<Button
    variant="outline"
    onclick={handleUpgradeClick}
    class={cn(
        "h-10 shrink-0 rounded-sm border-border bg-transparent px-4 font-semibold text-foreground shadow-none transition-colors hover:bg-muted",
        className,
    )}
>
    {#if showIcon}
        <Crown class="mr-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
    {/if}
    {text}
    <ExternalLink class="ml-2 h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
</Button>
