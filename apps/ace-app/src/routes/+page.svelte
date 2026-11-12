<script lang="ts">
    import { dashboardStore } from "$lib/stores/dashboard.svelte";
    import { engineStore } from "$lib/stores/engine.svelte";
    import {
        ArrowRight,
        ArrowUpRight,
        Clock3,
        LockKeyhole,
        Shield,
        ShieldCheck,
        SlidersHorizontal,
        WifiOff,
    } from "@lucide/svelte";
    import { SvelteDate } from "svelte/reactivity";
    import { resolve } from "$app/paths";
    import * as m from "$lib/paraglide/messages.js";
    import { countMessage } from "$lib/utils/count-message";
    import { getLocale } from "$lib/paraglide/runtime";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";
    import { findUpcomingBlockTransitions } from "$lib/utils/dashboard-transitions";

    const engineOnline = $derived(engineStore.isAlive);
    const activeSnapshots = $derived(
        dashboardStore.networkBlockSnapshots.filter(
            (snapshot) => snapshot.block.enabled && snapshot.block.isActive,
        ),
    );
    const previewSnapshots = $derived(activeSnapshots.slice(0, 5));
    const enabledSnapshots = $derived(
        dashboardStore.networkBlockSnapshots.filter((snapshot) => snapshot.block.enabled),
    );
    const enabledScheduledCount = $derived(
        enabledSnapshots.filter(
            (snapshot) => snapshot.block.scheduleMode === "scheduled" && snapshot.scheduleEntries.length > 0,
        ).length,
    );
    const upcomingTransitions = $derived(
        findUpcomingBlockTransitions(dashboardStore.networkBlockSnapshots),
    );

    const protectionTitle = $derived.by(() => {
        if (activeSnapshots.length > 0) return m.control_protection_active();
        if (dashboardStore.totalBlocksCount === 0) return m.control_protection_empty();
        if (enabledScheduledCount > 0) return m.control_protection_scheduled();
        if (enabledSnapshots.length > 0) return m.control_protection_needs_schedule();
        return m.control_protection_off();
    });

    const protectionDescription = $derived.by(() => {
        if (activeSnapshots.length > 0) {
            return countMessage(
                activeSnapshots.length,
                m.control_protection_active_detail_zero,
                m.control_protection_active_detail_one,
                () => m.control_protection_active_detail({ count: activeSnapshots.length }),
            );
        }
        if (dashboardStore.totalBlocksCount === 0) return m.control_protection_empty_detail();
        if (enabledScheduledCount > 0) return m.control_protection_scheduled_detail();
        if (enabledSnapshots.length > 0) return m.control_protection_needs_schedule_detail();
        return m.control_protection_off_detail();
    });

    const resistanceLabel = $derived.by(() => {
        if (!dashboardStore.engineEnabled) return m.status_off();
        if (dashboardStore.resistanceStatus === "operational") return m.status_on_operational();
        return m.status_on_degraded();
    });

    function transitionTime(value: Date): string {
        return value.toLocaleString(getLocale(), {
            weekday: "short",
            hour: "2-digit",
            minute: "2-digit",
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

<PageLayout width="dashboard">
    {#snippet header()}
        <PageHeader title={m.control_title()} subtitle={m.control_subtitle()} />
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
        <div class="control-dashboard">
            <section class="status-band" class:quiet={activeSnapshots.length === 0} aria-labelledby="protection-title">
                <span class="status-icon" aria-hidden="true">
                    {#if activeSnapshots.length > 0}
                        <ShieldCheck size={23} strokeWidth={1.8} />
                    {:else}
                        <Shield size={23} strokeWidth={1.8} />
                    {/if}
                </span>
                <div class="status-copy">
                    <h2 id="protection-title">{protectionTitle}</h2>
                    <p>{protectionDescription}</p>
                </div>
                <div class="status-metrics">
                    <div class="inline-metric">
                        <strong>{activeSnapshots.length}<span> / {dashboardStore.totalBlocksCount}</span></strong>
                        <small>{m.control_metric_enforcing()}</small>
                    </div>
                    <div class="inline-metric">
                        <strong>{dashboardStore.blockedAttempts}</strong>
                        <small>{m.blocked_attempts_title()}</small>
                    </div>
                </div>
            </section>

            <div class="dashboard-grid">
                <section class="surface-panel active-panel" aria-labelledby="enforcing-now-title">
                    <header class="panel-heading">
                        <h2 id="enforcing-now-title">{m.enforcing_now()}</h2>
                        {#if dashboardStore.totalBlocksCount > 0 && activeSnapshots.length <= previewSnapshots.length}
                            <a class="panel-link" href={resolve("/network-blocks")}>
                                {m.control_view_all_blocks()}
                                <ArrowUpRight size={15} aria-hidden="true" />
                            </a>
                        {/if}
                    </header>

                    {#if activeSnapshots.length > 0}
                        <div class="enforcement-list">
                            {#each previewSnapshots as snapshot (snapshot.block.id)}
                                <a
                                    class="enforcement-row"
                                    href={resolve("/network-blocks") + "?block=" + encodeURIComponent(snapshot.block.id)}
                                >
                                    <span class="policy-rail" aria-hidden="true"></span>
                                    <span class="policy-copy">
                                        <strong>{snapshot.block.name}</strong>
                                        <span class="policy-facts">
                                            {#if snapshot.sitePackConnectionCount > 0}
                                                {snapshot.sitePackConnectionCount === 1
                                                    ? m.dashboard_policy_collection_count_one()
                                                    : m.dashboard_policy_collection_count_other({
                                                        count: snapshot.sitePackConnectionCount,
                                                    })}
                                            {/if}
                                            {#if snapshot.sitePackConnectionCount > 0 && snapshot.userRulesCount > 0}
                                                <span aria-hidden="true">·</span>
                                            {/if}
                                            {#if snapshot.userRulesCount > 0}
                                                {snapshot.userRulesCount === 1
                                                    ? m.dashboard_policy_website_count_one()
                                                    : m.dashboard_policy_website_count_other({
                                                        count: snapshot.userRulesCount,
                                                    })}
                                            {/if}
                                            {#if snapshot.sitePackConnectionCount === 0 && snapshot.userRulesCount === 0}
                                                {m.control_block_no_rules()}
                                            {/if}
                                        </span>
                                    </span>
                                    {#if snapshot.block.lockState.state !== "inactive"}
                                        <span class="sealed-note" title={m.control_lock_explanation_short()}>
                                            <LockKeyhole size={14} aria-hidden="true" />
                                            {snapshot.block.isLocked
                                                ? m.control_lock_active_label()
                                                : m.control_lock_configured_label()}
                                        </span>
                                    {/if}
                                    <ArrowUpRight class="row-arrow" size={15} aria-hidden="true" />
                                </a>
                            {/each}
                            {#if activeSnapshots.length > previewSnapshots.length}
                                <a class="preview-footer" href={resolve("/network-blocks")}>
                                    {m.control_view_all_blocks()}
                                    <ArrowRight size={15} aria-hidden="true" />
                                </a>
                            {/if}
                        </div>
                    {:else}
                        <div class="empty-enforcement">
                            <Shield size={24} aria-hidden="true" />
                            <strong>{m.control_no_enforcement_title()}</strong>
                            <p>
                                {dashboardStore.totalBlocksCount === 0
                                    ? m.no_network_blocks_description()
                                    : m.no_active_enforcement()}
                            </p>
                            {#if dashboardStore.totalBlocksCount === 0}
                                <a class="empty-action" href={resolve("/network-blocks/editor")}>
                                    {m.create_block()}
                                    <ArrowRight size={15} aria-hidden="true" />
                                </a>
                            {/if}
                        </div>
                    {/if}
                </section>

                <section class="surface-panel upcoming-panel" aria-labelledby="upcoming-title">
                    <header class="panel-heading">
                        <h2 id="upcoming-title">{m.control_upcoming_title()}</h2>
                        <Clock3 size={17} aria-hidden="true" />
                    </header>
                    {#if upcomingTransitions.length > 0}
                        <ol class="upcoming-list">
                            {#each upcomingTransitions as transition, index (transition.block.block.id + transition.at.toISOString() + index)}
                                <li class:next={index === 0}>
                                    <span class="timeline-rail" aria-hidden="true"></span>
                                    <time datetime={transition.at.toISOString()}>{transitionTime(transition.at)}</time>
                                    <span class="upcoming-copy">
                                        <strong>{transition.block.block.name}</strong>
                                        <small>
                                            {transition.kind === "start"
                                                ? m.control_schedule_starts()
                                                : m.control_schedule_ends()}
                                        </small>
                                    </span>
                                </li>
                            {/each}
                        </ol>
                    {:else}
                        <div class="upcoming-empty">
                            <Clock3 size={23} aria-hidden="true" />
                            <p>{m.no_upcoming_changes()}</p>
                        </div>
                    {/if}
                </section>
            </div>

            <a class="resistance-strip" href={resolve("/resistance-mode")}>
                <span class="resistance-icon" aria-hidden="true">
                    <SlidersHorizontal size={18} strokeWidth={1.8} />
                </span>
                <strong>{m.resistance_state()}</strong>
                {#if resistanceLockText()}
                    <span class="resistance-lock">
                        <LockKeyhole size={14} aria-hidden="true" />
                        {resistanceLockText()}
                    </span>
                {/if}
                <span class="resistance-state">{resistanceLabel}</span>
                <ArrowUpRight size={16} aria-hidden="true" />
            </a>
        </div>
    {/if}
</PageLayout>

<style>
    h2 {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .control-dashboard {
        display: grid;
        gap: 14px;
    }

    .status-band,
    .surface-panel,
    .resistance-strip {
        min-width: 0;
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 6px;
        background: var(--card);
    }

    .status-band {
        display: flex;
        align-items: center;
        gap: 17px;
        padding: 18px 22px;
        border-top: 3px solid var(--ace-signal);
    }

    .status-band.quiet {
        border-top-color: var(--muted-foreground);
    }

    .status-icon {
        display: grid;
        width: 46px;
        height: 46px;
        flex: none;
        place-items: center;
        border: 1px solid color-mix(in srgb, var(--ace-signal-deep) 35%, var(--border));
        border-radius: 5px;
        background: color-mix(in srgb, var(--ace-signal) 10%, var(--card));
        color: var(--ace-signal-deep);
    }

    .quiet .status-icon {
        border-color: var(--border);
        background: var(--muted);
        color: var(--muted-foreground);
    }

    .status-copy {
        min-width: 0;
        flex: 1 1 auto;
    }

    .status-copy h2 {
        margin: 0;
        font-size: 25px;
        font-weight: 600;
        line-height: 1.2;
    }

    .status-copy p {
        max-width: 570px;
        margin: 5px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.5;
    }

    .status-metrics {
        display: flex;
        flex: none;
        align-items: center;
        gap: 23px;
        padding-left: 23px;
        border-left: 1px solid var(--border);
    }

    .inline-metric {
        display: grid;
        gap: 4px;
        text-align: right;
    }

    .inline-metric strong {
        font-family: "IBM Plex Mono", monospace;
        font-size: 21px;
        font-weight: 600;
        line-height: 1.15;
        white-space: nowrap;
    }

    .inline-metric strong span {
        color: var(--muted-foreground);
        font-size: 13px;
        font-weight: 400;
    }

    .inline-metric small {
        color: var(--muted-foreground);
        font-size: 10px;
        line-height: 1.4;
    }

    .dashboard-grid {
        display: grid;
        grid-template-columns: minmax(0, 1.65fr) minmax(285px, 0.9fr);
        align-items: start;
        gap: 14px;
    }

    .panel-heading {
        display: flex;
        min-height: 62px;
        align-items: center;
        justify-content: space-between;
        gap: 12px;
        padding: 15px 19px;
        border-bottom: 1px solid var(--border);
    }

    .panel-heading h2 {
        margin: 0;
        font-size: 21px;
        font-weight: 600;
        line-height: 1.2;
    }

    .panel-heading :global(svg) {
        color: var(--muted-foreground);
        flex: none;
    }

    .panel-link {
        display: inline-flex;
        flex: none;
        align-items: center;
        gap: 6px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
    }

    .panel-link:hover {
        text-decoration: underline;
        text-underline-offset: 3px;
    }

    .enforcement-list {
        display: grid;
    }

    .enforcement-row {
        display: grid;
        position: relative;
        min-width: 0;
        min-height: 65px;
        grid-template-columns: minmax(0, 1fr) auto auto;
        align-items: center;
        gap: 12px;
        padding: 11px 16px 11px 20px;
        border-bottom: 1px solid var(--border);
        color: var(--foreground);
        text-decoration: none;
        transition: background 140ms ease;
    }

    .enforcement-row:last-child {
        border-bottom: 0;
    }

    .preview-footer {
        display: flex;
        min-height: 44px;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        padding: 10px 18px;
        border-top: 1px solid var(--border);
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
    }

    .enforcement-row:hover,
    .preview-footer:hover,
    .resistance-strip:hover {
        background: color-mix(in srgb, var(--muted) 65%, transparent);
    }

    .policy-rail {
        position: absolute;
        inset: 0 auto 0 0;
        width: 4px;
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
        margin-top: 3px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 10px;
        line-height: 1.4;
    }

    .policy-facts span {
        margin-inline: 5px;
    }

    .sealed-note {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        color: var(--muted-foreground);
        font-size: 11px;
        font-weight: 600;
        white-space: nowrap;
    }

    .enforcement-row :global(.row-arrow) {
        color: var(--muted-foreground);
    }

    .empty-enforcement {
        display: grid;
        min-height: 175px;
        place-items: center;
        align-content: center;
        gap: 7px;
        padding: 21px;
        text-align: center;
    }

    .empty-enforcement :global(svg),
    .upcoming-empty :global(svg) {
        color: var(--muted-foreground);
    }

    .empty-enforcement strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
        font-weight: 600;
    }

    .empty-enforcement p {
        max-width: 360px;
        margin: 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    .empty-action {
        display: inline-flex;
        min-height: 34px;
        align-items: center;
        gap: 8px;
        margin-top: 7px;
        padding: 0 11px;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
    }

    .empty-action:hover {
        background: var(--muted);
    }

    .upcoming-list {
        display: grid;
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .upcoming-list li {
        display: grid;
        position: relative;
        grid-template-columns: 4px minmax(0, 1fr);
        gap: 3px 11px;
        align-items: center;
        padding: 13px 18px;
        border-bottom: 1px solid var(--border);
    }

    .upcoming-list li:last-child {
        border-bottom: 0;
    }

    .timeline-rail {
        grid-row: span 2;
        align-self: stretch;
        border-radius: 3px;
        background: var(--border);
    }

    .upcoming-list li.next .timeline-rail {
        background: var(--ace-signal);
    }

    .upcoming-list time {
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        line-height: 1.4;
    }

    .upcoming-list li.next time {
        color: var(--foreground);
        font-size: 16px;
        font-weight: 600;
    }

    .upcoming-copy {
        display: grid;
        min-width: 0;
        gap: 2px;
    }

    .upcoming-copy strong {
        overflow: hidden;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 16px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .upcoming-copy small {
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .upcoming-empty {
        display: grid;
        min-height: 175px;
        place-items: center;
        align-content: center;
        gap: 10px;
        padding: 18px;
        color: var(--muted-foreground);
        text-align: center;
    }

    .upcoming-empty p {
        max-width: 250px;
        margin: 0;
        font-size: 12px;
        line-height: 1.5;
    }

    .resistance-strip {
        display: flex;
        min-height: 61px;
        align-items: center;
        gap: 13px;
        padding: 11px 17px;
        color: var(--foreground);
        text-decoration: none;
        transition: background 140ms ease;
    }

    .resistance-icon {
        display: grid;
        width: 34px;
        height: 34px;
        flex: none;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--muted-foreground);
    }

    .resistance-strip > strong {
        min-width: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
        font-weight: 600;
    }

    .resistance-lock {
        display: inline-flex;
        min-width: 0;
        align-items: center;
        gap: 6px;
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .resistance-lock :global(svg) {
        flex: none;
    }

    .resistance-state {
        margin-left: auto;
        color: var(--muted-foreground);
        font-size: 12px;
        font-weight: 600;
        white-space: nowrap;
    }

    .resistance-strip > :global(svg) {
        flex: none;
        color: var(--muted-foreground);
    }

    .offline-board {
        display: flex;
        gap: 13px;
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
        padding: 0 11px;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: transparent;
        color: var(--foreground);
        cursor: pointer;
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

    .control-dashboard :is(a, button):focus-visible,
    .offline-board :is(a, button):focus-visible {
        outline: 2px solid var(--foreground);
        outline-offset: 3px;
    }

    @container page (max-width: 930px) {
        .dashboard-grid {
            grid-template-columns: minmax(0, 1fr);
        }
    }

    @container page (max-width: 700px) {
        .status-band {
            flex-wrap: wrap;
            padding: 16px;
            gap: 12px;
        }

        .status-metrics {
            width: 100%;
            justify-content: flex-start;
            gap: 35px;
            padding: 12px 0 0;
            border-top: 1px solid var(--border);
            border-left: 0;
        }

        .inline-metric {
            text-align: left;
        }
    }

    @container page (max-width: 480px) {
        .status-icon {
            width: 40px;
            height: 40px;
        }

        .status-copy h2 {
            font-size: 22px;
        }

        .status-metrics {
            justify-content: space-between;
            gap: 12px;
        }

        .panel-heading {
            flex-wrap: wrap;
            padding: 14px 15px;
        }

        .enforcement-row {
            grid-template-columns: minmax(0, 1fr) auto;
            padding: 12px 14px 12px 18px;
        }

        .enforcement-row :global(.row-arrow) {
            display: none;
        }

        .sealed-note {
            font-size: 10px;
        }

        .resistance-strip {
            flex-wrap: wrap;
            gap: 8px 12px;
        }

        .resistance-lock {
            order: 5;
            flex-basis: 100%;
            padding-left: 46px;
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .enforcement-row,
        .resistance-strip {
            transition: none;
        }
    }
</style>
