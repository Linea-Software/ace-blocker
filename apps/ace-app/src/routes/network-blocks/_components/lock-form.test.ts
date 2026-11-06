import { describe, expect, it } from "vitest";
import { buildLockPayload, isUnchangedLock, parseLockEnd, validateLockDraft, type LockDraft } from "./lock-form";

const now = new Date(2026, 10, 5, 12, 0);
const format = (d: Date) => [
    d.getFullYear(), "-", String(d.getMonth() + 1).padStart(2, "0"),
    "-", String(d.getDate()).padStart(2, "0"), "T",
    String(d.getHours()).padStart(2, "0"), ":",
    String(d.getMinutes()).padStart(2, "0"), ":00",
].join("");

function draft(overrides: Partial<LockDraft> = {}): LockDraft {
    return {
        enabled: true,
        type: "soft_unlockable",
        end: "",
        original: { state: "inactive" },
        scheduleCount: 1,
        isCurrentlyLocked: false,
        now,
        ...overrides,
    };
}

describe("lock form invariants", () => {
    it("never sends a lock when its enable switch is off", () => {
        expect(buildLockPayload(draft({ enabled: false }))).toEqual({ lockState: "inactive" });
        expect(isUnchangedLock({ state: "inactive" }, { lockState: "inactive" })).toBe(true);
    });

    it("cannot remove an actively enforced lock", () => {
        expect(validateLockDraft(draft({
            enabled: false,
            original: { state: "active_during_schedule" },
            isCurrentlyLocked: true,
        }))).toBe("lock_cannot_be_removed");
        expect(buildLockPayload(draft({
            enabled: false, original: { state: "soft_unlockable" }, isCurrentlyLocked: true,
        }))).toBeNull();
    });

    it("requires a schedule before activating a schedule lock", () => {
        expect(validateLockDraft(draft({ type: "active_during_schedule", scheduleCount: 0 })))
            .toBe("schedule_required");
    });

    it("requires a real future time and enforces the engine's 90-day limit", () => {
        const timed = { type: "active_until_lock_end" as const };
        expect(validateLockDraft(draft(timed))).toBe("date_required");
        expect(validateLockDraft(draft({ ...timed, end: "2026-02-30T10:00:00" })))
            .toBe("date_invalid");
        expect(validateLockDraft(draft({ ...timed, end: "2026-11-05T11:00:00" })))
            .toBe("date_future");
        expect(validateLockDraft(draft({
            ...timed, end: format(new Date(now.getTime() + 91 * 86400_000)),
        }))).toBe("date_limit");
        expect(buildLockPayload(draft({
            ...timed, end: "2026-11-06T12:00:00",
        }))).toEqual({
            lockState: "active_until_lock_end", lockEnd: new Date("2026-11-06T12:00:00").toISOString(),
        });
    });

    it("rejects equal or earlier time-lock deadlines", () => {
        const original = { state: "active_until_lock_end", end: "2026-11-09T12:00:00" };
        expect(validateLockDraft(draft({
            type: "active_until_lock_end", original, end: "2026-11-09T12:00:00",
        }))).toBe("date_extension");
        expect(validateLockDraft(draft({
            type: "active_until_lock_end", original, end: "2026-11-09T12:01:00",
        }))).toBeNull();
    });

    it("parses only valid local wall-clock timestamps", () => {
        expect(parseLockEnd("2026-11-05T23:59:00")).toBeInstanceOf(Date);
        expect(parseLockEnd("2026-13-05T23:59:00")).toBeNull();
        expect(parseLockEnd("not a date")).toBeNull();
    });
});
