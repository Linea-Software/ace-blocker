import { createSignal, Show, onSettled } from "solid-js";
import { ExternalLink } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import { acceptTos } from "$engine-client";
import { userStore } from "$lib/stores/user-store";
import { toast } from "solid-sonner";
import { openUrl } from "@tauri-apps/plugin-opener";

export default function TosModal(props: {
    loading?: boolean;
    loadFailed?: boolean;
}) {
    const [termsAccepted, setTermsAccepted] = createSignal(false);
    const [privacyAccepted, setPrivacyAccepted] = createSignal(false);
    const [isSubmitting, setIsSubmitting] = createSignal(false);
    const canAccept = () =>
        !props.loading &&
        termsAccepted() &&
        privacyAccepted() &&
        !isSubmitting();
    let dialog: HTMLDivElement | undefined;
    onSettled(() => {
        const previousFocus =
            document.activeElement instanceof HTMLElement
                ? document.activeElement
                : null;
        dialog?.focus();
        const trapKeys = (e: KeyboardEvent) => {
            if (e.key === "Escape") {
                e.preventDefault();
                e.stopPropagation();
            }
            if (e.key === "Tab" && dialog) {
                const focusables = Array.from(
                    dialog.querySelectorAll<HTMLElement>(
                        'button:not([disabled]), input:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])',
                    ),
                );
                if (!focusables.length) {
                    e.preventDefault();
                    dialog.focus();
                    return;
                }
                const first = focusables[0],
                    last = focusables[focusables.length - 1];
                if (e.shiftKey && document.activeElement === first) {
                    e.preventDefault();
                    last.focus();
                } else if (!e.shiftKey && document.activeElement === last) {
                    e.preventDefault();
                    first.focus();
                }
            }
        };
        document.addEventListener("keydown", trapKeys, true);
        return () => {
            document.removeEventListener("keydown", trapKeys, true);
            if (previousFocus?.isConnected) previousFocus.focus();
        };
    });
    async function handleAccept() {
        if (!canAccept()) return;
        setIsSubmitting(true);
        try {
            const { error } = await acceptTos();
            if (error) {
                console.error("Failed to accept TOS:", error);
                toast.error(m.failed_to_accept_tos());
                return;
            }
            userStore.tosAccepted = true;
        } catch (err) {
            console.error("Failed to accept TOS:", err);
            toast.error(m.failed_to_accept_tos());
        } finally {
            setIsSubmitting(false);
        }
    }
    const link =
        "inline-flex items-center justify-center gap-2 rounded-md border border-input bg-background px-3 py-1.5 text-sm font-medium hover:bg-accent hover:text-accent-foreground";
    return (
        <div
            class="fixed inset-0 z-[100] flex items-center justify-center bg-black/80 p-4"
            role="presentation"
        >
            <div
                ref={dialog}
                tabindex={-1}
                role="dialog"
                aria-modal="true"
                aria-labelledby="tos-title"
                aria-describedby="tos-description"
                class="w-[95vw] max-w-lg gap-0 overflow-hidden rounded-lg border border-border bg-card p-0 shadow-2xl"
            >
                <div class="border-b border-border bg-muted p-6 pb-4">
                    <div class="flex items-center gap-3">
                        <div
                            class="shrink-0"
                            style={{
                                width: "88px",
                                height: "88px",
                                "border-radius": "20px",
                            }}
                        >
                            <img
                                src="/ace-icon-light-transparent.svg"
                                alt={m.legal_gate_logo_alt()}
                                class="h-full w-full dark:hidden"
                            />
                            <img
                                src="/ace-icon-dark-transparent.svg"
                                alt={m.legal_gate_logo_alt()}
                                class="hidden h-full w-full dark:block"
                            />
                        </div>
                        <div>
                            <h2
                                id="tos-title"
                                class="text-xl font-bold tracking-tight"
                            >
                                {props.loading
                                    ? m.legal_gate_checking_title()
                                    : m.legal_gate_title()}
                            </h2>
                            <p
                                id="tos-description"
                                class="text-sm text-muted-foreground"
                            >
                                {props.loading
                                    ? m.legal_gate_checking_description()
                                    : m.legal_gate_description()}
                            </p>
                        </div>
                    </div>
                </div>
                <Show
                    when={!props.loading}
                    fallback={
                        <div
                            class="p-6 text-sm leading-relaxed text-muted-foreground"
                            role="status"
                            aria-live="polite"
                        >
                            <Show
                                when={props.loadFailed}
                                fallback={m.legal_gate_checking_body()}
                            >
                                <div role="alert" class="space-y-3">
                                    <p>{m.account_info_load_failed()}</p>
                                    <button
                                        type="button"
                                        class={link}
                                        onClick={() =>
                                            void userStore.retryAccountInfo()
                                        }
                                    >
                                        {m.retry()}
                                    </button>
                                </div>
                            </Show>
                        </div>
                    }
                >
                    <div class="max-h-[50vh] space-y-6 overflow-y-auto p-6">
                        <div class="space-y-4 text-sm leading-relaxed text-muted-foreground">
                            <p>{m.legal_gate_intro()}</p>
                            <div class="space-y-2">
                                <h3 class="font-semibold text-foreground">
                                    {m.legal_gate_terms_summary_title()}
                                </h3>
                                <ul class="list-disc space-y-1 pl-5">
                                    <li>
                                        {m.legal_gate_terms_responsibility()}
                                    </li>
                                    <li>{m.legal_gate_terms_billing()}</li>
                                    <li>
                                        {m.legal_gate_terms_reverse_engineering()}
                                    </li>
                                    <li>{m.legal_gate_terms_as_is()}</li>
                                </ul>
                            </div>
                            <div class="space-y-2">
                                <h3 class="font-semibold text-foreground">
                                    {m.legal_gate_privacy_summary_title()}
                                </h3>
                                <ul class="list-disc space-y-1 pl-5">
                                    <li>{m.legal_gate_privacy_no_history()}</li>
                                    <li>{m.legal_gate_privacy_no_sale()}</li>
                                    <li>{m.legal_gate_privacy_minimal()}</li>
                                </ul>
                            </div>
                        </div>
                        <div class="flex flex-wrap items-center gap-3">
                            <button
                                type="button"
                                class={link}
                                onClick={() =>
                                    void openUrl("https://aceblocker.com/terms")
                                }
                            >
                                {m.legal_gate_terms_link()}{" "}
                                <ExternalLink class="h-3.5 w-3.5" />
                            </button>
                            <button
                                type="button"
                                class={link}
                                onClick={() =>
                                    void openUrl(
                                        "https://aceblocker.com/privacy",
                                    )
                                }
                            >
                                {m.legal_gate_privacy_link()}{" "}
                                <ExternalLink class="h-3.5 w-3.5" />
                            </button>
                        </div>
                    </div>
                    <div class="flex flex-col gap-4 border-t border-border bg-muted p-6 pt-4">
                        <label class="flex cursor-pointer select-none items-start gap-3">
                            <input
                                type="checkbox"
                                class="mt-0.5 size-4 accent-primary"
                                checked={termsAccepted()}
                                disabled={isSubmitting()}
                                onChange={(e) =>
                                    setTermsAccepted(e.currentTarget.checked)
                                }
                            />
                            <span class="text-sm leading-relaxed">
                                {m.legal_gate_accept_terms()}
                            </span>
                        </label>
                        <label class="flex cursor-pointer select-none items-start gap-3">
                            <input
                                type="checkbox"
                                class="mt-0.5 size-4 accent-primary"
                                checked={privacyAccepted()}
                                disabled={isSubmitting()}
                                onChange={(e) =>
                                    setPrivacyAccepted(e.currentTarget.checked)
                                }
                            />
                            <span class="text-sm leading-relaxed">
                                {m.legal_gate_accept_privacy()}
                            </span>
                        </label>
                        <button
                            type="button"
                            class="h-12 w-full rounded-md bg-primary text-primary-foreground text-base font-bold disabled:opacity-50"
                            disabled={!canAccept()}
                            onClick={() => void handleAccept()}
                        >
                            {isSubmitting()
                                ? m.legal_gate_accepting()
                                : m.legal_gate_continue()}
                        </button>
                    </div>
                </Show>
            </div>
        </div>
    );
}
