import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { onSettled } from "solid-js";
import {
    Download,
    RefreshCw,
    Trash2,
    Search,
    Plus,
    TriangleAlert,
    X,
    ArrowRight,
} from "@lucide/solid";
import { toast } from "solid-sonner";
import * as m from "$lib/paraglide/messages.js";
import { getLocale } from "$lib/paraglide/runtime";
import {
    deleteSitePack,
    resyncSitePack,
    getAllSitePacks,
    type SitePackSchema,
    type SitePackImportPhase,
} from "$engine-client";
import { syncStore } from "$lib/stores/syncing-store";
import { userStore } from "$lib/stores/user-store";
import {
    getBlockingListDisplayName,
    isAceCategory,
} from "$lib/utils/category-presentation";
import {
    groupSitePackCategories,
    getSitePackCoverage,
    preferredCategoryPack,
    extendedCategoryPack,
} from "$lib/utils/site-pack-presentation";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";
import ProGatedButton from "$lib/components/ProGatedButton";
import Inspector from "$lib/components/ace/structural/Inspector";
import SitePackRow from "$lib/components/ace/domain/SitePackRow";
import LockSeal from "$lib/components/ace/domain/LockSeal";
import ImportPackModal from "./_ImportPackModal";
import "./site-packs.css";

function getSubtitle(pack: SitePackSchema): string {
    const name = pack.name.toLowerCase();
    if (name.includes("porn") || name.includes("adult"))
        return m.pack_subtitle_adult();
    if (name.includes("gaming") || name.includes("game"))
        return m.pack_subtitle_gaming();
    if (name.includes("social")) return m.pack_subtitle_social();
    if (
        name.includes("gambling") ||
        name.includes("betting") ||
        name.includes("casino")
    )
        return m.pack_subtitle_betting();
    if (name.includes("streaming") || name.includes("video"))
        return m.pack_subtitle_streaming();
    if (name.includes("news")) return m.pack_subtitle_news();
    return pack.description || "";
}

function formatDate(dateStr: string | null | undefined): string {
    if (!dateStr) return "—";
    const date = new Date(dateStr);
    return Number.isNaN(date.getTime())
        ? "—"
        : date.toLocaleDateString(getLocale());
}

function phaseLabel(phase: SitePackImportPhase | undefined): string | null {
    switch (phase) {
        case "INITIALIZING":
            return m.pack_sync_phase_initializing();
        case "DOWNLOADING":
            return m.pack_sync_phase_downloading();
        case "EXTRACTING":
            return m.pack_sync_phase_extracting();
        case "IMPORTING":
            return m.pack_sync_phase_importing();
        case "RESTORING":
            return m.pack_sync_phase_restoring();
        case "FINALIZING":
            return m.pack_sync_phase_finalizing();
        default:
            return null;
    }
}

