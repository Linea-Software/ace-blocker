import { For } from "solid-js";
import { Dynamic } from "@solidjs/web";
import * as m from "$lib/paraglide/messages.js";
import { Sun, Moon, Monitor, Palette } from "@lucide/solid";
import "./_AppearanceSection.css";

export default function AppearanceSection(props: { selectedTheme: string; onThemeChange: (val: string) => void }) {
    const themeOptions = [
        { value: "light", label: m.light_mode(), icon: Sun },
        { value: "dark", label: m.dark_mode(), icon: Moon },
        { value: "system", label: m.system_mode(), icon: Monitor },
    ] as const;
    return (
        <section class="settings-section ace-settings-appearance">
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
            <For each={themeOptions}>{(option) => (<>
                <button
                    type="button"
                    class={props.selectedTheme === option.value ? "active" : ""}
                    aria-pressed={props.selectedTheme === option.value ? 'true' : 'false'}
                    onClick={() => props.onThemeChange(option.value)}
                >
                    <Dynamic component={option.icon} size={15} aria-hidden="true" />
                    <span>{option.label}</span>
                </button>
            </>)}</For>
        </div>
    </div>
</section>
    );
}
