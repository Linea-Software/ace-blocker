import { onSettled } from "solid-js";
import { reactiveFields } from "$lib/stores/reactive-fields";
import * as m from "$lib/paraglide/messages.js";
import { userStore } from "$lib/stores/user-store";
import { KeyRound, CircleCheck, Crown } from "@lucide/solid";
import UpgradeToProButton from "$lib/components/UpgradeToProButton";
import { openAccountPage } from "$lib/utils/account-browser";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./_LicenseSection.css";

export default function LicenseSection() {
    const state = {
        openingSignIn: false,
        linkCode: null as string | null,
        linkMessage: null as string | null,
        linkActive: false,
        linkFailure: false,
    };
    reactiveFields(state, [
        "openingSignIn",
        "linkCode",
        "linkMessage",
        "linkActive",
        "linkFailure",
    ]);
    let pollTimer: ReturnType<typeof setTimeout> | undefined;
    let generation = 0;
    type Start = {
        userCode: string;
        verificationUri: string;
        interval: number;
        expiresIn: number;
    };
    type Poll = {
        state: "pending" | "slow_down" | "linked" | "expired" | "consumed";
        retryAfter: number;
    };
    function queuePoll(seconds: number, cycle: number) {
        pollTimer = setTimeout(async () => {
            if (generation !== cycle || !state.linkActive) return;
            try {
                const reply = await invoke<Poll>("device_link_poll");
                if (generation !== cycle) return;
                if (reply.state === "linked") {
                    state.linkActive = false;
                    state.linkCode = null;
                    state.linkMessage = m.device_link_success();
                    void userStore.refreshAfterLoginOrEntitlementChange();
                } else if (
                    reply.state === "pending" ||
                    reply.state === "slow_down"
                ) {
                    queuePoll(Math.max(reply.retryAfter, 5), cycle);
                } else {
                    state.linkActive = false;
                    state.linkCode = null;
                    state.linkMessage = m.device_link_expired();
                    state.linkFailure = true;
                }
            } catch {
                if (generation !== cycle) return;
                state.linkActive = false;
                state.linkCode = null;
                state.linkMessage = m.device_link_failed();
                state.linkFailure = true;
            }
        }, seconds * 1000);
    }

    function stopLink() {
        generation++;
        if (pollTimer) clearTimeout(pollTimer);
        pollTimer = undefined;
        state.linkActive = false;
        state.linkCode = null;
        if (isTauri()) void invoke("device_link_cancel").catch(() => {});
    }

    onSettled(() => stopLink);

    async function signIn() {
        if (state.openingSignIn || state.linkActive) return;
        state.openingSignIn = true;
        state.linkFailure = false;
        state.linkMessage = null;
        try {
            if (!isTauri()) {
                await openAccountPage("sign-in");
                return;
            }
            const info = await invoke<Start>("device_link_start");
            state.linkCode = info.userCode;
            state.linkActive = true;
            const cycle = ++generation;
            try {
                await openUrl(info.verificationUri);
            } catch {
                state.linkMessage = m.device_link_open_browser_failed({
                    url: info.verificationUri,
                });
            }
            queuePoll(info.interval, cycle);
        } catch {
            state.linkMessage = m.device_link_failed();
            state.linkFailure = true;
        } finally {
            state.openingSignIn = false;
        }
    }

    return (
        <section class="settings-section ace-settings-license">
            <header>
                <span class="section-mark" aria-hidden="true">
                    <KeyRound size={16} />
                </span>
                <div>
                    <h2>{m.license()}</h2>
                    <p>{m.settings_license_description()}</p>
                </div>
            </header>

            <div class={userStore.isPro ? "license-row pro" : "license-row"}>
                <span class="plan-mark" aria-hidden="true">
                    {userStore.isPro ? (
                        <>
                            <CircleCheck size={18} />
                        </>
                    ) : (
                        <>
                            <Crown size={18} />
                        </>
                    )}
                </span>
                <div class="plan-copy">
                    <h3>
                        {userStore.isPro
                            ? m.pro_status_active()
                            : m.pro_status_inactive()}
                    </h3>
                    <span>
                        {userStore.isPro
                            ? m.pro_unlock_thanks()
                            : m.settings_license_upgrade_description()}
                    </span>
                </div>

                {!userStore.isPro ? (
                    <>
                        <div class="license-actions">
                            <UpgradeToProButton
                            />
                            <button
                                type="button"
                                class="sign-in-action"
                                disabled={
                                    state.openingSignIn || state.linkActive
                                }
                                onClick={signIn}
                            >
                                {m.device_link_start()}
                            </button>
                        </div>
                    </>
                ) : null}
            </div>
            {state.linkActive || state.linkMessage ? (
                <>
                    <div
                        class="device-link-panel"
                        role="status"
                        aria-live="polite"
                    >
                        {state.linkCode ? (
                            <>
                                <p class="font-semibold">
                                    {m.device_link_title()}
                                </p>
                                <p class="mt-1">
                                    {m.device_link_instructions()}
                                </p>
                                <p
                                    class="device-link-code"
                                    aria-label={m.device_link_code()}
                                >
                                    {state.linkCode}
                                </p>
                                <p>{m.device_link_waiting()}</p>
                                <button
                                    type="button"
                                    class="sign-in-action"
                                    onClick={stopLink}
                                >
                                    {m.device_link_cancel()}
                                </button>
                            </>
                        ) : state.linkMessage ? (
                            <>
                                <p
                                    class={
                                        state.linkFailure
                                            ? "device-link-error"
                                            : undefined
                                    }
                                >
                                    {state.linkMessage}
                                </p>
                            </>
                        ) : null}
                    </div>
                </>
            ) : null}
        </section>
    );
}
