<script lang="ts">
    import Inspector from "$lib/components/ace/structural/Inspector.svelte";
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

    let {
        open = $bindable(false),
        block = $bindable(null),
        onSave,
    }: {
        open: boolean;
        block: NetworkBlockSchema | null;
        onSave: (block: NetworkBlockSchema) => void;
    } = $props();

    let name = $state(block?.name ?? "");
    let blockIcon = $state<string | null>(block?.icon ?? null);
    let isSafeSearch = $state(block?.safeSearch ?? false);
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

    // UI State
    let activeSection = $state<"blocking" | "schedule">("blocking");
    let scheduleEditorOpen = $state(false);
    // Set when an existing block's editor data failed to load: saving against
    // empty rules/schedule/connections would clobber the server state.
    let dataLoadFailed = $state(false);
    let saveConfirmationWarning = $state(false);

    const editorSections = [
        { id: "blocking", label: m.blocking() },
        { id: "schedule", label: m.schedule() },
    ] as const;

    const showBlockingError = $derived(!name || name.trim() === "");
    const isScheduleViolation = $derived(
        block?.lockState.state !== "inactive" &&
            !scheduleDelta.isValidForLockedBlock(),
    );

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
            block ? block.lockState.state !== "inactive" : false,
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
                    dataLoadFailed = !(await loadData(canonical.block.id));
                }
                toast.error(m.block_save_unconfirmed());
                return;
            }

            onSave(outcome.block);
            open = false;
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

        async function goToBlockName() {
        if (scheduleEditorOpen) return;
        activeSection = "blocking";
        await tick();
        document.getElementById("name")?.focus();
    }

    function handleWindowKeydown(event: KeyboardEvent) {
        if (event.key === "Escape" && open && !isSubmitting) {
            if (scheduleEditorOpen) {
                event.preventDefault();
                return;
            }
            open = false;
        }
    }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<Inspector
    title={block ? m.edit_block() : m.create_block()}
    description={m.block_modal_description()}
    closeLabel={m.close()}
    onClose={() => (open = false)}
>
    {#snippet navigation()}
        <nav
            aria-label={m.block_editor_sections_label()}
            class="flex w-full gap-1 p-2"
        >
            {#each editorSections as section (section.id)}
                <Button
                    variant={activeSection === section.id ? "secondary" : "ghost"}
                    class="h-9 min-w-0 flex-1 rounded-sm px-3"
                    onclick={() => (activeSection = section.id)}
                    aria-pressed={activeSection === section.id}
                    disabled={scheduleEditorOpen &&
                        activeSection === "schedule" &&
                        section.id !== "schedule"}
                >
                    <span class="relative">
                        {section.label}
                        {#if section.id === "blocking" && showBlockingError}
                            <span
                                class="bg-destructive ring-background absolute -top-0.5 -right-2 size-1.5 rounded-full ring-1"
                                aria-hidden="true"
                            ></span>
                        {/if}
                    </span>
                </Button>
            {/each}
        </nav>
    {/snippet}

    {#if saveConfirmationWarning}
        <Alert.Root variant="destructive">
            <CircleAlert aria-hidden="true" />
            <Alert.Title>{m.block_save_unconfirmed_title()}</Alert.Title>
            <Alert.Description>{m.block_save_unconfirmed()}</Alert.Description>
        </Alert.Root>
    {/if}
    {#if isLoadingData}
        <div
            class="flex flex-col gap-5"
            role="status"
            aria-label={m.loading_block_configuration()}
        >
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
    {:else if activeSection === "blocking"}
        <div class="space-y-6">
            <SectionGeneral
                bind:name
                bind:icon={blockIcon}
                bind:isSafeSearch
                disabled={isSubmitting}
                isLocked={block ? block.lockState.state !== "inactive" : false}
            />

            <div class="border-border border-t"></div>

            <SectionSites
                delta={sitePacksDelta}
                bind:availableLists
                isLocked={block ? block.lockState.state !== "inactive" : false}
            />

            <div class="border-border border-t"></div>

            <SectionRules
                delta={rulesDelta}
                disabled={isSubmitting}
                isLocked={block ? block.lockState.state !== "inactive" : false}
            />
        </div>
    {:else}
        <SectionSchedule
            delta={scheduleDelta}
            isLocked={block ? block.lockState.state !== "inactive" : false}
            bind:hasOpenEditor={scheduleEditorOpen}
        />
    {/if}

    {#snippet footer()}
        <div class="flex flex-wrap items-center justify-between gap-3">
            <div class="min-w-0 flex-1" aria-live="polite">
                {#if scheduleEditorOpen}
                    <div class="text-muted-foreground flex items-center gap-2 text-xs">
                        <CircleAlert class="size-4 shrink-0" aria-hidden="true" />
                        <span>{m.schedule_period_finish_edit()}</span>
                    </div>
                {:else if !name || name.trim() === ""}
                    <div class="text-muted-foreground flex flex-wrap items-center gap-2 text-xs">
                        <CircleAlert class="size-4 shrink-0" aria-hidden="true" />
                        <span>{activeSection === "schedule"
                            ? m.schedule_missing_name_hint()
                            : m.enter_block_name_warning()}</span>
                        {#if activeSection === "schedule"}
                            <button
                                type="button"
                                class="text-foreground focus-visible:ring-ring rounded-sm underline underline-offset-4 focus-visible:outline-none focus-visible:ring-2"
                                onclick={goToBlockName}
                            >
                                {m.schedule_go_to_blocking()}
                            </button>
                        {/if}
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
                    onclick={() => (open = false)}
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
                        isScheduleViolation}
                    class="min-w-24 px-6"
                >
                    {#if isSubmitting}
                        <Spinner data-icon="inline-start" />
                    {/if}
                    {isSubmitting
                        ? block
                            ? m.saving()
                            : m.creating()
                        : m.save()}
                </Button>
            </div>
        </div>
    {/snippet}
</Inspector>

