<script lang="ts">
    import { dashboardStore } from "$lib/stores/dashboard.svelte";
    import { engineStore } from "$lib/stores/engine.svelte";
    import type { NetworkBlockSnapshot } from "$engine-client";
    import {
        ArrowUpRight,
        Clock3,
        LockKeyhole,
        ShieldCheck,
        WifiOff,
    } from "@lucide/svelte";
    import { SvelteDate } from "svelte/reactivity";
    import { resolve } from "$app/paths";
    import * as m from "$lib/paraglide/messages.js";
    import { getLocale } from "$lib/paraglide/runtime";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";
    import { findNextScheduleTransition } from "$lib/utils/schedule-transitions";

    type ScheduledTransition = {
        block: NetworkBlockSnapshot;
        kind: "start" | "end";
        at: Date;
    };

    const engineOnline = $derived(engineStore.isAlive);
    const activeSnapshots = $derived(
        dashboardStore.networkBlockSnapshots.filter(
            (snapshot) => snapshot.block.isActive,
        ),
    );
    const nextTransition = $derived.by(() =>
        findNextTransition(dashboardStore.networkBlockSnapshots),
    );

    const resistanceLabel = $derived.by(() => {
        if (!dashboardStore.engineEnabled) return m.status_off();
        if (dashboardStore.currentIntensity === "extreme") {
            return m.intensity_maximum();
        }
        if (dashboardStore.currentIntensity === "high") return m.intensity_high();
        if (dashboardStore.currentIntensity === "medium") {
            return m.intensity_medium();
        }
        if (dashboardStore.currentIntensity === "low") return m.intensity_low();
        return m.status_off();
    });

    function findNextTransition(
        snapshots: NetworkBlockSnapshot[],
    ): ScheduledTransition | null {
        const now = new Date();
        const candidates: ScheduledTransition[] = [];

        for (const block of snapshots) {
            const transition = findNextScheduleTransition(
                block.scheduleEntries,
                now,
            );
            if (!transition) continue;
            candidates.push({ block, ...transition });
        }

        candidates.sort((a, b) => a.at.getTime() - b.at.getTime());
        return candidates[0] ?? null;
    }

    function transitionLabel(transition: ScheduledTransition): string {
        const time = transition.at.toLocaleString(getLocale(), {
            weekday: "short",
            hour: "2-digit",
            minute: "2-digit",
        });

        return transition.kind === "start"
            ? m.policy_starts_at({
                  name: transition.block.block.name,
                  time,
              })
            : m.policy_ends_at({
                  name: transition.block.block.name,
                  time,
              });
    }

    function resistanceLockText(): string | null {
        if (!dashboardStore.isLocked || !dashboardStore.lockEndRaw) return null;

        const end = new SvelteDate(dashboardStore.lockEndRaw);
        if (Number.isNaN(end.getTime())) return dashboardStore.lockEndRaw;

        return m.until_date({ date: end.toLocaleString(getLocale()) });
    }
</script>

<svelte:head>
    <title>{m.control_title()} · {m.app_name()}</title>
</svelte:head>

