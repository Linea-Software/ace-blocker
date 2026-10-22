import {
    createNetworkBlock,
    renameNetworkBlock,
    setNetworkBlockIcon,
    setNetworkBlockSafeSearch,
    type NetworkBlockSchema,
} from "$engine-client";

/**
 * Network Block save is deliberately NOT a cross-resource transaction.
 * Each engine mutation has its own authorization/commit boundary. Execute in
 * order and report ambiguous/partial outcomes rather than claiming rollback.
 */
export type BlockSaveDraft = {
    block: NetworkBlockSchema | null;
    name: string;
    icon: string | null;
    safeSearch: boolean;
    rules: { changes: readonly unknown[]; pushChangesToServer(id: string): Promise<void> };
    schedule: { changes: readonly unknown[]; pushChangesToServer(id: string): Promise<void> };
    packs: { changes: readonly unknown[]; pushChangesToServer(id: string): Promise<void> };
};

export type BlockSaveResult =
    | { kind: "saved"; id: string }
    | { kind: "failed"; id?: string; attempted: boolean; error: unknown };

export async function saveNetworkBlock(draft: BlockSaveDraft): Promise<BlockSaveResult> {
    let id = draft.block?.id;
    let attempted = false;
    try {
        if (!id) {
            attempted = true;
            const result = await createNetworkBlock({
                body: { blockName: draft.name, icon: draft.icon },
            });
            if (result.error) throw result.error;
            if (!result.data?.networkBlock?.id) {
                throw new Error("Network Block creation returned no id");
            }
            id = result.data.networkBlock.id;
        } else {
            if (draft.name !== draft.block?.name) {
                attempted = true;
                const result = await renameNetworkBlock({
                    path: { network_block_id: id },
                    body: { newName: draft.name },
                });
                if (result.error) throw result.error;
            }

            if (draft.icon !== (draft.block?.icon ?? null)) {
                attempted = true;
                const result = await setNetworkBlockIcon({
                    path: { network_block_id: id },
                    body: { icon: draft.icon },
                });
                if (result.error) throw result.error;
            }
        }

        // Later batches must not begin after an earlier failure. A delta may
        // itself have partially committed (or unsuccessfully rolled back).
        for (const delta of [draft.rules, draft.schedule, draft.packs]) {
            if (delta.changes.length === 0) continue;
            attempted = true;
            await delta.pushChangesToServer(id);
        }

        if (!draft.block || draft.safeSearch !== draft.block.safeSearch) {
            attempted = true;
            const result = await setNetworkBlockSafeSearch({
                path: { network_block_id: id },
                body: { enabled: draft.safeSearch },
            });
            if (result.error) throw result.error;
        }

        return { kind: "saved", id };
    } catch (error) {
        // A failed RPC can have committed before the transport failed;
        // callers must reconcile even when this is the first operation.
        return { kind: "failed", id, attempted, error };
    }
}
