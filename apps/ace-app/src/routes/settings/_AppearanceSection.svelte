<script lang="ts">
    import * as m from "$lib/paraglide/messages.js";
    import { Sun, Moon, Monitor, Palette } from "@lucide/svelte";

    let {
        selectedTheme,
        onThemeChange,
    }: {
        selectedTheme: string;
        onThemeChange: (val: string) => void;
    } = $props();

    const themeOptions = [
        { value: "light", label: m.light_mode(), icon: Sun },
        { value: "dark", label: m.dark_mode(), icon: Moon },
        { value: "system", label: m.system_mode(), icon: Monitor },
    ] as const;
</script>

<section class="settings-section">
    <header>
        <span class="section-mark" aria-hidden="true"><Palette size={16} /></span>
        <div>
            <h2>{m.appearance()}</h2>
            <p>{m.appearance_description()}</p>
        </div>
    </header>

    <div class="setting-row">
        <div>
            <strong>{m.theme()}</strong>
            <p>{m.theme_description()}</p>
        </div>

        <div class="theme-options" role="group" aria-label={m.theme()}>
            {#each themeOptions as option (option.value)}
                <button
                    type="button"
                    class:active={selectedTheme === option.value}
                    aria-pressed={selectedTheme === option.value}
                    onclick={() => onThemeChange(option.value)}
                >
                    <option.icon size={15} aria-hidden="true" />
                    <span>{option.label}</span>
                </button>
            {/each}
        </div>
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

    header p,
    .setting-row p {
        margin: 3px 0 0;
        color: var(--muted-foreground);
        font-size: 12px;
        line-height: 1.4;
    }

    .setting-row {
        min-height: 72px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 24px;
        padding: 13px 14px;
        border: 1px solid var(--border);
        border-radius: 5px;
        background: var(--card);
    }

    .setting-row strong {
        font-family: "IBM Plex Sans Condensed", sans-serif;
        font-size: 14px;
        font-weight: 600;
    }

    .theme-options {
        display: grid;
        grid-template-columns: repeat(3, auto);
        border: 1px solid var(--border);
        border-radius: 4px;
        overflow: hidden;
    }

    :global(.dark) .theme-options {
        border-color: var(--input);
    }

    .theme-options button {
        min-height: 38px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 6px;
        border: 0;
        border-right: 1px solid var(--border);
        padding: 0 10px;
        background: transparent;
        color: var(--muted-foreground);
        font-size: 12px;
        font-weight: 600;
        cursor: pointer;
    }

    .theme-options button:last-child {
        border-right: 0;
    }

    .theme-options button:hover {
        background: var(--muted);
        color: var(--foreground);
    }

    .theme-options button.active {
        background: var(--foreground);
        color: var(--background);
    }

    @container page (max-width: 560px) {
        .setting-row {
            align-items: stretch;
            flex-direction: column;
        }

        .theme-options {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }
    }
</style>
