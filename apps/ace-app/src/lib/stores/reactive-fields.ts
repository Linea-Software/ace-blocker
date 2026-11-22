import { createRoot, createSignal, getOwner } from "solid-js";

/**
 * Preserve the public field-based API of the Svelte stores while replacing
 * Svelte runes with native Solid 2 signals. Every field has an independently
 * tracked getter and setter, so changes propagate to existing consumers.
 */
export function reactiveFields<T extends object>(
    target: T,
    keys: readonly string[],
): void {
    const attach = () => {
        for (const key of keys) {
            // Business operations may stage multiple draft mutations in one
            // synchronous event handler. Signals commit at the microtask
            // boundary in Solid 2, so keep an immediate imperative value
            // while tracking the signal for reactive component reads.
            let current: unknown = Reflect.get(target, key);
            const [track, notify] = createSignal<unknown>(current);
            Object.defineProperty(target, key, {
                configurable: true,
                enumerable: true,
                get() {
                    track(); // establish the Solid subscriber
                    return current;
                },
                set(value: unknown) {
                    current = value;
                    notify(() => value);
                },
            });
        }
    };
    // Route-level form state must be disposed with its component. Only
    // module-global singleton stores require a persistent detached root.
    if (getOwner()) attach();
    else createRoot(attach);
}
