import type { JSX } from "@solidjs/web";
import { cn } from "$lib/utils.js";
export type InputProps = JSX.InputHTMLAttributes<HTMLInputElement>;
export function Root(props: InputProps) {
    return (
        <input
            {...props}
            data-slot={props["data-slot"] ?? "input"}
            class={cn(
                props.type === "file"
                    ? "selection:bg-primary dark:bg-input/30 selection:text-primary-foreground border-input ring-offset-background placeholder:text-muted-foreground flex h-9 w-full min-w-0 rounded-md border bg-transparent px-3 pt-1.5 text-sm font-medium shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50"
                    : "border-input bg-background selection:bg-primary dark:bg-input/30 selection:text-primary-foreground ring-offset-background placeholder:text-muted-foreground flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
                "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
                props.class,
            )}
        />
    );
}
export { Root as Input };
