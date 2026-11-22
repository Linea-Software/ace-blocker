import { createMemo, createSignal, For, Show, Switch, Match, onSettled } from "solid-js";
import { Dynamic } from "@solidjs/web";
import { useNavigate } from "@solidjs/router";
import { Shield, Globe, Sparkles, ArrowLeft, ArrowRight, Check, Compass, BookOpen, TriangleAlert, LoaderCircle } from "@lucide/solid";
import { toast } from "solid-sonner";
import * as m from "$lib/paraglide/messages.js";
import { countMessage } from "$lib/utils/count-message";
import { onboardingStore } from "$lib/stores/onboarding";
import { dashboardStore } from "$lib/stores/dashboard";
import { getPackIcon, FALLBACK_ICON } from "../ace/solid-pack-icons";
import { ONBOARDING_BLOCK_ORDER, RECOMMENDED_BLOCK_IDS, enableStarterBlock,
  getDefaultBlockDescription, getDefaultBlockName } from "$lib/onboarding/default-blocks";
import { getDashboardSnapshot, setResistanceModeEnabled } from "$engine-client";

type BlockChoice = {
  id: string; name: string; description: string; icon: string | null; rulesCount: number;
  sitePackConnectionCount: number; alreadyEnabled: boolean; selected: boolean;
};
const STEP_COUNT = 5;
const standard = "inline-flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:pointer-events-none disabled:opacity-50";
const outline = "inline-flex items-center justify-center gap-2 rounded-md border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-accent hover:text-accent-foreground disabled:opacity-50";
const ghost = "inline-flex items-center justify-center gap-2 rounded-md px-4 py-2 text-sm font-medium hover:bg-accent hover:text-accent-foreground disabled:opacity-50";

