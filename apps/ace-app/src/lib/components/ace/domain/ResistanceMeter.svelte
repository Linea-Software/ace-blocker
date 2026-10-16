<script lang="ts">
    import type { PunishmentIntensity } from "$engine-client";
    import * as m from "$lib/paraglide/messages.js";

    export type ResistanceLevel = Exclude<PunishmentIntensity, "none">;

    let {
        enabled,
        currentIntensity,
        locked = false,
        disabled = false,
        proRequired = false,
        inFlight = false,
        onSelect,
    }: {
        enabled: boolean;
        currentIntensity: PunishmentIntensity;
        locked?: boolean;
        disabled?: boolean;
        proRequired?: boolean;
        inFlight?: boolean;
        onSelect: (level: ResistanceLevel) => void | Promise<void>;
    } = $props();

    const levels = [
        {
            value: "low",
            label: m.intensity_low,
            description: m.intensity_low_beta_desc,
        },
        {
            value: "medium",
            label: m.intensity_medium,
            description: m.intensity_medium_beta_desc,
        },
        {
            value: "high",
            label: m.intensity_high,
            description: m.intensity_high_beta_desc,
        },
        {
            value: "extreme",
            label: m.intensity_maximum,
            description: m.intensity_extreme_beta_desc,
        },
    ] as const satisfies ReadonlyArray<{
        value: ResistanceLevel;
        label: () => string;
        description: () => string;
    }>;

    const selected = $derived<ResistanceLevel | null>(
        enabled && currentIntensity !== "none"
            ? (currentIntensity as ResistanceLevel)
            : null,
    );

    const selectedItem = $derived(
        selected ? (levels.find((level) => level.value === selected) ?? null) : null,
    );

    function select(level: ResistanceLevel) {
        if (disabled || inFlight || locked || level === selected) return;
        void onSelect(level);
    }
</script>

<section
    class={["resistance-meter", locked && "locked", disabled && "disabled"]}
    aria-labelledby="resistance-meter-title"
>
    <div class="meter-heading">
        <h2 id="resistance-meter-title">{m.friction_intensity_title()}</h2>
        <p>{m.friction_intensity_description()}</p>
    </div>

    <div class="selector-shell">
        <div
            class="selector-plate"
            role="group"
            aria-label={m.friction_intensity_title()}
        >
            {#each levels as level (level.value)}
                {@const isSelected = selected === level.value}
                <button
                    type="button"
                    class={["selector-segment", isSelected && "selected"]}
                    aria-pressed={isSelected}
                    disabled={disabled || inFlight || locked}
                    data-testid={"rm-intensity-" + level.value}
                    onclick={() => select(level.value)}
                >
                    <span>{level.label()}</span>
                </button>
            {/each}
        </div>
    </div>

    <div class="effect-readout" aria-live="polite">
        {#if proRequired}
            <p>{m.resistance_intensity_pro_locked()}</p>
        {:else if selectedItem}
            <p>{selectedItem.description()}</p>
        {:else}
            <p>
                {enabled
                    ? m.intensity_none_warning()
                    : m.status_off_description()}
            </p>
        {/if}
    </div>
</section>

<style>
    .resistance-meter {
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 6px;
        background: var(--card);
    }

    .resistance-meter.locked {
        border-left: 6px solid var(--ace-signal);
        border-top-color: color-mix(in srgb, var(--ace-signal) 48%, var(--border));
        border-bottom-color: color-mix(in srgb, var(--ace-signal) 48%, var(--border));
    }

    .meter-heading {
        padding: 14px 18px 0;
    }

    h2 {
        margin: 0;
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 20px;
        font-weight: 600;
        line-height: 1;
        text-wrap: balance;
    }

    .meter-heading p {
        max-width: 640px;
        margin: 6px 0 0;
        color: var(--muted-foreground);
        font-size: 13px;
        line-height: 1.45;
    }

    .selector-shell {
        padding: 12px 18px 0;
    }

    .selector-plate {
        display: grid;
        grid-template-columns: repeat(4, minmax(0, 1fr));
        overflow: hidden;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: color-mix(in srgb, var(--muted) 38%, var(--card));
    }

    .selector-segment {
        position: relative;
        min-width: 0;
        height: 60px;
        display: flex;
        align-items: center;
        justify-content: center;
        border: 0;
        border-left: 1px solid var(--border);
        border-radius: 0;
        padding: 0 10px;
        background: transparent;
        color: var(--muted-foreground);
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 15px;
        font-weight: 600;
        line-height: 1;
        cursor: pointer;
        text-align: center;
        touch-action: manipulation;
        transition:
            background 140ms ease,
            color 140ms ease;
    }

    .selector-segment:first-child {
        border-left: 0;
    }

    .selector-segment:hover:not(:disabled):not(.selected) {
        background: color-mix(in srgb, var(--foreground) 5%, transparent);
        color: var(--foreground);
    }

    .selector-segment:focus-visible {
        outline: 2px solid var(--foreground);
        outline-offset: -4px;
        z-index: 1;
    }

    .selector-segment:disabled {
        cursor: not-allowed;
    }

    .selector-segment.selected {
        background: color-mix(in srgb, var(--foreground) 9%, var(--card));
        color: var(--foreground);
        font-weight: 700;
    }

    .selector-segment.selected::after {
        position: absolute;
        right: 0;
        bottom: 0;
        left: 0;
        height: 4px;
        background: var(--ace-signal);
        content: "";
    }

    .resistance-meter.locked .selector-plate {
        opacity: 0.76;
    }

    .resistance-meter.disabled:not(.locked) .selector-plate {
        opacity: 0.64;
    }

    .effect-readout {
        min-height: 38px;
        display: flex;
        align-items: center;
        padding: 9px 18px 13px;
    }

    .effect-readout p {
        margin: 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.45;
    }

    @media (prefers-reduced-motion: reduce) {
        .selector-segment {
            transition: none;
        }
    }

    @container page (max-width: 420px) {
        .meter-heading,
        .effect-readout {
            padding-right: 14px;
            padding-left: 14px;
        }

        .selector-shell {
            padding-right: 14px;
            padding-left: 14px;
        }

        .selector-plate {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }

        .selector-segment:nth-child(3) {
            border-left: 0;
        }

        .selector-segment:nth-child(n + 3) {
            border-top: 1px solid var(--border);
        }
    }
</style>
