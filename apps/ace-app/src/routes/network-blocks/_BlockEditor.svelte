<script lang="ts">
    import * as Alert from "$lib/components/ui/alert/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Skeleton } from "$lib/components/ui/skeleton/index.js";
    import { Spinner } from "$lib/components/ui/spinner/index.js";
    import { onMount, tick } from "svelte";
    import { invalidateAll } from "$app/navigation";
    import * as m from "$lib/paraglide/messages.js";
    import { toast } from "svelte-sonner";
    import {
        getDashboardSnapshot,
        getAllSitePacks,
        getScheduleEntries,
        getNetBlockToSitePackConnections,
        getUserRules,
    } from "$engine-client";
    import type {
        NetworkBlockSchema,
        ScheduleMode,
        ScheduleEntrySchema,
        SitePackSchema,
        UserDefinedRuleSchema,
    } from "$engine-client";

    import { UserDefinedRulesDelta } from "$lib/deltas/user-defined-rules-delta.svelte";
    import { ScheduleChanges } from "$lib/deltas/schedule-delta.svelte";
    import { SitePacksConnectionDelta } from "$lib/deltas/site-packs-connection-delta.svelte";
    import { saveNetworkBlock } from "./_save-network-block";
    import { isAceCategory } from "$lib/utils/category-presentation";

    import SectionGeneral from "./_components/SectionGeneral.svelte";
    import SectionRules from "./_components/SectionRules.svelte";
    import SectionSites from "./_components/SectionSites.svelte";
    import SectionSchedule from "./_components/SectionSchedule.svelte";

    import CircleAlert from "@lucide/svelte/icons/circle-alert";
    import { ArrowLeft, ArrowRight, LockKeyhole, ShieldCheck, Check } from "@lucide/svelte";

    let {
        block = $bindable(null),
        onSave,
        onClose = () => {},
        onProtect = () => {},
        step = $bindable<0 | 1 | 2>(0),
    }: {
        block: NetworkBlockSchema | null;
        onSave: (block: NetworkBlockSchema) => void;
        onClose?: () => void;
        onProtect?: () => void;
        step?: 0 | 1 | 2;
    } = $props();

    let name = $state(block?.name ?? "");
    let blockIcon = $state<string | null>(block?.icon ?? null);
    let isSafeSearch = $state(block?.safeSearch ?? false);
    let scheduleMode = $state<ScheduleMode>(block?.scheduleMode ?? "always");
    let isSubmitting = $state(false);
    let isLoadingData = $state(block !== null);

    // Deltas
    let rulesDelta = $state(new UserDefinedRulesDelta());
    let scheduleDelta = $state(new ScheduleChanges());
    let sitePacksDelta = $state(new SitePacksConnectionDelta());

    // Current data (for editing)
    let existingRules = $state<UserDefinedRuleSchema[]>([]);
    let existingEntries = $state<ScheduleEntrySchema[]>([]);
    let connectedListIds = $state<string[]>([]);
    let availableLists = $state<SitePackSchema[]>([]);

    // Each block is staged locally and persisted through a single engine RPC.
    let scheduleEditorOpen = $state(false);
    let dataLoadFailed = $state(false);
    let saveConfirmationWarning = $state(false);
    let websitesOpen = $state(false);
    function leaveEditor() { if (!isSubmitting && !scheduleEditorOpen) onClose(); }
    const steps = [m.block_wizard_content(), m.block_wizard_schedule(), m.block_wizard_review()];
    const categoryCount = $derived(sitePacksDelta.currentConnectedListIds.filter(id =>
        availableLists.some(pack => pack.id === id && isAceCategory(pack))
    ).length);
    const customListCount = $derived(sitePacksDelta.currentConnectedListIds.length - categoryCount);
    const websiteCount = $derived(rulesDelta.currentRules.filter(rule => rule.ruleType === "domain").length);
    const regexCount = $derived(rulesDelta.currentRules.filter(rule => rule.ruleType === "regex").length);
    const chosenNames = $derived(sitePacksDelta.currentConnectedListIds.map(id =>
        availableLists.find(pack => pack.id === id)?.name
    ).filter((name): name is string => !!name));
    async function showWebsites() {
        websitesOpen = true;
        await tick();
        document.getElementById("website-manager")?.scrollIntoView({ block: "center" });
    }
    async function goToStep(next: 0 | 1 | 2) {
        if (scheduleEditorOpen || isSubmitting) return;
        step = next;
        await tick();
        document.getElementById("editor-step-heading")?.focus();
    }

    const isScheduleViolation = $derived(
        (block?.isLocked ?? false) &&
        ((block?.scheduleMode === "always" && scheduleMode === "scheduled") ||
            (scheduleMode === "scheduled" && !scheduleDelta.isValidForLockedBlock())),
    );
    const needsSchedule = $derived(
        scheduleMode === "scheduled" && scheduleDelta.currentScheduleEntries.length === 0,
    );
    const targetCount = $derived(
        rulesDelta.currentRules.length + sitePacksDelta.currentConnectedListIds.length +
            (isSafeSearch ? 1 : 0),
    );
    const needsTargets = $derived(!block && targetCount === 0);
    // A lock must never be committed against an outdated unsaved draft.
    const hasUnsavedChanges = $derived(!!block && (
        name !== block.name ||
        blockIcon !== (block.icon ?? null) ||
        isSafeSearch !== block.safeSearch ||
        scheduleMode !== block.scheduleMode ||
        rulesDelta.changes.length > 0 ||
        scheduleDelta.changes.length > 0 ||
        sitePacksDelta.changes.length > 0
    ));

    async function loadData(blockId: string): Promise<boolean> {
        const [rulesRes, scheduleRes, connectionsRes, listsRes] =
            await Promise.all([
                getUserRules({ path: { network_block_id: blockId } }),
                getScheduleEntries({ path: { network_block_id: blockId } }),
                getNetBlockToSitePackConnections({
                    path: { network_block_id: blockId },
                }),
                getAllSitePacks(),
            ]);

        // Any read failure must prevent editing: saving against empty
        // rules/schedule/connections would silently wipe server state.
        const failed = [
            rulesRes.error && "rules",
            scheduleRes.error && "schedule",
            connectionsRes.error && "connections",
            listsRes.error && "site packs",
        ].filter(Boolean) as string[];

        if (rulesRes.error)
            console.error("Failed to load rules:", rulesRes.error);
        existingRules = rulesRes.data?.userDefinedRules ?? [];
        rulesDelta = new UserDefinedRulesDelta($state.snapshot(existingRules));

        if (scheduleRes.error)
            console.error("Failed to load schedule:", scheduleRes.error);
        existingEntries = scheduleRes.data?.scheduleEntries ?? [];
        scheduleDelta = new ScheduleChanges(
            $state.snapshot(existingEntries),
            block?.isLocked ?? false,
        );

        if (connectionsRes.error)
            console.error("Failed to load connections:", connectionsRes.error);
        connectedListIds = (connectionsRes.data?.connections ?? []).map(
            (c: any) => c.sitePackId,
        );
        sitePacksDelta = new SitePacksConnectionDelta(
            $state.snapshot(connectedListIds),
        );

        if (listsRes.error)
            console.error("Failed to load site packs:", listsRes.error);
        availableLists = listsRes.data?.sitePacks ?? [];

        return failed.length === 0;
    }

    onMount(async () => {
        if (block) {
            // If any editor read fails, keep the modal open in a read-only
            // state instead of degrading to empty fields the user could save
            // over the real data.
            const loaded = await loadData(block.id);
            if (!loaded) {
                dataLoadFailed = true;
                toast.error(m.failed_to_load_block());
            }
            isLoadingData = false;
        } else {
            const response = await getAllSitePacks();
            availableLists = response.data?.sitePacks ?? [];
        }
    });

    async function handleSave() {
        if (!name.trim() || scheduleEditorOpen || dataLoadFailed) return;
        isSubmitting = true;
        try {
            const outcome = await saveNetworkBlock({
                block: block ? $state.snapshot(block) : null,
                name,
                icon: blockIcon,
                safeSearch: isSafeSearch,
                scheduleMode,
                rules: rulesDelta,
                schedule: scheduleDelta,
                packs: sitePacksDelta,
            });

            if (outcome.kind === "failed") {
                console.error("Failed to save network block:", outcome.error);
                if (!outcome.uncertain) {
                    // A rejected transaction changes nothing. Keep the draft
                    // available so the user can fix the error and retry.
                    toast.error(m.block_save_rejected());
                    return;
                }

                // A lost acknowledgement can follow a successful COMMIT.
                // Reconcile rather than falsely declaring a rollback.
                saveConfirmationWarning = true;
                isLoadingData = true;
                await invalidateAll();
                const snapshot = await getDashboardSnapshot();
                const canonical = block && snapshot.data?.blocks.find(
                    (entry) => entry.block.id === block?.id,
                );
                if (snapshot.error || !canonical) {
                    dataLoadFailed = true;
                } else {
                    block = canonical.block;
                    name = canonical.block.name;
                    blockIcon = canonical.block.icon ?? null;
                    isSafeSearch = canonical.block.safeSearch;
                    scheduleMode = canonical.block.scheduleMode;
                    dataLoadFailed = !(await loadData(canonical.block.id));
                }
                toast.error(m.block_save_unconfirmed());
                return;
            }

            onSave(outcome.block);
        } catch (error) {
            console.error("Failed to save network block:", error);
            dataLoadFailed = true;
            saveConfirmationWarning = true;
            toast.error(m.block_save_unconfirmed());
        } finally {
            isLoadingData = false;
            isSubmitting = false;
        }
    }

