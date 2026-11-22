import type { JSX } from "@solidjs/web";
import { createContext, createSignal, useContext, Show } from "solid-js";
import { cn } from "$lib/utils.js";
import * as Popover from "../popover/solid";
type Ctx = {
    value: () => string;
    setValue: (v: string) => void;
    open: () => boolean;
    setOpen: (v: boolean) => void;
};
const Context = createContext<Ctx>();
function useSelect() {
    const v = useContext(Context);
    if (!v) throw Error("Select.Root required");
    return v;
}
export function Root(props: {
    children?: JSX.Element;
    type?: "single";
    value?: string;
    defaultValue?: string;
    onValueChange?: (v: string) => void;
    open?: boolean;
    onOpenChange?: (v: boolean) => void;
    disabled?: boolean;
}) {
    const [own, setOwn] = createSignal(props.defaultValue ?? "");
    const [uncontrolledOpen, setUncontrolledOpen] = createSignal(false);
    const ctx: Ctx = {
        value: () => props.value ?? own(),
        setValue: (v) => {
            setOwn(v);
            props.onValueChange?.(v);
        },
        open: () => props.open ?? uncontrolledOpen(),
        setOpen: (v) => {
            setUncontrolledOpen(v);
            props.onOpenChange?.(v);
        },
    };
    return (
        <Context value={ctx}>
            <Popover.Root open={ctx.open()} onOpenChange={ctx.setOpen}>
                {props.children}
            </Popover.Root>
        </Context>
    );
}
export function Trigger(
    props: JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
        size?: "default" | "sm";
    },
) {
    const ctx = useSelect();
    return (
        <Popover.Trigger
            {...props}
            data-slot="select-trigger"
            data-size={props.size ?? "default"}
            aria-haspopup="listbox"
            aria-expanded={ctx.open() ? "true" : "false"}
            onKeyDown={(e) => {
                if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                    e.preventDefault();
                    ctx.setOpen(true);
                }
            }}
            class={cn(
                "border-input data-[placeholder]:text-muted-foreground [&_svg:not([class*='text-'])]:text-muted-foreground focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:bg-input/30 dark:hover:bg-input/50 flex w-fit items-center justify-between gap-2 rounded-md border bg-transparent px-3 py-2 text-sm whitespace-nowrap shadow-xs transition-[color,box-shadow] outline-none select-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50 data-[size=default]:h-9 data-[size=sm]:h-8 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
                props.class,
            )}
        >
            {props.children}
            <svg
                class="size-4 opacity-50"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <path d="m6 9 6 6 6-6" />
            </svg>
        </Popover.Trigger>
    );
}
export function Content(
    props: JSX.HTMLAttributes<HTMLDivElement> & {
        sideOffset?: number;
        preventScroll?: boolean;
    },
) {
    return (
        <Popover.Content
            {...props}
            align="start"
            sideOffset={props.sideOffset ?? 4}
            data-slot="select-content"
            role="listbox"
            class={cn(
                "bg-popover text-popover-foreground data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-end-2 data-[side=right]:slide-in-from-start-2 data-[side=top]:slide-in-from-bottom-2 relative z-50 max-h-(--bits-select-content-available-height) min-w-[8rem] origin-(--bits-select-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-md border shadow-md data-[side=bottom]:translate-y-1 data-[side=left]:-translate-x-1 data-[side=right]:translate-x-1 data-[side=top]:-translate-y-1",
                props.class,
            )}
            onKeyDown={(e) => {
                const items = Array.from(
                    e.currentTarget.querySelectorAll<HTMLElement>(
                        '[role="option"]:not([aria-disabled="true"])',
                    ),
                );
                if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                    e.preventDefault();
                    const i = items.indexOf(
                        document.activeElement as HTMLElement,
                    );
                    items[
                        (i + (e.key === "ArrowDown" ? 1 : items.length - 1)) %
                            items.length
                    ]?.focus();
                } else if (e.key === "Home") {
                    e.preventDefault();
                    items[0]?.focus();
                } else if (e.key === "End") {
                    e.preventDefault();
                    items[items.length - 1]?.focus();
                } else if (e.key === "Enter" || e.key === " ") {
                    if (
                        document.activeElement?.getAttribute("role") ===
                        "option"
                    ) {
                        e.preventDefault();
                        (document.activeElement as HTMLElement).click();
                    }
                }
            }}
        >
            <div class="h-(--bits-select-anchor-height) w-full min-w-(--bits-select-anchor-width) scroll-my-1 p-1">
                {props.children}
            </div>
        </Popover.Content>
    );
}
export function Item(
    props: JSX.HTMLAttributes<HTMLDivElement> & {
        value: string;
        label?: string;
        disabled?: boolean;
    },
) {
    const ctx = useSelect();
    const selected = () => ctx.value() === props.value;
    return (
        <div
            {...props}
            data-slot="select-item"
            role="option"
            aria-selected={selected() ? "true" : "false"}
            aria-disabled={props.disabled ? "true" : undefined}
            tabindex={-1}
            data-state={selected() ? "checked" : "unchecked"}
            data-disabled={props.disabled || undefined}
            onClick={() => {
                if (props.disabled) return;
                ctx.setValue(props.value);
                ctx.setOpen(false);
            }}
            class={cn(
                "data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground [&_svg:not([class*='text-'])]:text-muted-foreground relative flex w-full cursor-default items-center gap-2 rounded-sm py-1.5 ps-2 pe-8 text-sm outline-hidden select-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4 *:[span]:last:flex *:[span]:last:items-center *:[span]:last:gap-2 focus:bg-accent",
                props.class,
            )}
        >
            <span class="absolute end-2 flex size-3.5 items-center justify-center">
                <Show when={selected()}>
                    <svg
                        class="size-4"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="m20 6-11 11-5-5" />
                    </svg>
                </Show>
            </span>
            {props.children ?? props.label ?? props.value}
        </div>
    );
}
export function Group(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div {...props} data-slot="select-group" role="group">
            {props.children}
        </div>
    );
}
export function Label(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="select-label"
            class={cn("text-muted-foreground px-2 py-1.5 text-xs", props.class)}
        >
            {props.children}
        </div>
    );
}
export function GroupHeading(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="select-group-heading"
            class={cn("text-muted-foreground px-2 py-1.5 text-xs", props.class)}
        >
            {props.children}
        </div>
    );
}
export function Separator(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <div
            {...props}
            data-slot="select-separator"
            role="separator"
            class={cn(
                "bg-border pointer-events-none -mx-1 my-1 h-px",
                props.class,
            )}
        />
    );
}
export function ScrollUpButton(
    props: JSX.ButtonHTMLAttributes<HTMLButtonElement>,
) {
    return (
        <button
            {...props}
            type="button"
            data-slot="select-scroll-up-button"
            class={cn(
                "flex cursor-default items-center justify-center py-1",
                props.class,
            )}
        >
            <svg
                class="size-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <path d="m18 15-6-6-6 6" />
            </svg>
        </button>
    );
}
export function ScrollDownButton(
    props: JSX.ButtonHTMLAttributes<HTMLButtonElement>,
) {
    return (
        <button
            {...props}
            type="button"
            data-slot="select-scroll-down-button"
            class={cn(
                "flex cursor-default items-center justify-center py-1",
                props.class,
            )}
        >
            <svg
                class="size-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
            >
                <path d="m6 9 6 6 6-6" />
            </svg>
        </button>
    );
}
export const Portal = Popover.Portal;
export {
    Root as Select,
    Trigger as SelectTrigger,
    Content as SelectContent,
    Item as SelectItem,
    Group as SelectGroup,
    Label as SelectLabel,
    GroupHeading as SelectGroupHeading,
    Separator as SelectSeparator,
    ScrollUpButton as SelectScrollUpButton,
    ScrollDownButton as SelectScrollDownButton,
    Portal as SelectPortal,
};
