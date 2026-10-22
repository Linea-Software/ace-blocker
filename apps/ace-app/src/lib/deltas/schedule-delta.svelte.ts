/**
 * SCHEDULE DELTA
 *
 * This file manages temporary changes for Network List Schedules.
 * Schedules define specific time windows (days of the week + time intervals) during which a Network List is active.
 *
 * The Delta pattern allows the user to make multiple schedule edits in the UI
 * and save them in a batch.
 *
 * See `docs/engine-docs.md` for more details on Schedule Entries.
 */
import {
    addScheduleEntries,
    deleteScheduleEntry,
    editScheduleEntry,
    type ScheduleEntrySchema,
} from "$engine-client";
import { Result } from "typescript-result";
import { isStrictlyMoreRestrictiveThan, isSupportedSchedulePeriod } from "./schedule-coverage";
import { type TempId, newTempId, withRollback, type AppliedOp } from "./common";

/** A simple hour+minute time representation used for schedule entries. */
export type PlainTime = { hour: number; minute: number };

function timeToString(time: PlainTime): string {
    return `${time.hour.toString().padStart(2, "0")}:${time.minute.toString().padStart(2, "0")}`;
}

export { isStrictlyMoreRestrictiveThan } from "./schedule-coverage";

/**
 * Represents the days of the week for a schedule entry.
 */
export type Weekday = {
    monday: boolean;
    tuesday: boolean;
    wednesday: boolean;
    thursday: boolean;
    friday: boolean;
    saturday: boolean;
    sunday: boolean;
};

/**
 * Represents a time range with a start and end time.
 */
export type TimeInterval = {
    startTime: PlainTime;
    endTime: PlainTime;
};

/**
 * Represents a pending operation on the schedule configuration.
 */
export type ScheduleChange =
    | {
          type: "ADD";
          scheduleId: TempId;
          weekday: Weekday;
          timeInterval: TimeInterval;
      }
    | { type: "REMOVE"; scheduleId: string }
    | {
          type: "UPDATE";
          scheduleId: string;
          weekday: Weekday;
          timeInterval: TimeInterval;
      };

export type ScheduleChangeError =
    | { type: "invalid_time_format"; message: string }
    | { type: "shallower_changes_not_allowed"; message: string };

/**
 * Manages pending schedule modifications.
 * Supports adding, removing, and updating schedule entries before persistence.
 */
export class ScheduleChanges {
    changes: ScheduleChange[] = $state([]);
    currentState: ScheduleEntrySchema[] = $state([]);
    isNetworkBlockLocked: boolean;
    private readonly initialEntries: ScheduleEntrySchema[];

    constructor(
        initialEntries: ScheduleEntrySchema[] = [],
        isNetworkBlockLocked: boolean = false,
    ) {
        this.initialEntries = initialEntries;
        this.currentState = [...initialEntries];
        this.isNetworkBlockLocked = isNetworkBlockLocked;
    }

    /** When the block is locked, returns true only if the current schedule
     *  covers every window the original schedule covered (i.e. is not shallower). */
    isValidForLockedBlock(): boolean {
        if (!this.isNetworkBlockLocked) return true;
        return isStrictlyMoreRestrictiveThan(
            this.initialEntries,
            this.currentState,
        );
    }

    /**
     * It's reactive!
     */
    get currentScheduleEntries(): ScheduleEntrySchema[] {
        return this.currentState;
    }

    /**
     * Stages a new schedule entry.
     * @param schedule The days of the week to apply.
     * @param startTime Start time of the active period.
     * @param endTime End time of the active period.
     * @returns A Result indicating success or failure (e.g., invalid time format).
     */
    addScheduleEntry(
        schedule: Weekday,
        startTime: PlainTime,
        endTime: PlainTime,
    ): Result<void, ScheduleChangeError> {
        if (!isSupportedSchedulePeriod(timeToString(startTime), timeToString(endTime))) {
            return Result.error({
                type: "invalid_time_format",
                message: "Unsupported schedule interval: end before start (except midnight)",
            });
        }
        const tempId = newTempId();

        // update the changes
        this.changes.push({
            type: "ADD",
            scheduleId: tempId,
            weekday: schedule,
            timeInterval: { startTime, endTime },
        });
        // update the current states
        this.currentState.push({
            networkBlockId: "",
            id: tempId,
            ...schedule,
            startTime: timeToString(startTime),
            endTime: timeToString(endTime),
        });
        return Result.ok();
    }

    /**
     * Removes a pending schedule entry that hasn't been saved yet.
     * @param scheduleId The temporary ID of the entry.
     */
    removeTempScheduleEntry(scheduleId: TempId) {
        // update the changes
        this.changes = this.changes.filter(
            (change) => change.scheduleId !== scheduleId,
        );
        // update the current states
        this.currentState = this.currentState.filter(
            (entry) => entry.id !== scheduleId,
        );
    }

    /**
     * Stages an existing schedule entry for removal.
     * @param scheduleId The persistent ID of the entry on the server.
     */
    removeScheduleEntry(scheduleId: string): Result<void, ScheduleChangeError> {
        if (this.isNetworkBlockLocked) {
            return Result.error({
                type: "shallower_changes_not_allowed",
                message:
                    "Cannot remove schedule entry because the list is locked",
            });
        }
        // update the changes
        this.changes = this.changes.filter(
            (change) =>
                !(change.type === "UPDATE" && change.scheduleId === scheduleId),
        );
        this.changes.push({ type: "REMOVE", scheduleId });
        // update the current states
        this.currentState = this.currentState.filter(
            (entry) => entry.id !== scheduleId,
        );
        return Result.ok();
    }

