<script lang="ts">
    import type { PageData } from "./$types";
    import * as m from "$lib/paraglide/messages.js";
    import {
        Plus,
        Globe,
        FileUp,
        Lock,
        WifiOff,
        ArrowRight,
    } from "@lucide/svelte";
    import BlockModal from "./_BlockModal.svelte";
    import LockModal from "./_components/_LockModal.svelte";
    import SoftUnlockModal from "./_components/_SoftUnlockModal.svelte";
    import NetworkPolicy from "$lib/components/ace/domain/NetworkPolicy.svelte";
    import { invalidateAll } from "$app/navigation";
    import { resolve } from "$app/paths";
    import type { NetworkBlockSchema } from "$engine-client";
    import { deleteNetworkBlock, setNetworkBlockEnabled } from "$engine-client";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { toast } from "svelte-sonner";
    import { onMount } from "svelte";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";

    let { data }: { data: PageData } = $props();

    let isEditorOpen = $state(false);
    let selectedBlock = $state<NetworkBlockSchema | null>(null);

    function blockPriority(block: (typeof data.blocks)[number]): number {
        if (block.enabled && block.isActive) return 0;
        if (block.lockState.state !== "inactive") return 1;
        if ((block.schedules?.length ?? 0) > 0) return 2;
        return 3;
    }

    const blocks = $derived(
        [...data.blocks].sort(
            (a, b) =>
                blockPriority(a) - blockPriority(b) ||
                a.name.localeCompare(b.name),
        ),
    );

    onMount(() => {
        const blockId = new URL(window.location.href).searchParams.get("block");
        if (!blockId) return;
        const match = blocks.find((block) => block.id === blockId);
        if (!match) return;
        selectedBlock = match;
        isEditorOpen = true;
    });

    let isLockModalOpen = $state(false);
    let selectedBlockForLock = $state<NetworkBlockSchema | null>(null);

    let softUnlockOpen = $state(false);
    let selectedBlockForSoftUnlock = $state<NetworkBlockSchema | null>(null);

    let isDeleteDialogOpen = $state(false);
    let blockToDelete = $state<NetworkBlockSchema | null>(null);
    let isDeleting = $state(false);

    function handleCreate() {
        selectedBlock = null;
        isEditorOpen = true;
    }

    function handleEdit(block: NetworkBlockSchema) {
        selectedBlock = block;
        isEditorOpen = true;
    }

    async function handleEditorSave() {
        await invalidateAll();
    }

    async function handleEnabledChange(block: NetworkBlockSchema, enabled: boolean) {
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
                if (error.status === 403) await invalidateAll();
                return;
            }

            await invalidateAll();
        } catch (error) {
            console.error("Failed to update network block enabled state:", error);
            toast.error(m.failed_to_sync());
        }
    }

    function handleLock(block: NetworkBlockSchema) {
        if (block.lockState.state === "soft_unlockable") {
            selectedBlockForSoftUnlock = block;
            softUnlockOpen = true;
            return;
        }

        selectedBlockForLock = block;
        isLockModalOpen = true;
    }

    async function handleLockModalSave() {
        await invalidateAll();
    }

    function requestDelete(block: NetworkBlockSchema) {
        if (block.lockState.state !== "inactive" || block.isLocked) return;
        blockToDelete = block;
        isDeleteDialogOpen = true;
    }

    async function confirmDelete() {
        if (!blockToDelete || isDeleting) return;

        isDeleting = true;
        const { error } = await deleteNetworkBlock({
            path: { network_block_id: blockToDelete.id },
        });

        if (error) {
            console.error("Failed to delete network block:", error);
            toast.error(m.failed_to_sync());
        } else {
            await invalidateAll();
        }

        isDeleting = false;
        isDeleteDialogOpen = false;
        blockToDelete = null;
    }
</script>

<svelte:head>
    <title>{m.network_blocks()} · {m.app_name()}</title>
</svelte:head>

