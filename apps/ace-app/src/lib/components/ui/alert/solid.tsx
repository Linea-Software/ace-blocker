import { tv, type VariantProps } from "tailwind-variants";
import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils.js";
export const alertVariants = tv({
    base: "relative grid w-full grid-cols-[0_1fr] items-start gap-y-0.5 rounded-lg border px-4 py-3 text-sm has-[>svg]:grid-cols-[calc(var(--spacing)*4)_1fr] has-[>svg]:gap-x-3 [&>svg]:size-4 [&>svg]:translate-y-0.5 [&>svg]:text-current",
    variants: {
        variant: {
            default: "bg-card text-card-foreground",
            destructive:
                "text-destructive bg-card *:data-[slot=alert-description]:text-destructive/90 [&>svg]:text-current",
        },
    },
    defaultVariants: { variant: "default" },
});
export type AlertVariant = VariantProps<typeof alertVariants>["variant"];
export function Root(
    props: JSX.HTMLAttributes<HTMLDivElement> & { variant?: AlertVariant },
) {
    return (
        <div
            {...props}
            data-slot="alert"
            role="alert"
            class={cn(alertVariants({ variant: props.variant }), props.class)}
        >
            {props.children}
        </div>
    );
}
export function Title(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="alert-title"
            class={cn(
                "col-start-2 line-clamp-1 min-h-4 font-medium tracking-tight",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function Description(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="alert-description"
            class={cn(
                "text-muted-foreground col-start-2 grid justify-items-start gap-1 text-sm [&_p]:leading-relaxed",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export { Root as Alert, Title as AlertTitle, Description as AlertDescription };
