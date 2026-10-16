<script lang="ts">
    import "../app.css";
    import { ModeWatcher } from "mode-watcher";
    import { Toaster } from "$lib/components/ui/sonner";
    import TosModal from "$lib/components/TosModal.svelte";
    import OnboardingWizard from "$lib/components/onboarding/OnboardingWizard.svelte";
    import TutorialTour from "$lib/components/onboarding/TutorialTour.svelte";
    import NavigationRail from "$lib/components/ace/structural/NavigationRail.svelte";
    import StatusStrip from "$lib/components/ace/structural/StatusStrip.svelte";

    import { client } from "$engine-client/client.gen";
    import { engineStore } from "$lib/stores/engine.svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { onboardingStore } from "$lib/stores/onboarding.svelte";
    import { dev } from "$app/environment";

    let { children } = $props();

    const legalGateLoading = $derived(
        !engineStore.isReady ||
            (engineStore.isAlive && !userStore.accountInfoLoaded),
    );

    const showTosModal = $derived(
        engineStore.isAlive &&
            userStore.accountInfoLoaded &&
            !userStore.tosAccepted,
    );

    const legalGateActive = $derived(legalGateLoading || showTosModal);

    const showOnboardingWizard = $derived(
        engineStore.isAlive &&
            userStore.accountInfoLoaded &&
            userStore.tosAccepted &&
            onboardingStore.loaded &&
            !onboardingStore.wizardCompleted &&
            !onboardingStore.tourRunning,
    );

    const showTutorialTour = $derived(
        !legalGateActive &&
            onboardingStore.loaded &&
            onboardingStore.tourRunning &&
            !onboardingStore.tourCompleted,
    );

    // Interceptors for debugging — dev builds only, never in production.
    if (dev) {
        client.interceptors.request.use((request, options) => {
            console.log(`[API Request] ${request.method} ${request.url}`);
            console.log(` - [REQUEST OPTIONS] ${JSON.stringify(options)}`);
            return request;
        });

        client.interceptors.response.use(async (response) => {
            const clone = response.clone();
            try {
                const text = await clone.text();
                let data;
                try {
                    data = JSON.parse(text);
                } catch {
                    data = text;
                }
                console.log(
                    `[API Response] ${response.status} ${response.url}`,
                    data,
                );
            } catch {
                console.log(
                    `[API Response] ${response.status} ${response.url} (Failed to read body)`,
                );
            }
            return response;
        });
    }
</script>

<ModeWatcher />
<Toaster />

{#if legalGateActive}
    <TosModal loading={legalGateLoading} />
{:else if showOnboardingWizard}
    <OnboardingWizard />
{/if}

{#if showTutorialTour}
    <TutorialTour />
{/if}

<div
    class="app-shell"
    inert={legalGateActive}
    aria-hidden={legalGateActive ? "true" : undefined}
>
    <NavigationRail />
    <div class="workspace-column">
        <StatusStrip />
        <main id="ace-workspace" class="workspace ace-scrollbar">
            {@render children?.()}
        </main>
    </div>
</div>

<style>
    .app-shell {
        display: flex;
        width: 100%;
        height: 100dvh;
        overflow: hidden;
        background: var(--background);
        color: var(--foreground);
    }

    .workspace-column {
        min-width: 0;
        flex: 1;
        display: flex;
        flex-direction: column;
        overflow: hidden;
    }

    .workspace {
        min-width: 0;
        min-height: 0;
        flex: 1;
        overflow: auto;
        scrollbar-gutter: stable;
        container-type: inline-size;
        container-name: ace-workspace;
    }
</style>
