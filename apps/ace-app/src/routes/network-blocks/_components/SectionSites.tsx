import { createMemo, createSignal, For, Show } from "solid-js";
import { Dynamic } from "@solidjs/web";
import { Badge } from "$lib/components/ui/badge/solid";
import { Input } from "$lib/components/ui/input/solid";
import { Search, Check, ChevronDown, ChevronRight } from "@lucide/solid";
import { getPackIcon } from "../../site-packs/_pack-icons";
import {
    getSitePackCoverage,
    getSitePackIconSlug,
    groupSitePackCategories,
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
    const [expandedCategoryNames, setExpandedCategoryNames] = createSignal<
        string[]
    >([]);
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

    function toggleCategory(name: string) {
        setExpandedCategoryNames((previous) =>
            previous.includes(name)
                ? previous.filter((candidate) => candidate !== name)
                : [...previous, name],
        );
    }
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

    // Tier swap is permitted only when the old connection can be removed.
    // A preexisting locked connection is never removed, even when Pro has a superset.
    function stageConnection(list: SitePackSchema) {
        if (isAceCategory(list)) {
            for (const sibling of props.availableLists) {
                if (
                    sibling.id === list.id ||
                    !isAceCategory(sibling) ||
                    getBlockingListDisplayName(sibling) !==
                        getBlockingListDisplayName(list) ||
                    !props.delta.isConnected(sibling.id) ||
                    (props.isLocked &&
                        props.delta.isInitialConnection(sibling.id))
                )
                    continue;
                if (props.delta.isNewConnection(sibling.id))
                    props.delta.removeTempConnection(sibling.id);
                else props.delta.removeConnection(sibling.id);
            }
        }
        props.delta.addConnection(list.id);
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
            if (props.delta.isNewConnection(list.id))
                props.delta.removeTempConnection(list.id);
            else props.delta.removeConnection(list.id);
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
            if (freshList) stageConnection(freshList);
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

    function ListRow(row: { list: SitePackSchema; variant?: boolean }) {
        const isConnected = () => connectedListIds().includes(row.list.id);
        const premium = () => requiresProForConnection(row.list);
        const proGated = () => premium() && isFreeUser() && !isConnected();
        const canToggle = () =>
            !proGated() &&
            (!isConnected() ||
                !(
                    props.isLocked &&
                    props.delta.isInitialConnection(row.list.id)
                ));
        const coverage = () => getSitePackCoverage(row.list);
        return (
            <button
                type="button"
                class={cn(
                    "flex w-full min-w-0 items-center gap-3 border border-border/50 bg-card px-3 py-2.5 text-left transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring",
                    row.variant && "border-t-0 pl-6",
                    canToggle()
                        ? "cursor-pointer hover:bg-muted/50"
                        : "cursor-default opacity-60",
                    isConnected() && "border-foreground/30 bg-muted/45",
                )}
                data-testid={
                    row.variant
                        ? "category-tier-option"
                        : "category-single-option"
                }
                onClick={() => {
                    if (proGated()) {
                        void openGetProPage();
                        return;
                    }
                    if (canToggle()) toggleList(row.list);
                }}
                aria-pressed={isConnected() ? "true" : "false"}
            >
                <span
                    class={cn(
                        "flex size-4 shrink-0 items-center justify-center rounded-sm border border-border",
                        isConnected() &&
                            "border-foreground bg-foreground text-background",
                    )}
                    aria-hidden="true"
                >
                    <Show when={isConnected()}>
                        <Check size={12} />
                    </Show>
                </span>
                <Show when={!row.variant}>
                    <span
                        class="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-muted"
                        aria-hidden="true"
                        data-testid="category-icon"
                    >
                        <Dynamic
                            component={getPackIcon(
                                getSitePackIconSlug(row.list),
                            )}
                            size={19}
                            strokeWidth={1.75}
                        />
                    </span>
                </Show>
                <span class="flex min-w-0 flex-1 flex-col gap-1">
                    <span class="flex flex-wrap items-center gap-2">
                        <span class="text-sm font-semibold">
                            {row.variant
                                ? row.list.requiresPro
                                    ? m.catalog_pro_extended()
                                    : m.catalog_free_essential()
                                : getBlockingListDisplayName(row.list)}
                        </span>
                        <Show when={!row.variant}>
                            <span class="text-[11px] text-muted-foreground">
                                {premium()
                                    ? m.pro_badge()
                                    : m.pack_access_free()}
                            </span>
                        </Show>
                        <Show when={isConnected() && premium() && isFreeUser()}>
                            <span class="text-xs text-muted-foreground">
                                {m.status_pro_required()}
                            </span>
                        </Show>
                    </span>
                </span>
                <span
                    class="shrink-0 text-right font-mono text-[11px] text-muted-foreground"
                    data-testid="category-coverage"
                >
                    {coverage().count !== null
                        ? `${coverage().count!.toLocaleString(getLocale())} ${m.domains_lowercase()}`
                        : "—"}
                </span>
            </button>
        );
    }

    function CategoryRow(row: { group: SitePackCategoryGroup }) {
        const selectedCount = () =>
            row.group.packs.filter((pack) =>
                connectedListIds().includes(pack.id),
            ).length;
        return (
            <Show
                when={row.group.packs.length > 1}
                fallback={
                    <For each={row.group.packs}>
                        {(pack) => <ListRow list={pack} />}
                    </For>
                }
            >
                <div
                    class="rounded-sm border border-border/50 bg-card"
                    data-testid="category-family"
                >
                    <button
                        type="button"
                        class="flex min-h-15 w-full min-w-0 items-center gap-3 px-3 py-2.5 text-left hover:bg-muted/50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                        data-testid="category-family-toggle"
                        aria-expanded={
                            expandedCategoryNames().includes(row.group.name)
                                ? "true"
                                : "false"
                        }
                        onClick={() => toggleCategory(row.group.name)}
                    >
                        <span
                            class="flex size-8 shrink-0 items-center justify-center rounded-md border border-border bg-muted"
                            aria-hidden="true"
                            data-testid="category-icon"
                        >
                            <Dynamic
                                component={getPackIcon(
                                    getSitePackIconSlug(row.group.packs[0]),
                                )}
                                size={19}
                                strokeWidth={1.75}
                            />
                        </span>
                        <span class="min-w-0 flex-1">
                            <span class="block text-sm font-semibold">
                                {row.group.name}
                            </span>
                            <span
                                class="mt-1 flex flex-wrap gap-x-2 gap-y-0.5 font-mono text-[11px] text-muted-foreground"
                                data-testid="category-tier-comparison"
                            >
                                <For each={row.group.packs}>
                                    {(pack) => (
                                        <span>
                                            {pack.requiresPro
                                                ? m.pro_badge()
                                                : m.pack_access_free()}{" "}
                                            {getSitePackCoverage(
                                                pack,
                                            ).count?.toLocaleString(
                                                getLocale(),
                                            ) ?? "—"}
                                        </span>
                                    )}
                                </For>
                                <span>{m.domains_lowercase()}</span>
                            </span>
                        </span>
                        <Show when={selectedCount() > 0}>
                            <span class="shrink-0 text-foreground">
                                <Check size={16} aria-hidden="true" />
                                <span class="sr-only">
                                    {m.block_wizard_category_count({
                                        count: selectedCount(),
                                    })}
                                </span>
                            </span>
                        </Show>
                        <Show
                            when={expandedCategoryNames().includes(
                                row.group.name,
                            )}
                            fallback={
                                <ChevronRight
                                    size={17}
                                    class="shrink-0 text-muted-foreground"
                                    aria-hidden="true"
                                />
                            }
                        >
                            <ChevronDown
                                size={17}
                                class="shrink-0 text-muted-foreground"
                                aria-hidden="true"
                            />
                        </Show>
                    </button>
                    <Show
                        when={expandedCategoryNames().includes(row.group.name)}
                    >
                        <div
                            class="border-t border-border/50"
                            data-testid="category-tier-options"
                        >
                            <For each={row.group.packs}>
                                {(pack) => <ListRow list={pack} variant />}
                            </For>
                        </div>
                    </Show>
                </div>
            </Show>
        );
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
                            count: selectedLists().filter(isAceCategory).length,
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
                    class="fixed inset-0 z-[80] grid place-items-center bg-black/70 p-4"
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
            <style>{`
                .category-visibility-toggle { display:flex; width:100%; min-height:38px; align-items:center; justify-content:center; margin-top:8px; padding:8px 12px; border:1px solid var(--border); border-radius:4px; background:var(--card); color:var(--foreground); cursor:pointer; font-size:12px; font-weight:600; }
                .category-visibility-toggle:hover { background:var(--muted); }
                .category-visibility-toggle:focus-visible { outline:2px solid var(--ring); outline-offset:2px; }
                .pro-categories { border-top:1px solid var(--border); padding-top:12px; margin-top:10px; }
                .pro-categories summary { font-size:13px; color:var(--muted-foreground); cursor:pointer; }
            `}</style>
        </div>
    );
}