<PageLayout>
    {#snippet header()}
        <PageHeader
            title={m.network_blocks()}
            subtitle={m.network_blocks_description()}
        >
            <button
                class="create-action"
                type="button"
                data-tour="create-block"
                disabled={data.engineOffline}
                title={data.engineOffline ? m.status_engine_offline_description() : undefined}
                onclick={handleCreate}
            >
                <Plus size={17} strokeWidth={2} aria-hidden="true" />
                {m.create_block()}
            </button>
        </PageHeader>
    {/snippet}

    {#if data.engineOffline}
        <div class="engine-offline" role="status">
            <WifiOff size={19} aria-hidden="true" />
            <div>
                <strong>{m.status_engine_offline()}</strong>
                <p>{m.status_engine_offline_description()}</p>
                <a href={resolve("/getting-started")}>{m.getting_started()}</a>
            </div>
        </div>
    {:else if data.loadError}
        <div class="engine-offline" role="alert">
            <WifiOff size={19} aria-hidden="true" />
            <div>
                <strong>{m.network_blocks_load_failed_title()}</strong>
                <p>{m.network_blocks_load_failed_description()}</p>
            </div>
            <button type="button" onclick={() => invalidateAll()}>{m.retry()}</button>
        </div>
    {:else if blocks.length > 0}
        <div class="policy-stack">
            {#each blocks as block (block.id)}
                <NetworkPolicy
                    policy={block}
                    onEdit={handleEdit}
                    onDelete={requestDelete}
                    onLock={handleLock}
                    onEnabledChange={handleEnabledChange}
                />
            {/each}
        </div>
    {:else}
        <div class="empty-policy">
            <div class="empty-rule" aria-hidden="true"></div>
            <div class="empty-copy">
                <h2>{m.no_network_blocks_title()}</h2>
                <p>{m.no_network_blocks_description()}</p>
                <button type="button" class="create-action" onclick={handleCreate}>
                    <Plus size={17} aria-hidden="true" />
                    {m.create_block()}
                </button>
            </div>
        </div>

        <div class="starter-paths">
            <a href={resolve("/site-packs")}>
                <Globe size={18} aria-hidden="true" />
                <span>
                    <strong>{m.empty_state_packs_title()}</strong>
                    <small>{m.empty_state_packs_body()}</small>
                </span>
                <ArrowRight size={15} aria-hidden="true" />
            </a>
            <a
                href={resolve("/site-packs")}
                onclick={(event) => {
                    event.preventDefault();
                    window.location.assign(resolve("/site-packs") + "?import=1");
                }}
            >
                <FileUp size={18} aria-hidden="true" />
                <span>
                    <strong>{m.empty_state_import_title()}</strong>
                    <small>{m.empty_state_import_body()}</small>
                </span>
                <ArrowRight size={15} aria-hidden="true" />
            </a>
            <a href={resolve("/getting-started")}>
                <Lock size={18} aria-hidden="true" />
                <span>
                    <strong>{m.empty_state_lock_title()}</strong>
                    <small>{m.empty_state_lock_body()}</small>
                </span>
                <ArrowRight size={15} aria-hidden="true" />
            </a>
        </div>
    {/if}
</PageLayout>

{#if isEditorOpen}
    <BlockModal
        bind:open={isEditorOpen}
        block={selectedBlock}
        onSave={handleEditorSave}
    />
{/if}

<LockModal
    bind:open={isLockModalOpen}
    block={selectedBlockForLock}
    onSave={handleLockModalSave}
/>

{#if selectedBlockForSoftUnlock}
    <SoftUnlockModal
        bind:open={softUnlockOpen}
        block={selectedBlockForSoftUnlock}
    />
{/if}

<AlertDialog.Root bind:open={isDeleteDialogOpen}>
    <AlertDialog.Content class="rounded-md border-border bg-card p-7 shadow-none">
        <AlertDialog.Header class="gap-3">
            <AlertDialog.Title class="font-condensed text-2xl font-semibold">
                {m.delete_block_title()}
            </AlertDialog.Title>
            <AlertDialog.Description class="text-sm">
                {m.delete_block_description({
                    name: blockToDelete?.name ?? "",
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
                {isDeleting ? m.deleting() : m.confirm()}
            </AlertDialog.Action>
        </AlertDialog.Footer>
    </AlertDialog.Content>
</AlertDialog.Root>

<style>
    h2 {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .create-action {
        min-height: 40px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        border: 1px solid var(--foreground);
        border-radius: 4px;
        padding: 0 14px;
        background: var(--foreground);
        color: var(--background);
        font-size: 13px;
        font-weight: 600;
        cursor: pointer;
        transition:
            opacity 140ms ease,
            transform 140ms ease;
    }

    .create-action:hover:not(:disabled) {
        opacity: 0.86;
    }

    .create-action:disabled {
        cursor: not-allowed;
        opacity: 0.4;
    }

    .create-action:active {
        transform: translateY(1px);
    }

    .policy-stack {
        display: grid;
        gap: 12px;
    }

    .engine-offline {
        display: flex;
        gap: 12px;
        padding: 16px 18px;
        border: 1px solid color-mix(in srgb, var(--ace-danger) 40%, var(--border));
        border-left: 5px solid var(--ace-danger);
        border-radius: 5px;
        background: var(--card);
    }

    .engine-offline strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
    }

    .engine-offline p {
        margin: 3px 0 0;
        color: var(--muted-foreground);
        font-size: 13px;
    }

    .engine-offline a {
        display: inline-block;
        margin-top: 12px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-underline-offset: 3px;
    }

    .empty-policy {
        display: grid;
        grid-template-columns: 8px 1fr;
        min-height: 210px;
        border: 1px solid var(--border);
        border-radius: 7px;
        background: var(--card);
        overflow: hidden;
    }

    .empty-rule {
        background: var(--ace-ash);
    }

    .empty-copy {
        align-self: center;
        max-width: 660px;
        padding: 30px 34px;
    }

    .empty-copy h2 {
        margin: 0;
        font-size: 27px;
        font-weight: 600;
    }

    .empty-copy p {
        margin: 7px 0 20px;
        color: var(--muted-foreground);
        font-size: 14px;
        line-height: 1.55;
    }

    .starter-paths {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        margin-top: 14px;
        border: 1px solid var(--border);
        border-radius: 5px;
        overflow: hidden;
    }

    .starter-paths a {
        min-width: 0;
        display: grid;
        grid-template-columns: auto 1fr auto;
        align-items: start;
        gap: 10px;
        padding: 15px;
        color: var(--foreground);
        text-decoration: none;
        background: color-mix(in srgb, var(--card) 62%, transparent);
        transition: background 140ms ease;
    }

    .starter-paths a + a {
        border-left: 1px solid var(--border);
    }

    .starter-paths a:hover {
        background: var(--card);
    }

    .starter-paths strong,
    .starter-paths small {
        display: block;
    }

    .starter-paths strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 14px;
        font-weight: 600;
    }

    .starter-paths small {
        margin-top: 3px;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.35;
    }

    @container page (max-width: 760px) {
        .starter-paths {
            grid-template-columns: 1fr;
        }

        .starter-paths a + a {
            border-top: 1px solid var(--border);
            border-left: 0;
        }
    }

    @container page (max-width: 560px) {
        .create-action {
            align-self: flex-start;
        }
    }
</style>
