<script lang="ts">
    import type {
        NetworkBlockSchema,
        NetworkListLockSchema,
        ScheduleEntrySchema,
    } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";
    import { getLocale } from "$lib/paraglide/runtime";
    import { localizationStore } from "$lib/stores/localization.svelte";
    import { onMount } from "svelte";
    import { Eye, Lock, LockOpen, Pencil, Trash2, Ellipsis } from "@lucide/svelte";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
    import ScheduleTimeline from "./ScheduleTimeline.svelte";
    import LockSeal from "./LockSeal.svelte";
    import { findNextScheduleTransition } from "$lib/utils/schedule-transitions";

    export type NetworkPolicyModel = NetworkBlockSchema & {
        lockInfo?: NetworkListLockSchema | null;
        schedules?: ScheduleEntrySchema[];
        sitesListCount?: number;
        rulesCount?: number;
    };

    let {
        policy,
        onEdit,
        onDelete,
        onLock,
        onEnabledChange,
    }: {
        policy: NetworkPolicyModel;
        onEdit: (policy: NetworkPolicyModel) => void;
        onDelete: (policy: NetworkPolicyModel) => void;
        onLock: (policy: NetworkPolicyModel) => void;
        onEnabledChange: (policy: NetworkPolicyModel, enabled: boolean) => Promise<void>;
    } = $props();

    let isUpdatingEnabled = $state(false);

    async function toggleEnabled() {
        if (isUpdatingEnabled || (policy.isLocked && policy.enabled)) return;
        isUpdatingEnabled = true;
        try {
            await onEnabledChange(policy, !policy.enabled);
        } finally {
            isUpdatingEnabled = false;
        }
    }

    onMount(() => {
        localizationStore.detectSystemTimeFormat();
    });

    // A scheduled lock can be configured while its protected hours are
    // inactive. Only the engine's isLocked flag should disable editing.
    const locked = $derived(policy.lockState.state !== "inactive");
    const lockedNow = $derived(policy.isLocked);
    const hasSchedule = $derived((policy.schedules?.length ?? 0) > 0);

    const stateLabel = $derived.by(() => {
        if (!policy.enabled) return m.block_state_disabled();
        if (policy.isActive) return m.block_state_enforcing();
        if (hasSchedule) return m.block_state_scheduled();
        return m.block_state_unscheduled();
    });

    const lockLabel = $derived.by(() => {
        if (policy.lockState.state === "soft_unlockable") {
            return m.lock_soft_active_tooltip();
        }
        if (policy.lockState.state === "active_during_schedule") {
            return lockedNow
                ? m.lock_schedule_active_tooltip()
                : m.lock_schedule_configured_tooltip();
        }
        if (policy.lockState.state === "active_until_lock_end") {
            return lockedNow
                ? m.lock_time_active_tooltip()
                : m.lock_time_expired_tooltip();
        }
        return m.lock_this_block_tooltip();
    });

    const lockActionLabel = $derived.by(() => {
        if (policy.lockState.state === "soft_unlockable") return m.unlock_block();
        if (locked) return m.review_lock();
        return m.lock_protect_action();
    });

    const lockActionAria = $derived.by(() => {
        if (policy.lockState.state === "soft_unlockable") return m.unlock_block();
        if (locked) return m.review_lock();
        return m.lock_this_block_tooltip();
    });

    const lockDescription = $derived.by(() => {
        if (policy.lockState.state === "soft_unlockable") {
            return m.lock_soft_active_description();
        }
        if (policy.lockState.state === "active_during_schedule" && !lockedNow) {
            return m.lock_schedule_configured_description();
        }
        if (policy.lockState.state === "active_until_lock_end" && !lockedNow) {
            return m.lock_time_expired_description();
        }
        return m.lock_hard_active_description();
    });

    const scheduleText = $derived.by(() => {
        if (!policy.schedules || policy.schedules.length === 0) {
            return m.block_unscheduled_hint();
        }

        const first = policy.schedules[0];
        const keys = [
            "monday",
            "tuesday",
            "wednesday",
            "thursday",
            "friday",
            "saturday",
            "sunday",
        ] as const;
        const labels = [
            m.monday_short(),
            m.tuesday_short(),
            m.wednesday_short(),
            m.thursday_short(),
            m.friday_short(),
            m.saturday_short(),
            m.sunday_short(),
        ];
        const activeDays = keys.flatMap((key, index) =>
            first[key] ? [labels[index]] : [],
        );

        let days = activeDays.join(", ");
        if (activeDays.length === 7) {
            days = m.schedule_all_week();
        } else if (
            activeDays.length === 5 &&
            keys.slice(0, 5).every((key) => first[key])
        ) {
            days = m.schedule_weekdays_compact();
        } else if (
            activeDays.length === 2 &&
            first.saturday &&
            first.sunday
        ) {
            days = m.schedule_weekend_compact();
        }

        const allDay = first.startTime === first.endTime;
        const range = allDay
            ? m.schedule_all_day()
            : localizationStore.formatTime(first.startTime ?? "00:00") +
              "–" +
              localizationStore.formatTime(first.endTime ?? "23:59");
        const extra =
            policy.schedules.length > 1
                ? " " +
                  (policy.schedules.length === 2
                      ? m.schedule_extra_count_one()
                      : m.schedule_extra_count({ count: policy.schedules.length - 1 }))
                : "";

        return days + ", " + range + extra;
    });

    const nextTransition = $derived.by(() =>
        policy.enabled
            ? findNextScheduleTransition(policy.schedules ?? [])
            : null,
    );

    const transitionText = $derived.by(() => {
        if (!nextTransition) return null;
        const time = nextTransition.at.toLocaleString(getLocale(), {
            weekday: "short",
            hour: "2-digit",
            minute: "2-digit",
        });
        return nextTransition.kind === "start"
            ? m.block_next_starts({ time })
            : m.block_next_ends({ time });
    });

    function formatLockEnd(value: string | null | undefined): string | null {
        if (!value) return null;
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return String(value);
        return m.until_date({ date: date.toLocaleString(getLocale()) });
    }
