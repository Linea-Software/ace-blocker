import type { NetworkBlockSnapshot } from "$engine-client";
import { findNextScheduleTransition } from "./schedule-transitions";

export type BlockTransition = {
    block: NetworkBlockSnapshot;
    kind: "start" | "end";
    at: Date;
};

/**
 * Upcoming real enforcement changes, not hypothetical transitions for
 * disabled blocks. Include multiple changes from one block so a dashboard
 * with just one active schedule can still describe what comes next.
 */
export function findUpcomingBlockTransitions(
    blocks: NetworkBlockSnapshot[],
    from = new Date(),
    limit = 4,
): BlockTransition[] {
    const candidates: BlockTransition[] = [];
    for (const block of blocks) {
        if (!block.block.enabled || block.block.scheduleMode === "always" || block.scheduleEntries.length === 0) continue;
        let cursor = new Date(from);
        for (let i = 0; i < Math.min(limit, 4); i += 1) {
            const transition = findNextScheduleTransition(block.scheduleEntries, cursor, 7);
            if (!transition) break;
            candidates.push({ block, ...transition });
            cursor = new Date(transition.at.getTime() + 60_000);
        }
    }
    return candidates.sort((a, b) => a.at.getTime() - b.at.getTime()).slice(0, limit);
}