export default function SitePacksPage() {
    const [packs, setPacks] = createSignal<SitePackSchema[]>([]);
    const [searchQuery, setSearchQuery] = createSignal("");
    const [isLoading, setIsLoading] = createSignal(false);
    const [loadError, setLoadError] = createSignal(false);
    const [isImportModalOpen, setIsImportModalOpen] = createSignal(false);
    const [selectedPack, setSelectedPack] = createSignal<SitePackSchema | null>(
        null,
    );
    const [deleteDialogOpen, setDeleteDialogOpen] = createSignal(false);
    const [packToDelete, setPackToDelete] = createSignal<SitePackSchema | null>(
        null,
    );
    const [isDeleting, setIsDeleting] = createSignal(false);

    async function fetchPacks() {
        setIsLoading(true);
        setLoadError(false);
        const selectedId = selectedPack()?.id ?? null;
        try {
            const { data, error } = await getAllSitePacks();
            if (error) {
                setLoadError(true);
                if (packs().length > 0) toast.error(m.failed_to_sync());
                return;
            }
            if (data) {
                const next = data.sitePacks ?? [];
                setPacks(next);
                if (selectedId)
                    setSelectedPack(
                        next.find((pack) => pack.id === selectedId) ?? null,
                    );
            }
        } catch (error) {
            console.error("Failed to load site packs:", error);
            setLoadError(true);
            if (packs().length > 0) toast.error(m.failed_to_sync());
        } finally {
            setIsLoading(false);
        }
    }

    onSettled(() => {
        document.title = `${m.site_packs()} · ${m.app_name()}`;
        void fetchPacks();
        if (
            new URLSearchParams(window.location.search).get("import") === "1" &&
            userStore.isPro
        ) {
            setIsImportModalOpen(true);
        }
    });

    let lastProContentRevision: number | undefined;
    createEffect(
        () => userStore.proContentRevision,
        (revision) => {
            if (
                lastProContentRevision !== undefined &&
                revision > lastProContentRevision
            )
                void fetchPacks();
            lastProContentRevision = revision;
        },
    );

    function matchesSearch(pack: SitePackSchema): boolean {
        if (!searchQuery().trim()) return true;
        const query = searchQuery().trim().toLocaleLowerCase(getLocale());
        return (
            getBlockingListDisplayName(pack)
                .toLocaleLowerCase(getLocale())
                .includes(query) ||
            pack.name.toLocaleLowerCase(getLocale()).includes(query) ||
            getSubtitle(pack).toLocaleLowerCase(getLocale()).includes(query)
        );
    }

    const categories = createMemo(() =>
        packs()
            .filter(isAceCategory)
            .sort((a, b) =>
                getBlockingListDisplayName(a).localeCompare(
                    getBlockingListDisplayName(b),
                ),
            ),
    );
    const categoryGroups = createMemo(() =>
        groupSitePackCategories(categories()).filter((group) => group.packs.some(matchesSearch)),
    );
    const customLists = createMemo(() =>
        packs()
            .filter((pack) => !isAceCategory(pack))
            .filter(matchesSearch)
            .sort((a, b) => a.name.localeCompare(b.name)),
    );

    async function handleResync(pack: SitePackSchema) {
        if (syncStore.isSyncing(pack.id) || pack.isLocked) return;
        try {
            const { stream } = await resyncSitePack({
                path: { site_pack_id: pack.id },
                onStreamError: (error: unknown) => {
                    throw error;
                },
            });
            if (!stream) {
                toast.error(m.failed_to_sync());
                return;
            }
            const outcome = await syncStore.handleSyncStream(pack.id, stream);
            if (outcome === "failed") {
                toast.error(m.pack_sync_failed_title());
                return;
            }
            await fetchPacks();
            window.setTimeout(() => {
                if (syncStore.statusOf(pack.id) === "completed")
                    syncStore.clear(pack.id);
            }, 4000);
        } catch (error) {
            console.error("Failed to synchronize site pack:", error);
            toast.error(m.pack_sync_failed_title());
        }
    }

    function requestDelete(pack: SitePackSchema) {
        if (pack.isLocked) return;
        setPackToDelete(pack);
        setDeleteDialogOpen(true);
    }

    async function confirmDelete() {
        const pack = packToDelete();
        if (!pack || isDeleting()) return;
        setIsDeleting(true);
        try {
            const deletedId = pack.id;
            const { error } = await deleteSitePack({
                path: { site_pack_id: deletedId },
            });
            if (error) {
                const code =
                    typeof error === "object"
                        ? (error as Record<string, unknown>).code
                        : undefined;
                if (code === "locked") {
                    toast.error(m.pack_sync_failed_locked());
                    await fetchPacks();
                } else if (code === "conflict") {
                    toast.error(m.pack_sync_failed_in_use());
                } else toast.error(m.failed_to_sync());
            } else {
                syncStore.clear(deletedId);
                if (selectedPack()?.id === deletedId) setSelectedPack(null);
                await fetchPacks();
            }
        } catch (error) {
            console.error("Failed to delete site pack:", error);
            toast.error(m.failed_to_sync());
        } finally {
            setIsDeleting(false);
            setDeleteDialogOpen(false);
            setPackToDelete(null);
        }
    }

    const selectedStatus = () =>
        selectedPack() ? syncStore.statusOf(selectedPack()!.id) : undefined;
    const selectedSyncing = () =>
        selectedPack() ? syncStore.isSyncing(selectedPack()!.id) : false;
    const selectedProgress = () =>
        selectedPack() ? (syncStore.progressOf(selectedPack()!.id) ?? 0) : 0;

    return (
        <div class="site-packs-page">
            <PageLayout
                header={
                    <PageHeader
                        title={m.site_packs()}
                        subtitle={m.site_packs_description()}
                    >
                        <ProGatedButton
                            isPro={userStore.isPro}
                            label={m.import_custom_pack()}
                            icon={Plus}
                            class="import-pack-action"
                            onClick={() => setIsImportModalOpen(true)}
                        />
                    </PageHeader>
                }
            >
                <div class="catalog-toolbar">
                    <div class="search-field">
                        <Search size={15} aria-hidden="true" />
                        <label class="sr-only" for="catalog-search">
                            {m.search_lists_placeholder()}
                        </label>
                        <input
                            id="catalog-search"
                            type="search"
                            placeholder={m.search_lists_placeholder()}
                            value={searchQuery()}
                            onInput={(e) =>
                                setSearchQuery(e.currentTarget.value)
                            }
                        />
                        <Show when={searchQuery()}>
                            <button
                                type="button"
                                class="search-clear"
                                aria-label={m.clear_search()}
                                title={m.clear_search()}
                                onClick={() => setSearchQuery("")}
                            >
                                <X size={14} aria-hidden="true" />
                            </button>
                        </Show>
                    </div>
                </div>
                <Show
                    when={!(isLoading() && packs().length === 0)}
                    fallback={
                        <div class="pack-ledger loading" aria-busy="true">
                            <For each={[0, 1, 2, 3, 4]}>
                                {() => <div class="loading-row" />}
                            </For>
                        </div>
                    }
                >
                    <Show
                        when={!(loadError() && packs().length === 0)}
                        fallback={
                            <div class="load-error" role="alert">
                                <TriangleAlert size={18} aria-hidden="true" />
                                <div>
                                    <strong>
                                        {m.site_packs_load_failed_title()}
                                    </strong>
                                    <p>
                                        {m.site_packs_load_failed_description()}
                                    </p>
                                    <button
                                        type="button"
                                        onClick={() => void fetchPacks()}
                                    >
                                        <RefreshCw
                                            size={14}
                                            aria-hidden="true"
                                        />
                                        {m.retry()}
                                    </button>
                                </div>
                            </div>
                        }
                    >
                        <Show
                            when={
                                categoryGroups().length > 0 ||
                                customLists().length > 0
                            }
                            fallback={
                                <div class="empty-inventory" role="status">
                                    <Search size={20} aria-hidden="true" />
                                    <Show
                                        when={searchQuery().trim()}
                                        fallback={
                                            <>
                                                <h2>
                                                    {m.no_site_packs_title()}
                                                </h2>
                                                <p>
                                                    {m.no_site_packs_description()}
                                                </p>
                                            </>
                                        }
                                    >
                                        <h2>
                                            {m.site_packs_no_search_results()}
                                        </h2>
                                        <p>
                                            {m.site_packs_no_search_results_detail(
                                                { query: searchQuery().trim() },
                                            )}
                                        </p>
                                        <button
                                            type="button"
                                            class="clear-query-action"
                                            onClick={() => setSearchQuery("")}
                                        >
                                            {m.clear_search()}
                                        </button>
                                    </Show>
                                </div>
                            }
                        >
                            <div class="catalog">
                                <Show when={categoryGroups().length > 0}>
                                    <section
                                        class="inventory-group"
                                        aria-label={m.site_packs()}
                                    >
                                        <p class="catalog-count-notice">
                                            {m.catalog_counts_disclaimer()}
                                        </p>
                                        <div class="category-groups">
                                            <For each={categoryGroups()}>
                                                {(group) => (
                                                    <div
                                                        class="category-family"
                                                        role="group"
                                                        aria-label={group.name}
                                                        data-testid="site-pack-family"
                                                    >
                                                        <SitePackRow
                                                            pack={preferredCategoryPack(group, userStore.isPro)}
                                                            extendedPack={extendedCategoryPack(group)}
                                                            basicPack={group.packs.find((pack) => !pack.requiresPro)}
                                                            isPro={userStore.isPro}
                                                            selected={selectedPack()?.id === preferredCategoryPack(group, userStore.isPro).id}
                                                            status={syncStore.statusOf(preferredCategoryPack(group, userStore.isPro).id)}
                                                            progress={syncStore.progressOf(preferredCategoryPack(group, userStore.isPro).id) ?? 0}
                                                            phase={phaseLabel(syncStore.phaseOf(preferredCategoryPack(group, userStore.isPro).id))}
                                                            onSelect={setSelectedPack}
                                                        />
                                                    </div>
                                                )}
                                            </For>
                                        </div>
                                    </section>
                                </Show>

                                <Show when={customLists().length > 0}>
                                    <section
                                        class="inventory-group"
                                        aria-labelledby="custom-lists-title"
                                    >
                                        <header>
                                            <div>
                                                <h2 id="custom-lists-title">
                                                    {m.custom_lists()}
                                                </h2>
                                                <p>
                                                    {m.custom_lists_group_description()}
                                                </p>
                                            </div>
                                            <span>{customLists().length}</span>
                                        </header>
                                        <div class="pack-ledger">
                                            <For each={customLists()}>
                                                {(pack) => (
                                                    <SitePackRow
                                                        pack={pack}
                                                        isPro={userStore.isPro}
                                                        subtitle={getSubtitle(
                                                            pack,
                                                        )}
                                                        selected={
                                                            selectedPack()
                                                                ?.id === pack.id
                                                        }
                                                        status={syncStore.statusOf(
                                                            pack.id,
                                                        )}
                                                        progress={
                                                            syncStore.progressOf(
                                                                pack.id,
                                                            ) ?? 0
                                                        }
                                                        phase={phaseLabel(
                                                            syncStore.phaseOf(
                                                                pack.id,
                                                            ),
                                                        )}
                                                        onSelect={
                                                            setSelectedPack
                                                        }
                                                    />
                                                )}
                                            </For>
                                        </div>
                                    </section>
                                </Show>
                            </div>
                        </Show>
                    </Show>
                </Show>
            </PageLayout>

            <Show when={selectedPack()}>
                {(pack) => (
                    <Inspector
                        title={getBlockingListDisplayName(pack())}
                        description={
                            isAceCategory(pack())
                                ? m.category_details()
                                : m.custom_list_details()
                        }
                        closeLabel={m.close()}
                        onClose={() => setSelectedPack(null)}
                    >
                        <div class="pack-inspector">
                            <Show when={getSubtitle(pack())}>
                                <p class="pack-description">
                                    {getSubtitle(pack())}
                                </p>
                            </Show>
                            <div class="pack-application">
                                <p>{m.category_apply_note()}</p>
                                <a href="/network-blocks">
                                    {m.category_use_in_block()}
                                    <ArrowRight size={14} aria-hidden="true" />
                                </a>
                            </div>
                            <dl class="pack-facts">
                                <div>
                                    <dt>{m.domains_lowercase()}</dt>
                                    <dd>
                                        {getSitePackCoverage(
                                            pack(),
                                        ).count?.toLocaleString(getLocale()) ??
                                            "—"}
                                        <Show
                                            when={
                                                getSitePackCoverage(pack())
                                                    .source === "catalog"
                                            }
                                        >
                                            <small>
                                                {m.catalog_snapshot_note()}
                                            </small>
                                        </Show>
                                    </dd>
                                </div>
                                <div>
                                    <dt>{m.updated_label()}</dt>
                                    <dd>{formatDate(pack().lastUpdatedAt)}</dd>
                                </div>
                                <div>
                                    <dt>{m.download()}</dt>
                                    <dd>
                                        {pack().isSynced
                                            ? m.downloaded()
                                            : m.not_downloaded()}
                                    </dd>
                                </div>
                                <div>
                                    <dt>{m.license()}</dt>
                                    <dd>
                                        {pack().requiresPro
                                            ? m.pro_badge()
                                            : m.pack_access_free()}
                                    </dd>
                                </div>
                            </dl>
                            <Show when={pack().isLocked}>
                                <LockSeal
                                    title={m.locked()}
                                    description={m.pack_locked_tooltip()}
                                />
                            </Show>
                            <Show
                                when={
                                    selectedStatus() === "queued" ||
                                    selectedStatus() === "downloading"
                                }
                                fallback={
                                    <Show when={selectedStatus() === "failed"}>
                                        <div class="sync-error" role="alert">
                                            <TriangleAlert
                                                size={15}
                                                aria-hidden="true"
                                            />
                                            <span>
                                                {m.pack_sync_failed_title()}
                                            </span>
                                        </div>
                                    </Show>
                                }
                            >
                                <div
                                    class="sync-readout"
                                    role="status"
                                    aria-live="polite"
                                >
                                    <div>
                                        <strong>
                                            {selectedStatus() === "queued"
                                                ? m.pack_sync_queued()
                                                : (phaseLabel(
                                                      syncStore.phaseOf(
                                                          pack().id,
                                                      ),
                                                  ) ?? m.syncing())}
                                        </strong>
                                        <span>
                                            {Math.round(selectedProgress())}%
                                        </span>
                                    </div>
                                    <span class="sync-track">
                                        <span
                                            style={{
                                                width: `${Math.round(selectedProgress())}%`,
                                            }}
                                        />
                                    </span>
                                </div>
                            </Show>
                            <div class="inspector-actions">
                                <Show
                                    when={!selectedSyncing()}
                                    fallback={
                                        <button type="button" disabled>
                                            <span class="spinner">
                                                <RefreshCw
                                                    size={15}
                                                    aria-hidden="true"
                                                />
                                            </span>
                                            {m.syncing()}
                                        </button>
                                    }
                                >
                                    <Show
                                        when={
                                            !(
                                                pack().requiresPro &&
                                                !userStore.isPro
                                            )
                                        }
                                        fallback={
                                            <ProGatedButton
                                                isPro={false}
                                                label={
                                                    pack().isSynced
                                                        ? m.resync()
                                                        : m.download()
                                                }
                                                icon={Download}
                                            />
                                        }
                                    >
                                        <Show
                                            when={pack().isSynced}
                                            fallback={
                                                <button
                                                    type="button"
                                                    disabled={pack().isLocked}
                                                    onClick={() =>
                                                        void handleResync(
                                                            pack(),
                                                        )
                                                    }
                                                >
                                                    <Download
                                                        size={15}
                                                        aria-hidden="true"
                                                    />
                                                    {m.download()}
                                                </button>
                                            }
                                        >
                                            <button
                                                type="button"
                                                disabled={pack().isLocked}
                                                title={
                                                    pack().isLocked
                                                        ? m.pack_locked_tooltip()
                                                        : m.resync()
                                                }
                                                onClick={() =>
                                                    void handleResync(pack())
                                                }
                                            >
                                                <RefreshCw
                                                    size={15}
                                                    aria-hidden="true"
                                                />
                                                {selectedStatus() === "failed"
                                                    ? m.retry()
                                                    : m.resync()}
                                            </button>
                                        </Show>
                                    </Show>
                                </Show>
                                <button
                                    type="button"
                                    class="danger-action"
                                    disabled={
                                        pack().isLocked || selectedSyncing()
                                    }
                                    title={
                                        pack().isLocked
                                            ? m.pack_locked_tooltip()
                                            : m.delete_site_pack_title()
                                    }
                                    onClick={() => requestDelete(pack())}
                                >
                                    <Trash2 size={15} aria-hidden="true" />
                                    {m.delete()}
                                </button>
                            </div>
                        </div>
                    </Inspector>
                )}
            </Show>

            <Show when={deleteDialogOpen()}>
                <div
                    class="fixed inset-0 z-[70] grid place-items-center ace-modal-overlay p-4"
                    role="presentation"
                    onClick={(e) => {
                        if (e.currentTarget === e.target && !isDeleting())
                            setDeleteDialogOpen(false);
                    }}
                >
                    <section
                        role="alertdialog"
                        aria-modal="true"
                        aria-labelledby="delete-pack-title"
                        aria-describedby="delete-pack-description"
                        class="w-full max-w-md rounded-md border border-border bg-card p-7 shadow-none"
                        onKeyDown={(e) => {
                            if (e.key === "Escape" && !isDeleting())
                                setDeleteDialogOpen(false);
                        }}
                    >
                        <h2
                            id="delete-pack-title"
                            class="font-condensed text-2xl font-semibold"
                        >
                            {m.delete_site_pack_title()}
                        </h2>
                        <p
                            id="delete-pack-description"
                            class="mt-3 text-sm leading-relaxed text-muted-foreground"
                        >
                            {m.delete_site_pack_detailed_description({
                                name: packToDelete()
                                    ? getBlockingListDisplayName(
                                          packToDelete()!,
                                      )
                                    : "",
                            })}
                        </p>
                        <div class="mt-6 flex justify-end gap-2">
                            <button
                                type="button"
                                class="rounded-sm border border-border px-4 py-2 text-sm"
                                disabled={isDeleting()}
                                onClick={() => setDeleteDialogOpen(false)}
                            >
                                {m.cancel()}
                            </button>
                            <button
                                type="button"
                                class="rounded-sm bg-destructive px-4 py-2 text-sm text-destructive-foreground hover:bg-destructive/90"
                                disabled={isDeleting()}
                                onClick={() => void confirmDelete()}
                            >
                                {isDeleting()
                                    ? m.processing()
                                    : m.yes_delete_pack()}
                            </button>
                        </div>
                    </section>
                </div>
            </Show>

            <ImportPackModal
                open={isImportModalOpen()}
                onOpenChange={setIsImportModalOpen}
                onSave={fetchPacks}
            />
        </div>
    );
}
