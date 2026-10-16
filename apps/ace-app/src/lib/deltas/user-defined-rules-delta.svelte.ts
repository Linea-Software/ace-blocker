/**
 * USER DEFINED RULES DELTA
 *
 * Handles temporary changes for User Defined Rules.
 * User Defined Rules can be domain matches or regex-based patterns.
 *
 * This class captures additions and removals of these rules in the UI state
 * before applying them to the actual Network List on the backend.
 *
 * See `docs/engine-docs.md` for more information on User Defined Rules.
 */
import {
    addUserRules,
    deleteUserRule,
    type UserDefinedRuleSchema,
    type UserDefinedRuleType,
} from "$engine-client";
import { type TempId, newTempId, withRollback, type AppliedOp } from "./common";

/**
 * A pending change (add or delete) for a user rule.
 */
export type UserDefinedRuleChange =
    | {
          type: "ADD";
          rule: string;
          ruleType: UserDefinedRuleType;
          tempId: TempId;
      }
    | { type: "DELETE"; userRuleId: string };

/**
 * Manages the set of pending changes for User Defined Rules.
 */
export class UserDefinedRulesDelta {
    public changes: UserDefinedRuleChange[] = $state([]);

    /**
     * The live state of rules.
     */
    private currentState: UserDefinedRuleSchema[] = $state([]);
    private initialRuleIds: string[] = [];
    /** Original rules keyed by ID, used for rollback. */
    private initialRules: UserDefinedRuleSchema[] = [];

    constructor(initialRules: UserDefinedRuleSchema[] = []) {
        this.currentState = [...initialRules];
        this.initialRuleIds = initialRules.map((r) => r.id);
        this.initialRules = [...initialRules];
    }

    /**
     * Checks if a rule was initially present (persistent).
     */
    public isInitialRule(ruleId: string): boolean {
        return this.initialRuleIds.includes(ruleId);
    }

    /**
     * Reactive getter for the current rules (after pending changes).
     */
    public get currentRules(): UserDefinedRuleSchema[] {
        return this.currentState;
    }

    /**
     * Stages a new rule to be added.
     * Avoids duplicates if the same typed rule is already staged.
     * @param rule The rule value.
     * @param ruleType Whether the rule is a domain or regex rule.
     */
    addRule(rule: string, ruleType: UserDefinedRuleType) {
        if (
            this.currentState.some(
                (r) => r.rule === rule && r.ruleType === ruleType,
            )
        ) {
            return;
        }
        const tempId = newTempId();
        this.changes.push({ type: "ADD", rule, ruleType, tempId });
        this.currentState.push({
            id: tempId,
            networkBlockId: "",
            rule,
            ruleType,
        });
    }

    /**
     * Removes a rule from the pending additions list.
     * @param tempId The temporary ID of the rule to remove from staging.
     */
    removeTempRule(tempId: TempId) {
        this.changes = this.changes.filter(
            (change) => change.type !== "ADD" || change.tempId !== tempId,
        );
        this.currentState = this.currentState.filter((r) => r.id !== tempId);
    }

    /**
     * Stages an existing persistent rule for removal.
     * @param userRuleId The ID of the rule to delete.
     */
    removeRule(userRuleId: string) {
        this.changes.push({ type: "DELETE", userRuleId });
        this.currentState = this.currentState.filter(
            (r) => r.id !== userRuleId,
        );
    }

    /**
     * Checks if there are any pending changes to be pushed.
     */
    public hasChanges(): boolean {
        return this.changes.length > 0;
    }

    /** Look up the original rule by its persistent ID. */
    private getOriginalRule(
        ruleId: string,
    ): UserDefinedRuleSchema | undefined {
        return this.initialRules.find((r) => r.id === ruleId);
    }

    /**
     * Commits all pending rule changes to the backend.
     * Failed operations are rolled back so the server stays consistent.
     * @param networkListId The ID of the Network List to update.
     */
    async pushChangesToServer(networkListId: string) {
        if (!this.hasChanges()) return;

        const removals = this.changes.filter(
            (change) => change.type === "DELETE",
        );
        const adds = this.changes.filter((change) => change.type === "ADD");

        await withRollback(async (ops: AppliedOp[]) => {
            // ── Removals ──────────────────────────────────────────────
            for (const removal of removals) {
                if (removal.type !== "DELETE") continue;

                const original = this.getOriginalRule(removal.userRuleId);

                const { error } = await deleteUserRule({
                    path: {
                        user_rule_id: removal.userRuleId,
                    },
                });
                if (error) {
                    console.error("Failed to delete user rule:", error);
                    throw new Error("Failed to delete user rule");
                }

                // Register rollback: re-add the original rule
                if (original) {
                    ops.push({
                        label: `delete-rule-${removal.userRuleId}`,
                        rollback: async () => {
                            const { error: rbErr } = await addUserRules({
                                path: {
                                    network_block_id: networkListId,
                                },
                                body: {
                                    rules: [
                                        {
                                            rule: original.rule,
                                            ruleType: original.ruleType,
                                        },
                                    ],
                                },
                            });
                            return !rbErr;
                        },
                    });
                }
            }

            // ── Adds ─────────────────────────────────────────────────
            if (adds.length > 0) {
                const { error } = await addUserRules({
                    path: {
                        network_block_id: networkListId,
                    },
                    body: {
                        rules: adds.map((add) => ({
                            rule: add.rule,
                            ruleType: add.ruleType,
                        })),
                    },
                });
                if (error) {
                    console.error("Failed to add user rules:", error);
                    throw new Error("Failed to add user rules");
                }
                // Adds are a single batch call — if it fails, nothing was
                // added server-side. If it succeeds, all rules are live.
                // No individual rollback registration needed here.
            }
        });

        // Clear changes only after successful push
        this.changes = [];
        // Sync initial state with current state
        this.initialRuleIds = this.currentState.map((r) => r.id);
        this.initialRules = [...this.currentState];
    }
}
