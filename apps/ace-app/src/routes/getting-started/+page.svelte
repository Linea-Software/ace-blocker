<script lang="ts">
    import * as Accordion from "$lib/components/ui/accordion/index.js";
    import {
        Compass,
        Shield,
        Globe,
        Sparkles,
        Lock,
        CalendarClock,
        RotateCcw,
    } from "@lucide/svelte";
    import { resolve } from "$app/paths";
    import * as m from "$lib/paraglide/messages.js";
    import { onboardingStore } from "$lib/stores/onboarding.svelte";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";

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
</script>

<svelte:head>
    <title>{m.getting_started()} · {m.app_name()}</title>
</svelte:head>

<PageLayout width="narrow" data-tour="getting-started-page">
    {#snippet header()}
        <PageHeader
            title={m.getting_started()}
            subtitle={m.guide_subtitle()}
        />
    {/snippet}

    <div class="guide-workspace">
        <section class="guide-section" aria-labelledby="guide-concepts-title">
            <div class="section-heading">
                <h2 id="guide-concepts-title">{m.guide_concepts_title()}</h2>
                <p>{m.guide_concepts_description()}</p>
            </div>

            <div class="concept-ledger">
                {#each concepts as concept (concept.id)}
                    <a class="concept-row" href={resolve(concept.href)}>
                        <span class="concept-mark" aria-hidden="true">
                            <concept.icon size={17} strokeWidth={1.8} />
                        </span>
                        <span class="concept-copy">
                            <strong>{concept.title}</strong>
                            <span>{concept.body}</span>
                        </span>
                        <span class="open-label">{concept.action}</span>
                    </a>
                {/each}
            </div>
        </section>

        <section class="replay-strip">
            <div>
                <h2>{m.guide_replay_title()}</h2>
                <p>{m.guide_replay_description()}</p>
            </div>
            <div class="replay-actions">
                <button type="button" onclick={() => onboardingStore.startTour()}>
                    <Compass size={15} aria-hidden="true" />
                    {m.guide_start_tour()}
                </button>
                <button type="button" onclick={() => onboardingStore.restartWizard()}>
                    <RotateCcw size={15} aria-hidden="true" />
                    {m.guide_restart_wizard()}
                </button>
            </div>
        </section>

        <section class="guide-section" aria-labelledby="guide-setup-title">
            <div class="section-heading">
                <h2 id="guide-setup-title">{m.guide_setup_title()}</h2>
                <p>{m.guide_setup_description()}</p>
            </div>

            <ol class="setup-line">
                {#each setupSteps as setupStep, index (setupStep.id)}
                    <li>
                        <span class="step-index">{index + 1}</span>
                        <div>
                            <h3>{setupStep.title}</h3>
                            <p>{setupStep.body}</p>
                        </div>
                    </li>
                {/each}
            </ol>
        </section>

        <section class="guide-section" aria-labelledby="guide-faq-title">
            <div class="section-heading">
                <h2 id="guide-faq-title">{m.guide_faq_title()}</h2>
                <p>{m.guide_faq_description()}</p>
            </div>

            <Accordion.Root type="single" class="faq-ledger">
                {#each faqs as faq (faq.id)}
                    <Accordion.Item value={faq.id} class="border-border">
                        <Accordion.Trigger class="px-4 text-left font-condensed text-sm font-semibold">
                            {faq.question}
                        </Accordion.Trigger>
                        <Accordion.Content class="px-4 pb-4 text-sm leading-relaxed text-muted-foreground">
                            {faq.answer}
                        </Accordion.Content>
                    </Accordion.Item>
                {/each}
            </Accordion.Root>
        </section>

        <section class="support-strip">
            <div>
                <h2>{m.guide_still_stuck_title()}</h2>
                <p>{m.guide_still_stuck_description()}</p>
            </div>
            <a href={resolve("/help")}>{m.help_support()}</a>
        </section>
    </div>
</PageLayout>

<style>
    .guide-workspace {
        display: grid;
        gap: 28px;
    }

    h2,
    h3 {
        font-family: "IBM Plex Sans Condensed", sans-serif;
    }

    .replay-strip,
    .support-strip {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 24px;
        padding: 16px 17px;
        border: 1px solid var(--border);
        border-left: 5px solid var(--ace-ash);
        border-radius: 5px;
        background: var(--card);
    }

    .replay-strip h2,
    .support-strip h2 {
        margin: 0;
        font-size: 17px;
        font-weight: 600;
    }

    .replay-strip p,
    .support-strip p {
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    .replay-actions {
        display: flex;
        gap: 7px;
        flex-shrink: 0;
    }

    .replay-actions button,
    .support-strip a {
        min-height: 36px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 7px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 11px;
        background: transparent;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
        cursor: pointer;
    }

    .replay-actions button:hover,
    .support-strip a:hover {
        background: var(--muted);
    }

    .guide-section {
        display: grid;
        gap: 13px;
    }

    .section-heading h2 {
        margin: 0;
        font-size: 20px;
        font-weight: 600;
    }

    .section-heading p {
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    .concept-ledger {
        border: 1px solid var(--border);
        border-radius: 5px;
        overflow: hidden;
        background: var(--card);
    }

    .concept-row {
        min-height: 73px;
        display: grid;
        grid-template-columns: 37px minmax(0, 1fr) auto;
        align-items: center;
        gap: 13px;
        padding: 10px 14px;
        border-bottom: 1px solid var(--border);
        color: var(--foreground);
        text-decoration: none;
        transition: background 140ms ease;
    }

    .concept-row:last-child {
        border-bottom: 0;
    }

    .concept-row:hover {
        background: color-mix(in srgb, var(--muted) 45%, transparent);
    }

    .concept-mark {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--muted-foreground);
    }

    .concept-copy {
        min-width: 0;
    }

    .concept-copy strong,
    .concept-copy > span {
        display: block;
    }

    .concept-copy strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 600;
    }

    .concept-copy > span {
        margin-top: 3px;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .open-label {
        color: var(--muted-foreground);
        font-size: 12px;
        font-weight: 600;
    }

    .setup-line {
        margin: 0;
        padding: 0;
        border-top: 1px solid var(--border);
        list-style: none;
    }

    .setup-line li {
        display: grid;
        grid-template-columns: 32px minmax(0, 1fr);
        gap: 11px;
        padding: 15px 4px;
        border-bottom: 1px solid var(--border);
    }

    .step-index {
        padding-top: 2px;
        color: var(--muted-foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 11px;
        font-weight: 500;
        font-variant-numeric: tabular-nums;
    }

    .setup-line h3 {
        margin: 0;
        font-size: 15px;
        font-weight: 600;
    }

    .setup-line p {
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.5;
    }

    :global(.faq-ledger) {
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .support-strip {
        border-left-color: var(--ace-signal);
    }

    @container page (max-width: 560px) {
        .replay-strip,
        .support-strip {
            align-items: stretch;
            flex-direction: column;
        }

        .replay-actions {
            flex-wrap: wrap;
        }

        .concept-row {
            grid-template-columns: 37px minmax(0, 1fr);
        }

        .open-label {
            grid-column: 2;
        }
    }
</style>
