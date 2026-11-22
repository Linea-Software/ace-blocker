import { onSettled } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { Mail, Info, UserPlus, Cpu, AppWindow } from "@lucide/solid";
import { invoke } from "@tauri-apps/api/core";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";
import "./page.css";

export default function HelpPage() {
    const supportEmail = "support@aceblocker.com";
    const ctx = { os: "Unknown", version: "Unknown" };
    reactiveFields(ctx, ["os", "version"]);
    onSettled(() => {
        document.title = `${m.help_title()} · ${m.app_name()}`;
        const ua = navigator.userAgent;
        if (ua.includes("Win")) ctx.os = "Windows";
        if (ua.includes("Mac")) ctx.os = "macOS";
        if (ua.includes("Linux")) ctx.os = "Linux";
        void invoke<string>("app_version")
            .then((v) => {
                ctx.version = v;
            })
            .catch((err) => {
                console.error(
                    "Failed to get app version via tauri invoke:",
                    err,
                );
            });
    });
    return (
        <PageLayout
            width="narrow"
            header={
                <PageHeader
                    title={m.help_title()}
                    subtitle={m.help_subtitle()}
                />
            }
        >
            {/* Header is provided through the shared layout prop. */}

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
                                <span class="support-address">
                                    {supportEmail}
                                </span>
                            </div>
                            <a
                                class="contact-action"
                                href={`mailto:${supportEmail}`}
                            >
                                <Mail size={15} aria-hidden="true" />
                                {m.contact_support_btn()}
                            </a>
                        </div>

                        <dl class="system-facts">
                            <div>
                                <dt>
                                    <Cpu size={14} aria-hidden="true" />{" "}
                                    {m.help_system()}
                                </dt>
                                <dd>{ctx.os}</dd>
                            </div>
                            <div>
                                <dt>
                                    <AppWindow size={14} aria-hidden="true" />{" "}
                                    {m.help_version()}
                                </dt>
                                <dd>{ctx.version}</dd>
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
                    <a href={"/getting-started"}>
                        <span class="resource-mark" aria-hidden="true">
                            <Info size={17} />
                        </span>
                        <span>
                            <strong>{m.help_user_guide()}</strong>
                            <small>{m.help_user_guide_description()}</small>
                        </span>
                        <span class="resource-action">
                            {m.guide_open_section()}
                        </span>
                    </a>

                    <div class="resource-disabled" aria-disabled="true">
                        <span class="resource-mark" aria-hidden="true">
                            <UserPlus size={17} />
                        </span>
                        <span>
                            <strong>{m.help_community()}</strong>
                            <small>{m.help_community_description()}</small>
                        </span>
                        <span class="resource-action">{m.coming_soon()}</span>
                    </div>
                </section>
            </div>
        </PageLayout>
    );
}