<PageLayout>
    {#snippet header()}
        <PageHeader
            title={m.control_title()}
            subtitle={m.control_subtitle()}
        />
    {/snippet}

    {#if !engineOnline}
        <div class="offline-board" role="status">
            <WifiOff size={20} aria-hidden="true" />
            <div>
                <strong>{m.status_engine_offline()}</strong>
                <p>{m.status_engine_offline_description()}</p>
                <a class="offline-guide-link" href={resolve("/getting-started")}>
                    {m.getting_started()}
                    <ArrowUpRight size={14} aria-hidden="true" />
                </a>
            </div>
        </div>
    {:else if dashboardStore.loadError}
        <div class="offline-board" role="alert">
            <WifiOff size={20} aria-hidden="true" />
            <div>
                <strong>{m.dashboard_load_failed_title()}</strong>
                <p>{m.dashboard_load_failed_description()}</p>
                <button type="button" onclick={() => dashboardStore.refresh()}>
                    {m.retry()}
                </button>
            </div>
        </div>
    {:else}
        <div class="control-surface">
            <div class="state-board">
                <section class="now-board" aria-labelledby="enforcing-now-title">
                    <div class="section-heading">
                        <h2 id="enforcing-now-title">{m.enforcing_now()}</h2>
                        <span class="count-readout">
                            {m.policy_count_active({
                                active: dashboardStore.activeBlocksCount,
                                total: dashboardStore.totalBlocksCount,
                            })}
                        </span>
                    </div>

                    {#if activeSnapshots.length > 0}
                        <div class="enforcement-list">
                            {#each activeSnapshots as snapshot (snapshot.block.id)}
                                <a
                                    class="enforcement-row"
                                    href={resolve(
                                        `/network-blocks?block=${encodeURIComponent(snapshot.block.id)}`,
                                    )}
                                >
                                    <span class="policy-rail" aria-hidden="true"></span>
                                    <span class="policy-copy">
                                        <strong>{snapshot.block.name}</strong>
                                        {#if snapshot.sitePackConnectionCount > 0 ||
                                        snapshot.userRulesCount > 0}
                                            <span class="policy-facts">
                                                {#if snapshot.sitePackConnectionCount > 0}
                                                    {snapshot.sitePackConnectionCount === 1
                                                        ? m.dashboard_policy_collection_count_one()
                                                        : m.dashboard_policy_collection_count_other({
                                                              count: snapshot.sitePackConnectionCount,
                                                          })}
                                                {/if}
                                                {#if snapshot.sitePackConnectionCount > 0 &&
                                                snapshot.userRulesCount > 0}
                                                    <span aria-hidden="true">·</span>
                                                {/if}
                                                {#if snapshot.userRulesCount > 0}
                                                    {snapshot.userRulesCount === 1
                                                        ? m.dashboard_policy_website_count_one()
                                                        : m.dashboard_policy_website_count_other({
                                                              count: snapshot.userRulesCount,
                                                          })}
                                                {/if}
                                            </span>
                                        {/if}
                                    </span>
                                    {#if snapshot.block.lockState.state !== "inactive"}
                                        <span class="sealed-note">
                                            <LockKeyhole size={13} aria-hidden="true" />
                                            {m.locked()}
                                        </span>
                                    {/if}
                                    <ArrowUpRight size={16} aria-hidden="true" />
                                </a>
                            {/each}
                        </div>
                    {:else}
                        <div class="empty-enforcement">
                            <ShieldCheck size={20} aria-hidden="true" />
                            <p>{m.no_active_enforcement()}</p>
                            <a href={resolve("/network-blocks")}>{m.manage_blocks()}</a>
                        </div>
                    {/if}
                </section>

                <aside class="change-board" aria-labelledby="next-change-title">
                    <div class="section-heading">
                        <h2 id="next-change-title">{m.next_change()}</h2>
                        <Clock3 size={18} aria-hidden="true" />
                    </div>

                    <div class="change-readout">
                        {#if nextTransition}
                            <time datetime={nextTransition.at.toISOString()}>
                                {nextTransition.at.toLocaleString(getLocale(), {
                                    month: "short",
                                    day: "numeric",
                                    hour: "2-digit",
                                    minute: "2-digit",
                                })}
                            </time>
                            <strong>{transitionLabel(nextTransition)}</strong>
                        {:else}
                            <span class="quiet-time">—</span>
                            <strong>{m.no_upcoming_changes()}</strong>
                        {/if}
                    </div>

                    <div class="resistance-readout">
                        <div>
                            <span>{m.resistance_state()}</span>
                            <strong>{resistanceLabel}</strong>
                        </div>
                        {#if resistanceLockText()}
                            <p>
                                <LockKeyhole size={13} aria-hidden="true" />
                                {resistanceLockText()}
                            </p>
                        {:else}
                            <p>
                                {dashboardStore.engineEnabled
                                    ? m.dashboard_resistance_active_description()
                                    : m.dashboard_resistance_inactive_description()}
                            </p>
                        {/if}
                        <a href={resolve("/resistance-mode")}>
                            {m.configure_resistance()}
                        </a>
                    </div>
                </aside>
            </div>

            <div class="activity-strip">
                <div>
                    <span>{m.blocked_attempts_title()}</span>
                    <strong>
                        {dashboardStore.blockedAttempts === 1
                            ? m.blocked_attempts_one()
                            : m.blocked_attempts_other({
                                  count: dashboardStore.blockedAttempts,
                              })}
                    </strong>
                </div>
                <p>{m.blocked_attempts_description()}</p>
            </div>
        </div>
    {/if}
</PageLayout>

<style>
    h2 {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .control-surface {
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 6px;
        background: var(--card);
    }

    .state-board {
        display: grid;
        grid-template-columns: minmax(0, 1.65fr) minmax(300px, 0.8fr);
        align-items: start;
        background: var(--card);
    }

    .now-board,
    .change-board {
        min-width: 0;
    }

    .now-board {
        align-self: start;
    }

    .change-board {
        border-left: 1px solid var(--border);
        background: color-mix(in srgb, var(--muted) 30%, var(--card));
    }

    .section-heading {
        min-height: 56px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        padding: 9px 16px;
        border-bottom: 1px solid var(--border);
    }

    h2 {
        margin: 0;
        font-size: 20px;
        font-weight: 600;
    }

    .count-readout {
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
    }

    .enforcement-list {
        display: grid;
    }

    .enforcement-row {
        position: relative;
        min-width: 0;
        min-height: 70px;
        display: grid;
        grid-template-columns: 6px minmax(0, 1fr) auto auto;
        align-items: center;
        gap: 13px;
        padding: 0 16px 0 0;
        border-bottom: 1px solid var(--border);
        color: var(--foreground);
        text-decoration: none;
        transition: background 140ms ease;
    }

    .enforcement-row:last-child {
        border-bottom: 0;
    }

    .enforcement-row:hover {
        background: color-mix(in srgb, var(--muted) 48%, transparent);
    }

    .policy-rail {
        align-self: stretch;
        background: var(--ace-signal);
    }

    .policy-copy {
        min-width: 0;
    }

    .policy-copy strong,
    .policy-facts {
        display: block;
    }

    .policy-copy strong {
        overflow: hidden;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .policy-facts {
        margin-top: 4px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
    }

    .policy-facts span {
        margin-inline: 5px;
    }

    .sealed-note {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        color: var(--foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 12px;
        font-weight: 600;
    }

    .empty-enforcement {
        min-height: 190px;
        display: grid;
        place-items: center;
        align-content: center;
        gap: 8px;
        padding: 28px;
        text-align: center;
    }

    .empty-enforcement p {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 13px;
    }

    .empty-enforcement a,
    .resistance-readout a {
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-underline-offset: 3px;
    }

    .change-readout {
        display: grid;
        align-content: start;
        gap: 4px;
        padding: 10px 16px 11px;
        border-bottom: 1px solid var(--border);
    }

    .change-readout time,
    .quiet-time {
        color: var(--foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        font-weight: 600;
    }

    .change-readout strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 19px;
        font-weight: 600;
        line-height: 1.1;
    }

    .resistance-readout {
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        align-items: start;
        gap: 3px 14px;
        padding: 9px 16px 10px;
    }

    .resistance-readout > div {
        grid-column: 1 / -1;
        display: flex;
        align-items: baseline;
        justify-content: space-between;
        gap: 16px;
    }

    .resistance-readout span {
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .resistance-readout strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
        font-weight: 600;
    }

    .resistance-readout p {
        grid-column: 1;
        display: flex;
        align-items: flex-start;
        gap: 6px;
        margin: 0;
        color: var(--muted-foreground);
        font-size: 10px;
        line-height: 1.35;
    }

    .resistance-readout > a {
        grid-column: 2;
        grid-row: 2;
        align-self: start;
        white-space: nowrap;
    }

    .activity-strip {
        display: grid;
        grid-template-columns: minmax(220px, auto) minmax(0, 1fr);
        align-items: center;
        gap: 22px;
        padding: 12px 16px;
        border-top: 1px solid var(--border);
        background: var(--background);
    }

    .activity-strip > div {
        display: flex;
        align-items: baseline;
        gap: 10px;
    }

    .activity-strip span {
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .activity-strip strong {
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        font-weight: 500;
    }

    .activity-strip p {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.4;
    }

    .offline-board {
        display: flex;
        gap: 13px;
        margin-top: 22px;
        padding: 18px;
        border: 1px solid color-mix(in srgb, var(--ace-danger) 38%, var(--border));
        border-left: 6px solid var(--ace-danger);
        border-radius: 5px;
        background: var(--card);
    }

    .offline-board strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 16px;
    }

    .offline-board p {
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
    }

    .offline-board button {
        min-height: 34px;
        margin-top: 12px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 11px;
        background: transparent;
        color: var(--foreground);
        cursor: pointer;
    }

    .offline-board button:hover {
        background: var(--muted);
    }

    .offline-guide-link {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        margin-top: 13px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: underline;
        text-underline-offset: 3px;
    }

    @container page (max-width: 760px) {
        .state-board {
            grid-template-columns: 1fr;
        }

        .change-board {
            border-top: 1px solid var(--border);
            border-left: 0;
        }
    }

    @container page (max-width: 560px) {
        .enforcement-row {
            grid-template-columns: 6px minmax(0, 1fr) auto;
        }

        .sealed-note {
            font-size: 11px;
        }

        .activity-strip {
            grid-template-columns: 1fr;
            gap: 6px;
        }
    }
</style>
