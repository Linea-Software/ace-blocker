<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { userStore } from "$lib/stores/user-store.svelte";
    import { KeyRound, CircleCheck, Crown } from "@lucide/svelte";
    import UpgradeToProButton from "$lib/components/UpgradeToProButton.svelte";
    import { openAccountPage } from "$lib/utils/account-browser";

    const isPro = $derived(userStore.isPro);
    let openingSignIn = $state(false);

    async function signIn() {
        if (openingSignIn) return;
        openingSignIn = true;
        try {
            await openAccountPage("sign-in");
        } finally {
            openingSignIn = false;
        }
    }
</script>

<section class="settings-section">
    <header>
        <span class="section-mark" aria-hidden="true"><KeyRound size={16} /></span>
        <div>
            <h2>{m.license()}</h2>
            <p>{m.settings_license_description()}</p>
        </div>
    </header>

    <div class:pro={isPro} class="license-row">
        <span class="plan-mark" aria-hidden="true">
            {#if isPro}
                <CircleCheck size={18} />
            {:else}
                <Crown size={18} />
            {/if}
        </span>
        <div class="plan-copy">
            <h3>
                {isPro ? m.pro_status_active() : m.pro_status_inactive()}
            </h3>
            <span>
                {isPro
                    ? m.pro_unlock_thanks()
                    : m.settings_license_upgrade_description()}
            </span>
        </div>

        {#if !isPro}
            <div class="license-actions">
                <UpgradeToProButton text={m.pro_status_inactive()} />
                <button
                    type="button"
                    class="sign-in-action"
                    disabled={openingSignIn}
                    onclick={signIn}
                >
                    {m.already_purchased()}
                </button>
            </div>
        {/if}
    </div>
</section>

<style>
    .settings-section {
        display: grid;
        gap: 12px;
    }

    header {
        display: grid;
        grid-template-columns: 28px minmax(0, 1fr);
        gap: 8px;
        align-items: start;
    }

    .section-mark {
        width: 24px;
        height: 24px;
        display: grid;
        place-items: center;
        border-left: 3px solid var(--ace-ash);
        color: var(--muted-foreground);
    }

    h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 18px;
        font-weight: 600;
    }

    header p {
        margin: 3px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .license-row {
        min-height: 78px;
        display: grid;
        grid-template-columns: 38px minmax(0, 1fr) auto;
        align-items: center;
        gap: 12px;
        padding: 13px 14px;
        border: 1px solid var(--border);
        border-left: 5px solid var(--ace-ash);
        border-radius: 5px;
        background: var(--card);
    }

    .license-row.pro {
        border-left-color: var(--ace-success);
    }

    .plan-mark {
        width: 34px;
        height: 34px;
        display: grid;
        place-items: center;
        border: 1px solid var(--border);
        border-radius: 4px;
        color: var(--muted-foreground);
    }

    .pro .plan-mark {
        color: var(--ace-success);
    }

    .plan-copy {
        min-width: 0;
    }

    .plan-copy h3,
    .plan-copy span {
        display: block;
    }

    .plan-copy h3 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 600;
    }

    .plan-copy span {
        margin-top: 3px;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .license-actions {
        min-width: 0;
        display: grid;
        justify-items: start;
        gap: 7px;
    }

    .sign-in-action {
        padding: 4px 0;
        border: 0;
        background: transparent;
        color: var(--muted-foreground);
        font-size: 12px;
        text-align: left;
        text-decoration: underline;
        text-underline-offset: 3px;
        cursor: pointer;
    }

    .sign-in-action:hover {
        color: var(--foreground);
    }

    .sign-in-action:disabled {
        opacity: 0.5;
        cursor: wait;
    }

    @container page (max-width: 700px) {
        .license-row {
            grid-template-columns: 38px minmax(0, 1fr);
        }

        .license-actions {
            grid-column: 2;
        }
    }
</style>
