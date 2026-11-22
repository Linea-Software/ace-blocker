import type { JSX } from "@solidjs/web";
import { Toaster as SolidSonner, toast } from "solid-sonner";
import { cn } from "$lib/utils.js";
type Props = Parameters<typeof SolidSonner>[0];
const icon = (path: string, extra = "") => (
    <svg
        class={cn("size-4", extra)}
        viewBox="0 0 24 24"
        stroke="currentColor"
        stroke-width="2"
        fill="none"
        stroke-linecap="round"
        stroke-linejoin="round"
    >
        <path d={path} />
    </svg>
);
export function Toaster(props: Props) {
    return (
        <SolidSonner
            {...props}
            theme={props.theme ?? "system"}
            class={cn("toaster group", props.class)}
            style={
                {
                    "--normal-bg": "var(--color-popover)",
                    "--normal-text": "var(--color-popover-foreground)",
                    "--normal-border": "var(--color-border)",
                    ...(props.style ?? {}),
                } as JSX.CSSProperties
            }
            icons={{
                loading: icon("M21 12a9 9 0 1 1-6.219-8.56", "animate-spin"),
                success: icon("M20 6 9 17l-5-5"),
                error: icon("M18 6 6 18M6 6l12 12"),
                info: icon(
                    "M12 16v-4M12 8h.01M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20",
                ),
                warning: icon(
                    "m10.29 3.86-8.4 14.55A2 2 0 0 0 3.62 21h16.76a2 2 0 0 0 1.73-3L13.7 3.86a2 2 0 0 0-3.41 0",
                ),
            }}
        />
    );
}
export { toast };
