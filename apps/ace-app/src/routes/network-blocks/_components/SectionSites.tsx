import { createMemo, createSignal, For, Show } from "solid-js";
import "./SectionSites.css";
import { Dynamic } from "@solidjs/web";
import { Input } from "$lib/components/ui/input/solid";
import { Search, Check, LockKeyhole } from "@lucide/solid";
import { getPackIcon } from "../../site-packs/_pack-icons";
import {
    getSitePackCoverage,
    getSitePackIconSlug,
    groupSitePackCategories,
    preferredCategoryPack,
    extendedCategoryPack,
    type SitePackCategoryGroup,
} from "$lib/utils/site-pack-presentation";
import { getLocale } from "$lib/paraglide/runtime";
import * as m from "$lib/paraglide/messages.js";
import { Progress } from "$lib/components/ui/progress/solid";
import { cn, openGetProPage } from "$lib/utils";
import {
    resyncSitePack,
    getAllSitePacks,
    type SitePackSchema,
} from "$engine-client";
import { syncStore } from "$lib/stores/syncing-store";
import type { SitePacksConnectionDelta } from "$lib/deltas/site-packs-connection-delta";
import { userStore } from "$lib/stores/user-store";
import {
    getBlockingListDisplayName,
    isAceCategory,
    requiresProForConnection,
} from "$lib/utils/category-presentation";

export interface SectionSitesProps {
    delta: SitePacksConnectionDelta;
    availableLists: SitePackSchema[];
    onAvailableListsChange: (lists: SitePackSchema[]) => void;
    isLocked?: boolean;
}

const CATEGORY_PREVIEW_LIMIT = 5;

