<script lang="ts">
    /**
     * First-run setup wizard.
     *
     * Shown once, after the Terms of Service have been accepted. It explains the
     * three pillars of the app and gets the user to an actually-protecting
     * configuration instead of dropping them on an empty dashboard.
     *
     * Everything the wizard configures is persisted by the engine; only the
     * "has been through onboarding" flag lives in `onboardingStore`.
     */
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import { Progress } from "$lib/components/ui/progress/index.js";
    import { Spinner } from "$lib/components/ui/spinner/index.js";
    import {
        Shield,
        Globe,
        Sparkles,
        ArrowLeft,
        ArrowRight,
        Check,
        Compass,
        BookOpen,
        TriangleAlert,
    } from "@lucide/svelte";
    import { toast } from "svelte-sonner";
    import { onMount } from "svelte";
    import { goto, invalidateAll } from "$app/navigation";
    import { resolve } from "$app/paths";

    import * as m from "$lib/paraglide/messages.js";
    import { onboardingStore } from "$lib/stores/onboarding.svelte";
    import { getPackIcon, FALLBACK_ICON } from "$lib/utils/pack-icons";
    import {
        ONBOARDING_BLOCK_ORDER,
        RECOMMENDED_BLOCK_IDS,
        enableStarterBlock,
        getDefaultBlockDescription,
        getDefaultBlockName,
    } from "$lib/onboarding/default-blocks";
    import { getDashboardSnapshot } from "$engine-client";
    import { setResistanceModeEnabled } from "$engine-client";

    type BlockChoice = {
        id: string;
        name: string;
        description: string;
        icon: string | null;
        rulesCount: number;
        /** Connected site packs — seeded blocks that dropped their regex
         *  lists in favour of packs report 0 user rules but pack coverage. */
        sitePackConnectionCount: number;
        /** Already enabled — the wizard leaves these alone. */
        alreadyEnabled: boolean;
        selected: boolean;
    };

    const STEP_COUNT = 5;

    let stepIndex = $state(0);
    let isLoading = $state(true);
    let isApplying = $state(false);
    let loadFailed = $state(false);

    let blockChoices = $state<BlockChoice[]>([]);
    let enableResistanceMode = $state(true);
    let resistanceAlreadyEnabled = $state(false);

    const progress = $derived(((stepIndex + 1) / STEP_COUNT) * 100);
    const enabledCount = $derived(
        blockChoices.filter((choice) => choice.alreadyEnabled).length,
    );

    /** Pulls the seeded default blocks and current resistance settings. */
    async function loadState() {
        isLoading = true;
        loadFailed = false;

        const { data, error } = await getDashboardSnapshot();

        if (error || !data) {
            console.error("Onboarding: failed to load engine state", error);
            loadFailed = true;
            isLoading = false;
            return;
        }

        const snapshotsById = new Map(
            data.blocks.map((snapshot) => [snapshot.block.id, snapshot]),
        );

        blockChoices = ONBOARDING_BLOCK_ORDER.flatMap((blockId) => {
            const snapshot = snapshotsById.get(blockId);
            if (!snapshot) return [];

            const alreadyEnabled = snapshot.block.enabled;

            return [
                {
                    id: blockId,
                    name: getDefaultBlockName(blockId, snapshot.block.name),
                    description: getDefaultBlockDescription(blockId),
                    icon: snapshot.block.icon ?? null,
                    rulesCount: snapshot.userRulesCount,
                    sitePackConnectionCount: snapshot.sitePackConnectionCount,
                    alreadyEnabled,
                    selected:
                        alreadyEnabled ||
                        (!onboardingStore.wizardPreviouslyCompleted &&
                            RECOMMENDED_BLOCK_IDS.includes(blockId)),
                },
            ];
        });

        resistanceAlreadyEnabled = data.resistanceModeSettings?.enabled ?? false;
        enableResistanceMode = true;
        isLoading = false;
    }

    /** Writes the user's choices to the engine. Called when leaving the last
     *  configuration step, so the summary reflects what actually happened. */
    async function applyChoices(): Promise<boolean> {
        isApplying = true;

        try {
            for (const choice of blockChoices) {
                if (!choice.selected || choice.alreadyEnabled) continue;
                await enableStarterBlock(choice.id);
                // Keep successfully enabled blocks out of any retry.
                choice.alreadyEnabled = true;
            }

            if (enableResistanceMode && !resistanceAlreadyEnabled) {
                const { error } = await setResistanceModeEnabled({
                    body: { enabled: true },
                });
                if (error) {
                    console.error(
                        "Onboarding: failed to enable resistance mode",
                        error,
                    );
                    throw new Error("Failed to enable resistance mode");
                }
            }

            await invalidateAll();
            return true;
        } catch (error) {
            console.error("Onboarding: failed to apply choices", error);
            toast.error(m.onboarding_apply_failed());
            return false;
        } finally {
            isApplying = false;
        }
    }

    async function goToNext() {
        // Step 3 is the last configuration step — commit before the summary.
        if (stepIndex === 3) {
            const applied = await applyChoices();
            if (!applied) return;
        }

        if (stepIndex < STEP_COUNT - 1) stepIndex += 1;
    }

    function goToPrevious() {
        if (stepIndex > 0) stepIndex -= 1;
    }

    function finish() {
        onboardingStore.completeWizard();
    }

    function finishAndTour() {
        onboardingStore.completeWizard();
        onboardingStore.startTour();
    }

    async function finishAndReadGuide() {
        onboardingStore.completeWizard();
        await goto(resolve("/getting-started"));
    }

    function skip() {
        onboardingStore.completeWizard();
    }

    /** One-line coverage summary per block: rules when present, otherwise the
     *  connected site packs (seeded blocks are pack-backed), else "0 rules". */
    function getRuleSummary(choice: BlockChoice): string {
        if (choice.rulesCount > 0) {
            return m.onboarding_block_rule_count({
                count: choice.rulesCount,
            });
        }
        if (choice.sitePackConnectionCount > 0) {
            return choice.sitePackConnectionCount === 1
                ? m.onboarding_block_pack_count_one()
                : m.onboarding_block_pack_count_other({
                      count: choice.sitePackConnectionCount,
                  });
        }
        return m.onboarding_block_rule_count({ count: 0 });
    }

    onMount(() => {
        loadState();
    });
