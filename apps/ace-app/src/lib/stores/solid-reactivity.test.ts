import { describe, expect, it } from "vitest";
import { createMemo, createRoot, flush } from "solid-js";
import { reactiveFields } from "./reactive-fields";
import { ReactiveMap } from "./reactive-map";

describe("ported Solid 2 state adapters", () => {
    it("tracks scalar and object assignments without losing field-based APIs", () => {
        // Component/root initializers are owned scopes. As in the real app,
        // writes come from events, not from inside a tracked computation.
        const { record, status, dispose } = createRoot((dispose) => {
            const record = {
                enabled: false,
                message: null as string | null,
                items: [] as string[],
            };
            reactiveFields(record, ["enabled", "message", "items"]);
            const status = createMemo(
                () =>
                    (record.enabled ? "enabled" : "disabled") +
                    ":" +
                    record.items.length,
            );
            return { record, status, dispose };
        });
        expect(status()).toBe("disabled:0");
        record.enabled = true;
        record.items = ["alpha", "beta"];
        flush();
        expect(status()).toBe("enabled:2");
        record.message = "saved";
        flush();
        expect(record.message).toBe("saved");
        dispose();
    });

    it("makes per-pack map membership and progress reactive", () => {
        const { values, current, dispose } = createRoot((dispose) => {
            const values = new ReactiveMap<string, number>();
            const current = createMemo(() => values.get("pack") ?? -1);
            return { values, current, dispose };
        });
        expect(current()).toBe(-1);
        values.set("pack", 25);
        flush();
        expect(current()).toBe(25);
        values.set("pack", 100);
        flush();
        expect(current()).toBe(100);
        values.delete("pack");
        flush();
        expect(current()).toBe(-1);
        dispose();
    });
});
