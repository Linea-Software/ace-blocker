<script lang="ts">
    import * as Dialog from "$lib/components/ui/dialog/index.js";
    import { Button } from "$lib/components/ui/button/index.js";
    import { Checkbox } from "$lib/components/ui/checkbox/index.js";
    import { ExternalLink } from "@lucide/svelte";
    import * as m from "$lib/paraglide/messages.js";
    import { acceptTos } from "$engine-client";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { toast } from "svelte-sonner";
    import { openUrl } from "@tauri-apps/plugin-opener";

    let { loading = false, loadFailed = false }: { loading?: boolean; loadFailed?: boolean } = $props();

    let termsAccepted = $state(false);
    let privacyAccepted = $state(false);
    let isSubmitting = $state(false);

    const canAccept = $derived(
        !loading && termsAccepted && privacyAccepted && !isSubmitting,
    );

    async function handleAccept() {
        if (!canAccept) return;

        isSubmitting = true;
        try {
            // The SDK never throws on HTTP errors: check the error result
            // instead of assuming success.
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
            isSubmitting = false;
        }
    }

    function openTerms() {
        void openUrl("https://aceblocker.com/terms");
    }

    function openPrivacy() {
        void openUrl("https://aceblocker.com/privacy");
    }
</script>

<Dialog.Root open={true}>
    <Dialog.Content
        showCloseButton={false}
        escapeKeydownBehavior="ignore"
        interactOutsideBehavior="ignore"
        class="w-[95vw] max-w-lg gap-0 overflow-hidden rounded-lg bg-card p-0 shadow-2xl"
    >
        <div class="border-b border-border bg-muted p-6 pb-4">
            <div class="flex items-center gap-3">
                <div
                    class="shrink-0"
                    style="width: 88px; height: 88px; border-radius: 20px;"
                >
                    <img
                        src="/ace-icon-light.svg"
                        alt={m.legal_gate_logo_alt()}
                        class="h-full w-full"
                        style="border-radius: 20px;"
                    />
                </div>
                <div>
                    <Dialog.Title class="text-xl font-bold tracking-tight">
                        {loading
                            ? m.legal_gate_checking_title()
                            : m.legal_gate_title()}
                    </Dialog.Title>
                    <Dialog.Description class="text-sm text-muted-foreground">
                        {loading
                            ? m.legal_gate_checking_description()
                            : m.legal_gate_description()}
                    </Dialog.Description>
                </div>
            </div>
        </div>

        {#if loading}
            <div
                class="p-6 text-sm leading-relaxed text-muted-foreground"
                role="status"
                aria-live="polite"
            >
                {#if loadFailed}
                    <div role="alert" class="space-y-3">
                        <p>{m.account_info_load_failed()}</p>
                        <Button variant="outline" onclick={() => void userStore.retryAccountInfo()}>
                            {m.retry()}
                        </Button>
                    </div>
                {:else}
                    {m.legal_gate_checking_body()}
                {/if}
            </div>
        {:else}
            <div class="max-h-[50vh] space-y-6 overflow-y-auto p-6">
                <div
                    class="space-y-4 text-sm leading-relaxed text-muted-foreground"
                >
                    <p>{m.legal_gate_intro()}</p>

                    <div class="space-y-2">
                        <h3 class="font-semibold text-foreground">
                            {m.legal_gate_terms_summary_title()}
                        </h3>
                        <ul class="list-disc space-y-1 pl-5">
                            <li>{m.legal_gate_terms_responsibility()}</li>
                            <li>{m.legal_gate_terms_billing()}</li>
                            <li>{m.legal_gate_terms_reverse_engineering()}</li>
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
                    <Button
                        variant="outline"
                        size="sm"
                        class="gap-2"
                        onclick={openTerms}
                    >
                        {m.legal_gate_terms_link()}
                        <ExternalLink class="h-3.5 w-3.5" />
                    </Button>
                    <Button
                        variant="outline"
                        size="sm"
                        class="gap-2"
                        onclick={openPrivacy}
                    >
                        {m.legal_gate_privacy_link()}
                        <ExternalLink class="h-3.5 w-3.5" />
                    </Button>
                </div>
            </div>

            <div
                class="flex flex-col gap-4 border-t border-border bg-muted p-6 pt-4"
            >
                <label class="flex cursor-pointer select-none items-start gap-3">
                    <Checkbox
                        bind:checked={termsAccepted}
                        disabled={isSubmitting}
                        class="mt-0.5"
                    />
                    <span class="text-sm leading-relaxed">
                        {m.legal_gate_accept_terms()}
                    </span>
                </label>

                <label class="flex cursor-pointer select-none items-start gap-3">
                    <Checkbox
                        bind:checked={privacyAccepted}
                        disabled={isSubmitting}
                        class="mt-0.5"
                    />
                    <span class="text-sm leading-relaxed">
                        {m.legal_gate_accept_privacy()}
                    </span>
                </label>

                <Button
                    class="h-12 w-full rounded-md text-base font-bold"
                    disabled={!canAccept}
                    onclick={handleAccept}
                >
                    {isSubmitting
                        ? m.legal_gate_accepting()
                        : m.legal_gate_continue()}
                </Button>
            </div>
        {/if}
    </Dialog.Content>
</Dialog.Root>
