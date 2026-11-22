import type { JSX } from "@solidjs/web";
// Solid 2 native label.
import { cn } from "$lib/utils.js";

export function Root(props: JSX.LabelHTMLAttributes<HTMLLabelElement>) {
    return (
        <label
            {...props}
            data-slot="label"
            class={cn(
                "flex items-center gap-2 text-sm leading-none font-medium select-none group-data-[disabled=true]:pointer-events-none group-data-[disabled=true]:opacity-50 peer-disabled:cursor-not-allowed peer-disabled:opacity-50",
                props.class,
            )}
        >
            {props.children}
        </label>
    );
}
export { Root as Label };
