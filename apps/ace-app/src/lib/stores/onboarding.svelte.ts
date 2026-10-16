/**
 * ONBOARDING STORE
 *
 * Tracks whether the user has been through the first-run experience:
 * the setup wizard and the guided tour.
 *
 * This is purely presentational state for this installation of the app, so it
 * lives in `localStorage` rather than in the engine. Anything the wizard
 * actually *configures* (activated blocks, resistance mode) is persisted by the
 * engine through its normal API.
 */

const STORAGE_KEY = "ace:onboarding:v1";

type PersistedOnboarding = {
    wizardCompleted: boolean;
    tourCompleted: boolean;
    wizardPreviouslyCompleted?: boolean;
};

const DEFAULT_STATE: PersistedOnboarding = {
    wizardCompleted: false,
    tourCompleted: false,
    wizardPreviouslyCompleted: false,
};

class OnboardingStore {
    /** Whether the setup wizard has been completed or explicitly skipped. */
    wizardCompleted = $state(true);
    /** Whether the guided tour has been finished or explicitly skipped. */
    tourCompleted = $state(true);
    /** Whether this is a repeat setup, even after restart/reload. */
    wizardPreviouslyCompleted = $state(false);
    /** True once the persisted state has been read. Guards against showing the
     *  wizard for a frame before we know whether it was already completed. */
    loaded = $state(false);

    /** Set while the tour is running so the wizard stays out of the way. */
    tourRunning = $state(false);

    constructor() {
        this.load();
    }

    private load() {
        if (typeof localStorage === "undefined") return;

        try {
            const raw = localStorage.getItem(STORAGE_KEY);
            const parsed: PersistedOnboarding = raw
                ? { ...DEFAULT_STATE, ...JSON.parse(raw) }
                : DEFAULT_STATE;

            this.wizardCompleted = parsed.wizardCompleted === true;
            this.wizardPreviouslyCompleted =
                parsed.wizardPreviouslyCompleted === true || parsed.wizardCompleted === true;
            this.tourCompleted = parsed.tourCompleted === true;
        } catch (error) {
            console.error("Failed to read onboarding state:", error);
            this.wizardCompleted = false;
            this.tourCompleted = false;
        } finally {
            this.loaded = true;
        }
    }

    private persist() {
        if (typeof localStorage === "undefined") return;

        try {
            const payload: PersistedOnboarding = {
                wizardCompleted: this.wizardCompleted,
                tourCompleted: this.tourCompleted,
                wizardPreviouslyCompleted: this.wizardPreviouslyCompleted,
            };
            localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
        } catch (error) {
            console.error("Failed to persist onboarding state:", error);
        }
    }

    /** Marks the wizard as done. Used both by "Finish" and "Skip". */
    completeWizard() {
        this.wizardCompleted = true;
        this.wizardPreviouslyCompleted = true;
        this.persist();
    }

    /** Marks the tour as done. Used both by "Done" and "Skip tour". */
    completeTour() {
        this.tourCompleted = true;
        this.tourRunning = false;
        this.persist();
    }

    /** Re-opens the wizard from the getting started guide. */
    restartWizard() {
        this.wizardCompleted = false;
        this.wizardPreviouslyCompleted = true;
        this.persist();
    }

    /** Re-runs the guided tour from the getting started guide. */
    startTour() {
        this.tourCompleted = false;
        this.tourRunning = true;
        this.persist();
    }
}

export const onboardingStore = new OnboardingStore();
