import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils.js";
export function Root(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="card"
            class={cn(
                "bg-card text-card-foreground flex flex-col gap-6 rounded-lg border py-6 shadow-sm",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function Header(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="card-header"
            class={cn(
                "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-1.5 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function Title(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="card-title"
            class={cn("leading-none font-semibold", props.class)}
        >
            {props.children}
        </div>
    );
}
export function Description(props: JSX.HTMLAttributes<HTMLParagraphElement>) {
    return (
        <p
            {...props}
            data-slot="card-description"
            class={cn("text-muted-foreground text-sm", props.class)}
        >
            {props.children}
        </p>
    );
}
export function Action(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="card-action"
            class={cn(
                "col-start-2 row-span-2 row-start-1 self-start justify-self-end",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function Content(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="card-content"
            class={cn("px-6", props.class)}
        >
            {props.children}
        </div>
    );
}
export function Footer(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="card-footer"
            class={cn("flex items-center px-6 [.border-t]:pt-6", props.class)}
        >
            {props.children}
        </div>
    );
}
export {
    Root as Card,
    Header as CardHeader,
    Title as CardTitle,
    Description as CardDescription,
    Action as CardAction,
    Content as CardContent,
    Footer as CardFooter,
};
