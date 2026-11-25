import { groupSitePackCategories } from "$lib/utils/site-pack-presentation";
import { createMemo, createSignal, For, Show, onSettled } from "solid-js";
import * as Alert from "$lib/components/ui/alert/solid";
import { Button } from "$lib/components/ui/button/solid";
import { Skeleton } from "$lib/components/ui/skeleton/solid";
import { Spinner } from "$lib/components/ui/spinner/solid";
import * as m from "$lib/paraglide/messages.js";
import { toast } from "solid-sonner";
import {
    getDashboardSnapshot,
    getAllSitePacks,
    getScheduleEntries,
    getNetBlockToSitePackConnections,
    getUserRules,
    type NetworkBlockSchema,
    type ScheduleMode,
    type ScheduleEntrySchema,
    type SitePackSchema,
    type UserDefinedRuleSchema,
} from "$engine-client";
import { UserDefinedRulesDelta } from "$lib/deltas/user-defined-rules-delta";
import { ScheduleChanges } from "$lib/deltas/schedule-delta";
import { SitePacksConnectionDelta } from "$lib/deltas/site-packs-connection-delta";
import { saveNetworkBlock } from "./_save-network-block";
import { isAceCategory } from "$lib/utils/category-presentation";
import SectionGeneral from "./_components/SectionGeneral";
import SectionSafeSearch from "./_components/SectionSafeSearch";
import SectionRules from "./_components/SectionRules";
import SectionSites from "./_components/SectionSites";
import SectionSchedule from "./_components/SectionSchedule";
import {
    CircleAlert,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    LockKeyhole,
    ShieldCheck,
    Check,
} from "@lucide/solid";
import "./block-editor.css";

export interface BlockEditorProps {
    block: NetworkBlockSchema | null;
    onSave: (block: NetworkBlockSchema) => void;
    onClose?: () => void;
    onProtect?: () => void;
    step?: 0 | 1 | 2;
    onStepChange?: (step: 0 | 1 | 2) => void;
}

