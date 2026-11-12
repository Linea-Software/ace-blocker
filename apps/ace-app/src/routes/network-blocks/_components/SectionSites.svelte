<script lang="ts">
    import { Badge } from "$lib/components/ui/badge/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Search, Check } from "@lucide/svelte";
    import * as m from "$lib/paraglide/messages.js";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { Progress } from "$lib/components/ui/progress/index.js";
    import { cn } from "$lib/utils.js";
    import {
        resyncSitePack,
        getAllSitePacks,
        type SitePackSchema,
    } from "$engine-client";
    import { syncStore } from "$lib/stores/syncing-store.svelte";
    import { invalidateAll } from "$app/navigation";
    import type { SitePacksConnectionDelta } from "$lib/deltas/site-packs-connection-delta.svelte.ts";
    import { userStore } from "$lib/stores/user-store.svelte";
    import {
        getBlockingListDisplayName,
        isAceCategory,
        requiresProForConnection,
    } from "$lib/utils/category-presentation";

    let {
        delta,
        availableLists = $bindable([]),
        isLocked = false,
    }: {
        delta: SitePacksConnectionDelta;
        availableLists: SitePackSchema[];
        isLocked?: boolean;
    } = $props();

    const CATEGORY_PREVIEW_LIMIT = 5;
    let searchQuery = $state("");
    let showAllCategories = $state(false);
    let showAllProCategories = $state(false);

    let downloadModalOpen = $state(false);
    let listToDownload = $state<SitePackSchema | null>(null);
    let downloadStatus = $state<"idle" | "downloading" | "error">("idle");
    let downloadProgress = $derived(
        listToDownload ? (syncStore.getProgress(listToDownload.id) ?? 0) : 0,
    );

    let connectedListIds = $derived(delta.currentConnectedListIds);
    let isFreeUser = $derived(!userStore.isPro);

    let sortedAllLists = $derived(
        [...availableLists].sort((a, b) => {
            const kindOrder = Number(isAceCategory(b)) - Number(isAceCategory(a));
            if (kindOrder !== 0) return kindOrder;
            if (requiresProForConnection(a) !== requiresProForConnection(b))
                return requiresProForConnection(a) ? 1 : -1;
            return getBlockingListDisplayName(a).localeCompare(
                getBlockingListDisplayName(b),
            );
        }),
    );

    let selectedLists = $derived(sortedAllLists.filter((item) => connectedListIds.includes(item.id)));
    let filteredLists = $derived(
        searchQuery.trim()
            ? sortedAllLists.filter((list) =>
                  getBlockingListDisplayName(list)
                      .toLowerCase()
                      .includes(searchQuery.trim().toLowerCase()),
              )
            : sortedAllLists,
    );
    let filteredCategories = $derived(filteredLists.filter(isAceCategory));
    // Keep the existing Free-user Pro catalog behind its disclosure.
    // On Pro accounts, all available categories share the same preview.
    let primaryCategories = $derived(
        filteredCategories.filter(
            (item) => !requiresProForConnection(item) || !isFreeUser || connectedListIds.includes(item.id),
        ),
    );
    let gatedProCategories = $derived(
        isFreeUser
            ? filteredCategories.filter(
                  (item) => requiresProForConnection(item) && !connectedListIds.includes(item.id),
              )
            : [],
    );
    const isSearching = $derived(searchQuery.trim().length > 0);
    let visibleCategories = $derived(
        isSearching || showAllCategories
            ? primaryCategories
            : primaryCategories.filter(
                  (item, index) => index < CATEGORY_PREVIEW_LIMIT || connectedListIds.includes(item.id),
              ),
    );
    let visibleProCategories = $derived(
        isSearching || showAllProCategories
            ? gatedProCategories
            : gatedProCategories.slice(0, CATEGORY_PREVIEW_LIMIT),
    );
    let filteredCustomLists = $derived(
        filteredLists.filter((list) => !isAceCategory(list)),
    );

    async function confirmDownload() {
        if (!listToDownload) return;
        downloadStatus = "downloading";
        try {
            const result = await resyncSitePack({
                path: { site_pack_id: listToDownload.id },
                    onStreamError: (err: unknown) => {
                    throw err;
                },
            });
            const outcome = await syncStore.handleSyncStream(
                listToDownload.id,
                result.stream,
            );
            if (outcome === "failed") {
                downloadStatus = "error";
                return;
            }

            const res = await getAllSitePacks();
            if (res.data) {
                availableLists = res.data.sitePacks;
            }

            await invalidateAll();
            const freshList = availableLists.find(
                (list) => list.id === listToDownload!.id,
            );
            if (freshList) {
                delta.addConnection(freshList.id);
            }
            window.setTimeout(() => {
                if (syncStore.statusOf(listToDownload!.id) === "completed") {
                    syncStore.clear(listToDownload!.id);
                }
            }, 4000);
            downloadModalOpen = false;
            listToDownload = null;
        } catch (error) {
            console.error("Failed to download list:", error);
            downloadStatus = "error";
        }
    }

    function toggleList(list: SitePackSchema) {
        if (requiresProForConnection(list) && isFreeUser && !delta.isConnected(list.id)) return;

        if (!list.isSynced) {
            listToDownload = list;
            downloadStatus = "idle";
            downloadModalOpen = true;
            return;
        }

        const listId = list.id;
        const isConnected = delta.isConnected(listId);
        const isInitiallyConnected = delta.isInitialConnection(listId);

        if (isConnected) {
            if (isLocked && isInitiallyConnected) return;
            if (delta.isNewConnection(listId)) {
                delta.removeTempConnection(listId);
            } else {
                delta.removeConnection(listId);
            }
        } else {
            delta.addConnection(listId);
        }
    }