    /**
     * Stages an update to an existing schedule entry.
     * @param scheduleId The persistent ID of the entry to update.
     * @param schedule The new days configuration.
     * @param timeInterval The new time range configuration.
     */
    updateScheduleEntry(
        scheduleId: string,
        schedule: Weekday,
        timeInterval: TimeInterval,
    ): Result<void, ScheduleChangeError> {
        if (!isSupportedSchedulePeriod(
            timeToString(timeInterval.startTime),
            timeToString(timeInterval.endTime),
        )) {
            return Result.error({
                type: "invalid_time_format",
                message: "Unsupported schedule interval: end before start (except midnight)",
            });
        }
        this.changes.push({
            type: "UPDATE",
            scheduleId,
            weekday: schedule,
            timeInterval,
        });
        // update the current states
        this.currentState = this.currentState.map((entry) => {
            if (entry.id === scheduleId) {
                return {
                    ...entry,
                    ...schedule,
                    startTime: timeToString(timeInterval.startTime),
                    endTime: timeToString(timeInterval.endTime),
                };
            }
            return entry;
        });
        return Result.ok();
    }

    /**
     * Checks if there are any pending changes to be pushed.
     */
    public hasChanges(): boolean {
        return this.changes.length > 0;
    }

    /** Look up the original (server-side) entry by its persistent ID. */
    private getOriginalEntry(
        scheduleId: string,
    ): ScheduleEntrySchema | undefined {
        return this.initialEntries.find((e) => e.id === scheduleId);
    }

    /**
     * Pushes all pending adds, removals, and updates to the backend API.
     * Failed operations are rolled back so the server stays consistent.
     * @param networkListId The ID of the Network List these schedules belong to.
     */
    async pushChangesToServer(networkListId: string) {
        if (!this.hasChanges()) return;

        const removals = this.changes.filter(
            (change) => change.type === "REMOVE",
        );
        const updates = this.changes.filter(
            (change) => change.type === "UPDATE",
        );
        const adds = this.changes.filter((change) => change.type === "ADD");

        await withRollback(async (ops: AppliedOp[]) => {
            // ── Removals ──────────────────────────────────────────────
            for (const removal of removals) {
                const original = this.getOriginalEntry(removal.scheduleId);

                const { error } = await deleteScheduleEntry({
                    path: {
                        schedule_entry_id: removal.scheduleId,
                    },
                });
                if (error) {
                    console.error(
                        "Failed to delete schedule entry:",
                        error,
                    );
                    throw new Error("Failed to delete schedule entry");
                }

                // Register rollback: re-add the original entry
                if (original) {
                    ops.push({
                        label: `delete-schedule-${removal.scheduleId}`,
                        rollback: async () => {
                            const { error: rbErr } =
                                await addScheduleEntries({
                                    path: {
                                        network_block_id: networkListId,
                                    },
                                    body: {
                                        scheduleEntries: [{
                                            monday: original.monday,
                                            tuesday: original.tuesday,
                                            wednesday: original.wednesday,
                                            thursday: original.thursday,
                                            friday: original.friday,
                                            saturday: original.saturday,
                                            sunday: original.sunday,
                                            startTime: original.startTime,
                                            endTime: original.endTime,
                                        }],
                                    },
                                });
                            return !rbErr;
                        },
                    });
                }
            }

            // ── Updates ──────────────────────────────────────────────
            for (const update of updates) {
                const original = this.getOriginalEntry(update.scheduleId);

                const { error } = await editScheduleEntry({
                    path: {
                        schedule_entry_id: update.scheduleId,
                    },
                    body: {
                        ...update.weekday,
                        startTime: timeToString(
                            update.timeInterval.startTime,
                        ),
                        endTime: timeToString(
                            update.timeInterval.endTime,
                        ),
                    },
                });
                if (error) {
                    console.error(
                        "Failed to edit schedule entry:",
                        error,
                    );
                    throw new Error("Failed to edit schedule entry");
                }

                // Register rollback: restore original values
                if (original) {
                    ops.push({
                        label: `edit-schedule-${update.scheduleId}`,
                        rollback: async () => {
                            const { error: rbErr } = await editScheduleEntry(
                                {
                                    path: {
                                        schedule_entry_id:
                                            update.scheduleId,
                                    },
                                    body: {
                                        monday: original.monday,
                                        tuesday: original.tuesday,
                                        wednesday: original.wednesday,
                                        thursday: original.thursday,
                                        friday: original.friday,
                                        saturday: original.saturday,
                                        sunday: original.sunday,
                                        startTime: original.startTime,
                                        endTime: original.endTime,
                                    },
                                },
                            );
                            return !rbErr;
                        },
                    });
                }
            }

            // ── Adds ─────────────────────────────────────────────────
            if (adds.length > 0) {
                const { error } = await addScheduleEntries({
                    path: {
                        network_block_id: networkListId,
                    },
                    body: {
                        scheduleEntries: adds.map((add) => ({
                            ...add.weekday,
                            startTime: timeToString(
                                add.timeInterval.startTime,
                            ),
                            endTime: timeToString(
                                add.timeInterval.endTime,
                            ),
                        })),
                    },
                });
                if (error) {
                    console.error(
                        "Failed to add schedule entries:",
                        error,
                    );
                    throw new Error("Failed to add schedule entries");
                }
                // Note: we don't have server-side IDs for newly added
                // entries, so we can't roll them back individually.
                // However, adds are always processed last, so any
                // preceding removals/updates will still be rolled back.
            }
        });

        // Clear changes only after successful push
        this.changes = [];
        // Sync initial state with current state
        this.initialEntries.length = 0;
        this.initialEntries.push(...this.currentState);
    }
}
