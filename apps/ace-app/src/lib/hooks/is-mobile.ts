import { createSignal, onSettled } from "solid-js";

const DEFAULT_MOBILE_BREAKPOINT = 768;

/** Live media-query state with the same .current interface as Svelte MediaQuery. */
export class IsMobile {
    private readonly signal = createSignal(false);
    get current(): boolean { return this.signal[0](); }

    constructor(breakpoint: number = DEFAULT_MOBILE_BREAKPOINT) {
        if (typeof window === "undefined" || !window.matchMedia) return;
        const query = window.matchMedia(`(max-width: ${breakpoint - 1}px)`);
        const update = () => this.signal[1](query.matches);
        update();
        onSettled(() => {
            query.addEventListener("change", update);
            return () => query.removeEventListener("change", update);
        });
    }
}
