import {
    saveNetworkBlock as saveNetworkBlockRpc,
    type BaseScheduleEntrySchema,
    type NetworkBlockSchema,
    type ScheduleMode,
    type UserDefinedRule,
} from "$engine-client";

/** UI edits are local drafts until the single atomic engine save RPC. */
export type BlockSaveDraft = {
    block: NetworkBlockSchema | null;
    name: string;
    icon: string | null;
    safeSearch: boolean;
    scheduleMode: ScheduleMode;
    rules: { currentRules: readonly UserDefinedRule[] };
    schedule: { currentScheduleEntries: readonly BaseScheduleEntrySchema[] };
    packs: { currentConnectedListIds: readonly string[] };
};

export type BlockSaveResult =
    | { kind: "saved"; block: NetworkBlockSchema }
    | { kind: "failed"; error: unknown; uncertain: boolean };

export async function saveNetworkBlock(draft: BlockSaveDraft): Promise<BlockSaveResult> {
    try {
        const result = await saveNetworkBlockRpc({
            body: {
                id: draft.block?.id ?? null,
                name: draft.name,
                icon: draft.icon,
                safeSearch: draft.safeSearch,
                scheduleMode: draft.scheduleMode,
                rules: draft.rules.currentRules.map(({ rule, ruleType }) => ({
                    rule,
                    ruleType,
                })),
                scheduleEntries: draft.schedule.currentScheduleEntries.map(
                    ({ startTime, endTime, monday, tuesday, wednesday, thursday, friday, saturday, sunday }) => ({
                        startTime, endTime, monday, tuesday, wednesday, thursday, friday, saturday, sunday,
                    }),
                ),
                sitePackIds: [...draft.packs.currentConnectedListIds],
            },
        });

        if (result.error) {
            // Engine errors reject the entire SQLite transaction. Transport
            // loss, however, can happen after the server has committed.
            return {
                kind: "failed",
                error: result.error,
                uncertain: result.error.code === "transport_unavailable",
            };
        }
        if (!result.data?.networkBlock) {
            return { kind: "failed", error: new Error("Missing save acknowledgement"), uncertain: true };
        }
        return { kind: "saved", block: result.data.networkBlock };
    } catch (error) {
        return { kind: "failed", error, uncertain: true };
    }
}