export default function SectionSites(props: SectionSitesProps) {
    const [searchQuery, setSearchQuery] = createSignal("");
    const [showAllCategories, setShowAllCategories] = createSignal(false);
    const [showAllProCategories, setShowAllProCategories] = createSignal(false);
    const [downloadModalOpen, setDownloadModalOpen] = createSignal(false);
    const [listToDownload, setListToDownload] =
        createSignal<SitePackSchema | null>(null);
    const [downloadStatus, setDownloadStatus] = createSignal<
        "idle" | "downloading" | "error"
    >("idle");
    const downloadProgress = () =>
        listToDownload()
            ? (syncStore.getProgress(listToDownload()!.id) ?? 0)
            : 0;
    const connectedListIds = () => props.delta.currentConnectedListIds;
    const isFreeUser = () => !userStore.isPro;

    const sortedAllLists = createMemo(() =>
        [...props.availableLists].sort((a, b) => {
            const kindOrder =
                Number(isAceCategory(b)) - Number(isAceCategory(a));
            if (kindOrder !== 0) return kindOrder;
            if (requiresProForConnection(a) !== requiresProForConnection(b)) {
                return requiresProForConnection(a) ? 1 : -1;
            }
            return getBlockingListDisplayName(a).localeCompare(
                getBlockingListDisplayName(b),
            );
        }),
    );
    const selectedLists = createMemo(() =>
        sortedAllLists().filter((item) => connectedListIds().includes(item.id)),
    );
    const filteredLists = createMemo(() =>
        searchQuery().trim()
            ? sortedAllLists().filter((list) =>
                  getBlockingListDisplayName(list)
                      .toLowerCase()
                      .includes(searchQuery().trim().toLowerCase()),
              )
            : sortedAllLists(),
    );
    const filteredCategories = createMemo(() =>
        filteredLists().filter(isAceCategory),
    );
    const categoryGroups = createMemo(() =>
        groupSitePackCategories(filteredCategories()),
    );
    const primaryCategories = createMemo(() =>
        categoryGroups().filter((group) =>
            group.packs.some(
                (pack) =>
                    !requiresProForConnection(pack) ||
                    !isFreeUser() ||
                    connectedListIds().includes(pack.id),
            ),
        ),
    );
    const gatedProCategories = createMemo(() =>
        categoryGroups().filter(
            (group) => !primaryCategories().includes(group),
        ),
    );
    const isSearching = () => searchQuery().trim().length > 0;
    const visibleCategories = createMemo(() =>
        isSearching() || showAllCategories()
            ? primaryCategories()
            : primaryCategories().filter(
                  (group, index) =>
                      index < CATEGORY_PREVIEW_LIMIT ||
                      group.packs.some((pack) =>
                          connectedListIds().includes(pack.id),
                      ),
              ),
    );
    const visibleProCategories = createMemo(() =>
        isSearching() || showAllProCategories()
            ? gatedProCategories()
            : gatedProCategories().slice(0, CATEGORY_PREVIEW_LIMIT),
    );
    const filteredCustomLists = createMemo(() =>
        filteredLists().filter((list) => !isAceCategory(list)),
    );

    function stageConnection(list: SitePackSchema) {
        if (requiresProForConnection(list) && isFreeUser()) return;
        props.delta.addConnection(list.id);
    }

    function removeConnection(list: SitePackSchema) {
        if (props.delta.isNewConnection(list.id))
            props.delta.removeTempConnection(list.id);
        else props.delta.removeConnection(list.id);
    }

    function toggleList(list: SitePackSchema) {
        if (
            requiresProForConnection(list) &&
            isFreeUser() &&
            !props.delta.isConnected(list.id)
        )
            return;
        if (!list.isSynced) {
            setListToDownload(list);
            setDownloadStatus("idle");
            setDownloadModalOpen(true);
            return;
        }
        if (props.delta.isConnected(list.id)) {
            if (props.isLocked && props.delta.isInitialConnection(list.id))
                return;
            removeConnection(list);
        } else stageConnection(list);
    }

    async function confirmDownload() {
        const list = listToDownload();
        if (!list) return;
        setDownloadStatus("downloading");
        try {
            const result = await resyncSitePack({
                path: { site_pack_id: list.id },
                onStreamError: (error: unknown) => {
                    throw error;
                },
            });
            if (!result.stream) throw new Error("Site pack stream unavailable");
            const outcome = await syncStore.handleSyncStream(
                list.id,
                result.stream,
            );
            if (outcome === "failed") {
                setDownloadStatus("error");
                return;
            }
            const res = await getAllSitePacks();
            if (!res.data)
                throw new Error("Failed to reload site packs after download");
            props.onAvailableListsChange(res.data.sitePacks);
            const freshList = res.data.sitePacks.find(
                (item) => item.id === list.id,
            );
            if (!freshList?.isSynced) throw new Error("Downloaded category is unavailable");
            if (requiresProForConnection(freshList) && isFreeUser()) throw new Error("Pro entitlement changed during download");
            stageConnection(freshList);
            window.setTimeout(() => {
                if (syncStore.statusOf(list.id) === "completed")
                    syncStore.clear(list.id);
            }, 4000);
            setDownloadModalOpen(false);
            setListToDownload(null);
        } catch (error) {
            console.error("Failed to download list:", error);
            setDownloadStatus("error");
        }
    }
    function closeDownload() {
        if (downloadStatus() === "downloading") return;
        setDownloadModalOpen(false);
        setListToDownload(null);
    }

    function ListRow(row: { list: SitePackSchema; group?: SitePackCategoryGroup }) {
        const members = () => row.group?.packs ?? [row.list];
        const connected = () => members().filter((pack) => props.delta.isConnected(pack.id));
        const isConnected = () => connected().length > 0;
        const current = () => connected().find((pack) => pack.requiresPro) ?? connected()[0] ?? row.list;
        const extended = () => row.group && extendedCategoryPack(row.group);
        const basic = () => row.group?.packs.find((pack) => !pack.requiresPro);
        const premium = () => requiresProForConnection(current());
        const proGated = () => premium() && isFreeUser() && !isConnected();
        const locked = () => !!props.isLocked && connected().some((pack) => props.delta.isInitialConnection(pack.id));
        const count = (pack: SitePackSchema) => getSitePackCoverage(pack).count?.toLocaleString(getLocale()) ?? "—";
        return (
            <button
                type="button"
                class={cn("category-choice flex w-full min-w-0 items-center gap-3 border border-border/50 bg-card px-3 py-2.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                    locked() ? "cursor-default" : "cursor-pointer hover:bg-muted/50",
                    isConnected() && "category-choice-selected")}
                data-testid={row.group ? "category-family-option" : "category-single-option"}
                aria-pressed={isConnected() ? "true" : "false"}
                aria-disabled={locked() ? "true" : undefined}
                onClick={() => {
                    if (locked()) return;
                    if (proGated()) { void openGetProPage(); return; }
                    if (isConnected()) {
                        for (const pack of connected()) removeConnection(pack);
                    } else toggleList(row.list);
                }}
            >
                <span class={cn("flex size-4 shrink-0 items-center justify-center rounded-sm border border-border",
                    isConnected() && "category-choice-check")} aria-hidden="true">
                    <Show when={isConnected()}><Check size={12} /></Show>
                    <Show when={proGated()}><LockKeyhole size={11} /></Show>
                </span>
                <span class="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-muted" aria-hidden="true" data-testid="category-icon">
                    <Dynamic component={getPackIcon(getSitePackIconSlug(current()))} size={19} strokeWidth={1.75} />
                </span>
                <span class="flex min-w-0 flex-1 flex-col gap-1">
                    <span class="text-sm font-semibold">{getBlockingListDisplayName(current())}</span>
                    <span class="text-[11px] text-muted-foreground">
                        {extended() ? current().requiresPro ? m.catalog_pro_extended() : m.catalog_free_essential() : premium() ? m.pro_badge() : m.pack_access_free()}
                        <Show when={isConnected() && premium() && isFreeUser()}> · {m.catalog_retained_coverage()}</Show>
                        <Show when={locked()}> · {m.locked()}</Show>
                    </span>
                </span>
                <span class="category-choice-coverage" data-testid="category-coverage">
                    <span>{count(current())} {m.domains_lowercase()}</span>
                    <Show when={extended() && !current().requiresPro}>
                        <span class="category-choice-pro" data-testid="category-tier-comparison">
                            <Show when={isFreeUser()}><LockKeyhole size={12} aria-hidden="true" /></Show>
                            {m.catalog_count_with_pro({ count: count(extended()!) })}
                        </span>
                    </Show>
                    <Show when={extended() && current().requiresPro && basic()}>
                        <span class="text-muted-foreground">{m.catalog_count_with_free({ count: count(basic()!) })}</span>
                    </Show>
                    <Show when={proGated()}><span class="category-choice-pro">{m.status_pro_required()}</span></Show>
                </span>
            </button>
        );
    }

    function CategoryRow(row: { group: SitePackCategoryGroup }) {
        return <ListRow list={preferredCategoryPack(row.group, !isFreeUser())} group={row.group} />;
    }

    return (
        <div class="space-y-4" id="sites">
            <section class="space-y-3" aria-labelledby="categories-heading">
                <div class="flex flex-wrap items-end justify-between gap-2">
                    <div>
                        <h3
                            id="categories-heading"
                            class="font-condensed text-lg font-semibold text-foreground"
                        >
                            {m.site_packs()}
                        </h3>
                        <p class="text-muted-foreground mt-1 text-[13px]">
                            {m.categories_description()}
                        </p>
                    </div>
                    <span class="text-muted-foreground text-[13px]">
                        {m.block_wizard_category_count({
                            count: groupSitePackCategories(selectedLists().filter(isAceCategory)).length,
                        })}
                    </span>
                </div>
                <div class="relative">
                    <Search
                        class="text-muted-foreground absolute top-1/2 left-3 size-4 -translate-y-1/2"
                        aria-hidden="true"
                    />
                    <Input
                        placeholder={m.search_lists_placeholder()}
                        value={searchQuery()}
                        onInput={(event) =>
                            setSearchQuery(event.currentTarget.value)
                        }
                        class="bg-card rounded-sm pl-9"
                        aria-label={m.search_lists_placeholder()}
                    />
                </div>
                <Show
                    when={filteredCategories().length > 0}
                    fallback={
                        <p class="py-3 text-sm text-muted-foreground">
                            {m.no_lists_found()}
                        </p>
                    }
                >
                    <p class="text-xs text-muted-foreground">
                        {m.catalog_counts_disclaimer()}
                    </p>
                    <Show when={visibleCategories().length > 0}>
                        <div class="space-y-1.5" data-testid="category-preview">
                            <For each={visibleCategories()}>
                                {(group) => <CategoryRow group={group} />}
                            </For>
                        </div>
                        <Show
                            when={
                                !isSearching() &&
                                primaryCategories().length >
                                    CATEGORY_PREVIEW_LIMIT &&
                                (showAllCategories() ||
                                    visibleCategories().length <
                                        primaryCategories().length)
                            }
                        >
                            <button
                                type="button"
                                class="category-visibility-toggle"
                                aria-expanded={
                                    showAllCategories() ? "true" : "false"
                                }
                                onClick={() =>
                                    setShowAllCategories(!showAllCategories())
                                }
                            >
                                {showAllCategories()
                                    ? m.block_categories_show_less()
                                    : m.block_categories_show_all({
                                          count: primaryCategories().length,
                                      })}
                            </button>
                        </Show>
                    </Show>
                    <Show when={gatedProCategories().length > 0}>
                        <Show
                            when={isSearching()}
                            fallback={
                                <details class="pro-categories">
                                    <summary>
                                        {m.pro_badge()} ·{" "}
                                        {m.block_categories_toggle()}
                                    </summary>
                                    <div
                                        class="space-y-1.5 pt-3"
                                        data-testid="pro-category-preview"
                                    >
                                        <For each={visibleProCategories()}>
                                            {(group) => (
                                                <CategoryRow group={group} />
                                            )}
                                        </For>
                                    </div>
                                    <Show
                                        when={
                                            gatedProCategories().length >
                                            CATEGORY_PREVIEW_LIMIT
                                        }
                                    >
                                        <button
                                            type="button"
                                            class="category-visibility-toggle"
                                            aria-expanded={
                                                showAllProCategories()
                                                    ? "true"
                                                    : "false"
                                            }
                                            onClick={() =>
                                                setShowAllProCategories(
                                                    !showAllProCategories(),
                                                )
                                            }
                                        >
                                            {showAllProCategories()
                                                ? m.block_categories_show_less()
                                                : m.block_categories_show_all({
                                                      count: gatedProCategories()
                                                          .length,
                                                  })}
                                        </button>
                                    </Show>
                                </details>
                            }
                        >
                            <div
                                class="space-y-1.5"
                                data-testid="pro-category-search-results"
                            >
                                <For each={gatedProCategories()}>
                                    {(group) => <CategoryRow group={group} />}
                                </For>
                            </div>
                        </Show>
                    </Show>
                </Show>
            </section>
            <Show when={filteredCustomLists().length > 0}>
                <section
                    class="border-border space-y-3 border-t pt-5"
                    aria-labelledby="custom-lists-heading"
                >
                    <div>
                        <h3
                            id="custom-lists-heading"
                            class="font-condensed text-base font-semibold text-foreground"
                        >
                            {m.custom_lists()}
                        </h3>
                        <p class="text-muted-foreground mt-1 text-[13px]">
                            {m.custom_lists_description()}
                        </p>
                    </div>
                    <div class="space-y-1.5">
                        <For each={filteredCustomLists()}>
                            {(list) => <ListRow list={list} />}
                        </For>
                    </div>
                </section>
            </Show>
            <Show when={downloadModalOpen()}>
                <div
                    class="fixed inset-0 z-[80] grid place-items-center ace-modal-overlay p-4"
                    onClick={(event) => {
                        if (event.target === event.currentTarget)
                            closeDownload();
                    }}
                >
                    <section
                        role="alertdialog"
                        aria-modal="true"
                        aria-labelledby="download-pack-title"
                        class="w-full max-w-md rounded-md border border-border bg-card p-6 shadow-none"
                    >
                        <h3
                            id="download-pack-title"
                            class={cn(
                                "font-condensed text-xl font-semibold",
                                downloadStatus() === "error" &&
                                    "text-destructive",
                            )}
                        >
                            {downloadStatus() === "idle"
                                ? m.download_required_title()
                                : downloadStatus() === "downloading"
                                  ? m.downloading_pack_title()
                                  : m.download_failed_title()}
                        </h3>
                        <p class="mt-3 text-sm leading-relaxed text-muted-foreground">
                            {downloadStatus() === "idle"
                                ? m.download_required_description({
                                      name: listToDownload()?.name ?? "",
                                  })
                                : downloadStatus() === "downloading"
                                  ? m.downloading_pack_description()
                                  : m.download_failed_description()}
                            <Show
                                when={
                                    downloadStatus() === "error" &&
                                    !!listToDownload()
                                }
                            >
                                <span class="mt-1 block">
                                    {syncStore.errorOf(listToDownload()!.id)}
                                </span>
                            </Show>
                        </p>
                        <Show when={downloadStatus() === "downloading"}>
                            <div class="space-y-3 py-2">
                                <div class="text-muted-foreground flex items-center justify-between text-xs font-medium">
                                    <span>
                                        {m.downloading_pack({
                                            name: listToDownload()?.name ?? "",
                                        })}
                                    </span>
                                    <span class="text-foreground font-mono">
                                        {Math.round(downloadProgress())}%
                                    </span>
                                </div>
                                <Progress
                                    value={downloadProgress()}
                                    class="bg-muted h-2"
                                />
                            </div>
                        </Show>
                        <Show when={downloadStatus() !== "downloading"}>
                            <div class="border-border/60 mt-4 flex justify-end gap-2 border-t pt-4">
                                <button
                                    type="button"
                                    class="rounded-sm border border-border px-4 py-2 text-sm"
                                    onClick={closeDownload}
                                >
                                    {downloadStatus() === "idle"
                                        ? m.cancel()
                                        : m.close()}
                                </button>
                                <button
                                    type="button"
                                    class="rounded-sm bg-primary px-4 py-2 text-sm text-primary-foreground"
                                    onClick={() => void confirmDownload()}
                                >
                                    {downloadStatus() === "idle"
                                        ? m.download_now()
                                        : m.retry()}
                                </button>
                            </div>
                        </Show>
                    </section>
                </div>
            </Show>
        </div>
    );
}
