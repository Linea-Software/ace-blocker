import type { LockState } from "$engine-client";

export type LockKind = Exclude<LockState["state"], "inactive">;
export type LockError =
    | "schedule_required"
    | "date_required"
    | "date_invalid"
    | "date_future"
    | "date_extension"
    | "date_limit"
    | "lock_cannot_be_removed";

export type LockDraft = {
    enabled: boolean;
    type: LockKind;
    end: string;
    original: LockState;
    scheduleCount: number;
    isCurrentlyLocked: boolean;
    now: Date;
};

export type LockPayload = { lockState: string; lockEnd?: string };

/** A strict local date parser: the engine expects a wall-clock timestamp without a zone. */
export function parseLockEnd(raw: string): Date | null {
    const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):00$/.exec(raw);
    if (!match) return null;
    const [, y, mo, d, h, mi] = match.map(Number);
    const result = new Date(y, mo - 1, d, h, mi);
    return result.getFullYear() === y &&
        result.getMonth() === mo - 1 &&
        result.getDate() === d &&
        result.getHours() === h &&
        result.getMinutes() === mi
        ? result
        : null;
}

/** Mirrors engine invariants before the user can submit a lock. */
export function validateLockDraft(draft: LockDraft): LockError | null {
    if (!draft.enabled) {
        return draft.isCurrentlyLocked && draft.original.state !== "inactive"
            ? "lock_cannot_be_removed"
            : null;
    }
    if (draft.type === "active_during_schedule" && draft.scheduleCount === 0) {
        return "schedule_required";
    }
    if (draft.type !== "active_until_lock_end") return null;
    if (!draft.end) return "date_required";
    const end = parseLockEnd(draft.end);
    if (!end) return "date_invalid";
    if (end.getTime() <= draft.now.getTime()) return "date_future";
    if (end.getTime() > draft.now.getTime() + 90 * 24 * 60 * 60 * 1000) {
        return "date_limit";
    }
    if (draft.original.state === "active_until_lock_end") {
        const previous = parseLockEnd(draft.original.end ?? "");
        if (previous && end.getTime() <= previous.getTime()) return "date_extension";
    }
    return null;
}

export function buildLockPayload(draft: LockDraft): LockPayload | null {
    if (validateLockDraft(draft)) return null;
    if (!draft.enabled) return { lockState: "inactive" };
    return draft.type === "active_until_lock_end"
        ? { lockState: draft.type, lockEnd: draft.end }
        : { lockState: draft.type };
}

export function isUnchangedLock(original: LockState, payload: LockPayload): boolean {
    return original.state === payload.lockState &&
        (original.state !== "active_until_lock_end" ||
            original.end === payload.lockEnd);
}
