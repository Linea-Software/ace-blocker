
    import { For, onSettled } from "solid-js";
    import { Dynamic } from "@solidjs/web";
    import * as Accordion from "$lib/components/ui/accordion/solid";
    import {
        Compass,
        Shield,
        Globe,
        Sparkles,
        Lock,
        CalendarClock,
        RotateCcw,
        ArrowRight,
        ChevronDown,
    } from "@lucide/solid";

    import * as m from "$lib/paraglide/messages.js";
    import { onboardingStore } from "$lib/stores/onboarding";
    import PageLayout from "$lib/components/PageLayout";
    import PageHeader from "$lib/components/PageHeader";


import "./page.css";
export default function GettingStartedPage() {
    const concepts = [
        {
            id: "network-blocks",
            icon: Shield,
            title: m.network_blocks(),
            body: m.guide_concept_blocks_body(),
            href: "/network-blocks",
            action: m.guide_open_blocks(),
        },
        {
            id: "site-packs",
            icon: Globe,
            title: m.site_packs(),
            body: m.guide_concept_site_packs_body(),
            href: "/site-packs",
            action: m.guide_open_packs(),
        },
        {
            id: "schedules",
            icon: CalendarClock,
            title: m.guide_concept_schedules_title(),
            body: m.guide_concept_schedules_body(),
            href: "/network-blocks",
            action: m.guide_open_blocks(),
        },
        {
            id: "locks",
            icon: Lock,
            title: m.guide_concept_locks_title(),
            body: m.guide_concept_locks_body(),
            href: "/network-blocks",
            action: m.guide_open_blocks(),
        },
        {
            id: "resistance-mode",
            icon: Sparkles,
            title: m.resistance_mode(),
            body: m.guide_concept_resistance_body(),
            href: "/resistance-mode",
            action: m.guide_open_resistance(),
        },
    ] as const;

    const setupSteps = [
        {
            id: "pick-blocks",
            title: m.guide_step_pick_blocks_title(),
            body: m.guide_step_pick_blocks_body(),
        },
        {
            id: "add-rules",
            title: m.guide_step_add_rules_title(),
            body: m.guide_step_add_rules_body(),
        },
        {
            id: "schedule",
            title: m.guide_step_schedule_title(),
            body: m.guide_step_schedule_body(),
        },
        {
            id: "lock",
            title: m.guide_step_lock_title(),
            body: m.guide_step_lock_body(),
        },
        {
            id: "resistance",
            title: m.guide_step_resistance_title(),
            body: m.guide_step_resistance_body(),
        },
    ] as const;

    const faqs = [
        {
            id: "why-blocked",
            question: m.guide_faq_why_blocked_question(),
            answer: m.guide_faq_why_blocked_answer(),
        },
        {
            id: "unblock",
            question: m.guide_faq_unblock_question(),
            answer: m.guide_faq_unblock_answer(),
        },
        {
            id: "locked-out",
            question: m.guide_faq_locked_out_question(),
            answer: m.guide_faq_locked_out_answer(),
        },
        {
            id: "certificate",
            question: m.guide_faq_certificate_question(),
            answer: m.guide_faq_certificate_answer(),
        },
        {
            id: "privacy",
            question: m.guide_faq_privacy_question(),
            answer: m.guide_faq_privacy_answer(),
        },
    ] as const;
    onSettled(() => { document.title = `${m.getting_started()} · ${m.app_name()}`; });
    return (
        <PageLayout width="narrow" data-tour="getting-started-page" header={<PageHeader title={m.getting_started()} subtitle={m.guide_subtitle()} />}>
    {/* Header is provided through the shared layout prop. */}

    <div class="guide-workspace">
        <section class="guide-section" aria-labelledby="guide-setup-title">
            <div class="section-heading setup-heading">
                <div>
                    <h2 id="guide-setup-title">{m.guide_setup_title()}</h2>
                    <p>{m.guide_setup_description()}</p>
                </div>
                <a class="start-action" href={"/network-blocks"}>
                    {m.guide_open_blocks()}
                    <ArrowRight size={15} aria-hidden="true" />
                </a>
            </div>

            <ol class="setup-line">
                <For each={setupSteps}>{(setupStep, index) => <>
                    <li>
                        <span class="step-index">{index() + 1}</span>
                        <div>
                            <h3>{setupStep.title}</h3>
                            <p>{setupStep.body}</p>
                        </div>
                    </li>
                </>}</For>
            </ol>
        </section>

        <section class="replay-strip">
            <div>
                <h2>{m.guide_replay_title()}</h2>
                <p>{m.guide_replay_description()}</p>
            </div>
            <div class="replay-actions">
                <button type="button" onClick={() => onboardingStore.startTour()}>
                    <Compass size={15} aria-hidden="true" />
                    {m.guide_start_tour()}
                </button>
                <button type="button" onClick={() => onboardingStore.restartWizard()}>
                    <RotateCcw size={15} aria-hidden="true" />
                    {m.guide_restart_wizard()}
                </button>
            </div>
        </section>

        <section class="guide-section" aria-labelledby="guide-concepts-title">
            <details class="concept-disclosure">
                <summary>
                    <span>
                        <h2 id="guide-concepts-title">{m.guide_concepts_title()}</h2>
                        <small>{m.guide_concepts_description()}</small>
                    </span>
                    <ChevronDown size={19} aria-hidden="true" />
                </summary>
                <div class="concept-ledger">
                <For each={concepts}>{(concept) => <>
                    <a class="concept-row" href={concept.href}>
                        <span class="concept-mark" aria-hidden="true">
                            <Dynamic component={concept.icon} size={17} strokeWidth={1.8} />
                        </span>
                        <span class="concept-copy">
                            <strong>{concept.title}</strong>
                            <span>{concept.body}</span>
                        </span>
                        <span class="open-label">{concept.action}</span>
                    </a>
                </>}</For>
                </div>
            </details>
        </section>

        <section class="guide-section" aria-labelledby="guide-faq-title">
            <div class="section-heading">
                <h2 id="guide-faq-title">{m.guide_faq_title()}</h2>
                <p>{m.guide_faq_description()}</p>
            </div>

            <Accordion.Root type="single" class="faq-ledger">
                <For each={faqs}>{(faq) => <>
                    <Accordion.Item value={faq.id} class="border-border">
                        <Accordion.Trigger class="px-4 text-left font-condensed text-sm font-semibold">
                            {faq.question}
                        </Accordion.Trigger>
                        <Accordion.Content class="px-4 pb-4 text-sm leading-relaxed text-muted-foreground">
                            {faq.answer}
                        </Accordion.Content>
                    </Accordion.Item>
                </>}</For>
            </Accordion.Root>
        </section>

        <section class="support-strip">
            <div>
                <h2>{m.guide_still_stuck_title()}</h2>
                <p>{m.guide_still_stuck_description()}</p>
            </div>
            <a href={"/help"}>{m.help_support()}</a>
        </section>
    </div>
</PageLayout>
    );
}
