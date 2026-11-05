<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import * as AlertDialog from "$lib/components/ui/alert-dialog";
    import {
        Download,
        RefreshCw,
        Trash2,
        Search,
        Plus,
        TriangleAlert,
        X,
        ArrowRight,
    } from "@lucide/svelte";
    import { SvelteDate } from "svelte/reactivity";
    import ImportPackModal from "./_ImportPackModal.svelte";
    import ProGatedButton from "$lib/components/ProGatedButton.svelte";
    import Inspector from "$lib/components/ace/structural/Inspector.svelte";
    import SitePackRow from "$lib/components/ace/domain/SitePackRow.svelte";

    import {
        deleteSitePack,
        resyncSitePack,
        getAllSitePacks,
        type SitePackSchema,
        type SitePackImportPhase,
    } from "$engine-client";
    import { syncStore } from "$lib/stores/syncing-store.svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { page } from "$app/state";
    import { resolve } from "$app/paths";
    import { onMount, untrack } from "svelte";
    import { toast } from "svelte-sonner";
    import { getLocale } from "$lib/paraglide/runtime";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";
    import LockSeal from "$lib/components/ace/domain/LockSeal.svelte";
    import {
        getBlockingListDisplayName,
        isAceCategory,
    } from "$lib/utils/category-presentation";

    let packs = $state<SitePackSchema[]>([]);
    let searchQuery = $state("");
    let isLoading = $state(false);
    let loadError = $state(false);
    let isImportModalOpen = $state(false);
    let selectedPack = $state<SitePackSchema | null>(null);

    let isDeleteDialogOpen = $state(false);
    let packToDelete = $state<SitePackSchema | null>(null);
    let isDeleting = $state(false);

    async function fetchPacks() {
        isLoading = true;
        loadError = false;
        const selectedId = selectedPack?.id ?? null;
        const { data, error } = await getAllSitePacks();

        if (error) {
            loadError = true;
            // The inline error already explains an initial failure. Avoid a duplicate toast.
            if (packs.length > 0) toast.error(m.failed_to_sync());
        } else if (data) {
            packs = data.sitePacks ?? [];
            if (selectedId) {
                selectedPack =
                    packs.find((pack) => pack.id === selectedId) ?? null;
            }
        }

        isLoading = false;
    }

    onMount(() => {
        void fetchPacks();

        if (page.url.searchParams.get("import") === "1" && userStore.isPro) {
            isImportModalOpen = true;
        }
    });

    // Newly available Pro packs are registered by the engine after checkout.
    // Refresh this open view as soon as that persistent catalogue is updated.
    let lastProContentRevision = userStore.proContentRevision;
    $effect(() => {
        const revision = userStore.proContentRevision;
        if (revision <= lastProContentRevision) return;
        lastProContentRevision = revision;
        untrack(() => void fetchPacks());
    });

    const categories = $derived(
        packs
            .filter(isAceCategory)
            .filter(matchesSearch)
            .sort((a, b) =>
                getBlockingListDisplayName(a).localeCompare(
                    getBlockingListDisplayName(b),
                ),
            ),
    );

    const customLists = $derived(
        packs
            .filter((pack) => !isAceCategory(pack))
            .filter(matchesSearch)
            .sort((a, b) => a.name.localeCompare(b.name)),
    );

    function matchesSearch(pack: SitePackSchema): boolean {
        if (!searchQuery.trim()) return true;
        const query = searchQuery.trim().toLocaleLowerCase(getLocale());
        return (
            getBlockingListDisplayName(pack).toLocaleLowerCase(getLocale()).includes(query) ||
            pack.name.toLocaleLowerCase(getLocale()).includes(query) ||
            getSubtitle(pack).toLocaleLowerCase(getLocale()).includes(query)
        );
    }

    function getSubtitle(pack: SitePackSchema): string {
        const name = pack.name.toLowerCase();
        if (name.includes("porn") || name.includes("adult")) {
            return m.pack_subtitle_adult();
        }
        if (name.includes("gaming") || name.includes("game")) {
            return m.pack_subtitle_gaming();
        }
        if (name.includes("social")) return m.pack_subtitle_social();
        if (
            name.includes("gambling") ||
            name.includes("betting") ||
            name.includes("casino")
        ) {
            return m.pack_subtitle_betting();
        }
        if (name.includes("streaming") || name.includes("video")) {
            return m.pack_subtitle_streaming();
        }
        if (name.includes("news")) return m.pack_subtitle_news();
        return pack.description || "";
    }

    function formatDate(dateStr: string | null | undefined): string {
        if (!dateStr) return "—";
        const date = new SvelteDate(dateStr);
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

    async function handleResync(pack: SitePackSchema) {
        if (syncStore.isSyncing(pack.id) || pack.isLocked) return;

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
            if (syncStore.statusOf(pack.id) === "completed") {
                syncStore.clear(pack.id);
            }
        }, 4000);
    }

    function requestDelete(pack: SitePackSchema) {
        if (pack.isLocked) return;
        packToDelete = pack;
        isDeleteDialogOpen = true;
    }

    async function confirmDelete() {
        if (!packToDelete || isDeleting) return;

        isDeleting = true;
        const deletedId = packToDelete.id;
        const { error } = await deleteSitePack({
            path: { site_pack_id: deletedId },
        });

        if (error) {
            const code =
                error && typeof error === "object"
                    ? (error as Record<string, unknown>).code
                    : undefined;

            if (code === "locked") {
                toast.error(m.pack_sync_failed_locked());
                await fetchPacks();
            } else if (code === "conflict") {
                toast.error(m.pack_sync_failed_in_use());
            } else {
                toast.error(m.failed_to_sync());
            }
        } else {
            syncStore.clear(deletedId);
            if (selectedPack?.id === deletedId) selectedPack = null;
            await fetchPacks();
        }

        isDeleting = false;
        isDeleteDialogOpen = false;
        packToDelete = null;
    }

    function selectPack(pack: SitePackSchema) {
        selectedPack = pack;
    }
