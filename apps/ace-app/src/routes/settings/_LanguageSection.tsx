import { reactiveFields } from "$lib/stores/reactive-fields";
import { Languages } from "@lucide/solid";
import * as m from "$lib/paraglide/messages.js";
import { localizationStore } from "$lib/stores/localization";
import { changeAppLanguage } from "$lib/utils/app-language";
import { isLanguagePreference } from "$lib/utils/language-preference";
import "./_LanguageSection.css";

export default function LanguageSection() {
    const state = { error: false, busy: false };
    reactiveFields(state, ["error", "busy"]);
    async function changeLanguage(value: string) {
        if (!isLanguagePreference(value) || state.busy) return;
        state.error = false;
        state.busy = true;
        try {
            await changeAppLanguage(value);
        } catch (cause) {
            console.error("Failed to save app language:", cause);
            state.error = true;
        } finally {
            state.busy = false;
        }
    }
    return (
        <section class="settings-section ace-settings-language">
            <header>
                <span class="section-mark" aria-hidden="true">
                    <Languages size={16} />
                </span>
                <div>
                    <h2>{m.settings_language_title()}</h2>
                    <p>{m.settings_language_description()}</p>
                </div>
            </header>
            <div class="setting-row">
                <label for="app-language">{m.settings_language_label()}</label>
                <select
                    id="app-language"
                    value={localizationStore.language}
                    disabled={state.busy}
                    onChange={(event) =>
                        void changeLanguage(event.currentTarget.value)
                    }
                >
                    <option value="system">
                        {m.settings_language_system()}
                    </option>
                    <option value="en">English</option>
                    <option value="it">Italiano</option>
                    <option value="de">Deutsch</option>
                    <option value="es">Español</option>
                </select>
            </div>
            {state.error ? (
                <>
                    <p class="error" role="alert">
                        {m.settings_language_save_failed()}
                    </p>
                </>
            ) : null}
        </section>
    );
}
