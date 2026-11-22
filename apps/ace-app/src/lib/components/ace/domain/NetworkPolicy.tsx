import "./NetworkPolicy.css";
import { createMemo, createSignal, Show, onSettled } from "solid-js";
import { Dynamic } from "@solidjs/web";
import type { NetworkBlockSchema, NetworkListLockSchema, ScheduleEntrySchema } from "$engine-client";
import * as m from "$lib/paraglide/messages.js";
import { getLocale } from "$lib/paraglide/runtime";
import { localizationStore } from "$lib/stores/localization";
import { Eye, Globe, Lock, LockOpen, Pencil, Trash2, Ellipsis } from "@lucide/solid";
import LockSeal from "./LockSeal";
import { findNextScheduleTransition } from "$lib/utils/schedule-transitions";
import { getPackIcon } from "../solid-pack-icons";

export type NetworkPolicyModel = NetworkBlockSchema & {
  lockInfo?: NetworkListLockSchema | null; schedules?: ScheduleEntrySchema[];
  sitesListCount?: number; rulesCount?: number;
};
export default function NetworkPolicy(props: {
  policy: NetworkPolicyModel; onEdit: (policy: NetworkPolicyModel) => void;
  onDelete: (policy: NetworkPolicyModel) => void; onLock: (policy: NetworkPolicyModel) => void;
  onEnabledChange: (policy: NetworkPolicyModel, enabled: boolean) => Promise<void>;
}) {
  const [updating, setUpdating] = createSignal(false);
  const [menuOpen, setMenuOpen] = createSignal(false);
  let menuRoot: HTMLDivElement | undefined;
  const locked = createMemo(() => props.policy.lockState.state !== "inactive");
  const lockedNow = createMemo(() => props.policy.isLocked);
  const icon = createMemo(() => props.policy.icon ? getPackIcon(props.policy.icon) : Globe);
  const hasSchedule = createMemo(() => props.policy.scheduleMode === "always" || (props.policy.schedules?.length ?? 0) > 0);
  const stateLabel = createMemo(() => !props.policy.enabled ? m.block_state_disabled() :
    props.policy.isActive ? m.block_state_enforcing() : hasSchedule() ? m.block_state_scheduled() : m.block_state_unscheduled());
  const lockLabel = createMemo(() => {
    switch (props.policy.lockState.state) {
      case "soft_unlockable": return m.lock_soft_active_tooltip();
      case "active_during_schedule": return lockedNow() ? m.lock_schedule_active_tooltip() : m.lock_schedule_configured_tooltip();
      case "active_until_lock_end": return lockedNow() ? m.lock_time_active_tooltip() : m.lock_time_expired_tooltip();
      default: return m.lock_this_block_tooltip();
    }
  });
  const lockActionLabel = createMemo(() => props.policy.lockState.state === "soft_unlockable"
    ? m.unlock_block() : locked() ? m.review_lock() : m.lock_protect_action());
  const lockActionAria = createMemo(() => props.policy.lockState.state === "soft_unlockable"
    ? m.unlock_block() : locked() ? m.review_lock() : m.lock_this_block_tooltip());
  const lockDescription = createMemo(() => {
    if (props.policy.lockState.state === "soft_unlockable") return m.lock_soft_active_description();
    if (props.policy.lockState.state === "active_during_schedule" && !lockedNow()) return m.lock_schedule_configured_description();
    if (props.policy.lockState.state === "active_until_lock_end" && !lockedNow()) return m.lock_time_expired_description();
    return m.lock_hard_active_description();
  });
  const scheduleText = createMemo(() => {
    const p = props.policy;
    if (p.scheduleMode === "always") return m.block_mode_always();
    if (!p.schedules?.length) return m.block_unscheduled_hint();
    const first = p.schedules[0];
    const days = ["monday","tuesday","wednesday","thursday","friday","saturday","sunday"] as const;
    const labels = [m.monday_short(),m.tuesday_short(),m.wednesday_short(),m.thursday_short(),m.friday_short(),m.saturday_short(),m.sunday_short()];
    const active = days.flatMap((key,i) => first[key] ? [labels[i]] : []);
    let dayText = active.join(", ");
    if (active.length === 7) dayText = m.schedule_all_week();
    else if (active.length === 5 && days.slice(0,5).every(day=>first[day])) dayText = m.schedule_weekdays_compact();
    else if (active.length === 2 && first.saturday && first.sunday) dayText = m.schedule_weekend_compact();
    const range = first.startTime === first.endTime ? m.schedule_all_day()
      : localizationStore.formatTime(first.startTime ?? "00:00") + "–" + localizationStore.formatTime(first.endTime ?? "23:59");
    const extra = p.schedules.length > 1 ? " " + (p.schedules.length === 2 ? m.schedule_extra_count_one()
      : m.schedule_extra_count({count:p.schedules.length-1})) : "";
    return dayText+", "+range+extra;
  });
  const nextTransition = createMemo(() => props.policy.enabled && props.policy.scheduleMode === "scheduled"
    ? findNextScheduleTransition(props.policy.schedules ?? []) : null);
  const transitionText = createMemo(() => {
    const t = nextTransition();
    if (!t) return null;
    const time = t.at.toLocaleString(getLocale(),{weekday:"short",hour:"2-digit",minute:"2-digit"});
    return t.kind === "start" ? m.block_next_starts({time}) : m.block_next_ends({time});
  });
  const formatLockEnd = (raw?:string|null) => {
    if (!raw) return null;
    const date = new Date(raw);
    return Number.isNaN(date.getTime()) ? raw : m.until_date({date:date.toLocaleString(getLocale())});
  };
  async function toggleEnabled() {
    if (updating() || (props.policy.isLocked && props.policy.enabled)) return;
    setUpdating(true);
    try { await props.onEnabledChange(props.policy,!props.policy.enabled); }
    finally { setUpdating(false); }
  }
  onSettled(() => {
    localizationStore.detectSystemTimeFormat();
    const outside = (e:PointerEvent)=>{if(!menuRoot?.contains(e.target as Node))setMenuOpen(false);};
    const escape = (e:KeyboardEvent)=>{if(e.key==="Escape")setMenuOpen(false);};
    document.addEventListener("pointerdown",outside);
    document.addEventListener("keydown",escape);
    return ()=>{document.removeEventListener("pointerdown",outside);document.removeEventListener("keydown",escape);};
  });
  return <article class={["network-policy",{active:props.policy.enabled && props.policy.isActive,locked:locked(),
      disabled:!props.policy.enabled,unscheduled:!hasSchedule()}]} data-testid="network-policy">
    <div class="state-rail" aria-hidden="true"><span/></div>
    <div class="policy-body">
      <Show when={locked()}><LockSeal compact title={lockLabel()} description={lockDescription()}
        detail={formatLockEnd(props.policy.lockInfo?.lockEnd)}/></Show>
      <div class="policy-header">
        <div class="identity">
          <div class="state-line"><span class="state-word">{stateLabel()}</span></div>
          <div class="policy-name-row">
            <span class="policy-icon" data-testid="network-policy-icon" aria-hidden="true">
              <Dynamic component={icon()} size={21} strokeWidth={1.75}/>
            </span>
            <div class="policy-name-content"><h2>{props.policy.name}</h2>
              <div class={["schedule-copy",{"unscheduled-copy":!hasSchedule()}]}>
                <span>{scheduleText()}</span>
                <Show when={transitionText()}><span class="transition-copy">{transitionText()}</span></Show>
              </div>
            </div>
          </div>
        </div>
        <div class="actions">
          <div class="enable-action"><span>{m.block_enabled_label()}</span>
            <button type="button" role="switch" class="enable-switch" data-testid="block-enabled-switch"
              aria-label={m.block_enabled_switch_aria({name:props.policy.name})} aria-checked={(!!props.policy.enabled) ? "true" : "false"}
              aria-busy={(updating()) ? "true" : "false"} disabled={updating() || (props.policy.isLocked && props.policy.enabled)}
              title={props.policy.isLocked && props.policy.enabled ? m.block_locked_tooltip() : m.block_enabled_label()}
              onClick={()=>void toggleEnabled()}><span class="enable-thumb" aria-hidden="true"/></button>
          </div>
          <button type="button" class="quiet-action" onClick={()=>props.onEdit(props.policy)}>
            <Show when={lockedNow()} fallback={<><Pencil size={15} aria-hidden="true"/><span>{m.edit()}</span></>}>
              <Eye size={15} aria-hidden="true"/><span>{m.view()}</span>
            </Show>
          </button>
          <button type="button" class={["quiet-action",{"commit-action":!locked()}]}
            onClick={()=>props.onLock(props.policy)} aria-label={lockActionAria()} title={lockLabel()}>
            <Show when={locked()} fallback={<LockOpen size={15} aria-hidden="true"/>}>
              <Lock size={15} aria-hidden="true"/>
            </Show>
            <span>{lockActionLabel()}</span>
          </button>
          <div class="relative" ref={menuRoot}>
            <button type="button" onClick={()=>setMenuOpen(v=>!v)} aria-haspopup="menu" aria-expanded={(menuOpen()) ? "true" : "false"}
              class="inline-flex size-9 items-center justify-center rounded-sm border border-transparent text-muted-foreground transition-colors hover:border-border hover:bg-muted hover:text-foreground"
              aria-label={m.more_options()} title={m.more_options()}><Ellipsis size={16} aria-hidden="true"/></button>
            <Show when={menuOpen()}>
              <div role="menu" class="absolute z-50 right-0 top-full mt-1 min-w-36 rounded-sm border border-border bg-popover p-1 shadow-none">
                <button type="button" role="menuitem" disabled={lockedNow()}
                  onClick={()=>{setMenuOpen(false);props.onDelete(props.policy);}}
                  title={lockedNow()?m.block_locked_tooltip():m.delete()}
                  class="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-destructive hover:bg-destructive/10 disabled:opacity-50">
                  <Trash2 size={15} aria-hidden="true"/><span>{m.delete()}</span>
                </button>
              </div>
            </Show>
          </div>
        </div>
      </div>
      <div class="policy-grid"><dl class="policy-facts">
        <div><dt>{m.sites_lists()}</dt><dd>{props.policy.sitesListCount ?? 0}</dd></div>
        <div><dt>{m.custom_rules()}</dt><dd>{props.policy.rulesCount ?? 0}</dd></div>
        <Show when={props.policy.safeSearch}><div data-testid="network-policy-safe-search">
          <dt>{m.force_safe_search()}</dt><dd>{m.block_safe_search_included()}</dd></div></Show>
      </dl></div>
    </div>
  </article>;
}