</script>

{#snippet listRow(list: SitePackSchema)}
    {@const isConnected = connectedListIds.includes(list.id)}
    {@const isPremium = requiresProForConnection(list)}
    {@const isProGated = isPremium && isFreeUser && !isConnected}
    {@const canToggle =
        !isProGated &&
        (!isConnected || !(isLocked && delta.isInitialConnection(list.id)))}

    <button
        type="button"
        class={cn(
            "bg-card border-border/50 flex w-full items-center rounded-sm border px-3 py-2.5 text-left transition-colors",
            canToggle && "hover:border-border cursor-pointer",
            !canToggle && "cursor-default opacity-55",
            isConnected && "border-foreground/30 bg-muted/45",
        )}
        onclick={() => {
            if (isProGated) {
                import("$lib/utils.js").then(({ openGetProPage }) =>
                    openGetProPage(),
                );
                return;
            }
            if (!canToggle) return;
            toggleList(list);
        }}
        aria-pressed={isConnected}
    >
        <span class="flex min-w-0 flex-1 items-center gap-3">
            <span
                class={cn(
                    "border-border flex size-4 shrink-0 items-center justify-center rounded-sm border",
                    isConnected &&
                        "bg-foreground text-background border-foreground",
                )}
                aria-hidden="true"
            >
                {#if isConnected}
                    <Check class="size-3" />
                {/if}
            </span>

            <span class="min-w-0 flex-1">
                <span class="flex min-w-0 items-center gap-2">
                    <span class="truncate text-sm font-semibold">
                        {getBlockingListDisplayName(list)}
                    </span>
                    {#if isPremium}
                        <Badge
                            variant="outline"
                            class="border-border h-4 shrink-0 rounded-sm bg-transparent px-1.5 font-mono text-[11px] text-muted-foreground"
                        >
                            {m.pro_badge()}
                        </Badge>
                    {:else if isAceCategory(list)}
                        <Badge variant="outline"
                            class="border-border h-4 shrink-0 rounded-sm bg-transparent px-1.5 font-mono text-[11px] text-muted-foreground">
                            {m.pack_access_free()}
                        </Badge>
                    {/if}
                </span>

                {#if isConnected && isPremium && isFreeUser}
                    <span class="text-xs text-muted-foreground">{m.status_pro_required()}</span>
                {/if}
                {#if list.description || list.entriesCount || !list.isSynced}
                    <span
                        class="text-muted-foreground mt-0.5 flex min-w-0 items-center gap-2 text-xs"
                    >
                        {#if list.description}
                            <span class="min-w-0 truncate">
                                {list.description}
                            </span>
                        {/if}
                        {#if list.entriesCount}
                            <span class="shrink-0 font-mono text-[11px]">
                                {new Intl.NumberFormat("en-US").format(
                                    list.entriesCount,
                                )}
                                {m.domains_lowercase()}
                            </span>
                        {/if}
                        {#if !list.isSynced}
                            <Badge
                                variant="secondary"
                                class="bg-muted/50 h-4 shrink-0 rounded-sm border-none px-1.5 text-[10px] text-muted-foreground shadow-none"
                            >
                                {m.not_downloaded()}
                            </Badge>
                        {/if}
                    </span>
                {/if}
            </span>
        </span>
    </button>
{/snippet}

<div class="space-y-4" id="sites">
    <section class="space-y-3" aria-labelledby="categories-heading">
        <div class="flex flex-wrap items-end justify-between gap-2">
            <div>
                <h3 id="categories-heading" class="font-condensed text-lg font-semibold text-foreground">
                    {m.site_packs()}
                </h3>
                <p class="text-muted-foreground mt-1 text-[13px]">{m.categories_description()}</p>
            </div>
            <span class="text-muted-foreground text-[13px]">
                {m.block_wizard_category_count({count:selectedLists.filter(isAceCategory).length})}
            </span>
        </div>
        <div class="relative">
            <Search class="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2" aria-hidden="true" />
            <Input
                placeholder={m.search_lists_placeholder()}
                bind:value={searchQuery}
                class="bg-card rounded-sm pl-9"
                aria-label={m.search_lists_placeholder()}
            />
        </div>
        {#if filteredCategories.length === 0}
            <p class="text-muted-foreground py-3 text-sm">{m.no_lists_found()}</p>
        {:else}
            {#if visibleCategories.length > 0}
                <div class="space-y-1.5" data-testid="category-preview">
                    {#each visibleCategories as list (list.id)}
                        {@render listRow(list)}
                    {/each}
                </div>
                {#if !isSearching && primaryCategories.length > CATEGORY_PREVIEW_LIMIT && (showAllCategories || visibleCategories.length < primaryCategories.length)}
                    <button
                        type="button"
                        class="category-visibility-toggle"
                        aria-expanded={showAllCategories}
                        onclick={() => (showAllCategories = !showAllCategories)}
                    >
                        {showAllCategories
                            ? m.block_categories_show_less()
                            : m.block_categories_show_all({ count: primaryCategories.length })}
                    </button>
                {/if}
            {/if}
            {#if gatedProCategories.length > 0}
                {#if isSearching}
                    <div class="space-y-1.5" data-testid="pro-category-search-results">
                        {#each gatedProCategories as list (list.id)}
                            {@render listRow(list)}
                        {/each}
                    </div>
                {:else}
                    <details class="pro-categories">
                        <summary>{m.pro_badge()} · {m.block_categories_toggle()}</summary>
                        <div class="space-y-1.5 pt-3" data-testid="pro-category-preview">
                            {#each visibleProCategories as list (list.id)}
                                {@render listRow(list)}
                            {/each}
                        </div>
                        {#if gatedProCategories.length > CATEGORY_PREVIEW_LIMIT}
                            <button
                                type="button"
                                class="category-visibility-toggle"
                                aria-expanded={showAllProCategories}
                                onclick={() => (showAllProCategories = !showAllProCategories)}
                            >
                                {showAllProCategories
                                    ? m.block_categories_show_less()
                                    : m.block_categories_show_all({ count: gatedProCategories.length })}
                            </button>
                        {/if}
                    </details>
                {/if}
            {/if}
        {/if}
    </section>
    {#if filteredCustomLists.length > 0}
        <section class="border-border space-y-3 border-t pt-5" aria-labelledby="custom-lists-heading">
            <div>
                <h3 id="custom-lists-heading" class="font-condensed text-base font-semibold text-foreground">{m.custom_lists()}</h3>
                <p class="text-muted-foreground mt-1 text-[13px]">{m.custom_lists_description()}</p>
            </div>
            <div class="space-y-1.5">
                {#each filteredCustomLists as list (list.id)}
                    {@render listRow(list)}
                {/each}
            </div>
        </section>
    {/if}
</div>

<AlertDialog.Root
    bind:open={downloadModalOpen}
    onOpenChange={(v) => {
        if (!v) {
            if (downloadStatus !== "downloading") {
                downloadModalOpen = false;
                listToDownload = null;
            }
        }
    }}
>
    <AlertDialog.Content
        class="rounded-md border-border bg-card shadow-none sm:max-w-md"
    >
        <AlertDialog.Header class="gap-3">
            <AlertDialog.Title
                class={cn(
                    "font-condensed text-xl font-semibold",
                    downloadStatus === "error" && "text-destructive",
                )}
            >
                {#if downloadStatus === "idle"}
                    {m.download_required_title()}
                {:else}
                    {downloadStatus === "downloading"
                        ? m.downloading_pack_title()
                        : m.download_failed_title()}
                {/if}
            </AlertDialog.Title>
            <AlertDialog.Description class="text-sm leading-relaxed">
                {#if downloadStatus === "idle"}
                    {m.download_required_description({
                        name: listToDownload?.name || "",
                    })}
                {:else if downloadStatus === "downloading"}
                    {m.downloading_pack_description()}
                {:else}
                    {m.download_failed_description()}
                    {#if listToDownload && syncStore.errorOf(listToDownload.id)}
                        <span class="mt-1 block text-sm text-muted-foreground">
                            {syncStore.errorOf(listToDownload.id)}
                        </span>
                    {/if}
                {/if}
            </AlertDialog.Description>
        </AlertDialog.Header>

        {#if downloadStatus === "downloading"}
            <div class="space-y-3 py-2">
                <div
                    class="text-muted-foreground flex items-center justify-between text-xs font-medium"
                >
                    <span>
                        {m.downloading_pack({
                            name: listToDownload?.name || "",
                        })}
                    </span>
                    <span class="text-foreground font-mono">
                        {Math.round(downloadProgress)}%
                    </span>
                </div>
                <Progress value={downloadProgress} class="bg-muted h-2" />
            </div>
        {/if}

        <AlertDialog.Footer
            class={cn(
                "border-border/60 mt-2 gap-2 border-t pt-4",
                downloadStatus === "downloading" && "hidden",
            )}
        >
            {#if downloadStatus === "idle"}
                <AlertDialog.Cancel
                    class="min-w-20"
                    onclick={() => (listToDownload = null)}
                >
                    {m.cancel()}
                </AlertDialog.Cancel>
                <AlertDialog.Action class="min-w-24" onclick={confirmDownload}>
                    {m.download_now()}
                </AlertDialog.Action>
            {:else if downloadStatus === "error"}
                <AlertDialog.Cancel
                    class="min-w-20"
                    onclick={() => {
                        downloadModalOpen = false;
                        listToDownload = null;
                    }}
                >
                    {m.close()}
                </AlertDialog.Cancel>
                <AlertDialog.Action class="min-w-24" onclick={confirmDownload}>
                    {m.retry()}
                </AlertDialog.Action>
            {/if}
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>


<style>
    .category-visibility-toggle {
        display: flex;
        width: 100%;
        min-height: 38px;
        align-items: center;
        justify-content: center;
        margin-top: 8px;
        padding: 8px 12px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--card);
        color: var(--foreground);
        cursor: pointer;
        font-size: 12px;
        font-weight: 600;
    }
    .category-visibility-toggle:hover {
        background: var(--muted);
    }
    .category-visibility-toggle:focus-visible {
        outline: 2px solid var(--ring);
        outline-offset: 2px;
    }
    .pro-categories {
        border-top: 1px solid var(--border);
        padding-top: 12px;
        margin-top: 10px;
    }
    .pro-categories summary {
        font-size: 13px;
        color: var(--muted-foreground);
        cursor: pointer;
    }
</style>
