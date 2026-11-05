<script lang="ts">
    import * as Popover from "$lib/components/ui/popover";
    import { X } from "@lucide/svelte";
    import * as m from "$lib/paraglide/messages.js";
    import {
        LIST_ICONS_GROUPED,
        FALLBACK_ICON,
        getPackIcon,
    } from "$lib/utils/pack-icons";

    let {
        selectedIcon = $bindable(null),
        open = $bindable(false),
        fallbackIcon = FALLBACK_ICON,
        label = "",
        size = "md",
        side = "bottom",
    }: {
        selectedIcon: string | null;
        open: boolean;
        fallbackIcon?: typeof FALLBACK_ICON;
        label?: string;
        size?: "sm" | "md";
        side?: "bottom" | "left";
    } = $props();

    let searchQuery = $state("");

    const allSlugs = $derived(LIST_ICONS_GROUPED.flatMap((g) => g.slugs));
    const filteredSlugs = $derived(
        searchQuery
            ? allSlugs.filter((slug) =>
                  slug.includes(searchQuery.toLowerCase()),
              )
            : allSlugs,
    );

    const triggerSize = $derived(size === "sm" ? "size-8" : "size-9");
    const gridCols = $derived(size === "sm" ? "grid-cols-8" : "grid-cols-7");
    const itemSize = $derived(size === "sm" ? "size-7" : "size-8");
</script>

<Popover.Root bind:open>
    <Popover.Trigger
        class="{triggerSize} shrink-0 rounded-sm border border-border bg-card flex items-center justify-center text-muted-foreground hover:border-primary/40 hover:text-foreground hover:bg-muted/50 transition-colors"
        aria-label={label || m.choose_icon()}
    >
        {@const PreviewIcon = selectedIcon
            ? getPackIcon(selectedIcon)
            : fallbackIcon}
        <PreviewIcon size={size === "sm" ? 16 : 18} />
    </Popover.Trigger>
    <Popover.Portal>
        <Popover.Content
            {side}
            align="start"
            sideOffset={10}
            collisionPadding={12}
            class="z-[80] w-72 max-h-[min(320px,calc(100dvh-32px))] overflow-y-auto rounded-sm border-border bg-card p-3 shadow-none"
        >
            <div class="mb-3">
                <input
                    type="text"
                    placeholder={m.search_icons()}
                    bind:value={searchQuery}
                    class="w-full h-8 px-2.5 text-sm rounded-md bg-muted border border-border/50 focus:outline-none focus:ring-1 focus:ring-amber-500/40"
                />
            </div>

            <div class="space-y-3">
                <!-- None button -->
                <button
                    type="button"
                    class="{itemSize} rounded-md flex items-center justify-center {selectedIcon ===
                    null
                        ? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 ring-1 ring-amber-500/40'
                        : 'bg-transparent hover:bg-muted text-muted-foreground hover:text-foreground'}"
                    onclick={() => {
                        selectedIcon = null;
                        open = false;
                    }}
                    aria-label={m.none_label()}
                >
                    <X size={size === "sm" ? 14 : 16} />
                </button>

                {#if searchQuery}
                    <div class="grid {gridCols} gap-1.5">
                        {#each filteredSlugs as slug (slug)}
                            {@const Icon = getPackIcon(slug)}
                            <button
                                type="button"
                                class="{itemSize} rounded-md flex items-center justify-center {selectedIcon ===
                                slug
                                    ? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 ring-1 ring-amber-500/40'
                                    : 'bg-transparent hover:bg-muted text-muted-foreground hover:text-foreground'}"
                                onclick={() => {
                                    selectedIcon = slug;
                                    open = false;
                                }}
                                aria-label={slug}
                            >
                                <Icon size={size === "sm" ? 14 : 16} />
                            </button>
                        {/each}
                    </div>
                {:else}
                    {#each LIST_ICONS_GROUPED as group (group.category)}
                        <div>
                            <p
                                class="text-xs font-semibold text-muted-foreground mb-1.5"
                            >
                                {group.category}
                            </p>
                            <div class="grid {gridCols} gap-1.5">
                                {#each group.slugs as slug (slug)}
                                    {@const Icon = getPackIcon(slug)}
                                    <button
                                        type="button"
                                        class="{itemSize} rounded-md flex items-center justify-center {selectedIcon ===
                                        slug
                                            ? 'bg-amber-500/15 text-amber-600 dark:text-amber-400 ring-1 ring-amber-500/40'
                                            : 'bg-transparent hover:bg-muted text-muted-foreground hover:text-foreground'}"
                                        onclick={() => {
                                            selectedIcon = slug;
                                            open = false;
                                        }}
                                        aria-label={slug}
                                    >
                                        <Icon size={size === "sm" ? 14 : 16} />
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/each}
                {/if}
            </div>
        </Popover.Content>
    </Popover.Portal>
</Popover.Root>
