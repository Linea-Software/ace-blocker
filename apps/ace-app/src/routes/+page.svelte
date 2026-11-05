<script lang="ts">
    import { dashboardStore } from "$lib/stores/dashboard.svelte";
    import { engineStore } from "$lib/stores/engine.svelte";
    import type { NetworkBlockSnapshot } from "$engine-client";
    import {
        ArrowRight,
        ArrowUpRight,
        Clock3,
        LockKeyhole,
        Shield,
        ShieldCheck,
        WifiOff,
    } from "@lucide/svelte";
    import { SvelteDate } from "svelte/reactivity";
    import { resolve } from "$app/paths";
    import * as m from "$lib/paraglide/messages.js";
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
    const enabledSnapshots = $derived(
        dashboardStore.networkBlockSnapshots.filter((snapshot) => snapshot.block.enabled),
    );
    const enabledScheduledCount = $derived(
        enabledSnapshots.filter((snapshot) => snapshot.scheduleEntries.length > 0).length,
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
            return m.control_protection_active_detail({
                count: activeSnapshots.length,
            });
        }
        if (dashboardStore.totalBlocksCount === 0) return m.control_protection_empty_detail();
        if (enabledScheduledCount > 0) return m.control_protection_scheduled_detail();
        if (enabledSnapshots.length > 0) return m.control_protection_needs_schedule_detail();
        return m.control_protection_off_detail();
    });

    const resistanceLabel = $derived.by(() => {
        if (!dashboardStore.engineEnabled) return m.status_off();
        if (dashboardStore.currentIntensity === "extreme") return m.intensity_maximum();
        if (dashboardStore.currentIntensity === "high") return m.intensity_high();
        if (dashboardStore.currentIntensity === "medium") return m.intensity_medium();
        if (dashboardStore.currentIntensity === "low") return m.intensity_low();
        return m.status_off();
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
            <section class="protection-overview" class:quiet={activeSnapshots.length === 0} aria-labelledby="protection-title">
                <div class="hero-row">
                    <span class="hero-symbol" aria-hidden="true">
                        {#if activeSnapshots.length > 0}
                            <ShieldCheck size={26} strokeWidth={1.8} />
                        {:else}
                            <Shield size={26} strokeWidth={1.8} />
                        {/if}
                    </span>
                    <div class="hero-copy">
                        <span class="eyebrow">{m.control_protection_heading()}</span>
                        <h2 id="protection-title">{protectionTitle}</h2>
                        <p>{protectionDescription}</p>
                    </div>
                    <a class="hero-action" href={resolve("/network-blocks")}>
                        {m.manage_blocks()}
                        <ArrowUpRight size={16} aria-hidden="true" />
                    </a>
                </div>

                <div class="metric-grid">
                    <div class="metric">
                        <span>{m.control_metric_enforcing()}</span>
                        <strong class="metric-number">
                            {activeSnapshots.length}
                            <span> / {dashboardStore.totalBlocksCount}</span>
                        </strong>
                        <small>{m.control_metric_enforcing_detail()}</small>
                    </div>
                    <div class="metric">
                        <span>{m.blocked_attempts_title()}</span>
                        <strong class="metric-number">{dashboardStore.blockedAttempts}</strong>
                        <small>{m.control_metric_blocked_detail()}</small>
                    </div>
                    <div class="metric">
                        <span>{m.resistance_state()}</span>
                        <strong class="metric-status">{resistanceLabel}</strong>
                        <small>{m.control_metric_resistance_detail()}</small>
                    </div>
                </div>
            </section>

            <div class="dashboard-grid">
                <section class="surface-panel active-panel" aria-labelledby="enforcing-now-title">
                    <header class="panel-heading">
                        <div>
                            <span class="eyebrow">{m.control_now_eyebrow()}</span>
                            <h2 id="enforcing-now-title">{m.enforcing_now()}</h2>
                        </div>
                        <span class="count-readout">
                            {m.policy_count_active({
                                active: activeSnapshots.length,
                                total: dashboardStore.totalBlocksCount,
                            })}
                        </span>
                    </header>

                    {#if activeSnapshots.length > 0}
                        <div class="enforcement-list">
                            {#each activeSnapshots as snapshot (snapshot.block.id)}
                                <a
                                    class="enforcement-row"
                                    href={resolve(`/network-blocks?block=${encodeURIComponent(snapshot.block.id)}`)}
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
                                    <ArrowUpRight class="row-arrow" size={16} aria-hidden="true" />
                                </a>
                            {/each}
                        </div>
                    {:else}
                        <div class="empty-enforcement">
                            <Shield size={26} aria-hidden="true" />
                            <strong>{m.control_no_enforcement_title()}</strong>
                            <p>{m.no_active_enforcement()}</p>
                            <a href={resolve("/network-blocks")}>{m.manage_blocks()}</a>
                        </div>
                    {/if}
                    <footer class="panel-footer">
                        <p>{m.control_active_panel_hint()}</p>
                        <a href={resolve("/network-blocks")}>
                            {m.control_view_all_blocks()}
                            <ArrowRight size={15} aria-hidden="true" />
                        </a>
                    </footer>
                </section>

                <div class="side-stack">
                    <section class="surface-panel upcoming-panel" aria-labelledby="upcoming-title">
                        <header class="panel-heading">
                            <div>
                                <span class="eyebrow">{m.control_next_eyebrow()}</span>
                                <h2 id="upcoming-title">{m.control_upcoming_title()}</h2>
                            </div>
                            <Clock3 size={18} aria-hidden="true" />
                        </header>
                        {#if upcomingTransitions.length > 0}
                            <ol class="upcoming-list">
                                {#each upcomingTransitions as transition, index (transition.block.block.id + transition.at.toISOString() + index)}
                                    <li>
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
                                <Clock3 size={22} aria-hidden="true" />
                                <p>{m.no_upcoming_changes()}</p>
                            </div>
                        {/if}
                        <p class="panel-caption">{m.control_upcoming_note()}</p>
                    </section>

                    <section class="surface-panel resistance-panel" aria-labelledby="resistance-title">
                        <header class="panel-heading">
                            <div>
                                <span class="eyebrow">{m.control_resistance_eyebrow()}</span>
                                <h2 id="resistance-title">{m.resistance_mode()}</h2>
                            </div>
                        </header>
                        <div class="resistance-content">
                            <strong>{resistanceLabel}</strong>
                            {#if resistanceLockText()}
                                <p class="resistance-lock">
                                    <LockKeyhole size={14} aria-hidden="true" />
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
                                <ArrowUpRight size={15} aria-hidden="true" />
                            </a>
                        </div>
                    </section>
                </div>
            </div>

            <section class="lock-help-strip" aria-labelledby="control-lock-help-title">
                <span class="lock-help-icon" aria-hidden="true">
                    <LockKeyhole size={21} strokeWidth={1.8} />
                </span>
                <div>
                    <h2 id="control-lock-help-title">{m.control_lock_help_title()}</h2>
                    <p>{m.control_lock_help_detail()}</p>
                </div>
                <a href={resolve("/network-blocks")}>
                    {m.control_lock_help_action()}
                    <ArrowRight size={15} aria-hidden="true" />
                </a>
            </section>
        </div>
    {/if}
</PageLayout>

<style>
    h2 {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .control-dashboard {
        display: grid;
        gap: 18px;
    }

    .protection-overview,
    .surface-panel,
    .lock-help-strip {
        min-width: 0;
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 6px;
        background: var(--card);
    }

    .protection-overview {
        border-top: 3px solid var(--ace-signal);
    }

    .protection-overview.quiet {
        border-top-color: var(--muted-foreground);
    }

    .hero-row {
        display: flex;
        align-items: center;
        gap: 20px;
        padding: 26px 28px;
    }

    .hero-symbol {
        display: grid;
        width: 56px;
        height: 56px;
        flex: none;
        place-items: center;
        border: 1px solid color-mix(in srgb, var(--ace-signal-deep) 35%, var(--border));
        border-radius: 6px;
        background: color-mix(in srgb, var(--ace-signal) 10%, var(--card));
        color: var(--ace-signal-deep);
    }

    .quiet .hero-symbol {
        border-color: var(--border);
        background: var(--muted);
        color: var(--muted-foreground);
    }

    .hero-copy {
        min-width: 0;
        flex: 1;
    }

    .eyebrow {
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 10px;
        font-weight: 600;
        letter-spacing: 0.08em;
        text-transform: uppercase;
    }

    .hero-copy h2 {
        margin: 5px 0 0;
        font-size: clamp(25px, 2.6cqi, 34px);
        font-weight: 600;
        line-height: 1.15;
    }

    .hero-copy p {
        max-width: 580px;
        margin: 7px 0 0;
        color: var(--muted-foreground);
        font-size: 13px;
        line-height: 1.55;
    }

    .hero-action {
        display: inline-flex;
        flex: none;
        align-items: center;
        justify-content: center;
        gap: 8px;
        min-height: 39px;
        padding: 0 13px;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
    }

    .metric-grid {
        display: grid;
        grid-template-columns: repeat(3, minmax(0, 1fr));
        border-top: 1px solid var(--border);
        background: color-mix(in srgb, var(--muted) 22%, var(--card));
    }

    .metric {
        display: flex;
        min-width: 0;
        flex-direction: column;
        gap: 5px;
        padding: 17px 26px;
    }

    .metric + .metric {
        border-left: 1px solid var(--border);
    }

    .metric > span {
        color: var(--muted-foreground);
        font-size: 12px;
    }

    .metric-number {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 32px;
        font-weight: 600;
        line-height: 1;
    }

    .metric-number span {
        color: var(--muted-foreground);
        font-size: 17px;
        font-weight: 400;
    }

    .metric-status {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 24px;
        line-height: 1.3;
        font-weight: 600;
    }

    .metric small {
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.35;
    }

    .dashboard-grid {
        display: grid;
        grid-template-columns: minmax(0, 1.55fr) minmax(310px, 0.9fr);
        align-items: start;
        gap: 18px;
    }

    .active-panel {
        display: flex;
        min-height: 370px;
        flex-direction: column;
    }

    .panel-heading {
        display: flex;
        min-height: 73px;
        align-items: center;
        justify-content: space-between;
        gap: 16px;
        padding: 17px 21px;
        border-bottom: 1px solid var(--border);
    }

    .panel-heading h2 {
        margin: 3px 0 0;
        font-size: 23px;
        font-weight: 600;
    }

    .panel-heading :global(svg) {
        color: var(--muted-foreground);
    }

    .count-readout {
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        text-align: right;
    }

    .enforcement-list {
        display: grid;
        flex: 1;
        align-content: start;
    }

    .enforcement-row {
        display: grid;
        position: relative;
        min-width: 0;
        min-height: 72px;
        grid-template-columns: minmax(0, 1fr) auto auto;
        align-items: center;
        gap: 13px;
        padding: 12px 18px 12px 25px;
        border-bottom: 1px solid var(--border);
        color: var(--foreground);
        text-decoration: none;
        transition: background 140ms ease;
    }

    .enforcement-row:hover,
    .hero-action:hover,
    .lock-help-strip a:hover {
        background: color-mix(in srgb, var(--muted) 65%, transparent);
    }

    .policy-rail {
        position: absolute;
        inset: 0 auto 0 0;
        width: 5px;
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
        font-size: 19px;
        font-weight: 600;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .policy-facts {
        margin-top: 4px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        line-height: 1.4;
    }

    .policy-facts span {
        margin-inline: 6px;
    }

    .sealed-note {
        display: inline-flex;
        align-items: center;
        gap: 5px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 12px;
        font-weight: 600;
    }

    .empty-enforcement {
        display: grid;
        min-height: 205px;
        flex: 1;
        place-items: center;
        align-content: center;
        gap: 9px;
        padding: 28px 20px;
        text-align: center;
    }

    .empty-enforcement :global(svg) {
        color: var(--muted-foreground);
    }

    .empty-enforcement strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
    }

    .empty-enforcement p {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 13px;
    }

    .empty-enforcement a,
    .panel-footer a,
    .resistance-content a {
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: underline;
        text-underline-offset: 3px;
    }

    .panel-footer {
        display: flex;
        min-height: 54px;
        align-items: center;
        justify-content: space-between;
        gap: 15px;
        padding: 12px 21px;
        border-top: 1px solid var(--border);
        margin-top: auto;
    }

    .panel-footer p {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.5;
    }

    .panel-footer a,
    .resistance-content a {
        display: inline-flex;
        flex: none;
        align-items: center;
        gap: 6px;
    }

    .side-stack {
        display: grid;
        min-width: 0;
        gap: 18px;
    }

    .upcoming-list {
        display: grid;
        margin: 0;
        padding: 0 21px;
        list-style: none;
    }

    .upcoming-list li {
        display: grid;
        position: relative;
        grid-template-columns: 4px minmax(90px, auto) minmax(0, 1fr);
        align-items: center;
        gap: 13px;
        padding: 12px 0;
        border-bottom: 1px solid var(--border);
    }

    .upcoming-list li:last-child {
        border-bottom: 0;
    }

    .timeline-rail {
        height: 28px;
        border-radius: 3px;
        background: var(--ace-signal);
    }

    .upcoming-list time {
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        color: var(--muted-foreground);
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
        font-size: 11px;
        color: var(--muted-foreground);
    }

    .upcoming-empty {
        display: grid;
        min-height: 100px;
        place-items: center;
        align-content: center;
        gap: 9px;
        padding: 20px;
        text-align: center;
        color: var(--muted-foreground);
    }

    .upcoming-empty p {
        margin: 0;
        font-size: 13px;
    }

    .panel-caption {
        padding: 12px 21px;
        border-top: 1px solid var(--border);
        margin: 0;
        color: var(--muted-foreground);
        font-size: 11px;
        line-height: 1.4;
    }

    .resistance-content {
        display: grid;
        gap: 9px;
        padding: 18px 21px;
    }

    .resistance-content > strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 24px;
        font-weight: 600;
    }

    .resistance-content p {
        display: flex;
        align-items: flex-start;
        gap: 6px;
        margin: 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.5;
    }

    .resistance-content a {
        justify-self: start;
        margin-top: 4px;
    }

    .lock-help-strip {
        display: flex;
        align-items: center;
        gap: 18px;
        padding: 18px 21px;
        background: color-mix(in srgb, var(--muted) 24%, var(--card));
    }

    .lock-help-icon {
        display: grid;
        width: 42px;
        height: 42px;
        flex: none;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        background: var(--card);
    }

    .lock-help-strip > div {
        min-width: 0;
        flex: 1;
    }

    .lock-help-strip h2 {
        margin: 0;
        font-size: 19px;
        font-weight: 600;
    }

    .lock-help-strip p {
        max-width: 680px;
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    .lock-help-strip a {
        display: inline-flex;
        flex: none;
        min-height: 39px;
        align-items: center;
        justify-content: center;
        gap: 6px;
        padding: 0 13px;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
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
            grid-template-columns: 1fr;
        }

        .side-stack {
            grid-template-columns: repeat(2, minmax(0, 1fr));
            align-items: start;
        }
    }

    @container page (max-width: 690px) {
        .hero-row {
            align-items: flex-start;
            flex-wrap: wrap;
            padding: 20px;
            gap: 13px;
        }

        .hero-action {
            margin-left: 69px;
        }

        .metric {
            padding: 14px 15px;
        }

        .side-stack {
            grid-template-columns: 1fr;
        }

        .lock-help-strip {
            align-items: flex-start;
            flex-wrap: wrap;
        }

        .lock-help-strip a {
            margin-left: 60px;
        }
    }

    @container page (max-width: 480px) {
        .metric-grid {
            grid-template-columns: 1fr;
        }

        .metric {
            padding: 14px 20px;
        }

        .metric + .metric {
            border-top: 1px solid var(--border);
            border-left: 0;
        }

        .hero-symbol {
            width: 43px;
            height: 43px;
        }

        .hero-action {
            margin-left: 0;
            width: 100%;
        }

        .panel-heading {
            flex-wrap: wrap;
            padding: 15px;
        }

        .enforcement-row {
            grid-template-columns: minmax(0, 1fr) auto;
            padding: 12px 14px 12px 18px;
        }

        .enforcement-row :global(.row-arrow) {
            display: none;
        }

        .sealed-note {
            font-size: 11px;
        }

        .panel-footer {
            align-items: flex-start;
            flex-direction: column;
            gap: 5px;
            padding: 14px 15px;
        }

        .upcoming-list {
            padding-inline: 14px;
        }

        .upcoming-list li {
            grid-template-columns: 4px minmax(74px, auto) minmax(0, 1fr);
            gap: 8px;
        }

        .lock-help-strip a {
            margin-left: 0;
            width: 100%;
        }
    }

    @media (prefers-reduced-motion: reduce) {
        .enforcement-row {
            transition: none;
        }
    }
</style>
