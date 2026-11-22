import { onSettled, Show, type ParentProps } from "solid-js";
import { Toaster } from "solid-sonner";
import { CircleCheck, Info, Loader2, OctagonX, TriangleAlert } from "@lucide/solid";
import { initializeAppLanguage } from "$lib/utils/app-language";
import { initializeTheme, themeMode } from "$lib/utils/theme";
import { engineStore } from "$lib/stores/engine";
import { userStore } from "$lib/stores/user-store";
import { onboardingStore } from "$lib/stores/onboarding";
import TosModal from "$lib/components/TosModal";
import OnboardingWizard from "$lib/components/onboarding/OnboardingWizard";
import TutorialTour from "$lib/components/onboarding/TutorialTour";
import NavigationRail from "$lib/components/ace/structural/NavigationRail";
import StatusStrip from "$lib/components/ace/structural/StatusStrip";
import { Router } from "./router";

function AppShell(props: ParentProps) {
    onSettled(() => {
        void initializeAppLanguage();
        return initializeTheme();
    });

    const legalGateLoading = () =>
        !engineStore.isReady ||
        (engineStore.isAlive && !userStore.accountInfoLoaded);
    const showTosModal = () =>
        engineStore.isAlive &&
        userStore.accountInfoLoaded &&
        !userStore.tosAccepted;
    const legalGateActive = () => legalGateLoading() || showTosModal();
    const showOnboardingWizard = () =>
        engineStore.isAlive &&
        userStore.accountInfoLoaded &&
        userStore.tosAccepted &&
        onboardingStore.loaded &&
        !onboardingStore.wizardCompleted &&
        !onboardingStore.tourRunning;
    // The first-run wizard is a true modal: keyboard and pointer interaction
    // with the workspace must remain disabled until it is dismissed.
    const shellBlocked = () => legalGateActive() || showOnboardingWizard();
    const showTutorialTour = () =>
        !legalGateActive() &&
        onboardingStore.loaded &&
        onboardingStore.tourRunning &&
        !onboardingStore.tourCompleted;

    return (
        <>
            <Toaster
                icons={{
                    loading: <Loader2 class="size-4 animate-spin" />,
                    success: <CircleCheck class="size-4" />,
                    error: <OctagonX class="size-4" />,
                    info: <Info class="size-4" />,
                    warning: <TriangleAlert class="size-4" />,
                }}
                position="bottom-right"
                theme={themeMode()}
                class="toaster group"
                style={{
                    "--normal-bg": "var(--popover)",
                    "--normal-text": "var(--popover-foreground)",
                    "--normal-border": "var(--border)",
                }}
            />
            <Show
                when={legalGateActive()}
                fallback={
                    <Show when={showOnboardingWizard()}>
                        <OnboardingWizard />
                    </Show>
                }
            >
                <TosModal
                    loading={legalGateLoading()}
                    loadFailed={userStore.accountLoadStatus === "error"}
                />
            </Show>
            <Show when={showTutorialTour()}>
                <TutorialTour />
            </Show>
            <div
                class="app-shell"
                inert={shellBlocked()}
                aria-hidden={shellBlocked() ? "true" : undefined}
            >
                <NavigationRail />
                <div class="workspace-column">
                    <StatusStrip />
                    <main id="ace-workspace" class="workspace ace-scrollbar">
                        {props.children}
                    </main>
                </div>
            </div>
        </>
    );
}

export default function App() {
    return <Router>{(route) => <AppShell>{route.children}</AppShell>}</Router>;
}