</script>

<main class="block-editor-page" aria-label={block ? m.edit_block() : m.create_block()}>
    <header class="editor-page-header">
        <button type="button" class="back-link" onclick={leaveEditor} aria-label={m.cancel()}>
            <ArrowLeft size={17} aria-hidden="true" /> {m.network_blocks()}
        </button>
        <div class="editor-title-row">
            <div>
                <h1>{block ? m.edit_block() : m.create_block()}</h1>
                <p>{m.block_wizard_intro()}</p>
            </div>
            {#if block?.isLocked}<span class="locked-state"><LockKeyhole size={14} aria-hidden="true" /> {m.lock_review_title()}</span>{/if}
        </div>
        <nav class="editor-steps" aria-label={m.block_wizard_navigation()}>
            {#each steps as label, index}
                <button type="button" class:current={step === index} class:complete={step > index}
                    aria-current={step === index ? "step" : undefined}
                    disabled={scheduleEditorOpen || isSubmitting} onclick={() => goToStep(index as 0 | 1 | 2)}>
                    <span class="step-progress" aria-hidden="true"></span>
                    <span class="step-label"><span class="step-number">
                        {#if step > index}<Check size={13} aria-hidden="true" />{:else}{index + 1}{/if}
                    </span>{label}</span>
                </button>
            {/each}
        </nav>
    </header>
    <div class="editor-page-body">
        {#if saveConfirmationWarning}
            <Alert.Root variant="destructive">
                <CircleAlert aria-hidden="true" />
                <Alert.Title>{m.block_save_unconfirmed_title()}</Alert.Title>
                <Alert.Description>{m.block_save_unconfirmed()}</Alert.Description>
            </Alert.Root>
        {/if}
        {#if isLoadingData}
            <div class="flex flex-col gap-5" role="status" aria-label={m.loading_block_configuration()}>
                <Skeleton class="h-10 w-full rounded-sm" />
                <Skeleton class="h-28 w-full rounded-sm" />
                <Skeleton class="h-24 w-full rounded-sm" />
            </div>
        {:else if dataLoadFailed}
            <Alert.Root variant="destructive">
                <CircleAlert aria-hidden="true" />
                <Alert.Title>{m.failed_to_load_block_title()}</Alert.Title>
                <Alert.Description>{m.failed_to_load_block()}</Alert.Description>
            </Alert.Root>
        {:else if step === 0}
            <section class="editor-step-content" aria-labelledby="editor-step-heading">
                <div class="step-intro content-intro">
                    <div>
                        <h2 id="editor-step-heading" tabindex="-1">{m.block_wizard_content_heading()}</h2>
                        <p>{m.block_wizard_content_description()}</p>
                    </div>
                    <button type="button" class="manage-shortcut" onclick={showWebsites}>
                        {m.block_manage_websites()} ({websiteCount})
                    </button>
                </div>
                <div class="step-content-stack">
                    <SectionSites delta={sitePacksDelta} bind:availableLists isLocked={block?.isLocked ?? false} />
                    <div class="secondary-section">
                        <details class="website-manager" id="website-manager" data-testid="website-manager" bind:open={websitesOpen}>
                            <summary>
                                <span><strong>{m.block_manage_websites()}</strong>
                                    <small>{m.block_manage_websites_hint()}</small>
                                    <small>{m.block_wizard_websites_count({count:websiteCount})}{#if regexCount} · {m.block_wizard_regex_count({count:regexCount})}{/if}</small>
                                </span>
                                <span class="text-sm font-semibold text-foreground">{websitesOpen ? m.close() : m.block_wizard_edit()}</span>
                            </summary>
                            <div class="manager-detail">
                                <SectionRules delta={rulesDelta} disabled={isSubmitting} isLocked={block?.isLocked ?? false} />
                            </div>
                        </details>
                    </div>
                </div>
                {#if needsTargets}<p class="inline-hint" role="status"><CircleAlert size={15} aria-hidden="true" />{m.block_need_targets()}</p>{/if}
            </section>
        {:else if step === 1}
            <section class="editor-step-content" id="block-when" aria-labelledby="editor-step-heading">
                <div class="step-intro">
                    <h2 id="editor-step-heading" tabindex="-1">{m.block_wizard_schedule_heading()}</h2>
                    <p>{m.block_wizard_schedule_description()}</p>
                </div>
                <div class="mode-selector" role="group" aria-label={m.block_when_heading()}>
                    <button type="button" class:selected={scheduleMode === "always"} aria-pressed={scheduleMode === "always"}
                        disabled={scheduleEditorOpen || isSubmitting} onclick={() => (scheduleMode = "always")}>
                        <span class="mode-radio" aria-hidden="true"></span>
                        <span><strong>{m.block_mode_always()}</strong><small>{m.block_mode_always_detail()}</small></span>
                    </button>
                    <button type="button" class:selected={scheduleMode === "scheduled"} aria-pressed={scheduleMode === "scheduled"}
                        disabled={scheduleEditorOpen || isSubmitting || (!!block?.isLocked && block.scheduleMode === "always")}
                        onclick={() => (scheduleMode = "scheduled")}>
                        <span class="mode-radio" aria-hidden="true"></span>
                        <span><strong>{m.block_mode_scheduled()}</strong><small>{m.block_mode_scheduled_detail()}</small></span>
                    </button>
                </div>
                {#if scheduleMode === "scheduled"}
                    <div class="schedule-detail">
                        <SectionSchedule delta={scheduleDelta} isLocked={block?.isLocked ?? false} bind:hasOpenEditor={scheduleEditorOpen} />
                    </div>
                {/if}
                <div class="enforcement-preview" aria-live="polite">
                    <span class="preview-rail" aria-hidden="true"></span>
                    <div>
                        <strong>{m.block_preview_heading()}</strong>
                        <p>{scheduleMode === "always" ? m.block_preview_always() : m.block_preview_scheduled()}</p>
                        {#if needsSchedule}<p class="preview-warning">{m.block_need_schedule()}</p>{/if}
                        {#if isScheduleViolation}<p class="preview-warning">{m.schedule_not_more_restrictive_warning()}</p>{/if}
                    </div>
                </div>
            </section>
        {:else}
            <section class="editor-step-content" aria-labelledby="editor-step-heading">
                <div class="step-intro">
                    <h2 id="editor-step-heading" tabindex="-1">{m.block_wizard_review_heading()}</h2>
                    <p>{m.block_wizard_review_description()}</p>
                </div>
                <div class="review-stack">
                    <div class="review-identity">
                        <SectionGeneral bind:name bind:icon={blockIcon} bind:isSafeSearch disabled={isSubmitting} isLocked={block?.isLocked ?? false} />
                    </div>
                    <div class="review-list" aria-label={m.block_wizard_summary()}>
                        <div class="review-row">
                            <div class="review-row-main">
                                <strong>{m.block_wizard_content()}</strong>
                                <p>{m.block_wizard_category_count({count:categoryCount})} · {m.block_wizard_custom_count({count:customListCount})}</p>
                                <p>{m.block_wizard_websites_count({count:websiteCount})}{#if regexCount} · {m.block_wizard_regex_count({count:regexCount})}{/if}</p>
                                {#if chosenNames.length}<p class="review-selected" title={chosenNames.join(", ")}>{chosenNames.slice(0,3).join(", ")}{chosenNames.length > 3 ? "…" : ""}</p>{/if}
                            </div>
                            <button type="button" class="review-edit" onclick={() => goToStep(0)}>{m.block_wizard_edit()}</button>
                        </div>
                        <div class="review-row">
                            <div class="review-row-main">
                                <strong>{m.block_wizard_schedule()}</strong>
                                <p>{scheduleMode === "always" ? m.block_mode_always_detail() : m.block_wizard_period_count({count:scheduleDelta.currentScheduleEntries.length})}</p>
                            </div>
                            <button type="button" class="review-edit" onclick={() => goToStep(1)}>{m.block_wizard_edit()}</button>
                        </div>
                        <div class="review-row">
                            <div class="review-row-main">
                                <strong>{m.block_wizard_activation()}</strong>
                                <p>{block ? (block.enabled ? m.block_wizard_enabled() : m.block_wizard_disabled()) : m.block_wizard_create_enabled()}</p>
                            </div>
                        </div>
                    </div>
                    <div class="protection-panel">
                        <LockKeyhole size={19} aria-hidden="true" />
                        <div>
                            <strong>{block?.isLocked ? m.lock_review_title() : m.lock_protect_title()}</strong>
                            <p>{m.control_lock_help_detail()}</p>
                        </div>
                        {#if block}
                            <button type="button" onclick={onProtect} disabled={isSubmitting || hasUnsavedChanges}
                                title={hasUnsavedChanges ? m.save() : m.lock_protect_action()}>{m.lock_protect_action()}</button>
                        {:else}
                            <span>{m.block_lock_after_save()}</span>
                        {/if}
                    </div>
                    {#if !name.trim() || needsSchedule || needsTargets || isScheduleViolation}
                        <div class="review-validation" role="status">
                            <CircleAlert size={17} aria-hidden="true" />
                            <span>{!name.trim() ? m.enter_block_name_warning() : needsTargets ? m.block_need_targets() : needsSchedule ? m.block_need_schedule() : m.schedule_not_more_restrictive_warning()}</span>
                        </div>
                    {:else}
                        <div class="review-ready" role="status">
                            <ShieldCheck size={18} aria-hidden="true" />
                            <span>{block ? m.block_wizard_ready_edit() : m.block_wizard_ready_create()}</span>
                        </div>
                    {/if}
                </div>
            </section>
        {/if}
    </div>
    <footer class="editor-page-footer">
        <div class="footer-status" aria-live="polite">
            <span>{m.block_wizard_step_counter({step:step + 1})}</span>
            {#if scheduleEditorOpen}<span class="footer-warning">{m.schedule_period_finish_edit()}</span>
            {:else if step === 2 && (!name.trim() || needsSchedule || needsTargets || isScheduleViolation)}
                <span class="footer-warning">{m.block_wizard_incomplete()}</span>
            {/if}
        </div>
        <div class="footer-actions">
            {#if step > 0}
                <Button variant="outline" onclick={() => goToStep((step - 1) as 0 | 1 | 2)}
                    disabled={isSubmitting || scheduleEditorOpen}><ArrowLeft size={15} aria-hidden="true" />{m.block_wizard_back()}</Button>
            {/if}
            {#if step < 2}
                <Button onclick={() => goToStep((step + 1) as 0 | 1 | 2)} disabled={isSubmitting || scheduleEditorOpen}>
                    {m.block_wizard_continue()}<ArrowRight size={15} aria-hidden="true" />
                </Button>
            {:else}
                <Button onclick={handleSave}
                    disabled={isSubmitting || isLoadingData || dataLoadFailed || !name.trim() || scheduleEditorOpen || isScheduleViolation || needsSchedule || needsTargets}
                    class="save-action">
                    {#if isSubmitting}<Spinner data-icon="inline-start" />{/if}
                    {isSubmitting ? (block ? m.saving() : m.creating()) : block ? m.save() : m.block_button_create_activate()}
                </Button>
            {/if}
        </div>
    </footer>
</main>

<style>
    .block-editor-page {
        width: min(940px, calc(100% - 44px)); min-height: 100%; margin: 0 auto;
        padding-top: 18px; display: flex; flex-direction: column; container: block-editor / inline-size;
    }
    .editor-page-header { flex: none; }
    .back-link { display: inline-flex; align-items: center; gap: 8px; min-height: 30px;
        color: var(--muted-foreground); font-size: 13px; font-weight: 600; }
    .back-link:hover { color: var(--foreground); }
    .editor-title-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding-top: 6px; }
    .editor-title-row h1 { font: 600 clamp(26px, 4cqi, 32px)/1.15 "IBM Plex Sans Condensed", sans-serif; }
    .editor-title-row p { margin-top: 6px; color: var(--muted-foreground); font-size: 13px; line-height: 1.5; }
    .locked-state { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--ace-signal-deep);
        padding: 7px 10px; border-radius: 4px; font-size: 12px; color: var(--foreground); white-space: nowrap; }
    .editor-steps { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; margin-top: 22px; }
    .editor-steps button { text-align: left; padding: 0 0 13px; color: var(--muted-foreground); min-width: 0; cursor: pointer; }
    .editor-steps button:disabled { cursor: not-allowed; opacity: .55; }
    .step-progress { display: block; height: 3px; border-radius: 2px; background: var(--border); transition: background 140ms; }
    .editor-steps button.current .step-progress { background: var(--ace-signal); }
    .editor-steps button.complete .step-progress { background: var(--foreground); }
    .step-label { display: flex; align-items: center; gap: 9px; padding-top: 10px; font-size: 13px; font-weight: 600; }
    .editor-steps button.current .step-label { color: var(--foreground); }
    .step-number { display: grid; place-items: center; border: 1px solid var(--border);
        width: 23px; height: 23px; border-radius: 4px; flex: none; font: 600 12px "IBM Plex Mono",monospace; }
    .editor-steps button.current .step-number { background: var(--ace-signal); color: #10100e; border-color: var(--ace-signal); }
    .editor-page-body { flex: 1; min-width: 0; padding: 20px 0 28px; }
    .content-intro { display: flex; align-items: start; justify-content: space-between; gap: 18px; }
    .manage-shortcut { flex: none; border: 1px solid var(--border); border-radius: 4px;
        padding: 9px 12px; min-height: 38px; color: var(--foreground); background: var(--card);
        font-size: 13px; font-weight: 600; }
    .manage-shortcut:hover { background: var(--muted); }
    .step-intro h2 { font: 600 25px/1.25 "IBM Plex Sans Condensed",sans-serif; outline: none; }
    .step-intro p { color: var(--muted-foreground); margin-top: 6px; font-size: 14px; line-height: 1.5; max-width: 660px; }
    .step-content-stack { margin-top: 22px; display: grid; gap: 23px; }
    .secondary-section { border-top: 1px solid var(--border); padding-top: 22px; }
    .website-manager { border: 1px solid var(--border); border-radius: 6px; background: var(--card); }
    .website-manager summary { display: flex; align-items: center; justify-content: space-between; gap: 16px;
        cursor: pointer; padding: 17px 18px; list-style: none; }
    .website-manager summary::-webkit-details-marker { display: none; }
    .website-manager summary:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
    .website-manager summary strong,.website-manager summary small { display: block; }
    .website-manager summary strong { font-size: 15px; }
    .website-manager summary small { font-size: 13px; color: var(--muted-foreground); margin-top: 5px; }
    .website-manager .manager-detail { padding: 18px; border-top: 1px solid var(--border); }
    .inline-hint { display: flex; align-items: start; gap: 8px; font-size: 13px; color: var(--muted-foreground); margin-top: 24px; }
    .mode-selector { display: grid; grid-template-columns: repeat(2,minmax(0,1fr)); gap: 10px; margin-top: 22px; }
    .mode-selector button { display: flex; gap: 12px; align-items: flex-start; text-align: left; min-width: 0;
        background: var(--card); border: 1px solid var(--border); border-radius: 6px; padding: 17px; cursor: pointer; }
    .mode-selector button:hover { border-color: var(--foreground); }
    .mode-selector button.selected { border-color: var(--ace-signal-deep); box-shadow: inset 0 3px var(--ace-signal); }
    .mode-selector button:disabled { opacity: .5; cursor: not-allowed; }
    .mode-radio { margin-top: 2px; width: 17px; height: 17px; border: 1.5px solid var(--muted-foreground); border-radius: 100%; flex: none; }
    .mode-selector button.selected .mode-radio { border: 5px solid var(--foreground); }
    .mode-selector strong,.mode-selector small { display: block; }
    .mode-selector strong { font-size: 15px; }
    .mode-selector small { margin-top: 6px; color: var(--muted-foreground); font-size: 13px; line-height: 1.5; }
    .schedule-detail { border-top: 1px solid var(--border); padding-top: 24px; margin-top: 24px; }
    .enforcement-preview { display: grid; grid-template-columns: 4px 1fr; gap: 15px;
        border: 1px solid var(--border); border-radius: 6px; background: var(--muted);
        margin-top: 24px; overflow: hidden; }
    .enforcement-preview .preview-rail { background: var(--ace-signal); }
    .enforcement-preview > div { padding: 15px 16px 15px 0; font-size: 13px; line-height: 1.6; }
    .enforcement-preview p { color: var(--muted-foreground); margin-top: 4px; }
    .enforcement-preview p.preview-warning { color: var(--destructive); }
    .review-stack { margin-top: 22px; display: grid; gap: 20px; }
    .review-identity { padding: 17px 19px; border: 1px solid var(--border); background: var(--card); border-radius: 6px; }
    .review-list { border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
    .review-row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 15px 19px; }
    .review-row + .review-row { border-top: 1px solid var(--border); }
    .review-row-main { min-width: 0; }
    .review-row-main strong { font-size: 14px; }
    .review-row-main p { color: var(--muted-foreground); font-size: 13px; margin-top: 4px; line-height: 1.4; }
    .review-row-main .review-selected { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 600px; }
    .review-edit { color: var(--foreground); border-bottom: 1px solid currentColor; font-size: 13px; font-weight: 600; }
    .review-edit:hover { color: var(--ace-signal-deep); }
    .protection-panel { display: grid; grid-template-columns: auto minmax(0,1fr) auto; align-items: center; gap: 14px;
        padding: 16px 19px; border: 1px solid var(--border); border-radius: 6px; background: var(--card); }
    .protection-panel strong { font-size: 14px; }
    .protection-panel p,.protection-panel span { font-size: 13px; line-height: 1.5; color: var(--muted-foreground); }
    .protection-panel button { min-height: 36px; padding: 7px 12px; border: 1px solid var(--border); border-radius: 4px;
        font-size: 13px; font-weight: 600; }
    .protection-panel button:hover { background: var(--muted); }
    .protection-panel button:disabled { opacity: .5; cursor: not-allowed; }
    .review-validation,.review-ready { display: flex; align-items: start; gap: 9px; border: 1px solid var(--border);
        padding: 13px 15px; border-radius: 5px; font-size: 13px; background: var(--muted); line-height: 1.5; }
    .review-validation :global(svg) { color: var(--destructive); flex: none; }
    .review-ready :global(svg) { color: var(--ace-signal-deep); flex: none; }
    .editor-page-footer { position: sticky; bottom: 0; z-index: 5; margin-top: auto; border-top: 1px solid var(--border);
        background: var(--background); padding: 13px 0; display: flex; align-items: center; justify-content: space-between; gap: 14px; }
    .footer-status { color: var(--muted-foreground); display: flex; flex-wrap: wrap; gap: 10px; font-size: 12px; }
    .footer-warning { color: var(--destructive); }
    .footer-actions { display: flex; justify-content: flex-end; gap: 9px; }
    .footer-actions :global(button) { min-height: 39px; white-space: nowrap; display: inline-flex; gap: 8px; }
    .footer-actions :global(.save-action) { min-width: 164px; }
    @container block-editor (max-width: 640px) {
        .protection-panel { grid-template-columns: auto 1fr; }
        .protection-panel button,.protection-panel > span { grid-column: 2; justify-self: start; }
        .editor-page-footer { align-items: end; }
        .editor-steps { gap: 4px; }
        .step-label { gap: 5px; font-size: 12px; }
        .mode-selector { grid-template-columns: 1fr; }
        .content-intro { flex-wrap: wrap; }
    }
    @container ace-workspace (max-width: 720px) { .block-editor-page { width: calc(100% - 28px); } }
    @media (prefers-reduced-motion: reduce) { .step-progress { transition: none; } }
</style>
