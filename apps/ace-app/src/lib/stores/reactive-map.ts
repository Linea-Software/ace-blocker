import { createRoot, createSignal } from "solid-js";

/** A Map with reactive value and membership reads for per-pack progress. */
export class ReactiveMap<K, V> extends Map<K, V> {
    private readonly track: () => number;
    private readonly invalidate: () => void;

    constructor(entries?: readonly (readonly [K, V])[] | null) {
        super(entries);
        const [read, write] = createRoot(() => createSignal(0));
        this.track = read;
        this.invalidate = () => { write(value => value + 1); };
    }

    override get(key: K): V | undefined {
        this.track();
        return super.get(key);
    }
    override has(key: K): boolean {
        this.track();
        return super.has(key);
    }
    override set(key: K, value: V): this {
        super.set(key, value);
        this.invalidate?.();
        return this;
    }
    override delete(key: K): boolean {
        const deleted = super.delete(key);
        if (deleted) this.invalidate();
        return deleted;
    }
    override clear(): void {
        if (super.size > 0) {
            super.clear();
            this.invalidate();
        }
    }
}
