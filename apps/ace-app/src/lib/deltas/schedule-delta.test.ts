import { describe, expect, it } from "vitest";
import type { ScheduleEntrySchema } from "$engine-client";
import { isStrictlyMoreRestrictiveThan, isSupportedSchedulePeriod } from "./schedule-coverage";

type Day = "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday";
function entry(startTime: string, endTime: string, ...days: Day[]): ScheduleEntrySchema {
    return {
        id: "test",
        networkBlockId: "block",
        startTime,
        endTime,
        monday: days.includes("monday"),
        tuesday: days.includes("tuesday"),
        wednesday: days.includes("wednesday"),
        thursday: days.includes("thursday"),
        friday: days.includes("friday"),
        saturday: days.includes("saturday"),
        sunday: days.includes("sunday"),
    };
}

describe("locked schedule preview coverage", () => {
    it.each([
        ["unchanged daytime", [entry("09:00", "11:00", "monday")], [entry("09:00", "11:00", "monday")], true],
        ["contiguous union", [entry("09:00", "11:00", "monday")], [entry("09:00", "10:00", "monday"), entry("10:00", "11:00", "monday")], true],
        ["removed interval", [entry("09:00", "11:00", "monday")], [], false],
        ["missing last minute", [entry("09:00", "11:00", "monday")], [entry("09:00", "10:59", "monday")], false],
        ["midnight end missing", [entry("22:00", "00:00", "monday")], [], false],
        ["midnight end retained", [entry("22:00", "00:00", "monday")], [entry("21:00", "00:00", "monday")], true],
        ["full day", [entry("00:00", "00:00", "tuesday")], [entry("09:00", "17:00", "tuesday")], false],
        ["Sunday does not spill into Monday", [entry("22:00", "00:00", "sunday")], [entry("22:00", "00:00", "monday")], false],
        ["Sunday to Sunday coverage", [entry("22:00", "00:00", "sunday")], [entry("22:00", "00:00", "sunday")], true],
        ["empty original", [], [], true],
        ["invalid overnight draft", [entry("09:00", "10:00", "monday")], [entry("22:00", "02:00", "monday")], false],
    ] as const)("%s", (_name, before, after, expected) => {
        expect(isStrictlyMoreRestrictiveThan([...before], [...after])).toBe(expected);
    });
});

describe("engine-compatible schedule interval validation", () => {
    it.each([
        ["09:00", "17:00", true],
        ["22:00", "00:00", true],
        ["00:00", "00:00", true],
        ["09:00", "09:00", true],
        ["22:00", "02:00", false],
        ["00:00", "23:00", true],
        ["9:00", "17:00", false],
        ["24:00", "00:00", false],
        ["09:60", "10:00", false],
        ["bad", "10:00", false],
    ])("%s to %s allowed: %s", (start, end, allowed) => {
        expect(isSupportedSchedulePeriod(start, end)).toBe(allowed);
    });
});