</script>

<article
    class:active={policy.enabled && policy.isActive}
    class:locked
    class:disabled={!policy.enabled}
    class:unscheduled={!hasSchedule}
    class="network-policy"
    data-testid="network-policy"
>
    <div class="state-rail" aria-hidden="true">
        <span></span>
    </div>

    <div class="policy-body">
        {#if locked}
            <LockSeal
                compact
                title={lockLabel}
                description={lockDescription}
                detail={formatLockEnd(policy.lockInfo?.lockEnd)}
            />
        {/if}

        <div class="policy-header">
            <div class="identity">
                <div class="state-line">
                    <span class="state-word">{stateLabel}</span>
                </div>
                <h2>{policy.name}</h2>
                <div class:unscheduled-copy={!hasSchedule} class="schedule-copy">
                    <span>{scheduleText}</span>
                    {#if transitionText}
                        <span class="transition-copy">{transitionText}</span>
                    {/if}
                </div>
            </div>

            <div class="actions">
                <div class="enable-action">
                    <span>{m.block_enabled_label()}</span>
                    <button
                        type="button"
                        role="switch"
                        class="enable-switch"
                        data-testid="block-enabled-switch"
                        aria-label={m.block_enabled_switch_aria({ name: policy.name })}
                        aria-checked={policy.enabled}
                        aria-busy={isUpdatingEnabled}
                        disabled={isUpdatingEnabled || (policy.isLocked && policy.enabled)}
                        title={policy.isLocked && policy.enabled
                            ? m.block_locked_tooltip()
                            : m.block_enabled_label()}
                        onclick={toggleEnabled}
                    >
                        <span class="enable-thumb" aria-hidden="true"></span>
                    </button>
                </div>
                <button
                    type="button"
                    class="quiet-action"
                    onclick={() => onEdit(policy)}
                >
                    {#if lockedNow}
                        <Eye size={15} aria-hidden="true" />
                        <span>{m.view()}</span>
                    {:else}
                        <Pencil size={15} aria-hidden="true" />
                        <span>{m.edit()}</span>
                    {/if}
                </button>
                <button
                    type="button"
                    class:commit-action={!locked}
                    class="quiet-action"
                    onclick={() => onLock(policy)}
                    aria-label={lockActionAria}
                    title={lockLabel}
                >
                    {#if locked}
                        <Lock size={15} aria-hidden="true" />
                    {:else}
                        <LockOpen size={15} aria-hidden="true" />
                    {/if}
                    <span>{lockActionLabel}</span>
                </button>
                <DropdownMenu.Root>
                    <DropdownMenu.Trigger
                        class="inline-flex size-9 items-center justify-center rounded-sm border border-transparent text-muted-foreground transition-colors hover:border-border hover:bg-muted hover:text-foreground"
                        aria-label={m.more_options()}
                        title={m.more_options()}
                    >
                        <Ellipsis size={16} aria-hidden="true" />
                    </DropdownMenu.Trigger>
                    <DropdownMenu.Content align="end" class="min-w-36 rounded-sm shadow-none">
                        <DropdownMenu.Item
                            variant="destructive"
                            disabled={lockedNow}
                            onclick={() => onDelete(policy)}
                            title={lockedNow ? m.block_locked_tooltip() : m.delete()}
                        >
                            <Trash2 size={15} aria-hidden="true" />
                            <span>{m.delete()}</span>
                        </DropdownMenu.Item>
                    </DropdownMenu.Content>
                </DropdownMenu.Root>
            </div>
        </div>

        <div class:no-schedule={!hasSchedule} class="policy-grid">
            {#if hasSchedule}
                <div class="schedule-zone">
                    <ScheduleTimeline
                        entries={policy.schedules ?? []}
                        active={policy.enabled && policy.isActive}
                    />
                </div>
            {/if}

            <dl class="policy-facts">
                <div>
                    <dt>{m.sites_lists()}</dt>
                    <dd>{policy.sitesListCount ?? 0}</dd>
                </div>
                <div>
                    <dt>{m.custom_rules()}</dt>
                    <dd>{policy.rulesCount ?? 0}</dd>
                </div>
            </dl>
        </div>
    </div>
</article>

<style>
    .network-policy {
        position: relative;
        display: grid;
        grid-template-columns: 8px minmax(0, 1fr);
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 7px;
        background: var(--card);
        transition:
            border-color 150ms ease,
            background 150ms ease;
    }

    .network-policy:hover {
        border-color: color-mix(in srgb, var(--foreground) 30%, var(--border));
    }

    .state-rail {
        position: relative;
        border-right: 1px solid var(--border);
        background: var(--muted);
    }

    .state-rail span {
        position: absolute;
        inset: 0;
        background: var(--ace-ash);
    }

    .network-policy.active .state-rail span {
        background: var(--ace-signal);
    }

    .network-policy.unscheduled .state-rail span {
        background: color-mix(in srgb, var(--ace-ash) 55%, var(--muted));
    }

    .network-policy.disabled .state-rail span {
        background: var(--ace-ash);
    }

    .network-policy.locked {
        border-color: color-mix(in srgb, var(--ace-signal-deep) 58%, var(--border));
    }

    .network-policy.locked .state-rail {
        background: var(--ace-signal-deep);
        border-right-color: var(--ace-signal-deep);
    }

    .network-policy.locked .state-rail span {
        background: var(--ace-signal);
    }

    .policy-body {
        min-width: 0;
    }

    .policy-header {
        min-width: 0;
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 20px;
        padding: 17px 18px 14px;
    }

    .identity {
        min-width: 0;
        flex: 1;
    }

    .state-line {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 4px;
    }

    .state-word {
        color: var(--muted-foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 13px;
        font-weight: 600;
    }

    .active .state-word {
        color: var(--ace-signal-deep);
    }

    .unscheduled .state-word {
        color: var(--muted-foreground);
    }

    h2 {
        margin: 0;
        overflow: hidden;
        color: var(--foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: clamp(20px, 3cqi, 24px);
        font-weight: 600;
        line-height: 1.05;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .schedule-copy {
        min-width: 0;
        margin: 6px 0 0;
        display: flex;
        align-items: center;
        gap: 9px;
        overflow: hidden;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 13px;
        line-height: 1.35;
    }

    .schedule-copy > span:first-child {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .unscheduled-copy {
        font-family: "IBM Plex Sans", sans-serif;
    }

    .transition-copy {
        flex: 0 0 auto;
        padding-left: 9px;
        border-left: 1px solid var(--border);
        color: var(--foreground);
        font-size: 12px;
    }

    .actions {
        display: flex;
        align-items: center;
        gap: 3px;
    }

    .enable-action {
        display: inline-flex;
        align-items: center;
        gap: 7px;
        margin-right: 5px;
        padding-right: 10px;
        border-right: 1px solid var(--border);
        color: var(--muted-foreground);
        font-size: 12px;
        font-weight: 500;
        white-space: nowrap;
    }

    button.enable-switch {
        display: inline-flex;
        align-items: center;
        width: 36px;
        height: 22px;
        min-height: 22px;
        padding: 2px;
        border: 1px solid var(--border);
        border-radius: 12px;
        background: var(--muted);
    }

    .enable-switch[aria-checked="true"] {
        border-color: var(--ace-signal-deep);
        background: var(--ace-signal);
    }

    .enable-thumb {
        display: block;
        width: 16px;
        height: 16px;
        flex: 0 0 16px;
        border-radius: 50%;
        background: var(--foreground);
        opacity: 0.7;
        transition: transform 140ms ease;
    }

    .enable-switch[aria-checked="true"] .enable-thumb {
        transform: translateX(14px);
        background: var(--ace-ink);
        opacity: 1;
    }

    .enable-switch:focus-visible {
        outline: 2px solid var(--foreground);
        outline-offset: 3px;
    }

    button {
        min-height: 36px;
        border: 1px solid transparent;
        border-radius: 4px;
        background: transparent;
        color: var(--foreground);
        cursor: pointer;
        transition:
            background 140ms ease,
            border-color 140ms ease,
            color 140ms ease;
    }

    .quiet-action {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        padding: 0 9px;
        color: var(--muted-foreground);
        font-size: 12px;
        font-weight: 500;
    }

    .quiet-action:hover {
        border-color: var(--border);
        background: var(--muted);
        color: var(--foreground);
    }

    .commit-action {
        border-color: color-mix(in srgb, var(--ace-signal-deep) 45%, var(--border));
        color: var(--ace-signal-deep);
    }

    .commit-action:hover {
        border-color: var(--ace-signal-deep);
        background: color-mix(in srgb, var(--ace-signal) 12%, transparent);
        color: var(--foreground);
    }

    button:disabled {
        cursor: not-allowed;
        opacity: 0.35;
    }

    /* Locked switches must remain recognizable even when non-interactive. */
    button.enable-switch:disabled {
        opacity: 0.65;
    }

    :global(.dark) button.enable-switch[aria-checked="false"] {
        border-color: var(--input);
    }

    .policy-grid {
        display: grid;
        grid-template-columns: minmax(240px, 1fr) auto;
        align-items: end;
        gap: 22px;
        padding: 0 18px 17px;
    }

    .policy-grid.no-schedule {
        grid-template-columns: 1fr;
        gap: 0;
    }

    .schedule-zone {
        min-width: 0;
        min-height: 76px;
        display: grid;
        align-items: center;
        padding-top: 12px;
        border-top: 1px solid var(--border);
    }

    .policy-facts {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 6px 16px;
        margin: 0;
        color: var(--ace-text-tertiary);
        font-size: 12px;
    }

    .policy-facts > div {
        display: inline-flex;
        align-items: baseline;
        gap: 5px;
        min-width: 0;
    }

    .policy-facts > div + div {
        padding-left: 16px;
        border-left: 1px solid var(--border);
    }

    dt {
        color: var(--ace-text-tertiary);
    }

    dd {
        margin: 0;
        color: var(--foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 12px;
        font-weight: 500;
    }

    @container page (max-width: 680px) {
        .policy-header {
            flex-direction: column;
            gap: 12px;
        }

        .actions {
            width: 100%;
        }

        .policy-grid {
            grid-template-columns: 1fr;
        }

        .policy-facts {
            justify-self: stretch;
        }
    }

    @container page (max-width: 480px) {
        .schedule-copy {
            align-items: flex-start;
            flex-direction: column;
            gap: 4px;
        }

        .transition-copy {
            padding-left: 0;
            border-left: 0;
        }

        .actions {
            flex-wrap: wrap;
        }
    }
</style>
