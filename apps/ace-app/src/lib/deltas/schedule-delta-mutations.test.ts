import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ScheduleChanges, type Weekday } from "./schedule-delta.svelte";

const monday: Weekday = {
    monday: true, tuesday: false, wednesday: false, thursday: false,
    friday: false, saturday: false, sunday: false,
};

beforeEach(() => {
    // Svelte runes are compiled by SvelteKit in the application; the draft's
    // plain array state is sufficient to check its validation before RPC.
    vi.stubGlobal("$state", <T>(value: T): T => value);
});
afterEach(() => vi.unstubAllGlobals());

describe("schedule delta rejects unsupported engine intervals before staging", () => {
    it("rejects 22:00–02:00 without changing the draft", () => {
        const draft = new ScheduleChanges();
        const result = draft.addScheduleEntry(monday, { hour: 22, minute: 0 }, { hour: 2, minute: 0 });
        expect(draft.changes).toEqual([]);
        expect(draft.currentScheduleEntries).toEqual([]);
        expect(result).toBeDefined();
    });

    it("accepts an end-at-midnight interval", () => {
        const draft = new ScheduleChanges();
        draft.addScheduleEntry(monday, { hour: 22, minute: 0 }, { hour: 0, minute: 0 });
        expect(draft.currentScheduleEntries).toHaveLength(1);
        expect(draft.currentScheduleEntries[0].endTime).toBe("00:00");
    });

    it("rejects an invalid update without changing the original entry", () => {
        const original = {
            networkBlockId: "block", id: "schedule-1", ...monday,
            startTime: "09:00", endTime: "17:00",
        };
        const draft = new ScheduleChanges([original]);
        const result = draft.updateScheduleEntry(
            original.id, monday,
            { startTime: { hour: 21, minute: 0 }, endTime: { hour: 3, minute: 0 } },
        );
        expect(draft.changes).toEqual([]);
        expect(draft.currentScheduleEntries).toEqual([original]);
        expect(result).toBeDefined();
    });
});
