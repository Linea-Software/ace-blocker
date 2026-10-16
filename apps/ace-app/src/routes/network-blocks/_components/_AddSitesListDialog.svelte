<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Badge } from "$lib/components/ui/badge/index.js";
    import {
        Check,
        Globe,
        CloudDownload,
        CircleCheck,
        Crown,
        ExternalLink,
        Search,
    } from "@lucide/svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { cn } from "$lib/utils.js";
    import ProBadge from "$lib/components/ProBadge.svelte";
    import { openGetProPage } from "$lib/utils.js";
    import type { SitePackSchema } from "$engine-client";

    let {
        open = $bindable(false),
        availableLists = [],
        selectedListIds = [],
        isFreeUser = false,
        onAdd = (_listId: string) => {},
        onCancel = () => {},
    }: {
        open: boolean;
        availableLists: SitePackSchema[];
        selectedListIds: string[];
        isFreeUser?: boolean;
        onAdd: (listId: string) => void;
        onCancel: () => void;
    } = $props();

    let searchQuery = $state("");
    let localOpen = $state(false);
    let selectedListId = $state<string | null>(null);

    $effect(() => {
        if (open) {
            localOpen = true;
            searchQuery = "";
            selectedListId = null;
        }
    });

    const sortedLists = $derived(
        [...availableLists].sort((a, b) => {
            return a.requiresPro === b.requiresPro ? 0 : a.requiresPro ? 1 : -1;
        }),
    );

    const filteredLists = $derived(
        searchQuery.trim()
            ? sortedLists.filter((l) =>
                  l.name.toLowerCase().includes(searchQuery.toLowerCase()),
              )
            : sortedLists,
    );

    const connectedIds = $derived(new Set(selectedListIds));

    function handleListClick(list: SitePackSchema) {
        if (list.requiresPro && isFreeUser) {
            openGetProPage();
            return;
        }
        if (connectedIds.has(list.id)) return;
        selectedListId = selectedListId === list.id ? null : list.id;
    }

    function handleOpenChange(next: boolean) {
        if (!next) handleCancel();
    }

    function handleCancel() {
        selectedListId = null;
        localOpen = false;
        open = false;
        onCancel();
    }

    function handleConfirm() {
        if (!selectedListId) return;
        onAdd(selectedListId);
        selectedListId = null;
        localOpen = false;
        open = false;
    }
</script>

