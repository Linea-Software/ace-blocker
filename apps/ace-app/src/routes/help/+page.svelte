<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { Mail, Info, UserPlus, Cpu, AppWindow } from "@lucide/svelte";
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { resolve } from "$app/paths";
    import PageLayout from "$lib/components/PageLayout.svelte";
    import PageHeader from "$lib/components/PageHeader.svelte";

    const supportEmail = "support@aceblocker.com";

    let userContext = $state({
        os: "Unknown",
        version: "Unknown",
    });

    onMount(async () => {
        const ua = navigator.userAgent;
        if (ua.indexOf("Win") !== -1) userContext.os = "Windows";
        if (ua.indexOf("Mac") !== -1) userContext.os = "macOS";
        if (ua.indexOf("Linux") !== -1) userContext.os = "Linux";

        try {
            userContext.version = await invoke<string>("app_version");
        } catch (error) {
            console.error(
                "Failed to get app version via tauri invoke:",
                error,
            );
        }
    });
</script>

<svelte:head>
    <title>{m.help_title()} · {m.app_name()}</title>
</svelte:head>

<PageLayout width="narrow">
    {#snippet header()}
        <PageHeader
            title={m.help_title()}
            subtitle={m.help_subtitle()}
        />
    {/snippet}

    <div class="help-workspace">
        <section class="support-section">
            <div class="section-heading">
                <Mail size={17} aria-hidden="true" />
                <div>
                    <h2>{m.contact_team_title()}</h2>
                    <p>{m.contact_team_description()}</p>
                </div>
            </div>

            <div class="contact-ledger">
                <div class="contact-row">
                    <div>
                        <span>{m.support_contact_email_label()}</span>
                        <span class="support-address">{supportEmail}</span>
                    </div>
                    <a class="contact-action" href="mailto:{supportEmail}">
                        <Mail size={15} aria-hidden="true" />
                        {m.contact_support_btn()}
                    </a>
                </div>

                <dl class="system-facts">
                    <div>
                        <dt><Cpu size={14} aria-hidden="true" /> {m.help_system()}</dt>
                        <dd>{userContext.os}</dd>
                    </div>
                    <div>
                        <dt><AppWindow size={14} aria-hidden="true" /> {m.help_version()}</dt>
                        <dd>{userContext.version}</dd>
                    </div>
                </dl>

                <p class="response-note">{m.response_time_notice()}</p>
            </div>
        </section>

        <section class="lock-notice">
            <span class="notice-rail" aria-hidden="true"></span>
            <div>
                <h2>{m.lock_notice_title()}</h2>
                <p>{m.lock_notice_description()}</p>
            </div>
        </section>

        <section class="resource-ledger" aria-label={m.help_support()}>
            <a href={resolve("/getting-started")}>
                <span class="resource-mark" aria-hidden="true"><Info size={17} /></span>
                <span>
                    <strong>{m.help_user_guide()}</strong>
                    <small>{m.help_user_guide_description()}</small>
                </span>
                <span class="resource-action">{m.guide_open_section()}</span>
            </a>

            <div class="resource-disabled" aria-disabled="true">
                <span class="resource-mark" aria-hidden="true"><UserPlus size={17} /></span>
                <span>
                    <strong>{m.help_community()}</strong>
                    <small>{m.help_community_description()}</small>
                </span>
                <span class="resource-action">{m.coming_soon()}</span>
            </div>
        </section>
    </div>
</PageLayout>

<style>
    .help-workspace {
        display: grid;
        gap: 26px;
    }

    h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-weight: 600;
    }

    .lock-notice {
        display: grid;
        grid-template-columns: 6px minmax(0, 1fr);
        gap: 14px;
        overflow: hidden;
        padding-right: 16px;
        border: 1px solid color-mix(in srgb, var(--ace-signal) 42%, var(--border));
        border-radius: 5px;
        background: color-mix(in srgb, var(--ace-signal) 7%, var(--card));
    }

    .notice-rail {
        background: var(--ace-signal);
    }

    .lock-notice > div {
        padding: 14px 0;
    }

    .lock-notice h2 {
        font-size: 16px;
    }

    .lock-notice p,
    .section-heading p {
        margin: 4px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.5;
    }

    .support-section {
        display: grid;
        gap: 12px;
    }

    .section-heading {
        display: grid;
        grid-template-columns: 24px minmax(0, 1fr);
        gap: 8px;
        color: var(--muted-foreground);
    }

    .section-heading h2 {
        color: var(--foreground);
        font-size: 18px;
    }

    .contact-ledger {
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .contact-row {
        min-height: 78px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 20px;
        padding: 13px 14px;
        border-bottom: 1px solid var(--border);
    }

    .contact-row span,
    .contact-row a {
        display: block;
    }

    .contact-row span {
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .support-address {
        margin-top: 3px;
        color: var(--foreground);
        font-family: "IBM Plex Mono", monospace;
        font-size: 12px;
        font-weight: 500;
    }

    .contact-action {
        min-height: 36px;
        display: inline-flex !important;
        align-items: center;
        gap: 7px;
        border: 1px solid var(--border);
        border-radius: 4px;
        padding: 0 11px;
        color: var(--foreground);
        font-size: 12px;
        font-weight: 600;
        text-decoration: none;
    }

    .contact-action:hover {
        background: var(--muted);
    }

    .system-facts {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        margin: 0;
    }

    .system-facts > div {
        padding: 12px 14px;
    }

    .system-facts > div + div {
        border-left: 1px solid var(--border);
    }

    dt {
        display: flex;
        align-items: center;
        gap: 6px;
        color: var(--muted-foreground);
        font-size: 11px;
    }

    dd {
        margin: 4px 0 0 20px;
        font-family: "IBM Plex Mono", monospace;
        font-size: 12px;
    }

    .response-note {
        margin: 0;
        padding: 10px 14px;
        border-top: 1px solid var(--border);
        color: var(--muted-foreground);
        font-size: 11px;
    }

    .resource-ledger {
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .resource-ledger > a,
    .resource-ledger > div {
        min-height: 74px;
        display: grid;
        grid-template-columns: 36px minmax(0, 1fr) auto;
        align-items: center;
        gap: 12px;
        padding: 10px 14px;
        color: var(--foreground);
        text-decoration: none;
    }

    .resource-ledger > * + * {
        border-top: 1px solid var(--border);
    }

    .resource-ledger > a:hover {
        background: color-mix(in srgb, var(--muted) 45%, transparent);
    }

    .resource-disabled {
        background: color-mix(in srgb, var(--muted) 24%, var(--card));
    }

    .resource-disabled .resource-mark,
    .resource-disabled strong,
    .resource-disabled small {
        opacity: 0.72;
    }

    .resource-mark {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--muted-foreground);
    }

    .resource-ledger strong,
    .resource-ledger small {
        display: block;
    }

    .resource-ledger strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 600;
    }

    .resource-ledger small {
        margin-top: 3px;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .resource-action {
        color: var(--muted-foreground);
        font-size: 12px;
        font-weight: 600;
    }

    @container page (max-width: 560px) {
        .contact-row {
            align-items: flex-start;
            flex-direction: column;
        }

        .system-facts {
            grid-template-columns: 1fr;
        }

        .system-facts > div + div {
            border-top: 1px solid var(--border);
            border-left: 0;
        }

        .resource-ledger > a,
        .resource-ledger > div {
            grid-template-columns: 36px minmax(0, 1fr);
        }

        .resource-action {
            grid-column: 2;
        }
    }
</style>
