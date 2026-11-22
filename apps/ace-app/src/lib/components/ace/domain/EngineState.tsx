import { createMemo, onSettled } from "solid-js";
import * as m from "$lib/paraglide/messages.js";
import { countMessage } from "$lib/utils/count-message";
import { engineStore } from "$lib/stores/engine";
import { dashboardStore } from "$lib/stores/dashboard";

export default function EngineState() {
    const online = createMemo(() => engineStore.isAlive);
    const enforcing = createMemo(
        () => online() && dashboardStore.activeBlocksCount > 0,
    );
    const title = createMemo(() =>
        !online()
            ? m.status_engine_offline()
            : enforcing()
              ? m.status_enforcement_active()
              : m.status_engine_ready(),
    );
    const description = createMemo(() => {
        if (!online()) return m.status_engine_offline_description();
        if (enforcing()) {
            return countMessage(
                dashboardStore.activeBlocksCount,
                m.status_enforcement_active_description_zero,
                m.status_enforcement_active_description_one,
                () =>
                    m.status_enforcement_active_description({
                        count: dashboardStore.activeBlocksCount,
                    }),
            );
        }
        return m.status_engine_ready_description();
    });
    onSettled(() => {
        void dashboardStore.refresh();
        const timer = setInterval(() => {
            if (engineStore.isAlive) void dashboardStore.refresh();
        }, 5000);
        return () => clearInterval(timer);
    });
    return (
        <div
            class={[
                "engine-state",
                { offline: !online(), enforcing: enforcing() },
            ]}
            role="status"
        >
            <span class="state-mark" aria-hidden="true" />
            <span class="state-copy">
                <strong>{title()}</strong>
                <span>{description()}</span>
            </span>
            <style>
                {
                    '\n    .engine-state {\n        min-width: 0;\n        display: flex;\n        align-items: center;\n        gap: 9px;\n        color: var(--foreground);\n    }\n\n    .state-mark {\n        width: 8px;\n        height: 8px;\n        flex: 0 0 8px;\n        border-radius: 50%;\n        background: var(--ace-ash);\n    }\n\n    .engine-state.enforcing .state-mark {\n        background: var(--ace-success);\n    }\n\n    .engine-state.offline .state-mark {\n        background: var(--ace-danger);\n    }\n\n    .state-copy {\n        min-width: 0;\n        display: grid;\n        grid-template-columns: max-content minmax(0, 1fr);\n        align-items: baseline;\n        gap: 8px;\n    }\n\n    strong {\n        font-family: "IBM Plex Sans Condensed", sans-serif;\n        font-size: 13px;\n        font-weight: 600;\n    }\n\n    .state-copy > span {\n        overflow: hidden;\n        color: var(--muted-foreground);\n        font-size: 12px;\n        text-overflow: ellipsis;\n        white-space: nowrap;\n    }\n\n    @media (max-width: 920px) {\n        .state-copy > span {\n            display: none;\n        }\n    }\n'
                }
            </style>
        </div>
    );
}
