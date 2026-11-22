import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils.js";
import * as m from "$lib/paraglide/messages.js";
export function Spinner(props: JSX.SvgSVGAttributes<SVGSVGElement>) {
    return (
        <svg
            {...props}
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            role="status"
            aria-label={m.ui_loading()}
            class={cn("size-4 animate-spin", props.class)}
        >
            <path d="M21 12a9 9 0 1 1-6.219-8.56" />
        </svg>
    );
}
