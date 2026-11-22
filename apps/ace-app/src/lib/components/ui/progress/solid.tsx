import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils.js";
export function Root(
    props: JSX.HTMLAttributes<HTMLDivElement> & {
        value?: number | null;
        max?: number | null;
    },
) {
    const max = () => Math.max(1, props.max ?? 100);
    const value = () => Math.min(max(), Math.max(0, props.value ?? 0));
    return (
        <div
            {...props}
            data-slot="progress"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={max()}
            aria-valuenow={props.value == null ? undefined : value()}
            class={cn(
                "bg-primary/20 relative h-2 w-full overflow-hidden rounded-full",
                props.class,
            )}
        >
            <div
                data-slot="progress-indicator"
                class="bg-primary h-full w-full flex-1 transition-all"
                style={{
                    transform: `translateX(-${100 - (100 * value()) / max()}%)`,
                }}
            />
        </div>
    );
}
export { Root as Progress };
