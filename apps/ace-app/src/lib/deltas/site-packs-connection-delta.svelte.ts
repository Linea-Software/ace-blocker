/**
 * SITE PACKS DELTA
 *
 * This file contains the logic for managing the state of changes to the Site Packs used by a Network Block temporarily.
 * IT DOES NOT HANDLE CREATION OR DELETION OF SITE PACKS.
 * Site Packs are large databases of domain names (backed by RocksDB in the engine) used for blocking or allowing content.
 *
 * The Delta pattern is used here to accumulate changes (additions/removals) in the UI state
 * before committing them to the backend API.
 *
 * See `docs/engine-docs.md` for more information on Site Packs and the Ace Engine architecture.
 */
import {
    addNetBlockToSitePackConnection,
    removeNetBlockToSitePackConnection,
} from "$engine-client";
import { withRollback, type AppliedOp } from "./common";

/**
 * Represents a specific change operation on the Site Packs configuration.
 * Can be an addition of a new pack or removal of an existing/temp pack.
 */
export type SitePacksConnectionChange =
    | { type: "ADD"; listId: string }
    | { type: "REMOVE"; listId: string };

/**
 * Manages the state of pending changes for Site Packs.
 * Allows adding and removing packs, and synchronizing these changes with the backend.
 */
export class SitePacksConnectionDelta {
    /**
     * Staged changes that haven't been pushed to the server yet.
     */
    public changes: SitePacksConnectionChange[] = $state([]);

    /**
     * The live state of connected pack IDs.
     */
    private currentState: string[] = $state([]);
    private initialListIds: string[] = [];

    constructor(initialListIds: string[] = []) {
        this.currentState = [...initialListIds];
        this.initialListIds = [...initialListIds];
    }

    /**
     * Checks if a pack was initially connected (persistent).
     */
    public isInitialConnection(listId: string): boolean {
        return this.initialListIds.includes(listId);
    }

    /**
     * Checks if a pack is a newly added temporary connection.
     */
    public isNewConnection(listId: string): boolean {
        return this.changes.some(
            (c) => c.type === "ADD" && c.listId === listId,
        );
    }

    /**
     * Reactive getter for the current connected pack IDs.
     */
    public get currentConnectedListIds(): string[] {
        return this.currentState;
    }

    /**
     * Checks if a pack ID is currently in the connected state.
     */
    public isConnected(listId: string): boolean {
        return this.currentState.includes(listId);
    }

    /**
     * Stages a new Site Pack to be added.
     * @param listId The persistent ID of the site pack to add.
     */
    public addConnection(listId: string) {
        if (this.currentState.includes(listId)) return;

        this.changes.push({ type: "ADD", listId });
        this.currentState.push(listId);
    }

    /**
     * Stages an existing Site Pack for removal.
     * @param listId The persistent ID of the site pack to remove.
     */
    public removeConnection(listId: string) {
        if (!this.currentState.includes(listId)) return;

        this.changes.push({ type: "REMOVE", listId });
        this.currentState = this.currentState.filter((id) => id !== listId);
    }

    /**
     * Removes a pending addition of a Site Pack.
     * @param listId The ID of the site pack to unstage.
     */
    public removeTempConnection(listId: string) {
        // Remove the ADD change
        this.changes = this.changes.filter(
            (c) => !(c.type === "ADD" && c.listId === listId),
        );
        // Remove from current state
        this.currentState = this.currentState.filter((id) => id !== listId);
    }

    /**
     * Retrieves the sorted list of pending changes.
     * Removals are prioritized over Additions to ensure correct processing order.
     */
    public getChanges() {
        // REMOVE SHOULD ALWAYS COME BEFORE ADD
        return [...this.changes].sort((a, b) => {
            if (a.type === "ADD" && b.type === "REMOVE") return 1;
            if (a.type === "REMOVE" && b.type === "ADD") return -1;
            return 0;
        });
    }

    /**
     * Checks if there are any pending changes to be pushed.
     */
    public hasChanges(): boolean {
        return this.changes.length > 0;
    }

    /**
     * Commits all pending changes to the backend API for a specific Network Block.
     * Executes removals first, then additions.
     * Failed operations are rolled back so the server stays consistent.
     * @param networkListId The ID of the Network Block to update.
     */
    public async pushChangesToServer(networkListId: string) {
        if (!this.hasChanges()) return;
        const changes = this.getChanges();

        await withRollback(async (ops: AppliedOp[]) => {
            for (const change of changes) {
                if (change.type === "REMOVE") {
                    // Determine if this pack was originally connected
                    const wasOriginallyConnected =
                        this.initialListIds.includes(change.listId);

                    const { error } =
                        await removeNetBlockToSitePackConnection({
                            path: {
                                network_block_id: networkListId,
                                site_pack_id: change.listId,
                            },
                        });
                    if (error) {
                        console.error(
                            "Failed to remove site pack connection:",
                            error,
                        );
                        throw new Error(
                            "Failed to remove site pack connection",
                        );
                    }

                    // Register rollback: re-add the connection
                    ops.push({
                        label: `remove-pack-${change.listId}`,
                        rollback: async () => {
                            const { error: rbErr } =
                                await addNetBlockToSitePackConnection({
                                    path: {
                                        network_block_id: networkListId,
                                    },
                                    body: {
                                        sitesListId: change.listId,
                                    },
                                });
                            return !rbErr;
                        },
                    });
                }

                if (change.type === "ADD") {
                    const { error } =
                        await addNetBlockToSitePackConnection({
                            path: {
                                network_block_id: networkListId,
                            },
                            body: {
                                sitesListId: change.listId,
                            },
                        });
                    if (error) {
                        console.error(
                            "Failed to add site pack connection:",
                            error,
                        );
                        throw new Error(
                            "Failed to add site pack connection",
                        );
                    }

                    // Register rollback: remove the newly added connection
                    ops.push({
                        label: `add-pack-${change.listId}`,
                        rollback: async () => {
                            const { error: rbErr } =
                                await removeNetBlockToSitePackConnection(
                                    {
                                        path: {
                                            network_block_id:
                                                networkListId,
                                            site_pack_id: change.listId,
                                        },
                                    },
                                );
                            return !rbErr;
                        },
                    });
                }
            }
        });

        // Clear changes only after successful push
        this.changes = [];
        // Sync initial state with current state
        this.initialListIds = [...this.currentState];
    }
}
