import { describe, expect, it } from "vitest";
import { createRoot, createMemo, flush } from "solid-js";
import type { UserDefinedRuleSchema, ScheduleEntrySchema } from "$engine-client";
import { SitePacksConnectionDelta } from "./site-packs-connection-delta";
import { UserDefinedRulesDelta } from "./user-defined-rules-delta";
import { ScheduleChanges } from "./schedule-delta";

function initialRule(id: string, rule: string): UserDefinedRuleSchema {
    return {id, networkBlockId: "block-1", rule, ruleType: "domain"};
}

function initialSchedule(id: string): ScheduleEntrySchema {
    return {
        id, networkBlockId: "block-1",
        startTime: "09:00", endTime: "17:00",
        monday: true, tuesday: false, wednesday: false, thursday: false,
        friday: false, saturday: false, sunday: false,
    };
}

describe("Solid 2 staged site pack connections", () => {
    it("tracks immutable changes and cancellable temporary additions", () => {
        const delta = new SitePacksConnectionDelta(["existing"]);
        const before = delta.currentConnectedListIds;
        delta.addConnection("temporary");
        expect(delta.currentConnectedListIds).toEqual(["existing", "temporary"]);
        expect(delta.currentConnectedListIds).not.toBe(before);
        expect(delta.hasChanges()).toBe(true);
        expect(delta.isNewConnection("temporary")).toBe(true);
        delta.removeTempConnection("temporary");
        expect(delta.currentConnectedListIds).toEqual(["existing"]);
        expect(delta.hasChanges()).toBe(false);
    });

    it("orders removals before additions without mutating original queue", () => {
        const delta = new SitePacksConnectionDelta(["free"]);
        delta.addConnection("pro");
        delta.removeConnection("free");
        expect(delta.getChanges().map(change => change.type)).toEqual(["REMOVE", "ADD"]);
        expect(delta.changes.map(change => change.type)).toEqual(["ADD", "REMOVE"]);
    });

    it("tracks updated state in Solid computations", () => {
        const {delta, count, dispose} = createRoot(dispose => {
            const delta = new SitePacksConnectionDelta(["alpha"]);
            const count = createMemo(() => delta.currentConnectedListIds.length);
            return {delta, count, dispose};
        });
        expect(count()).toBe(1);
        delta.addConnection("beta");
        flush();
        expect(count()).toBe(2);
        delta.removeConnection("alpha");
        flush();
        expect(count()).toBe(1);
        dispose();
    });
});

describe("Solid 2 staged user-defined rules", () => {
    it("rejects duplicate rules and can undo unsaved rules", () => {
        const delta = new UserDefinedRulesDelta([initialRule("stable", "old.example")]);
        delta.addRule("new.example", "domain");
        delta.addRule("new.example", "domain");
        expect(delta.currentRules.map(item => item.rule)).toEqual(["old.example", "new.example"]);
        const created = delta.currentRules.find(item => item.rule === "new.example")!;
        delta.removeTempRule(created.id as never);
        expect(delta.currentRules.map(item => item.rule)).toEqual(["old.example"]);
        expect(delta.hasChanges()).toBe(false);
    });

    it("stages deletion of saved rules and preserves the remaining rule", () => {
        const delta = new UserDefinedRulesDelta([initialRule("one", "one.example"), initialRule("two", "two.example")]);
        delta.removeRule("one");
        expect(delta.currentRules.map(item => item.id)).toEqual(["two"]);
        expect(delta.changes).toContainEqual({type: "DELETE", userRuleId: "one"});
    });
});

describe("Solid 2 staged schedule changes", () => {
    it("preserves original entries, stages changes, and handles temporary removal", () => {
        const previous = initialSchedule("persistent");
        const delta = new ScheduleChanges([previous], false);
        delta.addScheduleEntry(
            {monday: false, tuesday: true, wednesday: false, thursday: false,
                friday: false, saturday: false, sunday: false},
            {hour: 11, minute: 0}, {hour: 13, minute: 30},
        );
        expect(delta.currentScheduleEntries).toHaveLength(2);
        expect(previous.startTime).toBe("09:00");
        const added = delta.currentScheduleEntries.find(item => item.id !== "persistent")!;
        expect(added.startTime).toBe("11:00");
        expect(added.endTime).toBe("13:30");
        delta.removeTempScheduleEntry(added.id as never);
        expect(delta.currentScheduleEntries).toEqual([previous]);
        expect(delta.changes).toHaveLength(0);
    });

    it("retains coverage requirements when the original block is locked", () => {
        const delta = new ScheduleChanges([initialSchedule("existing")], true);
        expect(delta.isValidForLockedBlock()).toBe(true);
        const attempt = delta.removeScheduleEntry("existing");
        // Locked schedules cannot be relaxed: a rejected removal must leave
        // the original coverage and pending-delta queue unchanged.
        expect(attempt).toBeDefined();
        expect(delta.currentScheduleEntries).toHaveLength(1);
        expect(delta.changes).toHaveLength(0);
        expect(delta.isValidForLockedBlock()).toBe(true);
    });
});