<Dialog.Root bind:open={localOpen} onOpenChange={handleOpenChange}>
    <Dialog.Content
        class="max-h-[90dvh] rounded-md border-border bg-background shadow-none sm:max-w-[min(480px,calc(100vw-2rem))]"
    >
        <Dialog.Header>
            <Dialog.Title class="font-condensed text-xl font-semibold">
                {m.add_site_pack_dialog_title()}
            </Dialog.Title>
            <Dialog.Description class="text-sm text-muted-foreground">
                {m.add_site_pack_dialog_description()}
            </Dialog.Description>
        </Dialog.Header>

        <div class="space-y-4 py-4">
            <!-- Search -->
            <div class="relative">
                <Search
                    class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground/50"
                />
                <Input
                    placeholder={m.search_lists_placeholder()}
                    bind:value={searchQuery}
                    class="pl-9 bg-card"
                />
            </div>

            <!-- List -->
            <div class="max-h-[320px] overflow-y-auto space-y-1 -mx-1 px-1">
                {#if filteredLists.length === 0}
                    <p class="text-sm text-muted-foreground text-center py-8">
                        {m.no_lists_found()}
                    </p>
                {:else}
                    <p
                        class="px-2 pb-1 font-mono text-[11px] font-medium text-muted-foreground"
                    >
                        {m.available_lists()}
                    </p>
                    {#each filteredLists as list (list.id)}
                        {@const isConnected = connectedIds.has(list.id)}
                        {@const isProGated = list.requiresPro && isFreeUser}
                        {@const isSelected = selectedListId === list.id}

                        <button
                            type="button"
                            class={cn(
                                "w-full flex items-center px-3 py-2.5 rounded-sm border text-left transition-colors duration-150",
                                "bg-card",
                                isConnected &&
                                    "opacity-60 cursor-default border-border/30",
                                !isConnected &&
                                    !isProGated &&
                                    "border-border/50 hover:border-border cursor-pointer",
                                isSelected &&
                                    "border-foreground/35 bg-muted/45",
                                isProGated &&
                                    "cursor-pointer border-border opacity-50 hover:border-foreground/30",
                            )}
                            onclick={() => handleListClick(list)}
                            disabled={isConnected}
                        >
                            <div class="flex items-center w-full gap-3 min-w-0">
                                <!-- Check indicator: shows for connected OR selected -->
                                <div
                                    class={cn(
                                        "h-4 w-4 shrink-0 rounded-sm border flex items-center justify-center transition-all duration-200",
                                        isSelected
                                            ? "border-foreground bg-foreground text-background"
                                            : isConnected
                                              ? "bg-primary/10 border-primary/40 text-primary"
                                              : "border-border",
                                    )}
                                >
                                    {#if isSelected || isConnected}
                                        <Check class="h-3 w-3" />
                                    {/if}
                                </div>
                                <div class="flex flex-col flex-1 min-w-0">
                                    <div class="flex items-center gap-2">
                                        <span
                                            class="truncate font-semibold text-sm"
                                        >
                                            {list.name}
                                        </span>
                                        {#if isProGated}
                                            <ProBadge />
                                        {/if}
                                        {#if isConnected}
                                            <Badge
                                                variant="secondary"
                                                class="text-xs h-5 px-1.5 uppercase text-muted-foreground border-none shadow-none bg-muted/50 shrink-0"
                                            >
                                                {m.add_site_pack_dialog_already_connected()}
                                            </Badge>
                                        {/if}
                                    </div>
                                    <div
                                        class="flex items-center justify-between gap-3 mt-1"
                                    >
                                        {#if list.description}
                                            <span
                                                class="text-xs text-muted-foreground truncate max-w-[240px]"
                                            >
                                                {list.description}
                                            </span>
                                        {/if}
                                        <div
                                            class="flex items-center gap-3 shrink-0 ml-auto"
                                        >
                                            {#if !list.isSynced}
                                                <Badge
                                                    variant="secondary"
                                                    class="text-xs h-5 px-2 uppercase text-muted-foreground border-none shadow-none bg-muted/50"
                                                >
                                                    {m.not_downloaded()}
                                                </Badge>
                                                <CloudDownload
                                                    class="w-3.5 h-3.5 text-muted-foreground/40"
                                                />
                                            {:else}
                                                <CircleCheck
                                                    class="w-3.5 h-3.5 text-emerald-500/70"
                                                />
                                            {/if}

                                            {#if list.entriesCount}
                                                <span
                                                    class="text-xs font-mono text-muted-foreground/60"
                                                >
                                                    {new Intl.NumberFormat(
                                                        "en-US",
                                                    ).format(list.entriesCount)}
                                                    {m.domains_lowercase()}
                                                </span>
                                            {/if}
                                        </div>
                                    </div>
                                </div>

                                {#if isProGated}
                                    <div
                                        class="flex items-center gap-1.5 shrink-0 ml-2"
                                    >
                                        <Crown
                                            class="h-3.5 w-3.5 text-muted-foreground"
                                        />
                                        <span
                                            class="text-xs font-semibold text-foreground"
                                        >
                                            {m.get_pro()}
                                        </span>
                                        <ExternalLink
                                            class="h-3.5 w-3.5 text-muted-foreground opacity-70"
                                        />
                                    </div>
                                {/if}
                            </div>
                        </button>
                    {/each}
                {/if}
            </div>
        </div>

        <Dialog.Footer>
            <Button variant="ghost" onclick={handleCancel}>
                {m.cancel()}
            </Button>
            <Button
                onclick={handleConfirm}
                disabled={!selectedListId}
                class="rounded-sm px-6 font-semibold shadow-none"
            >
                {m.add_site_pack_confirm()}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
