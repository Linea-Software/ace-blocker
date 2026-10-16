/**
 * LOCK DELTA
 *
 * Manages pending changes to the lock status of a Network List.
 * Locks can be used to prevent modifications to a Network List during certain periods.
 */
import {
    setLock,
    type LockState,
    type SetNetworkLockRequest,
} from "$engine-client";
import { Result } from "typescript-result";
import { withRollback, type AppliedOp } from "./common";
import {Temporal} from "@js-temporal/polyfill"

type LockChangeError =
    | { type: "invalid_date"; message: string }
    | { type: "shallower_changes_not_allowed"; message: string };

function parseLockEnd(raw: string): Temporal.Instant | null {
    try {
        return Temporal.Instant.from(raw);
    } catch {
        return null;
    }
}

function instantIsInThePast(instant: Temporal.Instant): boolean {
    return Temporal.Instant.compare(instant, Temporal.Now.instant()) < 0;
}

export class LockChanges {
    originalState: LockState;
    currentState: LockState;

    constructor(lockState: LockState) {
        this.originalState = structuredClone(lockState);
        this.currentState = structuredClone(lockState);
    }

    /**
     * @param newLockState
     * @returns Result.ok() if the new lock respects locks policies and has a correct date, otherwise returns LockChangeError
     */
    setLock(newLockState: LockState): Result<void, LockChangeError> {
        if (
            (this.originalState.state == "active_until_lock_end" ||
                this.originalState.state == "active_during_schedule") &&
            newLockState.state == "inactive"
        ) {
            return Result.error({
                type: "shallower_changes_not_allowed",
                message: `cannot downgrade lock to inactive`,
            });
        }

        if (
            this.originalState.state == "active_until_lock_end" &&
            newLockState.state == "active_during_schedule"
        ) {
            return Result.error({
                type: "shallower_changes_not_allowed",
                message: `cannot downgrade lock to active_during_schedule`,
            });
        }

        if (newLockState.state == "active_until_lock_end") {
            if (!newLockState.end) {
                return Result.error({
                    type: "invalid_date",
                    message:
                        "Date is required when setting lock to active_until_lock_end",
                });
            }

            const lockEndInstant = parseLockEnd(newLockState.end);
            if (!lockEndInstant) {
                return Result.error({
                    type: "invalid_date",
                    message: `cannot convert ${newLockState.end} to an instant`,
                });
            }

            if (instantIsInThePast(lockEndInstant)) {
                return Result.error({
                    type: "invalid_date",
                    message: `${newLockState.end} should be after now`,
                });
            }
        }

        this.currentState = newLockState;
        return Result.ok();
    }

    /**
     * Compares two LockState values by normalising their `end` timestamps
     * to the same UTC-second representation.  This yields timezone-agnostic
     * equality so that `2026-07-25T12:00:00+02:00` and
     * `2026-07-25T10:00:00Z` are recognised as the same instant.
     */
    private statesAreEqual(a: LockState, b: LockState): boolean {
        if (a.state !== b.state) return false;
        if (
            a.state === "active_until_lock_end" &&
            b.state === "active_until_lock_end"
        ) {
            const normalize = (s: string | undefined | null) => {
                if (!s) return "";
                const instant = parseLockEnd(s);
                if (!instant) return s; // fallback: compare raw strings
                // Format as UTC YYYY-MM-DDTHH:mm:ss
                const zdt = instant.toZonedDateTimeISO("UTC");
                return zdt.toPlainDateTime().toString({
                    smallestUnit: "second",
                });
            };
            return normalize(a.end) === normalize(b.end);
        }
        return true;
    }

    /**
     * Commits the lock change to the server.
     */
    async pushChangesToServer(networkListId: string) {
        if (this.statesAreEqual(this.originalState, this.currentState)) return;

        let body: SetNetworkLockRequest;

        if (this.currentState.state == "active_until_lock_end") {
            body = {
                lockState: this.currentState.state,
                lockEnd: this.currentState.end,
            };
        } else {
            body = {
                lockState: this.currentState.state,
            };
        }

        // Record current state *before* the call so we can restore on failure
        const previousState = structuredClone(this.originalState);

        await withRollback(async (ops: AppliedOp[]) => {
            const { error } = await setLock({
                path: {
                    network_block_id: networkListId,
                },
                body: body,
            });

            if (error) {
                console.error("Failed to set lock:", error);
                throw new Error("Failed to set lock");
            }

            ops.push({
                label: "setLock",
                rollback: async () => {
                    // Restore the previous lock state on the server
                    let rollbackBody: SetNetworkLockRequest;
                    if (previousState.state === "active_until_lock_end") {
                        rollbackBody = {
                            lockState: previousState.state,
                            lockEnd: previousState.end,
                        };
                    } else {
                        rollbackBody = { lockState: previousState.state };
                    }
                    const { error: rbErr } = await setLock({
                        path: { network_block_id: networkListId },
                        body: rollbackBody,
                    });
                    return !rbErr;
                },
            });

            // Success — persist the new original state
            this.originalState = structuredClone(this.currentState);
        });
    }
}