export default function OnboardingWizard() {
  const navigate = useNavigate();
  const [stepIndex, setStepIndex] = createSignal(0);
  const [loading, setLoading] = createSignal(true);
  const [applying, setApplying] = createSignal(false);
  const [loadFailed, setLoadFailed] = createSignal(false);
  const [choices, setChoices] = createSignal<BlockChoice[]>([]);
  const [enableResistance, setEnableResistance] = createSignal(true);
  const [resistanceAlreadyEnabled, setResistanceAlreadyEnabled] = createSignal(false);
  const progress = createMemo(() => (stepIndex()+1)/STEP_COUNT*100);
  const enabledCount = createMemo(() => choices().filter(choice=>choice.alreadyEnabled).length);
  const heading = createMemo(() => [
    m.onboarding_welcome_title(),m.onboarding_how_it_works_title(),m.onboarding_blocks_title(),
    m.onboarding_resistance_title(),m.onboarding_done_title()][stepIndex()]);
  const subtitle = createMemo(() => [
    m.onboarding_welcome_subtitle(),m.onboarding_how_it_works_subtitle(),m.onboarding_blocks_subtitle(),
    m.onboarding_resistance_subtitle(),m.onboarding_done_subtitle()][stepIndex()]);

  async function loadState() {
    setLoading(true);
    setLoadFailed(false);
    try {
      const {data,error} = await getDashboardSnapshot();
      if (error || !data) throw error ?? new Error("Onboarding snapshot is empty");
      const blocks = new Map(data.blocks.map(snapshot => [snapshot.block.id,snapshot]));
      const next: BlockChoice[] = ONBOARDING_BLOCK_ORDER.flatMap(blockId => {
        const snapshot = blocks.get(blockId);
        if (!snapshot) return [];
        const alreadyEnabled = snapshot.block.enabled;
        return [{
          id: blockId, name: getDefaultBlockName(blockId,snapshot.block.name),
          description: getDefaultBlockDescription(blockId), icon: snapshot.block.icon ?? null,
          rulesCount: snapshot.userRulesCount, sitePackConnectionCount: snapshot.sitePackConnectionCount,
          alreadyEnabled,
          selected: alreadyEnabled || (!onboardingStore.wizardPreviouslyCompleted && RECOMMENDED_BLOCK_IDS.includes(blockId)),
        }];
      });
      setChoices(next);
      setResistanceAlreadyEnabled(data.resistanceModeSettings?.enabled ?? false);
      setEnableResistance(true);
    } catch(error) {
      console.error("Onboarding: failed to load engine state",error);
      setLoadFailed(true);
    } finally {setLoading(false);}
  }
  async function applyChoices() {
    setApplying(true);
    try {
      for (const choice of choices()) {
        if (!choice.selected || choice.alreadyEnabled) continue;
        await enableStarterBlock(choice.id);
        setChoices(items => items.map(item => item.id === choice.id ? {...item,alreadyEnabled:true} : item));
      }
      if (enableResistance() && !resistanceAlreadyEnabled()) {
        const {error} = await setResistanceModeEnabled({body:{enabled:true}});
        if (error) throw error;
      }
      await dashboardStore.refresh();
      return true;
    } catch(error) {
      console.error("Onboarding: failed to apply choices",error);
      toast.error(m.onboarding_apply_failed());
      return false;
    } finally {setApplying(false);}
  }
  async function next() {
    if (stepIndex() === 3 && !(await applyChoices())) return;
    if (stepIndex() < STEP_COUNT-1) setStepIndex(v=>v+1);
  }
  const previous = () => {if (stepIndex()>0)setStepIndex(v=>v-1);};
  const finish = () => onboardingStore.completeWizard();
  const finishTour = () => {finish();onboardingStore.startTour();};
  const finishGuide = () => {finish();navigate("/getting-started");};
  function getRuleSummary(choice:BlockChoice) {
    if (choice.rulesCount>0) return countMessage(choice.rulesCount,m.onboarding_block_rule_count_zero,
      m.onboarding_block_rule_count_one,()=>m.onboarding_block_rule_count({count:choice.rulesCount}));
    if (choice.sitePackConnectionCount>0) return choice.sitePackConnectionCount===1 ? m.onboarding_block_pack_count_one()
      : m.onboarding_block_pack_count_other({count:choice.sitePackConnectionCount});
    return countMessage(0,m.onboarding_block_rule_count_zero,m.onboarding_block_rule_count_one,
      ()=>m.onboarding_block_rule_count({count:0}));
  }
  const welcomePoints = [
    {icon:Shield,text:m.onboarding_welcome_point_block}, {icon:Sparkles,text:m.onboarding_welcome_point_resist},
    {icon:Compass,text:m.onboarding_welcome_point_track},
  ];
  const concepts = [
    {icon:Shield,title:m.network_blocks,body:m.onboarding_concept_blocks},
    {icon:Globe,title:m.site_packs,body:m.onboarding_concept_site_packs},
    {icon:Sparkles,title:m.resistance_mode,body:m.onboarding_concept_resistance},
  ];
  const toggleClass = "inline-flex h-6 w-11 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors bg-input data-[checked=true]:bg-primary disabled:cursor-not-allowed disabled:opacity-50";
  let dialogElement!: HTMLDivElement;
  onSettled(() => {
    void loadState();
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const focusable = () => [...dialogElement.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
    )].filter(el => el.getClientRects().length > 0 && getComputedStyle(el).visibility !== "hidden");
    const initialFrame = requestAnimationFrame(() => (focusable()[0] ?? dialogElement).focus());
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        // A mandatory setup cannot be dismissed using Escape.
        event.preventDefault();
        event.stopPropagation();
        return;
      }
      if (event.key !== "Tab") return;
      const candidates = focusable();
      if (!candidates.length) { event.preventDefault(); dialogElement.focus(); return; }
      const first = candidates[0], last = candidates[candidates.length - 1];
      if (!dialogElement.contains(document.activeElement)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && document.activeElement === first) {
        event.preventDefault(); last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault(); first.focus();
      }
    };
    document.addEventListener("keydown", onKeydown, true);
    return () => {
      cancelAnimationFrame(initialFrame);
      document.removeEventListener("keydown", onKeydown, true);
      // Restore focus after the workspace ceases to be inert.
      queueMicrotask(() => { if (previousFocus?.isConnected) previousFocus.focus(); });
    };
  });
  return (
    <div class="fixed inset-0 z-[100] flex items-center justify-center bg-black/80 p-4" role="presentation">
      <div ref={dialogElement} role="dialog" aria-modal="true" aria-labelledby="wizard-title" aria-describedby="wizard-description" tabindex={-1}
        class="w-[95vw] sm:max-w-2xl max-h-[95dvh] flex flex-col p-0 gap-0 overflow-hidden rounded-md border border-border bg-background shadow-none">
        <header class="border-b border-border bg-card px-6 pt-5 pb-4">
          <div class="flex items-center gap-3">
            <div class="flex h-10 w-8 shrink-0 items-center justify-center border-l-[4px] border-[var(--ace-signal)] text-muted-foreground">
              <Shield class="size-6" />
            </div>
            <div class="min-w-0">
              <h2 id="wizard-title" class="font-condensed text-2xl font-semibold">{heading()}</h2>
              <p id="wizard-description" class="text-sm text-muted-foreground truncate">{subtitle()}</p>
            </div>
          </div>
          <div class="mt-4 space-y-1.5">
            <div role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress()}
              class="h-1 relative w-full overflow-hidden rounded-full bg-primary/20">
              <div class="h-full bg-primary transition-all" style={{width:progress()+"%"}} />
            </div>
            <p class="font-mono text-[11px] font-medium text-muted-foreground">
              {m.onboarding_step_counter({current:stepIndex()+1,total:STEP_COUNT})}
            </p>
          </div>
        </header>
        <main class="px-6 py-6 overflow-y-auto max-h-[55vh]">
          <Switch>
            <Match when={stepIndex()===0}>
              <div class="space-y-5">
                <p class="text-sm text-muted-foreground leading-relaxed">{m.onboarding_welcome_body()}</p>
                <ul class="space-y-3"><For each={welcomePoints}>{point=>(
                  <li class="flex items-start gap-3">
                    <div class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-sm border border-border text-muted-foreground">
                      <Dynamic component={point.icon} class="size-4"/>
                    </div>
                    <span class="text-sm leading-relaxed">{point.text()}</span>
                  </li>
                )}</For></ul>
              </div>
            </Match>
            <Match when={stepIndex()===1}>
              <div class="space-y-4"><For each={concepts}>{concept=>(
                <div class="flex items-start gap-3 rounded-md border border-border bg-card p-4">
                  <div class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted/40 text-muted-foreground">
                    <Dynamic component={concept.icon} class="size-4.5"/>
                  </div>
                  <div class="space-y-1 min-w-0">
                    <h3 class="font-semibold text-sm">{concept.title()}</h3>
                    <p class="text-sm text-muted-foreground leading-relaxed">{concept.body()}</p>
                  </div>
                </div>
              )}</For></div>
            </Match>
            <Match when={stepIndex()===2}>
              <Show when={!loading()} fallback={<div class="flex items-center justify-center py-12">
                <LoaderCircle class="size-6 text-muted-foreground animate-spin"/></div>}>
                <Show when={!loadFailed()} fallback={
                  <div class="flex flex-col items-center gap-3 py-10 text-center">
                    <TriangleAlert class="size-8 text-[var(--ace-danger)]"/>
                    <p class="text-sm text-muted-foreground">{m.onboarding_load_failed()}</p>
                    <button type="button" class={outline} onClick={()=>void loadState()}>{m.onboarding_retry()}</button>
                  </div>
                }>
                  <Show when={choices().length>0} fallback={
                    <p class="text-sm text-muted-foreground text-center py-10 leading-relaxed">{m.onboarding_no_default_blocks()}</p>
                  }>
                    <div class="space-y-3">
                      <p class="text-sm text-muted-foreground leading-relaxed">{m.onboarding_blocks_body()}</p>
                      <For each={choices()}>{choice=>(
                        <label class="flex cursor-pointer items-center gap-3 rounded-md border border-border bg-card p-4 transition-colors hover:bg-muted/45">
                          <div class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted/40 text-muted-foreground">
                            <Dynamic component={choice.icon?getPackIcon(choice.icon):FALLBACK_ICON} class="size-4.5"/>
                          </div>
                          <div class="flex-1 min-w-0 space-y-0.5">
                            <div class="flex items-center gap-2">
                              <h3 class="font-semibold text-sm">{choice.name}</h3>
                              <Show when={choice.alreadyEnabled}>
                                <span class="border-l-2 border-[var(--ace-success)] pl-2 text-[11px] font-medium text-muted-foreground">
                                  {m.onboarding_block_already_active()}
                                </span>
                              </Show>
                            </div>
                            <p class="text-xs text-muted-foreground leading-relaxed">{choice.description}</p>
                            <p class="text-[0.7rem] text-muted-foreground/70">{getRuleSummary(choice)}</p>
                          </div>
                          <button type="button" role="switch" aria-label={choice.name} aria-checked={(choice.selected) ? "true" : "false"}
                            data-checked={choice.selected} disabled={choice.alreadyEnabled} class={toggleClass}
                            onClick={()=>setChoices(list=>list.map(item=>item.id===choice.id?{...item,selected:!item.selected}:item))}>
                            <span class={`pointer-events-none size-5 rounded-full bg-background shadow transition-transform ${choice.selected?"translate-x-5":"translate-x-0"}`}/>
                          </button>
                        </label>
                      )}</For>
                      <p class="text-xs text-muted-foreground/80 pt-1">{m.onboarding_blocks_footnote()}</p>
                    </div>
                  </Show>
                </Show>
              </Show>
            </Match>
            <Match when={stepIndex()===3}>
              <div class="space-y-4">
                <p class="text-sm text-muted-foreground leading-relaxed">{m.onboarding_resistance_body()}</p>
                <label class="flex cursor-pointer items-center gap-3 rounded-md border border-border bg-card p-4 transition-colors hover:bg-muted/45">
                  <div class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted/40 text-muted-foreground">
                    <Sparkles class="size-4.5"/>
                  </div>
                  <div class="flex-1 min-w-0 space-y-0.5">
                    <h3 class="font-semibold text-sm">{m.onboarding_resistance_toggle_label()}</h3>
                    <p class="text-xs text-muted-foreground leading-relaxed">
                      {resistanceAlreadyEnabled()?m.onboarding_resistance_already_enabled():m.onboarding_resistance_toggle_hint()}
                    </p>
                  </div>
                  <button type="button" role="switch" aria-label={m.onboarding_resistance_toggle_label()}
                    aria-checked={(enableResistance()) ? "true" : "false"} data-checked={enableResistance()}
                    disabled={resistanceAlreadyEnabled()} class={toggleClass} onClick={()=>setEnableResistance(v=>!v)}>
                    <span class={`pointer-events-none size-5 rounded-full bg-background shadow transition-transform ${enableResistance()?"translate-x-5":"translate-x-0"}`}/>
                  </button>
                </label>
                <p class="text-xs text-muted-foreground/80">{m.onboarding_resistance_footnote()}</p>
              </div>
            </Match>
            <Match when={stepIndex()===4}>
              <div class="space-y-5">
                <div class="flex items-center gap-3 rounded-md border border-border border-l-[5px] border-l-[var(--ace-success)] bg-card p-4">
                  <div class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border text-[var(--ace-success)]">
                    <Check class="size-4.5"/>
                  </div>
                  <p class="text-sm leading-relaxed">{countMessage(enabledCount(),m.onboarding_done_summary_zero,
                    m.onboarding_done_summary_one,()=>m.onboarding_done_summary({count:enabledCount()}))}</p>
                </div>
                <p class="text-sm text-muted-foreground leading-relaxed">{m.onboarding_done_body()}</p>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <button type="button" class={outline+" h-auto flex-col items-start gap-1 rounded-sm py-4 text-left"} onClick={finishTour}>
                    <span class="flex items-center gap-2 font-semibold text-sm"><Compass class="size-4"/>{m.onboarding_take_tour()}</span>
                    <span class="text-xs text-muted-foreground font-normal whitespace-normal">{m.onboarding_take_tour_hint()}</span>
                  </button>
                  <button type="button" class={outline+" h-auto flex-col items-start gap-1 rounded-sm py-4 text-left"} onClick={finishGuide}>
                    <span class="flex items-center gap-2 font-semibold text-sm"><BookOpen class="size-4"/>{m.onboarding_read_guide()}</span>
                    <span class="text-xs text-muted-foreground font-normal whitespace-normal">{m.onboarding_read_guide_hint()}</span>
                  </button>
                </div>
              </div>
            </Match>
          </Switch>
        </main>
        <footer class="flex items-center justify-between gap-3 border-t border-border bg-card px-6 py-4">
          <Show when={stepIndex()<STEP_COUNT-1} fallback={<div/>}>
            <button type="button" class={ghost+" text-muted-foreground"} onClick={finish}>{m.onboarding_skip()}</button>
          </Show>
          <div class="flex items-center gap-2">
            <Show when={stepIndex()>0 && stepIndex()<STEP_COUNT-1}>
              <button type="button" class={outline} onClick={previous} disabled={applying()}>
                <ArrowLeft class="size-4 mr-1"/>{m.onboarding_back()}
              </button>
            </Show>
            <Show when={stepIndex()===STEP_COUNT-1} fallback={
              <button type="button" class={standard} disabled={applying() || (stepIndex()===2 && loading())}
                onClick={()=>void next()}>
                <Show when={applying()} fallback={<>{m.onboarding_next()}<ArrowRight class="size-4 ml-1"/></>}>
                  <LoaderCircle class="size-4 mr-1 animate-spin"/>{m.onboarding_applying()}
                </Show>
              </button>
            }>
              <button type="button" class={standard} onClick={finish}>
                {m.onboarding_go_to_dashboard()}<Check class="size-4 ml-1"/>
              </button>
            </Show>
          </div>
        </footer>
      </div>
    </div>
  );
}