</script>

<Dialog.Root open={true}>
    <Dialog.Content
        class="w-[95vw] sm:max-w-2xl p-0 gap-0 overflow-hidden rounded-md border-border bg-background shadow-none"
        showCloseButton={false}
        interactOutsideBehavior="ignore"
        escapeKeydownBehavior="ignore"
    >
        <!-- Header -->
        <div class="border-b border-border bg-card px-6 pt-5 pb-4">
            <div class="flex items-center gap-3">
                <div
                    class="flex h-10 w-8 shrink-0 items-center justify-center border-l-[4px] border-[var(--ace-signal)] text-muted-foreground"
                >
                    <Shield class="size-6" />
                </div>
                <div class="min-w-0">
                    <Dialog.Title class="font-condensed text-2xl font-semibold">
                        {#if stepIndex === 0}
                            {m.onboarding_welcome_title()}
                        {:else if stepIndex === 1}
                            {m.onboarding_how_it_works_title()}
                        {:else if stepIndex === 2}
                            {m.onboarding_blocks_title()}
                        {:else if stepIndex === 3}
                            {m.onboarding_resistance_title()}
                        {:else}
                            {m.onboarding_done_title()}
                        {/if}
                    </Dialog.Title>
                    <Dialog.Description
                        class="text-sm text-muted-foreground truncate"
                    >
                        {#if stepIndex === 0}
                            {m.onboarding_welcome_subtitle()}
                        {:else if stepIndex === 1}
                            {m.onboarding_how_it_works_subtitle()}
                        {:else if stepIndex === 2}
                            {m.onboarding_blocks_subtitle()}
                        {:else if stepIndex === 3}
                            {m.onboarding_resistance_subtitle()}
                        {:else}
                            {m.onboarding_done_subtitle()}
                        {/if}
                    </Dialog.Description>
                </div>
            </div>

            <div class="mt-4 space-y-1.5">
                <Progress value={progress} class="h-1" />
                <p
                    class="font-mono text-[11px] font-medium text-muted-foreground"
                >
                    {m.onboarding_step_counter({
                        current: stepIndex + 1,
                        total: STEP_COUNT,
                    })}
                </p>
            </div>
        </div>

        <!-- Body -->
        <div class="px-6 py-6 overflow-y-auto max-h-[55vh]">
            {#if stepIndex === 0}
                <div class="space-y-5">
                    <p class="text-sm text-muted-foreground leading-relaxed">
                        {m.onboarding_welcome_body()}
                    </p>
                    <ul class="space-y-3">
                        {#each [{ icon: Shield, text: m.onboarding_welcome_point_block() }, { icon: Sparkles, text: m.onboarding_welcome_point_resist() }, { icon: Compass, text: m.onboarding_welcome_point_track() }] as point (point.text)}
                            <li class="flex items-start gap-3">
                                <div
                                    class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-sm border border-border text-muted-foreground"
                                >
                                    <point.icon class="size-4" />
                                </div>
                                <span class="text-sm leading-relaxed"
                                    >{point.text}</span
                                >
                            </li>
                        {/each}
                    </ul>
                </div>
            {:else if stepIndex === 1}
                <div class="space-y-4">
                    {#each [{ icon: Shield, title: m.network_blocks(), body: m.onboarding_concept_blocks() }, { icon: Globe, title: m.site_packs(), body: m.onboarding_concept_site_packs() }, { icon: Sparkles, title: m.resistance_mode(), body: m.onboarding_concept_resistance() }] as concept (concept.title)}
                        <div
                            class="flex items-start gap-3 rounded-md border border-border bg-card p-4"
                        >
                            <div
                                class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted/40 text-muted-foreground"
                            >
                                <concept.icon class="size-4.5" />
                            </div>
                            <div class="space-y-1 min-w-0">
                                <h3 class="font-semibold text-sm">
                                    {concept.title}
                                </h3>
                                <p
                                    class="text-sm text-muted-foreground leading-relaxed"
                                >
                                    {concept.body}
                                </p>
                            </div>
                        </div>
                    {/each}
                </div>
            {:else if stepIndex === 2}
                {#if isLoading}
                    <div class="flex items-center justify-center py-12">
                        <Spinner class="size-6 text-muted-foreground" />
                    </div>
                {:else if loadFailed}
                    <div
                        class="flex flex-col items-center gap-3 py-10 text-center"
                    >
                        <TriangleAlert class="size-8 text-[var(--ace-danger)]" />
                        <p class="text-sm text-muted-foreground">
                            {m.onboarding_load_failed()}
                        </p>
                        <Button variant="outline" size="sm" onclick={loadState}>
                            {m.onboarding_retry()}
                        </Button>
                    </div>
                {:else if blockChoices.length === 0}
                    <p
                        class="text-sm text-muted-foreground text-center py-10 leading-relaxed"
                    >
                        {m.onboarding_no_default_blocks()}
                    </p>
                {:else}
                    <div class="space-y-3">
                        <p
                            class="text-sm text-muted-foreground leading-relaxed"
                        >
                            {m.onboarding_blocks_body()}
                        </p>

                        {#each blockChoices as choice (choice.id)}
                            {@const ChoiceIcon = choice.icon
                                ? getPackIcon(choice.icon)
                                : FALLBACK_ICON}
                            <label
                                class="flex cursor-pointer items-center gap-3 rounded-md border border-border bg-card p-4 transition-colors hover:bg-muted/45"
                            >
                                <div
                                    class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted/40 text-muted-foreground"
                                >
                                    <ChoiceIcon class="size-4.5" />
                                </div>
                                <div class="flex-1 min-w-0 space-y-0.5">
                                    <div class="flex items-center gap-2">
                                        <h3 class="font-semibold text-sm">
                                            {choice.name}
                                        </h3>
                                        {#if choice.alreadyEnabled}
                                            <span
                                                class="border-l-2 border-[var(--ace-success)] pl-2 text-[11px] font-medium text-muted-foreground"
                                            >
                                                {m.onboarding_block_already_active()}
                                            </span>
                                        {/if}
                                    </div>
                                    <p
                                        class="text-xs text-muted-foreground leading-relaxed"
                                    >
                                        {choice.description}
                                    </p>
                                    <p
                                        class="text-[0.7rem] text-muted-foreground/70"
                                    >
                                        {getRuleSummary(choice)}
                                    </p>
                                </div>
                                <Switch
                                    bind:checked={choice.selected}
                                    disabled={choice.alreadyEnabled}
                                    aria-label={choice.name}
                                />
                            </label>
                        {/each}

                        <p class="text-xs text-muted-foreground/80 pt-1">
                            {m.onboarding_blocks_footnote()}
                        </p>
                    </div>
                {/if}
            {:else if stepIndex === 3}
                <div class="space-y-4">
                    <p class="text-sm text-muted-foreground leading-relaxed">
                        {m.onboarding_resistance_body()}
                    </p>

                    <label
                        class="flex cursor-pointer items-center gap-3 rounded-md border border-border bg-card p-4 transition-colors hover:bg-muted/45"
                    >
                        <div
                            class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border bg-muted/40 text-muted-foreground"
                        >
                            <Sparkles class="size-4.5" />
                        </div>
                        <div class="flex-1 min-w-0 space-y-0.5">
                            <h3 class="font-semibold text-sm">
                                {m.onboarding_resistance_toggle_label()}
                            </h3>
                            <p
                                class="text-xs text-muted-foreground leading-relaxed"
                            >
                                {resistanceAlreadyEnabled
                                    ? m.onboarding_resistance_already_enabled()
                                    : m.onboarding_resistance_toggle_hint()}
                            </p>
                        </div>
                        <Switch
                            bind:checked={enableResistanceMode}
                            disabled={resistanceAlreadyEnabled}
                            aria-label={m.onboarding_resistance_toggle_label()}
                        />
                    </label>

                    <p class="text-xs text-muted-foreground/80">
                        {m.onboarding_resistance_footnote()}
                    </p>
                </div>
            {:else}
                <div class="space-y-5">
                    <div
                        class="flex items-center gap-3 rounded-md border border-border border-l-[5px] border-l-[var(--ace-success)] bg-card p-4"
                    >
                        <div
                            class="flex size-9 shrink-0 items-center justify-center rounded-sm border border-border text-[var(--ace-success)]"
                        >
                            <Check class="size-4.5" />
                        </div>
                        <p class="text-sm leading-relaxed">
                            {m.onboarding_done_summary({
                                count: enabledCount,
                            })}
                        </p>
                    </div>

                    <p class="text-sm text-muted-foreground leading-relaxed">
                        {m.onboarding_done_body()}
                    </p>

                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                        <Button
                            variant="outline"
                            class="h-auto flex-col items-start gap-1 rounded-sm py-4 text-left"
                            onclick={finishAndTour}
                        >
                            <span
                                class="flex items-center gap-2 font-semibold text-sm"
                            >
                                <Compass class="size-4" />
                                {m.onboarding_take_tour()}
                            </span>
                            <span
                                class="text-xs text-muted-foreground font-normal whitespace-normal"
                            >
                                {m.onboarding_take_tour_hint()}
                            </span>
                        </Button>
                        <Button
                            variant="outline"
                            class="h-auto flex-col items-start gap-1 rounded-sm py-4 text-left"
                            onclick={finishAndReadGuide}
                        >
                            <span
                                class="flex items-center gap-2 font-semibold text-sm"
                            >
                                <BookOpen class="size-4" />
                                {m.onboarding_read_guide()}
                            </span>
                            <span
                                class="text-xs text-muted-foreground font-normal whitespace-normal"
                            >
                                {m.onboarding_read_guide_hint()}
                            </span>
                        </Button>
                    </div>
                </div>
            {/if}
        </div>

        <!-- Footer -->
        <div
            class="flex items-center justify-between gap-3 border-t border-border bg-card px-6 py-4"
        >
            {#if stepIndex < STEP_COUNT - 1}
                <Button
                    variant="ghost"
                    size="sm"
                    class="text-muted-foreground"
                    onclick={skip}
                >
                    {m.onboarding_skip()}
                </Button>
            {:else}
                <div></div>
            {/if}

            <div class="flex items-center gap-2">
                {#if stepIndex > 0 && stepIndex < STEP_COUNT - 1}
                    <Button
                        variant="outline"
                        onclick={goToPrevious}
                        disabled={isApplying}
                    >
                        <ArrowLeft class="size-4 mr-1" />
                        {m.onboarding_back()}
                    </Button>
                {/if}

                {#if stepIndex === STEP_COUNT - 1}
                    <Button onclick={finish}>
                        {m.onboarding_go_to_dashboard()}
                        <Check class="size-4 ml-1" />
                    </Button>
                {:else}
                    <Button
                        onclick={goToNext}
                        disabled={isApplying || (stepIndex === 2 && isLoading)}
                    >
                        {#if isApplying}
                            <Spinner class="size-4 mr-1" />
                            {m.onboarding_applying()}
                        {:else}
                            {m.onboarding_next()}
                            <ArrowRight class="size-4 ml-1" />
                        {/if}
                    </Button>
                {/if}
            </div>
        </div>
    </Dialog.Content>
</Dialog.Root>
