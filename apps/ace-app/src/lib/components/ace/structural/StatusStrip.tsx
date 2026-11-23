import "./StatusStrip.css";
import { Show } from "solid-js";
import * as m from "$lib/paraglide/messages.js";
import { engineStore } from "$lib/stores/engine";
import { Info, TriangleAlert } from "@lucide/solid";

export default function StatusStrip() {
    return (
        <>
        <Show when={!engineStore.isTauri && engineStore.isReady}>
            <div class="status-strip status-strip--demo" role="status">
                <Info size={16} aria-hidden="true" />
                <strong>{m.status_browser_demo()}</strong>
                <span>{m.status_browser_demo_description()}</span>
            </div>
        </Show>
        <Show
            when={
                engineStore.isTauri &&
                engineStore.isReady &&
                !engineStore.isAlive
            }
        >
            <div class="status-strip" role="alert">
                <TriangleAlert size={16} aria-hidden="true" />
                <strong>{m.status_engine_offline()}</strong>
                <span>{m.status_engine_offline_description()}</span>
            </div>
        </Show>
        </>
    );
}
