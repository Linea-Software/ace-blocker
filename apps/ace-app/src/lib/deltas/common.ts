import { z } from "zod";

const tempIdSchema = z.uuid().brand("TempId");
// A simple temporary id for delta objects.
// Not to be sent to the server, only used for local state management.
export type TempId = z.infer<typeof tempIdSchema>;

export const newTempId = () => crypto.randomUUID() as TempId;

// ── Rollback ────────────────────────────────────────────────────────────────

/** A single operation that was successfully applied to the server
 *  and may need to be reversed if a later operation in the same
 *  batch fails. */
export interface AppliedOp {
    /** Human-readable label for error messages. */
    label: string;
    /** Inverse API call to undo this operation on the server.
     *  Must be an async function that returns true on success. */
    rollback: () => Promise<boolean>;
}

/**
 * Wraps an async batch of server mutations with automatic rollback.
 *
 * - `ops` is built up as each call succeeds.
 * - If the `batch` callback throws, every op that was pushed into
 *   `ops` is rolled back in reverse order.
 * - If rollback itself fails, the error is logged but execution
 *   continues — the caller still receives the original failure.
 */
export async function withRollback(
    batch: (ops: AppliedOp[]) => Promise<void>,
): Promise<void> {
    const ops: AppliedOp[] = [];
    try {
        await batch(ops);
    } catch (err) {
        // Attempt rollback in reverse order
        for (let i = ops.length - 1; i >= 0; i--) {
            try {
                const ok = await ops[i].rollback();
                if (!ok) {
                    console.error(
                        `[Rollback] Failed to rollback "${ops[i].label}"`,
                    );
                }
            } catch (rollbackErr) {
                console.error(
                    `[Rollback] Error rolling back "${ops[i].label}":`,
                    rollbackErr,
                );
            }
        }
        throw err; // re-throw so the caller can surface the original error
    }
}
