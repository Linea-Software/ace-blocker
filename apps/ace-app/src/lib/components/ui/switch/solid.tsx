import type { JSX } from "@solidjs/web";
import { createSignal } from "solid-js";
import { cn } from "$lib/utils.js";
type Props = Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "onChange"> & {
    checked?: boolean;
    defaultChecked?: boolean;
    onCheckedChange?: (v: boolean) => void;
};
export function Root(props: Props) {
    const [own, setOwn] = createSignal(props.defaultChecked ?? false);
    const checked = () => props.checked ?? own();
    const toggle = () => {
        if (props.disabled) return;
        const next = !checked();
        setOwn(next);
        props.onCheckedChange?.(next);
    };
    return (
        <button
            {...props}
            type="button"
            role="switch"
            data-slot="switch"
            aria-checked={checked() ? "true" : "false"}
            data-state={checked() ? "checked" : "unchecked"}
            onClick={toggle}
            class={cn(
                "data-[state=checked]:bg-primary data-[state=unchecked]:bg-input focus-visible:border-ring focus-visible:ring-ring/50 dark:data-[state=unchecked]:bg-input/80 peer inline-flex h-[1.15rem] w-8 shrink-0 items-center rounded-full border border-transparent shadow-xs transition-all outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",
                props.class,
            )}
        >
            <span
                data-slot="switch-thumb"
                data-state={checked() ? "checked" : "unchecked"}
                class="bg-background dark:data-[state=unchecked]:bg-foreground dark:data-[state=checked]:bg-primary-foreground pointer-events-none block size-4 rounded-full ring-0 transition-transform data-[state=checked]:translate-x-[calc(100%-2px)] data-[state=unchecked]:translate-x-0"
            />
        </button>
    );
}
export { Root as Switch };
