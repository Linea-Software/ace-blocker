import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { Plus, Globe, FileUp, Lock, WifiOff, ArrowRight } from "@lucide/solid";
import { toast } from "solid-sonner";
import * as m from "$lib/paraglide/messages.js";
import {
    deleteNetworkBlock,
    setNetworkBlockEnabled,
    type NetworkBlockSchema,
} from "$engine-client";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";
import NetworkPolicy from "$lib/components/ace/domain/NetworkPolicy";
import LockModal from "./_components/_LockModal";
import SoftUnlockModal from "./_components/_SoftUnlockModal";
import {
    loadNetworkBlocks,
    type NetworkBlockWithLock,
    type NetworkBlocksLoad,
} from "./_data";
import "./network-blocks.css";

export default function NetworkBlocksPage() {
    const navigate = useNavigate();
    const [data, setData] = createSignal<NetworkBlocksLoad>({
        blocks: [],
        engineOffline: false,
        loadError: false,
    });
    const [loading, setLoading] = createSignal(true);
    const blocks = createMemo(() =>
        [...data().blocks].sort((a, b) => a.name.localeCompare(b.name)),
    );

    const [lockModalOpen, setLockModalOpen] = createSignal(false);
    const [selectedBlockForLock, setSelectedBlockForLock] =
        createSignal<NetworkBlockWithLock | null>(null);
    const [softUnlockOpen, setSoftUnlockOpen] = createSignal(false);
    const [selectedBlockForSoftUnlock, setSelectedBlockForSoftUnlock] =
        createSignal<NetworkBlockSchema | null>(null);
    const [deleteDialogOpen, setDeleteDialogOpen] = createSignal(false);
    const [blockToDelete, setBlockToDelete] =
        createSignal<NetworkBlockSchema | null>(null);
    const [deleting, setDeleting] = createSignal(false);

    async function refresh() {
        setLoading(true);
        try {
            setData(await loadNetworkBlocks());
        } catch (error) {
            console.error("Failed to fetch network blocks", error);
            setData({ blocks: [], engineOffline: false, loadError: true });
        } finally {
            setLoading(false);
        }
    }

    onSettled(() => {
        document.title = `${m.network_blocks()} · ${m.app_name()}`;
        const id = new URLSearchParams(window.location.search).get("block");
        if (id)
            void navigate(
                "/network-blocks/editor?block=" + encodeURIComponent(id),
                { replace: true },
            );
        void refresh();
    });

    function handleCreate() {
        void navigate("/network-blocks/editor");
    }

    function handleEdit(block: NetworkBlockSchema) {
        void navigate(
            "/network-blocks/editor?block=" + encodeURIComponent(block.id),
        );
    }

    function handleEditSchedule(block: NetworkBlockWithLock) {
        setLockModalOpen(false);
        void navigate(
            "/network-blocks/editor?block=" +
                encodeURIComponent(block.id) +
                "&section=schedule",
        );
    }

    async function handleEnabledChange(
        block: NetworkBlockSchema,
        enabled: boolean,
    ) {
        if (!enabled && block.isLocked) return;
        try {
            const { error } = await setNetworkBlockEnabled({
                path: { network_block_id: block.id },
                body: { enabled },
            });
            if (error) {
                toast.error(
                    error.status === 403
                        ? m.block_locked_tooltip()
                        : m.failed_to_sync(),
                );
                if (error.status === 403) await refresh();
                return;
            }
            await refresh();
        } catch (error) {
            console.error(
                "Failed to change network block enabled state:",
                error,
            );
            toast.error(m.failed_to_sync());
        }
    }

    function handleLock(block: NetworkBlockWithLock) {
        if (block.lockState.state === "soft_unlockable") {
            setSelectedBlockForSoftUnlock(block);
            setSoftUnlockOpen(true);
            return;
        }
        setSelectedBlockForLock(block);
        setLockModalOpen(true);
    }

    function requestDelete(block: NetworkBlockSchema) {
        if (block.isLocked) return;
        setBlockToDelete(block);
        setDeleteDialogOpen(true);
    }

    async function confirmDelete() {
        const block = blockToDelete();
        if (!block || deleting()) return;
        setDeleting(true);
        try {
            const { error } = await deleteNetworkBlock({
                path: { network_block_id: block.id },
            });
            if (error) {
                console.error("Failed to delete network block:", error);
                toast.error(m.failed_to_sync());
            } else {
                await refresh();
            }
        } catch (error) {
            console.error("Failed to delete network block:", error);
            toast.error(m.failed_to_sync());
        } finally {
            setDeleting(false);
            setDeleteDialogOpen(false);
            setBlockToDelete(null);
        }
    }

    return (
        <div class="network-blocks-page">
            <PageLayout
                header={
                    <PageHeader
                        title={m.network_blocks()}
                        subtitle={m.network_blocks_description()}
                    >
                        <button
                            class="create-action"
                            type="button"
                            data-tour="create-block"
                            disabled={loading() || data().engineOffline}
                            title={
                                data().engineOffline
                                    ? m.status_engine_offline_description()
                                    : undefined
                            }
                            onClick={handleCreate}
                        >
                            <Plus
                                size={17}
                                strokeWidth={2}
                                aria-hidden="true"
                            />
                            {m.create_block()}
                        </button>
                    </PageHeader>
                }
            >
                <Show
                    when={!loading()}
                    fallback={
                        <div
                            class="policy-stack"
                            aria-busy="true"
                            role="status"
                        >
                            {m.loading_block_configuration()}
                        </div>
                    }
                >
                    <Show
                        when={!data().engineOffline}
                        fallback={
                            <div class="engine-offline" role="status">
                                <WifiOff size={19} aria-hidden="true" />
                                <div>
                                    <strong>{m.status_engine_offline()}</strong>
                                    <p>
                                        {m.status_engine_offline_description()}
                                    </p>
                                    <a href="/getting-started">
                                        {m.getting_started()}
                                    </a>
                                </div>
                            </div>
                        }
                    >
                        <Show
                            when={!data().loadError}
                            fallback={
                                <div class="engine-offline" role="alert">
                                    <WifiOff size={19} aria-hidden="true" />
                                    <div>
                                        <strong>
                                            {m.network_blocks_load_failed_title()}
                                        </strong>
                                        <p>
                                            {m.network_blocks_load_failed_description()}
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        onClick={() => void refresh()}
                                    >
                                        {m.retry()}
                                    </button>
                                </div>
                            }
                        >
                            <Show
                                when={blocks().length > 0}
                                fallback={
                                    <>
                                        <div class="empty-policy">
                                            <div
                                                class="empty-rule"
                                                aria-hidden="true"
                                            />
                                            <div class="empty-copy">
                                                <h2>
                                                    {m.no_network_blocks_title()}
                                                </h2>
                                                <p>
                                                    {m.no_network_blocks_description()}
                                                </p>
                                                <button
                                                    type="button"
                                                    class="create-action"
                                                    onClick={handleCreate}
                                                >
                                                    <Plus
                                                        size={17}
                                                        aria-hidden="true"
                                                    />
                                                    {m.create_block()}
                                                </button>
                                            </div>
                                        </div>
                                        <div class="starter-paths">
                                            <a href="/site-packs">
                                                <Globe
                                                    size={18}
                                                    aria-hidden="true"
                                                />
                                                <span>
                                                    <strong>
                                                        {m.empty_state_packs_title()}
                                                    </strong>
                                                    <small>
                                                        {m.empty_state_packs_body()}
                                                    </small>
                                                </span>
                                                <ArrowRight
                                                    size={15}
                                                    aria-hidden="true"
                                                />
                                            </a>
                                            <a href="/site-packs?import=1">
                                                <FileUp
                                                    size={18}
                                                    aria-hidden="true"
                                                />
                                                <span>
                                                    <strong>
                                                        {m.empty_state_import_title()}
                                                    </strong>
                                                    <small>
                                                        {m.empty_state_import_body()}
                                                    </small>
                                                </span>
                                                <ArrowRight
                                                    size={15}
                                                    aria-hidden="true"
                                                />
                                            </a>
                                            <a href="/getting-started">
                                                <Lock
                                                    size={18}
                                                    aria-hidden="true"
                                                />
                                                <span>
                                                    <strong>
                                                        {m.empty_state_lock_title()}
                                                    </strong>
                                                    <small>
                                                        {m.empty_state_lock_body()}
                                                    </small>
                                                </span>
                                                <ArrowRight
                                                    size={15}
                                                    aria-hidden="true"
                                                />
                                            </a>
                                        </div>
                                    </>
                                }
                            >
                                <div class="policy-stack">
                                    <For each={blocks()}>
                                        {(block) => (
                                            <NetworkPolicy
                                                policy={block}
                                                onEdit={handleEdit}
                                                onDelete={requestDelete}
                                                onLock={handleLock}
                                                onEnabledChange={
                                                    handleEnabledChange
                                                }
                                            />
                                        )}
                                    </For>
                                </div>
                            </Show>
                        </Show>
                    </Show>
                </Show>
            </PageLayout>
            <LockModal
                open={lockModalOpen()}
                onOpenChange={setLockModalOpen}
                block={selectedBlockForLock()}
                onSave={refresh}
                onEditSchedule={handleEditSchedule}
            />
            <Show when={selectedBlockForSoftUnlock()}>
                {(block) => (
                    <SoftUnlockModal
                        open={softUnlockOpen()}
                        onOpenChange={setSoftUnlockOpen}
                        block={block()}
                    />
                )}
            </Show>
            <Show when={deleteDialogOpen()}>
                <div
                    class="fixed inset-0 z-50 grid place-items-center ace-modal-overlay p-4"
                    role="presentation"
                    onClick={(e) => {
                        if (e.currentTarget === e.target && !deleting())
                            setDeleteDialogOpen(false);
                    }}
                >
                    <section
                        class="w-full max-w-md rounded-md border border-border bg-card p-7 shadow-none"
                        role="alertdialog"
                        aria-modal="true"
                        aria-labelledby="delete-block-title"
                        aria-describedby="delete-block-description"
                        onKeyDown={(e) => {
                            if (e.key === "Escape" && !deleting())
                                setDeleteDialogOpen(false);
                        }}
                    >
                        <h2
                            id="delete-block-title"
                            class="font-condensed text-2xl font-semibold"
                        >
                            {m.delete_block_title()}
                        </h2>
                        <p
                            id="delete-block-description"
                            class="mt-3 text-sm text-muted-foreground"
                        >
                            {m.delete_block_description({
                                name: blockToDelete()?.name ?? "",
                            })}
                        </p>
                        <div class="mt-6 flex justify-end gap-2">
                            <button
                                type="button"
                                class="rounded-sm border border-border px-4 py-2 text-sm"
                                disabled={deleting()}
                                onClick={() => setDeleteDialogOpen(false)}
                            >
                                {m.cancel()}
                            </button>
                            <button
                                type="button"
                                class="rounded-sm bg-destructive px-4 py-2 text-sm text-destructive-foreground hover:bg-destructive/90"
                                disabled={deleting()}
                                onClick={() => void confirmDelete()}
                            >
                                {deleting() ? m.deleting() : m.confirm()}
                            </button>
                        </div>
                    </section>
                </div>
            </Show>
        </div>
    );
}
