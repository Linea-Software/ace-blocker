<script lang="ts">
    import "../app.css";
    import { onMount } from "svelte";
    import { initializeAppLanguage } from "$lib/utils/app-language";
    import { ModeWatcher } from "mode-watcher";
    import { Toaster } from "$lib/components/ui/sonner";
    import TosModal from "$lib/components/TosModal.svelte";
    import OnboardingWizard from "$lib/components/onboarding/OnboardingWizard.svelte";
    import TutorialTour from "$lib/components/onboarding/TutorialTour.svelte";
    import NavigationRail from "$lib/components/ace/structural/NavigationRail.svelte";
    import StatusStrip from "$lib/components/ace/structural/StatusStrip.svelte";

    import { engineStore } from "$lib/stores/engine.svelte";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { onboardingStore } from "$lib/stores/onboarding.svelte";

    let { children } = $props();

    onMount(() => {
        void initializeAppLanguage();
    });

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

</script>

<ModeWatcher />
<Toaster />

{#if legalGateActive}
    <TosModal loading={legalGateLoading} loadFailed={userStore.accountLoadStatus === "error"} />
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
