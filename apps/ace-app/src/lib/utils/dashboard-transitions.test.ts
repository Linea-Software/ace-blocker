import { describe, expect, it } from "vitest";
import type { NetworkBlockSnapshot, ScheduleEntrySchema } from "$engine-client";
import { findUpcomingBlockTransitions } from "./dashboard-transitions";

const schedule = {
    monday: true,
    tuesday: false,
    wednesday: false,
    thursday: false,
    friday: false,
    saturday: false,
    sunday: false,
    startTime: "09:00",
    endTime: "10:00",
} as ScheduleEntrySchema;

function snapshot(name: string, enabled: boolean, entries: ScheduleEntrySchema[] = [schedule]): NetworkBlockSnapshot {
    return {
        block: {
            id: name,
            name,
            enabled,
            scheduleMode: "scheduled",
            isActive: false,
            lockState: { state: "inactive" },
        },
        scheduleEntries: entries,
    } as NetworkBlockSnapshot;
}

describe("Control upcoming changes", () => {
    const from = new Date(2026, 10, 2, 8, 59);

    it("ignores disabled and unscheduled blocks", () => {
        expect(findUpcomingBlockTransitions([
            snapshot("Disabled", false),
            snapshot("Unscheduled", true, []),
        ], from)).toEqual([]);
    });

    it("lists both the start and end of one enabled block in chronological order", () => {
        const results = findUpcomingBlockTransitions([
            snapshot("Ignored", false),
            snapshot("Work", true),
        ], from, 2);
        expect(results).toHaveLength(2);
        expect(results.map((result) => result.kind)).toEqual(["start", "end"]);
        expect(results.map((result) => result.block.block.name)).toEqual(["Work", "Work"]);
        expect(results[0].at).toEqual(new Date(2026, 10, 2, 9, 0));
        expect(results[1].at).toEqual(new Date(2026, 10, 2, 10, 0));
    });

    it("does not invent start/end transitions for Always blocks with historical periods", () => {
        const always = snapshot("Always", true);
        always.block.scheduleMode = "always";
        expect(always.scheduleEntries).toHaveLength(1);
        expect(findUpcomingBlockTransitions([always], from)).toEqual([]);
    });

    it("limits results and never returns transitions for disabled blocks", () => {
        expect(findUpcomingBlockTransitions([snapshot("Work", true)], from, 1)).toHaveLength(1);
        expect(findUpcomingBlockTransitions([], from)).toEqual([]);
    });
});
