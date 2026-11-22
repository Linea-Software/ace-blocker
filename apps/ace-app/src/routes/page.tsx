import { createEffect, createMemo, For, onSettled } from "solid-js";
import { dashboardStore } from "$lib/stores/dashboard";
import { engineStore } from "$lib/stores/engine";
import { ArrowRight, ArrowUpRight, Clock3, LockKeyhole, Shield, ShieldCheck, SlidersHorizontal, WifiOff } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import { countMessage } from "$lib/utils/count-message";
import { getLocale } from "$lib/paraglide/runtime";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";
import { findUpcomingBlockTransitions } from "$lib/utils/dashboard-transitions";
import "./page.css";

export default function DashboardPage() {
 const engineOnline = () => engineStore.isAlive;
 createEffect(() => engineStore.isAlive, alive => {
  // Browser-only fixture previews have no privileged remoc channel.
  // Refresh only when the native Tauri runtime actually owns that channel.
  if (alive && engineStore.isTauri) void dashboardStore.refresh();
 });
 onSettled(() => { document.title = `${m.control_title()} · ${m.app_name()}`; });
 const activeSnapshots = createMemo(() => dashboardStore.networkBlockSnapshots.filter(snapshot=> snapshot.block.enabled && snapshot.block.isActive));
 const previewSnapshots = createMemo(() => activeSnapshots().slice(0, 5));
 const enabledSnapshots = createMemo(() => dashboardStore.networkBlockSnapshots.filter(snapshot=>snapshot.block.enabled));
 const enabledScheduledCount = createMemo(() => enabledSnapshots().filter(snapshot=>snapshot.block.scheduleMode === "scheduled" && snapshot.scheduleEntries.length > 0).length);
 const upcomingTransitions = createMemo(() => findUpcomingBlockTransitions(dashboardStore.networkBlockSnapshots));
 const protectionTitle = createMemo(() => {
   if (activeSnapshots().length > 0) return m.control_protection_active();
   if (dashboardStore.totalBlocksCount === 0) return m.control_protection_empty();
   if (enabledScheduledCount() > 0) return m.control_protection_scheduled();
   if (enabledSnapshots().length > 0) return m.control_protection_needs_schedule();
   return m.control_protection_off();
 });
 const protectionDescription = createMemo(() => {
   if (activeSnapshots().length > 0) {
     return countMessage(activeSnapshots().length,
       m.control_protection_active_detail_zero,
       m.control_protection_active_detail_one,
       () => m.control_protection_active_detail({count: activeSnapshots().length}));
   }
   if (dashboardStore.totalBlocksCount === 0) return m.control_protection_empty_detail();
   if (enabledScheduledCount() > 0) return m.control_protection_scheduled_detail();
   if (enabledSnapshots().length > 0) return m.control_protection_needs_schedule_detail();
   return m.control_protection_off_detail();
 });
 const resistanceLabel = () => {
   if (!dashboardStore.engineEnabled) return m.status_off();
   if (dashboardStore.resistanceStatus === "operational") return m.status_on_operational();
   return m.status_on_degraded();
 };
 function transitionTime(value:Date):string {
   return value.toLocaleString(getLocale(), {weekday:"short", hour:"2-digit", minute:"2-digit"});
 }
 function resistanceLockText():string|null {
   if (!dashboardStore.isLocked || !dashboardStore.lockEndRaw) return null;
   const end = new Date(dashboardStore.lockEndRaw);
   if (Number.isNaN(end.getTime())) return dashboardStore.lockEndRaw;
   return m.until_date({date: end.toLocaleString(getLocale())});
 }
 return (
<PageLayout width="dashboard" header={<PageHeader title={m.control_title()} subtitle={m.control_subtitle()} />}>


    {!engineOnline() ? (<>
        <div class="offline-board" role="status">
            <WifiOff size={20} aria-hidden="true" />
            <div>
                <strong>{m.status_engine_offline()}</strong>
                <p>{m.status_engine_offline_description()}</p>
                <a class="offline-guide-link" href={"/getting-started"}>
                    {m.getting_started()}
                    <ArrowUpRight size={14} aria-hidden="true" />
                </a>
            </div>
        </div>
    </>) : dashboardStore.loadError ? (<>
        <div class="offline-board" role="alert">
            <WifiOff size={20} aria-hidden="true" />
            <div>
                <strong>{m.dashboard_load_failed_title()}</strong>
                <p>{m.dashboard_load_failed_description()}</p>
                <button type="button" onClick={() => dashboardStore.refresh()}>
                    {m.retry()}
                </button>
            </div>
        </div>
    </>) : (<>
        <div class="control-dashboard">
            <section class={activeSnapshots().length === 0 ? "status-band quiet" : "status-band"} aria-labelledby="protection-title">
                <span class="status-icon" aria-hidden="true">
                    {activeSnapshots().length > 0 ? (<>
                        <ShieldCheck size={23} strokeWidth={1.8} />
                    </>) : (<>
                        <Shield size={23} strokeWidth={1.8} />
                    </>)}
                </span>
                <div class="status-copy">
                    <h2 id="protection-title">{protectionTitle()}</h2>
                    <p>{protectionDescription()}</p>
                </div>
                <div class="status-metrics">
                    <div class="inline-metric">
                        <strong>{activeSnapshots().length}<span> / {dashboardStore.totalBlocksCount}</span></strong>
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
                        {dashboardStore.totalBlocksCount > 0 && activeSnapshots().length <= previewSnapshots().length ? (<>
                            <a class="panel-link" href={"/network-blocks"}>
                                {m.control_view_all_blocks()}
                                <ArrowUpRight size={15} aria-hidden="true" />
                            </a>
                        </>) : null}
                    </header>

                    {activeSnapshots().length > 0 ? (<>
                        <div class="enforcement-list">
                            <For each={previewSnapshots()}>{(snapshot) => <>
                                <a
                                    class="enforcement-row"
                                    href={"/network-blocks" + "?block=" + encodeURIComponent(snapshot.block.id)}
                                >
                                    <span class="policy-rail" aria-hidden="true"></span>
                                    <span class="policy-copy">
                                        <strong>{snapshot.block.name}</strong>
                                        <span class="policy-facts">
                                            {snapshot.sitePackConnectionCount > 0 ? (<>
                                                {snapshot.sitePackConnectionCount === 1
                                                    ? m.dashboard_policy_collection_count_one()
                                                    : m.dashboard_policy_collection_count_other({
                                                        count: snapshot.sitePackConnectionCount,
                                                    })}
                                            </>) : null}
                                            {snapshot.sitePackConnectionCount > 0 && snapshot.userRulesCount > 0 ? (<>
                                                <span aria-hidden="true">·</span>
                                            </>) : null}
                                            {snapshot.userRulesCount > 0 ? (<>
                                                {snapshot.userRulesCount === 1
                                                    ? m.dashboard_policy_website_count_one()
                                                    : m.dashboard_policy_website_count_other({
                                                        count: snapshot.userRulesCount,
                                                    })}
                                            </>) : null}
                                            {snapshot.sitePackConnectionCount === 0 && snapshot.userRulesCount === 0 ? (<>
                                                {m.control_block_no_rules()}
                                            </>) : null}
                                        </span>
                                    </span>
                                    {snapshot.block.lockState.state !== "inactive" ? (<>
                                        <span class="sealed-note" title={m.control_lock_explanation_short()}>
                                            <LockKeyhole size={14} aria-hidden="true" />
                                            {snapshot.block.isLocked
                                                ? m.control_lock_active_label()
                                                : m.control_lock_configured_label()}
                                        </span>
                                    </>) : null}
                                    <ArrowUpRight class="row-arrow" size={15} aria-hidden="true" />
                                </a>
                            </>}</For>
                            {activeSnapshots().length > previewSnapshots().length ? (<>
                                <a class="preview-footer" href={"/network-blocks"}>
                                    {m.control_view_all_blocks()}
                                    <ArrowRight size={15} aria-hidden="true" />
                                </a>
                            </>) : null}
                        </div>
                    </>) : (<>
                        <div class="empty-enforcement">
                            <Shield size={24} aria-hidden="true" />
                            <strong>{m.control_no_enforcement_title()}</strong>
                            <p>
                                {dashboardStore.totalBlocksCount === 0
                                    ? m.no_network_blocks_description()
                                    : m.no_active_enforcement()}
                            </p>
                            {dashboardStore.totalBlocksCount === 0 ? (<>
                                <a class="empty-action" href={"/network-blocks/editor"}>
                                    {m.create_block()}
                                    <ArrowRight size={15} aria-hidden="true" />
                                </a>
                            </>) : null}
                        </div>
                    </>)}
                </section>

                <section class="surface-panel upcoming-panel" aria-labelledby="upcoming-title">
                    <header class="panel-heading">
                        <h2 id="upcoming-title">{m.control_upcoming_title()}</h2>
                        <Clock3 size={17} aria-hidden="true" />
                    </header>
                    {upcomingTransitions().length > 0 ? (<>
                        <ol class="upcoming-list">
                            <For each={upcomingTransitions()}>{(transition, index) => <>
                                <li class={index() === 0 ? "next" : ""}>
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
                            </>}</For>
                        </ol>
                    </>) : (<>
                        <div class="upcoming-empty">
                            <Clock3 size={23} aria-hidden="true" />
                            <p>{m.no_upcoming_changes()}</p>
                        </div>
                    </>)}
                </section>
            </div>

            <a class="resistance-strip" href={"/resistance-mode"}>
                <span class="resistance-icon" aria-hidden="true">
                    <SlidersHorizontal size={18} strokeWidth={1.8} />
                </span>
                <strong>{m.resistance_state()}</strong>
                {resistanceLockText() ? (<>
                    <span class="resistance-lock">
                        <LockKeyhole size={14} aria-hidden="true" />
                        {resistanceLockText()}
                    </span>
                </>) : null}
                <span class="resistance-state">{resistanceLabel()}</span>
                <ArrowUpRight size={16} aria-hidden="true" />
            </a>
        </div>
    </>)}
</PageLayout>
 );
}