export default function BlockEditor(props: BlockEditorProps) {
    const [canonicalBlock, setCanonicalBlock] =
        createSignal<NetworkBlockSchema | null>(props.block);
    const [name, setName] = createSignal(props.block?.name ?? "");
    const [blockIcon, setBlockIcon] = createSignal<string | null>(
        props.block?.icon ?? null,
    );
    const [isSafeSearch, setIsSafeSearch] = createSignal(
        props.block?.safeSearch ?? false,
    );
    const [scheduleMode, setScheduleMode] = createSignal<ScheduleMode>(
        props.block?.scheduleMode ?? "always",
    );
    const [isSubmitting, setIsSubmitting] = createSignal(false);
    const [isLoadingData, setIsLoadingData] = createSignal(
        props.block !== null,
    );
    const [rulesDelta, setRulesDelta] = createSignal(
        new UserDefinedRulesDelta(),
    );
    const [scheduleDelta, setScheduleDelta] = createSignal(
        new ScheduleChanges(),
    );
    const [sitePacksDelta, setSitePacksDelta] = createSignal(
        new SitePacksConnectionDelta(),
    );
    const [existingRules, setExistingRules] = createSignal<
        UserDefinedRuleSchema[]
    >([]);
    const [existingEntries, setExistingEntries] = createSignal<
        ScheduleEntrySchema[]
    >([]);
    const [connectedListIds, setConnectedListIds] = createSignal<string[]>([]);
    const [availableLists, setAvailableLists] = createSignal<SitePackSchema[]>(
        [],
    );
    const [scheduleEditorOpen, setScheduleEditorOpen] = createSignal(false);
    const [dataLoadFailed, setDataLoadFailed] = createSignal(false);
    const [saveConfirmationWarning, setSaveConfirmationWarning] =
        createSignal(false);
    const [websitesOpen, setWebsitesOpen] = createSignal(false);
    const [hasMoreBelow, setHasMoreBelow] = createSignal(false);
    const [currentStep, setCurrentStep] = createSignal<0 | 1 | 2>(
        props.step ?? 0,
    );

    const step = () => props.step ?? currentStep();
    const updateStep = (next: 0 | 1 | 2) => {
        setCurrentStep(next);
        props.onStepChange?.(next);
    };
    function leaveEditor() {
        if (!isSubmitting() && !scheduleEditorOpen()) props.onClose?.();
    }

    const steps = [
        m.block_wizard_content(),
        m.block_wizard_schedule(),
        m.block_wizard_review(),
    ];
    const chosenPacks = createMemo(() => availableLists().filter((pack) => sitePacksDelta().currentConnectedListIds.includes(pack.id)));
    const categoryCount = createMemo(() => groupSitePackCategories(chosenPacks()).length);
    const customListCount = createMemo(() => chosenPacks().filter((pack) => !isAceCategory(pack)).length);
    const websiteCount = createMemo(
        () =>
            rulesDelta().currentRules.filter(
                (rule) => rule.ruleType === "domain",
            ).length,
    );
    const regexCount = createMemo(
        () =>
            rulesDelta().currentRules.filter(
                (rule) => rule.ruleType === "regex",
            ).length,
    );
    const chosenNames = createMemo(() => [
        ...groupSitePackCategories(chosenPacks()).map((group) => group.name),
        ...chosenPacks().filter((pack) => !isAceCategory(pack)).map((pack) => pack.name),
    ]);
    function showWebsites() {
        setWebsitesOpen(true);
        queueMicrotask(() => {
            document
                .getElementById("website-manager")
                ?.scrollIntoView({ block: "center" });
            updateScrollHint();
        });
    }
    function goToStep(next: 0 | 1 | 2) {
        if (scheduleEditorOpen() || isSubmitting()) return;
        updateStep(next);
        queueMicrotask(() => {
            document.getElementById("editor-step-heading")?.focus();
            updateScrollHint();
        });
    }

    let updateScrollHint: () => void = () => {};
    function initializeScrollHint() {
        const editor =
            document.querySelector<HTMLElement>(".block-editor-page");
        if (!editor) return () => {};
        let scrollParent: HTMLElement | null = editor.parentElement;
        while (
            scrollParent &&
            !/(auto|scroll)/.test(getComputedStyle(scrollParent).overflowY)
        ) {
            scrollParent = scrollParent.parentElement;
        }
        const scroller = scrollParent ?? document.scrollingElement;
        if (!(scroller instanceof HTMLElement)) return () => {};
        const update = () =>
            setHasMoreBelow(
                scroller.scrollHeight -
                    scroller.scrollTop -
                    scroller.clientHeight >
                    20,
            );
        updateScrollHint = update;
        const observer = new ResizeObserver(update);
        observer.observe(editor);
        observer.observe(scroller);
        scroller.addEventListener("scroll", update, { passive: true });
        window.addEventListener("resize", update);
        const frame = requestAnimationFrame(update);
        return () => {
            cancelAnimationFrame(frame);
            observer.disconnect();
            scroller.removeEventListener("scroll", update);
            window.removeEventListener("resize", update);
            updateScrollHint = () => {};
        };
    }

    const isScheduleViolation = createMemo(() => {
        const block = canonicalBlock();
        return (
            !!block?.isLocked &&
            ((block?.scheduleMode === "always" &&
                scheduleMode() === "scheduled") ||
                (scheduleMode() === "scheduled" &&
                    !scheduleDelta().isValidForLockedBlock()))
        );
    });
    const needsSchedule = createMemo(
        () =>
            scheduleMode() === "scheduled" &&
            scheduleDelta().currentScheduleEntries.length === 0,
    );
    const targetCount = createMemo(
        () =>
            rulesDelta().currentRules.length +
            sitePacksDelta().currentConnectedListIds.length +
            (isSafeSearch() ? 1 : 0),
    );
    const needsTargets = createMemo(
        () => !canonicalBlock() && targetCount() === 0,
    );
    const hasUnsavedChanges = createMemo(() => {
        const block = canonicalBlock();
        return (
            !!block &&
            (name() !== block.name ||
                blockIcon() !== (block.icon ?? null) ||
                isSafeSearch() !== block.safeSearch ||
                scheduleMode() !== block.scheduleMode ||
                rulesDelta().changes.length > 0 ||
                scheduleDelta().changes.length > 0 ||
                sitePacksDelta().changes.length > 0)
        );
    });

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
        const failed = [
            rulesRes.error && "rules",
            scheduleRes.error && "schedule",
            connectionsRes.error && "connections",
            listsRes.error && "site packs",
        ].filter(Boolean);
        if (rulesRes.error)
            console.error("Failed to load rules:", rulesRes.error);
        const rules = rulesRes.data?.userDefinedRules ?? [];
        setExistingRules(rules);
        setRulesDelta(new UserDefinedRulesDelta(structuredClone(rules)));
        if (scheduleRes.error)
            console.error("Failed to load schedule:", scheduleRes.error);
        const entries = scheduleRes.data?.scheduleEntries ?? [];
        setExistingEntries(entries);
        setScheduleDelta(
            new ScheduleChanges(
                structuredClone(entries),
                canonicalBlock()?.isLocked ?? false,
            ),
        );
        if (connectionsRes.error)
            console.error("Failed to load connections:", connectionsRes.error);
        const ids = (connectionsRes.data?.connections ?? []).map(
            (item) => item.sitePackId,
        );
        setConnectedListIds(ids);
        setSitePacksDelta(new SitePacksConnectionDelta([...ids]));
        if (listsRes.error)
            console.error("Failed to load site packs:", listsRes.error);
        setAvailableLists(listsRes.data?.sitePacks ?? []);
        return failed.length === 0;
    }

    onSettled(() => {
        document.title = `${canonicalBlock() ? m.edit_block() : m.create_block()} · ${m.app_name()}`;
        const cleanup = initializeScrollHint();
        void (async () => {
            try {
                if (canonicalBlock()) {
                    const loaded = await loadData(canonicalBlock()!.id);
                    if (!loaded) {
                        setDataLoadFailed(true);
                        toast.error(m.failed_to_load_block());
                    }
                } else {
                    const response = await getAllSitePacks();
                    // New-block mode can start with an empty catalog when the
                    // native engine is unavailable. Existing block edits stay
                    // fail-closed; create attempts still require a real RPC.
                    if (!response.error)
                        setAvailableLists(response.data?.sitePacks ?? []);
                }
            } catch (error) {
                console.error(
                    "Failed to initialize network block editor:",
                    error,
                );
                setDataLoadFailed(true);
                toast.error(m.failed_to_load_block());
            } finally {
                setIsLoadingData(false);
            }
        })();
        return cleanup;
    });

    async function handleSave() {
        if (
            !name().trim() ||
            scheduleEditorOpen() ||
            dataLoadFailed() ||
            isSubmitting() ||
            needsSchedule() ||
            needsTargets() ||
            isScheduleViolation()
        )
            return;
        setIsSubmitting(true);
        try {
            const outcome = await saveNetworkBlock({
                block: canonicalBlock()
                    ? structuredClone(canonicalBlock()!)
                    : null,
                name: name(),
                icon: blockIcon(),
                safeSearch: isSafeSearch(),
                scheduleMode: scheduleMode(),
                rules: rulesDelta(),
                schedule: scheduleDelta(),
                packs: sitePacksDelta(),
            });
            if (outcome.kind === "failed") {
                console.error("Failed to save network block:", outcome.error);
                if (!outcome.uncertain) {
                    toast.error(m.block_save_rejected());
                    return;
                }
                // Reconciliation is mandatory when the server might have committed.
                setSaveConfirmationWarning(true);
                setIsLoadingData(true);
                const snapshot = await getDashboardSnapshot();
                const previousBlock = canonicalBlock();
                const canonical =
                    previousBlock &&
                    snapshot.data?.blocks.find(
                        (item) => item.block.id === previousBlock.id,
                    );
                if (snapshot.error || !canonical) {
                    setDataLoadFailed(true);
                } else {
                    setCanonicalBlock(canonical.block);
                    setName(canonical.block.name);
                    setBlockIcon(canonical.block.icon ?? null);
                    setIsSafeSearch(canonical.block.safeSearch);
                    setScheduleMode(canonical.block.scheduleMode);
                    setDataLoadFailed(!(await loadData(canonical.block.id)));
                }
                toast.error(m.block_save_unconfirmed());
                return;
            }
            props.onSave(outcome.block);
        } catch (error) {
            console.error("Failed to save network block:", error);
            setDataLoadFailed(true);
            setSaveConfirmationWarning(true);
            toast.error(m.block_save_unconfirmed());
        } finally {
            setIsLoadingData(false);
            setIsSubmitting(false);
        }
    }
    const safeSearchOnly = () =>
        isSafeSearch() &&
        rulesDelta().currentRules.length === 0 &&
        sitePacksDelta().currentConnectedListIds.length === 0;
    const invalidForSave = () =>
        !name().trim() ||
        needsSchedule() ||
        needsTargets() ||
        isScheduleViolation();
    const validationMessage = () =>
        !name().trim()
            ? m.enter_block_name_warning()
            : needsTargets()
              ? m.block_need_targets()
              : needsSchedule()
                ? m.block_need_schedule()
                : m.schedule_not_more_restrictive_warning();

    return (
        <main
            class="block-editor-page"
            aria-label={canonicalBlock() ? m.edit_block() : m.create_block()}
        >
            <header class="editor-page-header">
                <button
                    type="button"
                    class="back-link"
                    onClick={leaveEditor}
                    aria-label={m.cancel()}
                >
                    <ArrowLeft size={17} aria-hidden="true" />{" "}
                    {m.network_blocks()}
                </button>
                <div class="editor-title-row">
                    <div>
                        <h1>
                            {canonicalBlock()
                                ? m.edit_block()
                                : m.create_block()}
                        </h1>
                        <p>{m.block_wizard_intro()}</p>
                    </div>
                    <Show when={canonicalBlock()?.isLocked}>
                        <span class="locked-state">
                            <LockKeyhole size={14} aria-hidden="true" />{" "}
                            {m.lock_review_title()}
                        </span>
                    </Show>
                </div>
                <nav
                    class="editor-steps"
                    aria-label={m.block_wizard_navigation()}
                >
                    <For each={steps}>
                        {(label, index) => (
                            <button
                                type="button"
                                class={[
                                    "",
                                    {
                                        current: step() === index(),
                                        complete: step() > index(),
                                    },
                                ]}
                                aria-current={
                                    step() === index() ? "step" : undefined
                                }
                                disabled={
                                    scheduleEditorOpen() || isSubmitting()
                                }
                                onClick={() => goToStep(index() as 0 | 1 | 2)}
                            >
                                <span class="step-label">
                                    <span class="step-number">
                                        <Show
                                            when={step() > index()}
                                            fallback={index() + 1}
                                        >
                                            <Check
                                                size={13}
                                                aria-hidden="true"
                                            />
                                        </Show>
                                    </span>
                                    {label}
                                </span>
                            </button>
                        )}
                    </For>
                </nav>
            </header>
            <div class="editor-page-body">
                <Show when={saveConfirmationWarning()}>
                    <Alert.Root variant="destructive">
                        <CircleAlert aria-hidden="true" />
                        <Alert.Title>
                            {m.block_save_unconfirmed_title()}
                        </Alert.Title>
                        <Alert.Description>
                            {m.block_save_unconfirmed()}
                        </Alert.Description>
                    </Alert.Root>
                </Show>
                <Show
                    when={!isLoadingData()}
                    fallback={
                        <div
                            class="flex flex-col gap-5"
                            role="status"
                            aria-label={m.loading_block_configuration()}
                        >
                            <Skeleton class="h-10 w-full rounded-sm" />
                            <Skeleton class="h-28 w-full rounded-sm" />
                            <Skeleton class="h-24 w-full rounded-sm" />
                        </div>
                    }
                >
                    <Show
                        when={!dataLoadFailed()}
                        fallback={
                            <Alert.Root variant="destructive">
                                <CircleAlert aria-hidden="true" />
                                <Alert.Title>
                                    {m.failed_to_load_block_title()}
                                </Alert.Title>
                                <Alert.Description>
                                    {m.failed_to_load_block()}
                                </Alert.Description>
                            </Alert.Root>
                        }
                    >
                        <Show
                            when={step() === 0}
                            fallback={
                                <Show
                                    when={step() === 1}
                                    fallback={
                                        <section
                                            class="editor-step-content"
                                            aria-labelledby="editor-step-heading"
                                        >
                                            <div class="step-intro">
                                                <h2
                                                    id="editor-step-heading"
                                                    tabindex={-1}
                                                >
                                                    {m.block_wizard_review_heading()}
                                                </h2>
                                                <p>
                                                    {m.block_wizard_review_description()}
                                                </p>
                                            </div>
                                            <div class="review-stack">
                                                <div class="review-identity">
                                                    <SectionGeneral
                                                        name={name()}
                                                        onNameChange={setName}
                                                        icon={blockIcon()}
                                                        onIconChange={
                                                            setBlockIcon
                                                        }
                                                        disabled={isSubmitting()}
                                                        isLocked={
                                                            canonicalBlock()
                                                                ?.isLocked ??
                                                            false
                                                        }
                                                    />
                                                </div>
                                                <div
                                                    class="review-list"
                                                    aria-label={m.block_wizard_summary()}
                                                >
                                                    <div class="review-row">
                                                        <div class="review-row-main">
                                                            <strong>
                                                                {m.block_wizard_content()}
                                                            </strong>
                                                            <p>
                                                                {m.block_wizard_category_count(
                                                                    {
                                                                        count: categoryCount(),
                                                                    },
                                                                )}{" "}
                                                                ·{" "}
                                                                {m.block_wizard_custom_count(
                                                                    {
                                                                        count: customListCount(),
                                                                    },
                                                                )}
                                                            </p>
                                                            <p>
                                                                {m.block_wizard_websites_count(
                                                                    {
                                                                        count: websiteCount(),
                                                                    },
                                                                )}
                                                                <Show
                                                                    when={regexCount()}
                                                                >
                                                                    {" "}
                                                                    ·{" "}
                                                                    {m.block_wizard_regex_count(
                                                                        {
                                                                            count: regexCount(),
                                                                        },
                                                                    )}
                                                                </Show>
                                                            </p>
                                                            <p data-testid="review-safe-search">
                                                                {m.force_safe_search()}
                                                                :{" "}
                                                                {isSafeSearch()
                                                                    ? m.block_safe_search_included()
                                                                    : m.block_safe_search_not_included()}
                                                            </p>
                                                            <Show
                                                                when={safeSearchOnly()}
                                                            >
                                                                <p>
                                                                    {m.block_safe_search_only_hint()}
                                                                </p>
                                                            </Show>
                                                            <Show
                                                                when={
                                                                    chosenNames()
                                                                        .length
                                                                }
                                                            >
                                                                <p
                                                                    class="review-selected"
                                                                    title={chosenNames().join(
                                                                        ", ",
                                                                    )}
                                                                >
                                                                    {chosenNames()
                                                                        .slice(
                                                                            0,
                                                                            3,
                                                                        )
                                                                        .join(
                                                                            ", ",
                                                                        )}
                                                                    {chosenNames()
                                                                        .length >
                                                                    3
                                                                        ? "…"
                                                                        : ""}
                                                                </p>
                                                            </Show>
                                                        </div>
                                                        <button
                                                            type="button"
                                                            class="review-edit"
                                                            onClick={() =>
                                                                goToStep(0)
                                                            }
                                                        >
                                                            {m.block_wizard_edit()}
                                                        </button>
                                                    </div>
                                                    <div class="review-row">
                                                        <div class="review-row-main">
                                                            <strong>
                                                                {m.block_wizard_schedule()}
                                                            </strong>
                                                            <p>
                                                                {scheduleMode() ===
                                                                "always"
                                                                    ? m.block_mode_always_detail()
                                                                    : m.block_wizard_period_count(
                                                                          {
                                                                              count: scheduleDelta()
                                                                                  .currentScheduleEntries
                                                                                  .length,
                                                                          },
                                                                      )}
                                                            </p>
                                                        </div>
                                                        <button
                                                            type="button"
                                                            class="review-edit"
                                                            onClick={() =>
                                                                goToStep(1)
                                                            }
                                                        >
                                                            {m.block_wizard_edit()}
                                                        </button>
                                                    </div>
                                                    <div class="review-row">
                                                        <div class="review-row-main">
                                                            <strong>
                                                                {m.block_wizard_activation()}
                                                            </strong>
                                                            <p>
                                                                {canonicalBlock()
                                                                    ? canonicalBlock()!
                                                                          .enabled
                                                                        ? m.block_wizard_enabled()
                                                                        : m.block_wizard_disabled()
                                                                    : m.block_wizard_create_enabled()}
                                                            </p>
                                                        </div>
                                                    </div>
                                                </div>
                                                <div class="protection-panel">
                                                    <LockKeyhole
                                                        size={19}
                                                        aria-hidden="true"
                                                    />
                                                    <div>
                                                        <strong>
                                                            {canonicalBlock()
                                                                ?.isLocked
                                                                ? m.lock_review_title()
                                                                : m.lock_protect_title()}
                                                        </strong>
                                                        <p>
                                                            {m.control_lock_help_detail()}
                                                        </p>
                                                    </div>
                                                    <Show
                                                        when={canonicalBlock()}
                                                        fallback={
                                                            <span>
                                                                {m.block_lock_after_save()}
                                                            </span>
                                                        }
                                                    >
                                                        <button
                                                            type="button"
                                                            onClick={() =>
                                                                props.onProtect?.()
                                                            }
                                                            disabled={
                                                                isSubmitting() ||
                                                                hasUnsavedChanges()
                                                            }
                                                            title={
                                                                hasUnsavedChanges()
                                                                    ? m.save()
                                                                    : m.lock_protect_action()
                                                            }
                                                        >
                                                            {m.lock_protect_action()}
                                                        </button>
                                                    </Show>
                                                </div>
                                                <Show
                                                    when={invalidForSave()}
                                                    fallback={
                                                        <div
                                                            class="review-ready"
                                                            role="status"
                                                        >
                                                            <ShieldCheck
                                                                size={18}
                                                                aria-hidden="true"
                                                            />
                                                            <span>
                                                                {canonicalBlock()
                                                                    ? m.block_wizard_ready_edit()
                                                                    : m.block_wizard_ready_create()}
                                                            </span>
                                                        </div>
                                                    }
                                                >
                                                    <div
                                                        class="review-validation"
                                                        role="status"
                                                    >
                                                        <CircleAlert
                                                            size={17}
                                                            aria-hidden="true"
                                                        />
                                                        <span>
                                                            {validationMessage()}
                                                        </span>
                                                    </div>
                                                </Show>
                                            </div>
                                        </section>
                                    }
                                >
                                    <section
                                        class="editor-step-content"
                                        id="block-when"
                                        aria-labelledby="editor-step-heading"
                                    >
                                        <div class="step-intro">
                                            <h2
                                                id="editor-step-heading"
                                                tabindex={-1}
                                            >
                                                {m.block_wizard_schedule_heading()}
                                            </h2>
                                            <p>
                                                {m.block_wizard_schedule_description()}
                                            </p>
                                        </div>
                                        <div
                                            class="mode-selector"
                                            role="group"
                                            aria-label={m.block_when_heading()}
                                        >
                                            <button
                                                type="button"
                                                class={[
                                                    "",
                                                    {
                                                        selected:
                                                            scheduleMode() ===
                                                            "always",
                                                    },
                                                ]}
                                                aria-pressed={
                                                    scheduleMode() === "always"
                                                        ? "true"
                                                        : "false"
                                                }
                                                disabled={
                                                    scheduleEditorOpen() ||
                                                    isSubmitting()
                                                }
                                                onClick={() =>
                                                    setScheduleMode("always")
                                                }
                                            >
                                                <span
                                                    class="mode-radio"
                                                    aria-hidden="true"
                                                />
                                                <span>
                                                    <strong>
                                                        {m.block_mode_always()}
                                                    </strong>
                                                    <small>
                                                        {m.block_mode_always_detail()}
                                                    </small>
                                                </span>
                                            </button>
                                            <button
                                                type="button"
                                                class={[
                                                    "",
                                                    {
                                                        selected:
                                                            scheduleMode() ===
                                                            "scheduled",
                                                    },
                                                ]}
                                                aria-pressed={
                                                    scheduleMode() ===
                                                    "scheduled"
                                                        ? "true"
                                                        : "false"
                                                }
                                                disabled={
                                                    scheduleEditorOpen() ||
                                                    isSubmitting() ||
                                                    (!!canonicalBlock()
                                                        ?.isLocked &&
                                                        canonicalBlock()
                                                            ?.scheduleMode ===
                                                            "always")
                                                }
                                                onClick={() =>
                                                    setScheduleMode("scheduled")
                                                }
                                            >
                                                <span
                                                    class="mode-radio"
                                                    aria-hidden="true"
                                                />
                                                <span>
                                                    <strong>
                                                        {m.block_mode_scheduled()}
                                                    </strong>
                                                    <small>
                                                        {m.block_mode_scheduled_detail()}
                                                    </small>
                                                </span>
                                            </button>
                                        </div>
                                        <Show
                                            when={
                                                scheduleMode() === "scheduled"
                                            }
                                        >
                                            <div class="schedule-detail">
                                                <SectionSchedule
                                                    delta={scheduleDelta()}
                                                    isLocked={
                                                        canonicalBlock()
                                                            ?.isLocked ?? false
                                                    }
                                                    hasOpenEditor={scheduleEditorOpen()}
                                                    onHasOpenEditorChange={
                                                        setScheduleEditorOpen
                                                    }
                                                />
                                            </div>
                                        </Show>
                                        <div
                                            class="enforcement-preview"
                                            aria-live="polite"
                                        >
                                            <span
                                                class="preview-rail"
                                                aria-hidden="true"
                                            />
                                            <div>
                                                <strong>
                                                    {m.block_preview_heading()}
                                                </strong>
                                                <p>
                                                    {scheduleMode() === "always"
                                                        ? m.block_preview_always()
                                                        : m.block_preview_scheduled()}
                                                </p>
                                                <Show when={needsSchedule()}>
                                                    <p class="preview-warning">
                                                        {m.block_need_schedule()}
                                                    </p>
                                                </Show>
                                                <Show
                                                    when={isScheduleViolation()}
                                                >
                                                    <p class="preview-warning">
                                                        {m.schedule_not_more_restrictive_warning()}
                                                    </p>
                                                </Show>
                                            </div>
                                        </div>
                                    </section>
                                </Show>
                            }
                        >
                            <section
                                class="editor-step-content"
                                aria-labelledby="editor-step-heading"
                            >
                                <div class="step-intro content-intro">
                                    <div>
                                        <h2
                                            id="editor-step-heading"
                                            tabindex={-1}
                                        >
                                            {m.block_wizard_content_heading()}
                                        </h2>
                                        <p>
                                            {m.block_wizard_content_description()}
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        class="manage-shortcut"
                                        onClick={showWebsites}
                                    >
                                        {m.block_manage_websites()} (
                                        {websiteCount()})
                                    </button>
                                </div>
                                <div class="step-content-stack">
                                    <SectionSites
                                        delta={sitePacksDelta()}
                                        availableLists={availableLists()}
                                        onAvailableListsChange={
                                            setAvailableLists
                                        }
                                        isLocked={
                                            canonicalBlock()?.isLocked ?? false
                                        }
                                    />
                                    <SectionSafeSearch
                                        isSafeSearch={isSafeSearch()}
                                        onSafeSearchChange={setIsSafeSearch}
                                        disabled={isSubmitting()}
                                        isLocked={
                                            canonicalBlock()?.isLocked ?? false
                                        }
                                    />
                                    <Show when={safeSearchOnly()}>
                                        <p
                                            class="text-muted-foreground text-sm leading-relaxed"
                                            data-testid="safe-search-only-hint"
                                        >
                                            {m.block_safe_search_only_hint()}
                                        </p>
                                    </Show>
                                    <div class="secondary-section">
                                        <details
                                            class="website-manager"
                                            id="website-manager"
                                            data-testid="website-manager"
                                            open={websitesOpen()}
                                            onToggle={(event) =>
                                                setWebsitesOpen(
                                                    event.currentTarget.open,
                                                )
                                            }
                                        >
                                            <summary>
                                                <span>
                                                    <strong>
                                                        {m.block_manage_websites()}
                                                    </strong>
                                                    <small>
                                                        {m.block_manage_websites_hint()}
                                                    </small>
                                                    <small>
                                                        {m.block_wizard_websites_count(
                                                            {
                                                                count: websiteCount(),
                                                            },
                                                        )}
                                                        <Show
                                                            when={regexCount()}
                                                        >
                                                            {" "}
                                                            ·{" "}
                                                            {m.block_wizard_regex_count(
                                                                {
                                                                    count: regexCount(),
                                                                },
                                                            )}
                                                        </Show>
                                                    </small>
                                                </span>
                                                <span class="text-sm font-semibold text-foreground">
                                                    {websitesOpen()
                                                        ? m.close()
                                                        : m.block_wizard_edit()}
                                                </span>
                                            </summary>
                                            <div class="manager-detail">
                                                <SectionRules
                                                    delta={rulesDelta()}
                                                    disabled={isSubmitting()}
                                                    isLocked={
                                                        canonicalBlock()
                                                            ?.isLocked ?? false
                                                    }
                                                />
                                            </div>
                                        </details>
                                    </div>
                                </div>
                                <Show when={needsTargets()}>
                                    <p class="inline-hint" role="status">
                                        <CircleAlert
                                            size={15}
                                            aria-hidden="true"
                                        />
                                        {m.block_need_targets()}
                                    </p>
                                </Show>
                            </section>
                        </Show>
                    </Show>
                </Show>
            </div>
            <footer class="editor-page-footer">
                <div class="footer-status" aria-live="polite">
                    <span>
                        {m.block_wizard_step_counter({ step: step() + 1 })}
                    </span>
                    <Show when={hasMoreBelow()}>
                        <span class="scroll-cue">
                            <ArrowDown size={13} aria-hidden="true" />
                            {m.block_editor_scroll_more()}
                        </span>
                    </Show>
                    <Show
                        when={scheduleEditorOpen()}
                        fallback={
                            <Show when={step() === 2 && invalidForSave()}>
                                <span class="footer-warning">
                                    {m.block_wizard_incomplete()}
                                </span>
                            </Show>
                        }
                    >
                        <span class="footer-warning">
                            {m.schedule_period_finish_edit()}
                        </span>
                    </Show>
                </div>
                <div class="footer-actions">
                    <Show when={step() > 0}>
                        <Button
                            variant="outline"
                            disabled={isSubmitting() || scheduleEditorOpen()}
                            onClick={() => goToStep((step() - 1) as 0 | 1 | 2)}
                        >
                            <ArrowLeft size={15} aria-hidden="true" />
                            {m.block_wizard_back()}
                        </Button>
                    </Show>
                    <Show
                        when={step() < 2}
                        fallback={
                            <Button
                                onClick={() => void handleSave()}
                                disabled={
                                    isSubmitting() ||
                                    isLoadingData() ||
                                    dataLoadFailed() ||
                                    !name().trim() ||
                                    scheduleEditorOpen() ||
                                    isScheduleViolation() ||
                                    needsSchedule() ||
                                    needsTargets()
                                }
                                class="save-action editor-primary-action"
                            >
                                <Show when={isSubmitting()}>
                                    <Spinner data-icon="inline-start" />
                                </Show>
                                {isSubmitting()
                                    ? canonicalBlock()
                                        ? m.saving()
                                        : m.creating()
                                    : canonicalBlock()
                                      ? m.save()
                                      : m.block_button_create_activate()}
                            </Button>
                        }
                    >
                        <Button
                            class="editor-primary-action"
                            onClick={() => goToStep((step() + 1) as 0 | 1 | 2)}
                            disabled={isSubmitting() || scheduleEditorOpen()}
                        >
                            {m.block_wizard_continue()}
                            <ArrowRight size={15} aria-hidden="true" />
                        </Button>
                    </Show>
                </div>
            </footer>
        </main>
    );
}
