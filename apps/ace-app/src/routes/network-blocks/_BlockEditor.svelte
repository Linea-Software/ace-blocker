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

    import SectionGeneral from "./_components/SectionGeneral.svelte";
    import SectionRules from "./_components/SectionRules.svelte";
    import SectionSites from "./_components/SectionSites.svelte";
    import SectionSchedule from "./_components/SectionSchedule.svelte";

    import CircleAlert from "@lucide/svelte/icons/circle-alert";
    import { ArrowLeft, LockKeyhole } from "@lucide/svelte";

    let {
        block = $bindable(null),
        onSave,
        onClose = () => {},
        onProtect = () => {},
        initialSection = "blocking",
    }: {
        block: NetworkBlockSchema | null;
        onSave: (block: NetworkBlockSchema) => void;
        onClose?: () => void;
        onProtect?: () => void;
        initialSection?: "blocking" | "schedule";
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
    function leaveEditor() { if (!isSubmitting && !scheduleEditorOpen) onClose(); }

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
            if (initialSection === "schedule") {
                await tick();
                document.getElementById("block-when")?.scrollIntoView({ block: "center" });
            }
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
            <ArrowLeft size={19} aria-hidden="true" /> {m.network_blocks()}
        </button>
        <h1>{block ? m.edit_block() : m.create_block()}</h1>
        <p>{m.block_modal_description()}</p>
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
    {:else}
        <div class="block-editor-grid">
            <section class="editor-column space-y-5" aria-labelledby="block-what">
                <h3 id="block-what" class="editor-heading">
                    {m.block_what_heading()}
                </h3>
                <SectionGeneral
                    bind:name
                    bind:icon={blockIcon}
                    bind:isSafeSearch
                    disabled={isSubmitting}
                    isLocked={block?.isLocked ?? false}
                />

                <div class="content-section border-border border-t pt-5">
                    <SectionSites
                        delta={sitePacksDelta}
                        bind:availableLists
                        isLocked={block?.isLocked ?? false}
                    />
                </div>
                <div class="content-section border-border border-t pt-5">
                    <SectionRules
                        delta={rulesDelta}
                        disabled={isSubmitting}
                        isLocked={block?.isLocked ?? false}
                    />
                </div>
            </section>

            <section class="editor-column space-y-5" id="block-when" aria-labelledby="block-when-heading">
                <h3 id="block-when-heading" class="editor-heading">
                    {m.block_when_heading()}
                </h3>

                <div class="mode-selector" role="group" aria-label={m.block_when_heading()}>
                    <button
                        type="button"
                        class:selected={scheduleMode === "always"}
                        aria-pressed={scheduleMode === "always"}
                        disabled={scheduleEditorOpen || isSubmitting}
                        onclick={() => (scheduleMode = "always")}
                    >
                        <strong>{m.block_mode_always()}</strong>
                        <span>{m.block_mode_always_detail()}</span>
                    </button>
                    <button
                        type="button"
                        class:selected={scheduleMode === "scheduled"}
                        aria-pressed={scheduleMode === "scheduled"}
                        disabled={scheduleEditorOpen || isSubmitting ||
                            (!!block?.isLocked && block?.scheduleMode === "always")}
                        onclick={() => (scheduleMode = "scheduled")}
                    >
                        <strong>{m.block_mode_scheduled()}</strong>
                        <span>{m.block_mode_scheduled_detail()}</span>
                    </button>
                </div>

                {#if scheduleMode === "scheduled"}
                    <div class="border-border border-t pt-5">
                        <SectionSchedule
                            delta={scheduleDelta}
                            isLocked={block?.isLocked ?? false}
                            bind:hasOpenEditor={scheduleEditorOpen}
                        />
                    </div>
                {/if}

                <div class="enforcement-preview" aria-live="polite">
                    <span class="preview-rail" aria-hidden="true"></span>
                    <div class="space-y-2">
                        <p class="preview-heading">{m.block_preview_heading()}</p>
                        <p>{scheduleMode === "always"
                            ? m.block_preview_always()
                            : m.block_preview_scheduled()}</p>
                        {#if needsSchedule}
                            <p class="preview-warning">{m.block_need_schedule()}</p>
                        {/if}
                        {#if needsTargets}
                            <p class="preview-warning">{m.block_need_targets()}</p>
                        {/if}
                    </div>
                </div>
            </section>

            <section class="editor-column protection-section" aria-labelledby="block-protection-heading">
                <h3 class="editor-heading" id="block-protection-heading">{m.lock_protect_action()}</h3>
                <div class="protection-panel">
                    <LockKeyhole size={20} aria-hidden="true" />
                    <div>
                        <strong>{block?.isLocked ? m.lock_review_title() : m.lock_protect_title()}</strong>
                        <p>{m.control_lock_help_detail()}</p>
                    </div>
                    {#if block}
                        <button type="button" onclick={onProtect} disabled={isSubmitting || isLoadingData || hasUnsavedChanges}
                            title={hasUnsavedChanges ? m.save() : m.lock_protect_action()}>{m.lock_protect_action()}</button>
                    {:else}
                        <span>{m.block_lock_after_save()}</span>
                    {/if}
                </div>
            </section>
        </div>
    {/if}

    </div>
    <footer class="editor-page-footer">
        <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="min-w-0 flex-1" aria-live="polite">
                {#if scheduleEditorOpen}
                    <div class="text-muted-foreground flex items-center gap-2 text-xs">
                        <CircleAlert class="size-4 shrink-0" aria-hidden="true" />
                        <span>{m.schedule_period_finish_edit()}</span>
                    </div>
                {:else if !name || name.trim() === ""}
                    <div class="text-muted-foreground flex items-center gap-2 text-sm">
                        <CircleAlert class="size-4 shrink-0" aria-hidden="true" />
                        <span>{m.enter_block_name_warning()}</span>
                    </div>
                {:else if needsSchedule || needsTargets}
                    <div class="text-muted-foreground flex items-center gap-2 text-sm">
                        <CircleAlert class="size-4 shrink-0" aria-hidden="true" />
                        <span>{needsSchedule ? m.block_need_schedule() : m.block_need_targets()}</span>
                    </div>
                {:else if isScheduleViolation}
                    <div
                        class="text-destructive flex items-center gap-2 text-xs"
                    >
                        <CircleAlert
                            class="size-4 shrink-0"
                            aria-hidden="true"
                        />
                        <span>{m.schedule_not_more_restrictive_warning()}</span>
                    </div>
                {/if}
            </div>

            <div class="flex shrink-0 items-center justify-end gap-2">
                <Button
                    variant="outline"
                    onclick={leaveEditor}
                    disabled={isSubmitting}
                >
                    {m.cancel()}
                </Button>
                <Button
                    onclick={handleSave}
                    disabled={isSubmitting ||
                        isLoadingData ||
                        dataLoadFailed ||
                        !name ||
                        name.trim() === "" ||
                        scheduleEditorOpen ||
                        isScheduleViolation ||
                        needsSchedule ||
                        needsTargets}
                    class="min-w-24 px-6"
                >
                    {#if isSubmitting}
                        <Spinner data-icon="inline-start" />
                    {/if}
                    {isSubmitting
                        ? block
                            ? m.saving()
                            : m.creating()
                        : block
                            ? m.save()
                            : m.block_button_create_activate()}
                </Button>
            </div>
        </div>
    </footer>
</main>


<style>
    .block-editor-grid {
        display: grid;
        gap: 28px;
        align-items: start;
    }
    .editor-column {
        min-width: 0;
    }
    .editor-heading {
        display: flex;
        align-items: baseline;
        gap: 11px;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 22px;
        font-weight: 600;
        line-height: 1.2;
    }
    .protection-section {
        border-top: 1px solid var(--border);
        padding-top: 25px;
    }
    .protection-panel {
        display: grid;
        grid-template-columns: auto minmax(0, 1fr) auto;
        align-items: center;
        gap: 16px;
        margin-top: 16px;
        padding: 18px;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }
    .protection-panel strong { font-size: 14px; }
    .protection-panel p,
    .protection-panel span {
        font-size: 13px;
        line-height: 1.5;
        color: var(--muted-foreground);
    }
    .protection-panel button {
        padding: 8px 12px;
        min-height: 36px;
        border: 1px solid var(--border);
        border-radius: 4px;
        font-size: 13px;
        font-weight: 600;
    }
    .protection-panel button:hover { background: var(--muted); }
    @container block-editor (max-width: 610px) {
        .protection-panel { grid-template-columns: auto 1fr; }
        .protection-panel button,
        .protection-panel > span { grid-column: 2; justify-self: start; }
    }
    .mode-selector {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 9px;
    }
    .mode-selector button {
        display: grid;
        min-height: 88px;
        align-content: center;
        gap: 6px;
        padding: 14px;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
        text-align: left;
        cursor: pointer;
    }
    .mode-selector button.selected {
        border-color: var(--ace-signal-deep);
        box-shadow: inset 0 3px var(--ace-signal);
    }
    .mode-selector button:disabled {
        cursor: not-allowed;
        opacity: .55;
    }
    .mode-selector strong {
        font-size: 15px;
        line-height: 1.2;
    }
    .mode-selector span {
        color: var(--muted-foreground);
        font-size: 13px;
        line-height: 1.45;
    }
    .enforcement-preview {
        display: grid;
        grid-template-columns: 4px minmax(0, 1fr);
        gap: 14px;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--muted);
        padding: 0 15px 0 0;
        font-size: 13px;
        line-height: 1.5;
    }
    .enforcement-preview > div {
        padding-block: 14px;
    }
    .preview-rail {
        background: var(--ace-signal);
    }
    .preview-heading {
        font-weight: 600;
    }
    .preview-warning {
        color: var(--destructive);
    }
    .block-editor-page {
        width: min(920px, calc(100% - 44px));
        min-height: 100%;
        margin: 0 auto;
        padding-top: 22px;
        display: flex;
        flex-direction: column;
        container: block-editor / inline-size;
    }
    .editor-page-header {
        padding: 4px 0 20px;
        border-bottom: 1px solid var(--border);
    }
    .back-link {
        display: inline-flex;
        gap: 8px;
        align-items: center;
        min-height: 34px;
        padding: 0;
        color: var(--muted-foreground);
        font-size: 13px;
        font-weight: 600;
    }
    .back-link:hover { color: var(--foreground); }
    .editor-page-header h1 {
        margin-top: 10px;
        font: 600 clamp(27px, 5cqi, 35px)/1.15 "IBM Plex Sans Condensed", sans-serif;
    }
    .editor-page-header p {
        margin-top: 7px;
        font-size: 14px;
        color: var(--muted-foreground);
    }
    .editor-page-body {
        flex: 1;
        min-width: 0;
        padding: 25px 0 24px;
    }
    .editor-page-footer {
        position: sticky;
        bottom: 0;
        z-index: 5;
        margin: 0;
        padding: 15px 0;
        border-top: 1px solid var(--border);
        background: var(--background);
    }
    @container ace-workspace (max-width: 720px) {
        .block-editor-page { width: calc(100% - 28px); }
        .editor-page-footer { margin: 0; }
    }
</style>