</script>

<svelte:head>
    <title>{m.site_packs()} · {m.app_name()}</title>
</svelte:head>

<PageLayout>
    {#snippet header()}
        <PageHeader
            title={m.site_packs()}
            subtitle={m.site_packs_description()}
        >
            <ProGatedButton
                isPro={userStore.isPro}
                label={m.import_custom_pack()}
                icon={Plus}
                class="import-pack-action"
                onclick={() => (isImportModalOpen = true)}
            />
        </PageHeader>
    {/snippet}

    <div class="catalog-toolbar">
        <div class="search-field">
            <Search size={15} aria-hidden="true" />
            <label class="sr-only" for="catalog-search">{m.search_packs_placeholder()}</label>
            <input
                id="catalog-search"
                type="search"
                placeholder={m.search_packs_placeholder()}
                bind:value={searchQuery}
            />
            {#if searchQuery}
                <button
                    type="button"
                    class="search-clear"
                    aria-label={m.clear_search()}
                    title={m.clear_search()}
                    onclick={() => (searchQuery = "")}
                >
                    <X size={14} aria-hidden="true" />
                </button>
            {/if}
        </div>
    </div>

    {#if isLoading && packs.length === 0}
        <div class="pack-ledger loading" aria-busy="true">
            {#each Array(5) as _, index (index)}
                <div class="loading-row"></div>
            {/each}
        </div>
    {:else if loadError && packs.length === 0}
        <div class="load-error" role="alert">
            <TriangleAlert size={18} aria-hidden="true" />
            <div>
                <strong>{m.site_packs_load_failed_title()}</strong>
                <p>{m.site_packs_load_failed_description()}</p>
                <button type="button" onclick={fetchPacks}>
                    <RefreshCw size={14} aria-hidden="true" />
                    {m.retry()}
                </button>
            </div>
        </div>
    {:else if categories.length === 0 && customLists.length === 0}
        <div class="empty-inventory" role="status">
            <Search size={20} aria-hidden="true" />
            {#if searchQuery.trim()}
                <h2>{m.site_packs_no_search_results()}</h2>
                <p>{m.site_packs_no_search_results_detail({ query: searchQuery.trim() })}</p>
                <button type="button" class="clear-query-action" onclick={() => (searchQuery = "")}>
                    {m.clear_search()}
                </button>
            {:else}
                <h2>{m.no_site_packs_title()}</h2>
                <p>{m.no_site_packs_description()}</p>
            {/if}
        </div>
    {:else}
        <div class="catalog">
            {#if categories.length > 0}
                <section class="inventory-group" aria-labelledby="categories-title">
                    <header>
                        <div>
                            <h2 id="categories-title">{m.free_packs()}</h2>
                            <p>{m.category_group_description()}</p>
                        </div>
                        <span>{categories.length}</span>
                    </header>
                    <div class="pack-ledger">
                        {#each categories as pack (pack.id)}
                            <SitePackRow
                                {pack}
                                isPro={userStore.isPro}
                                subtitle={getSubtitle(pack)}
                                selected={selectedPack?.id === pack.id}
                                status={syncStore.statusOf(pack.id)}
                                progress={syncStore.progressOf(pack.id) ?? 0}
                                phase={phaseLabel(syncStore.phaseOf(pack.id))}
                                onSelect={selectPack}
                            />
                        {/each}
                    </div>
                </section>
            {/if}

            {#if customLists.length > 0}
                <section class="inventory-group" aria-labelledby="custom-lists-title">
                    <header>
                        <div>
                            <h2 id="custom-lists-title">{m.pro_packs()}</h2>
                            <p>{m.custom_lists_group_description()}</p>
                        </div>
                        <span>{customLists.length}</span>
                    </header>
                    <div class="pack-ledger">
                        {#each customLists as pack (pack.id)}
                            <SitePackRow
                                {pack}
                                isPro={userStore.isPro}
                                subtitle={getSubtitle(pack)}
                                selected={selectedPack?.id === pack.id}
                                status={syncStore.statusOf(pack.id)}
                                progress={syncStore.progressOf(pack.id) ?? 0}
                                phase={phaseLabel(syncStore.phaseOf(pack.id))}
                                onSelect={selectPack}
                            />
                        {/each}
                    </div>
                </section>
            {/if}
        </div>
    {/if}
</PageLayout>

{#if selectedPack}
    {@const inspectorPack = selectedPack}
    {@const selectedStatus = syncStore.statusOf(inspectorPack.id)}
    {@const selectedSyncing = syncStore.isSyncing(inspectorPack.id)}
    {@const selectedProgress = syncStore.progressOf(inspectorPack.id) ?? 0}

    <Inspector
        title={getBlockingListDisplayName(inspectorPack)}
        description={isAceCategory(inspectorPack)
            ? m.category_details()
            : m.custom_list_details()}
        closeLabel={m.close()}
        onClose={() => (selectedPack = null)}
    >
        <div class="pack-inspector">
            {#if getSubtitle(inspectorPack)}
                <p class="pack-description">{getSubtitle(inspectorPack)}</p>
            {/if}

            <div class="pack-application">
                <p>{m.category_apply_note()}</p>
                <a href={resolve("/network-blocks")}>
                    {m.category_use_in_block()}
                    <ArrowRight size={14} aria-hidden="true" />
                </a>
            </div>

            <dl class="pack-facts">
                <div>
                    <dt>{m.domains_lowercase()}</dt>
                    <dd>
                        {inspectorPack.isSynced
                            ? (inspectorPack.entriesCount?.toLocaleString(getLocale()) ?? "—")
                            : "—"}
                    </dd>
                </div>
                <div>
                    <dt>{m.updated_label()}</dt>
                    <dd>{formatDate(inspectorPack.lastUpdatedAt)}</dd>
                </div>
                <div>
                    <dt>{m.download()}</dt>
                    <dd>
                        {inspectorPack.isSynced
                            ? m.downloaded()
                            : m.not_downloaded()}
                    </dd>
                </div>
                <div>
                    <dt>{m.license()}</dt>
                    <dd>
                        {inspectorPack.requiresPro
                            ? m.pro_badge()
                            : m.plan_free()}
                    </dd>
                </div>
            </dl>

            {#if inspectorPack.isLocked}
                <LockSeal
                    title={m.locked()}
                    description={m.pack_locked_tooltip()}
                />
            {/if}

            {#if selectedStatus === "queued" || selectedStatus === "downloading"}
                <div class="sync-readout" role="status" aria-live="polite">
                    <div>
                        <strong>
                            {selectedStatus === "queued"
                                ? m.pack_sync_queued()
                                : (phaseLabel(syncStore.phaseOf(inspectorPack.id)) ??
                                  m.syncing())}
                        </strong>
                        <span>{Math.round(selectedProgress)}%</span>
                    </div>
                    <span class="sync-track">
                        <span
                            style:width={String(Math.round(selectedProgress)) + "%"}
                        ></span>
                    </span>
                </div>
            {:else if selectedStatus === "failed"}
                <div class="sync-error" role="alert">
                    <TriangleAlert size={15} aria-hidden="true" />
                    <span>{m.pack_sync_failed_title()}</span>
                </div>
            {/if}

            <div class="inspector-actions">
                {#if selectedSyncing}
                    <button type="button" disabled>
                        <span class="spinner"><RefreshCw size={15} aria-hidden="true" /></span>
                        {m.syncing()}
                    </button>
                {:else if inspectorPack.requiresPro && !userStore.isPro}
                    <ProGatedButton
                        isPro={false}
                        label={inspectorPack.isSynced ? m.resync() : m.download()}
                        icon={Download}
                    />
                {:else if inspectorPack.isSynced}
                    <button
                        type="button"
                        disabled={inspectorPack.isLocked}
                        title={inspectorPack.isLocked
                            ? m.pack_locked_tooltip()
                            : m.resync()}
                        onclick={() => handleResync(inspectorPack)}
                    >
                        <RefreshCw size={15} aria-hidden="true" />
                        {selectedStatus === "failed" ? m.retry() : m.resync()}
                    </button>
                {:else}
                    <button
                        type="button"
                        disabled={inspectorPack.isLocked}
                        onclick={() => handleResync(inspectorPack)}
                    >
                        <Download size={15} aria-hidden="true" />
                        {m.download()}
                    </button>
                {/if}

                <button
                    type="button"
                    class="danger-action"
                    disabled={inspectorPack.isLocked || selectedSyncing}
                    title={inspectorPack.isLocked
                        ? m.pack_locked_tooltip()
                        : m.delete_site_pack_title()}
                    onclick={() => requestDelete(inspectorPack)}
                >
                    <Trash2 size={15} aria-hidden="true" />
                    {m.delete()}
                </button>
            </div>
        </div>
    </Inspector>
{/if}

<AlertDialog.Root bind:open={isDeleteDialogOpen}>
    <AlertDialog.Content class="rounded-md border-border bg-card shadow-none">
        <AlertDialog.Header>
            <AlertDialog.Title class="font-condensed text-2xl font-semibold">
                {m.delete_site_pack_title()}
            </AlertDialog.Title>
            <AlertDialog.Description class="text-sm leading-relaxed">
                {m.delete_site_pack_detailed_description({
                    name: packToDelete
                        ? getBlockingListDisplayName(packToDelete)
                        : "",
                })}
            </AlertDialog.Description>
        </AlertDialog.Header>
        <AlertDialog.Footer class="mt-6 gap-2">
            <AlertDialog.Cancel>{m.cancel()}</AlertDialog.Cancel>
            <AlertDialog.Action
                class="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                onclick={confirmDelete}
                disabled={isDeleting}
            >
                {isDeleting ? m.processing() : m.yes_delete_pack()}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<ImportPackModal bind:open={isImportModalOpen} onSave={fetchPacks} />

<style>
    h2 {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .catalog-toolbar {
        display: flex;
        justify-content: flex-end;
        margin-bottom: 14px;
    }

    .search-field {
        width: min(100%, 320px);
        min-height: 38px;
        display: flex;
        align-items: center;
        gap: 8px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 10px;
        background: var(--card);
        color: var(--muted-foreground);
    }

    .search-field:focus-within {
        border-color: var(--ace-signal-deep);
    }

    .search-field input {
        min-width: 0;
        width: 100%;
        border: 0;
        outline: 0;
        background: transparent;
        color: var(--foreground);
        font-size: 12px;
    }

    :global(.import-pack-action) {
        min-width: 0 !important;
        height: 38px !important;
    }

    .catalog {
        display: grid;
        gap: 18px;
    }

    .inventory-group {
        border: 1px solid var(--border);
        border-radius: 5px;
        overflow: hidden;
        background: var(--card);
    }

    .inventory-group > header {
        min-height: 54px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 18px;
        padding: 8px 14px;
        border-bottom: 1px solid var(--border);
        background: color-mix(in srgb, var(--muted) 35%, var(--card));
    }

    .inventory-group h2 {
        margin: 0;
        font-size: 14px;
        font-weight: 600;
    }

    .inventory-group header p {
        margin: 2px 0 0;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.35;
    }

    .inventory-group header > span {
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
    }

    .pack-ledger :global(.site-pack-row:last-child) {
        border-bottom: 0;
    }

    .loading {
        border: 1px solid var(--border);
        border-radius: 5px;
        overflow: hidden;
    }

    .loading-row {
        height: 68px;
        border-bottom: 1px solid var(--border);
        background: var(--muted);
        opacity: 0.62;
    }

    .loading-row:last-child {
        border-bottom: 0;
    }

    .load-error,
    .empty-inventory {
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .load-error {
        display: flex;
        gap: 12px;
        padding: 18px;
        border-left: 6px solid var(--ace-danger);
    }

    .load-error strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .load-error p {
        margin: 4px 0 11px;
        color: var(--muted-foreground);
        font-size: 12px;
    }

    .load-error button,
    .inspector-actions button {
        min-height: 36px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 7px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 11px;
        background: transparent;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
    }

    .load-error button:hover,
    .inspector-actions button:hover:not(:disabled) {
        background: var(--muted);
    }

    .empty-inventory {
        min-height: 220px;
        display: grid;
        place-items: center;
        align-content: center;
        gap: 8px;
        padding: 28px;
        text-align: center;
    }

    .empty-inventory h2 {
        margin: 0;
        font-size: 22px;
        font-weight: 600;
    }

    .empty-inventory p {
        max-width: 420px;
        margin: 0;
        color: var(--muted-foreground);
        font-size: 12px;
    }

    .pack-inspector {
        display: grid;
        gap: 18px;
    }

    .pack-description {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 13px;
        line-height: 1.55;
    }

    .pack-facts {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        margin: 0;
        border: 1px solid var(--border);
        border-radius: 4px;
        overflow: hidden;
    }

    .pack-facts div {
        min-width: 0;
        padding: 12px;
        background: color-mix(in srgb, var(--muted) 32%, transparent);
    }

    .pack-facts div:nth-child(even) {
        border-left: 1px solid var(--border);
    }

    .pack-facts div:nth-child(n + 3) {
        border-top: 1px solid var(--border);
    }

    .pack-facts dt {
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .pack-facts dd {
        margin: 4px 0 0;
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
    }

    .sync-readout {
        display: grid;
        gap: 8px;
        padding: 12px;
        border: 1px solid var(--border);
        border-radius: 4px;
    }

    .sync-readout > div {
        display: flex;
        justify-content: space-between;
        gap: 14px;
        font-size: 11px;
    }

    .sync-readout span {
        font-family: "IBM Plex Mono", monospace;
    }

    .sync-track {
        height: 3px;
        overflow: hidden;
        background: var(--muted);
    }

    .sync-track > span {
        display: block;
        height: 100%;
        background: var(--ace-information);
        transition: width 180ms ease;
    }

    .sync-error {
        display: flex;
        align-items: center;
        gap: 8px;
        color: var(--ace-danger);
        font-size: 12px;
    }

    .inspector-actions {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        padding-top: 4px;
        border-top: 1px solid var(--border);
    }

    .inspector-actions button:disabled {
        cursor: not-allowed;
        opacity: 0.4;
    }

    .inspector-actions .danger-action {
        color: var(--ace-danger);
    }

    .inspector-actions .danger-action:hover:not(:disabled) {
        border-color: color-mix(in srgb, var(--ace-danger) 38%, var(--border));
        background: color-mix(in srgb, var(--ace-danger) 8%, transparent);
    }

    .spinner {
        display: inline-flex;
        animation: spin 900ms linear infinite;
    }

    @keyframes spin {
        to {
            transform: rotate(360deg);
        }
    }

    @container page (max-width: 560px) {
        .catalog-toolbar {
            justify-content: stretch;
        }

        .search-field {
            width: 100%;
        }

        .pack-facts {
            grid-template-columns: 1fr;
        }

        .pack-facts div:nth-child(even) {
            border-left: 0;
        }

        .pack-facts div + div {
            border-top: 1px solid var(--border);
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .spinner {
            animation: none;
        }
    }

    .search-field input::-webkit-search-cancel-button {
        appearance: none;
    }

    .search-clear {
        width: 27px;
        height: 27px;
        flex: 0 0 27px;
        display: grid;
        place-items: center;
        border: 0;
        border-radius: 3px;
        background: transparent;
        color: var(--muted-foreground);
        cursor: pointer;
    }

    .search-clear:hover {
        background: var(--muted);
        color: var(--foreground);
    }

    .clear-query-action {
        margin-top: 8px;
        min-height: 36px;
        padding: 0 12px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--foreground);
        color: var(--background);
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
    }

    .pack-application {
        padding: 12px 14px;
        border-left: 3px solid var(--ace-signal);
        background: color-mix(in srgb, var(--muted) 54%, transparent);
    }

    .pack-application p {
        margin: 0;
        font-size: 12px;
        line-height: 1.5;
    }

    .pack-application a {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        margin-top: 9px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: underline;
        text-underline-offset: 3px;
    }
</style>
