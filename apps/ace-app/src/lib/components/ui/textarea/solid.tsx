import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils.js";
export function Root(props: JSX.TextareaHTMLAttributes<HTMLTextAreaElement>) {
    return (
        <textarea
            {...props}
            data-slot={props["data-slot"] ?? "textarea"}
            class={cn(
                "border-input placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:bg-input/30 flex field-sizing-content min-h-16 w-full rounded-md border bg-transparent px-3 py-2 text-base shadow-xs transition-[color,box-shadow] outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                props.class,
            )}
        >
            {props.children}
        </textarea>
    );
}
export { Root as Textarea };
