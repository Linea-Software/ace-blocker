import type { JSX } from "@solidjs/web";
import { createContext, createSignal, useContext, Show } from "solid-js";
import { cn } from "$lib/utils.js";
type Val = string | string[];
type AC = {
    open: (v: string) => boolean;
    toggle: (v: string) => void;
    id: string;
};
const Context = createContext<AC>();
const ItemContext = createContext<string>();
let seq = 0;
function useAccordion() {
    const c = useContext(Context);
    if (!c) throw Error("Accordion.Root required");
    return c;
}
function useItem() {
    const x = useContext(ItemContext);
    if (!x) throw Error("Accordion.Item required");
    return x;
}
export function Root(
    props: JSX.HTMLAttributes<HTMLDivElement> & {
        type?: "single" | "multiple";
        value?: Val;
        defaultValue?: Val;
        collapsible?: boolean;
        onValueChange?: (v: Val) => void;
    },
) {
    const [own, setOwn] = createSignal<Val>(
        props.defaultValue ?? (props.type === "multiple" ? [] : ""),
    );
    const value = () => props.value ?? own();
    const ctx: AC = {
        id: "ace-accordion-" + ++seq,
        open: (v) =>
            Array.isArray(value())
                ? (value() as string[]).includes(v)
                : value() === v,
        toggle: (v) => {
            const current = value();
            const multiple = props.type === "multiple";
            const active = Array.isArray(current)
                ? current.includes(v)
                : current === v;
            const next = multiple
                ? active
                    ? (current as string[]).filter((x) => x !== v)
                    : [...(Array.isArray(current) ? current : []), v]
                : active && props.collapsible
                  ? ""
                  : v;
            setOwn(next);
            props.onValueChange?.(next);
        },
    };
    return (
        <Context value={ctx}>
            <div {...props} data-slot="accordion">
                {props.children}
            </div>
        </Context>
    );
}
export function Item(
    props: JSX.HTMLAttributes<HTMLDivElement> & {
        value: string;
        disabled?: boolean;
    },
) {
    const ctx = useAccordion();
    return (
        <ItemContext value={props.value}>
            <div
                {...props}
                data-slot="accordion-item"
                data-state={ctx.open(props.value) ? "open" : "closed"}
                data-disabled={props.disabled || undefined}
                class={cn("border-b last:border-b-0", props.class)}
            >
                {props.children}
            </div>
        </ItemContext>
    );
}
export function Trigger(
    props: JSX.ButtonHTMLAttributes<HTMLButtonElement> & { level?: number },
) {
    const ctx = useAccordion(),
        val = useItem(),
        open = () => ctx.open(val);
    return (
        <h3 class="flex">
            <button
                {...props}
                type="button"
                data-slot="accordion-trigger"
                id={ctx.id + "-trigger-" + val}
                aria-controls={ctx.id + "-panel-" + val}
                aria-expanded={open() ? "true" : "false"}
                data-state={open() ? "open" : "closed"}
                onClick={() => ctx.toggle(val)}
                class={cn(
                    "focus-visible:border-ring focus-visible:ring-ring/50 flex flex-1 items-start justify-between gap-4 rounded-md py-4 text-start text-sm font-medium transition-all outline-none hover:underline focus-visible:ring-[3px] disabled:pointer-events-none disabled:opacity-50 [&[data-state=open]>svg]:rotate-180",
                    props.class,
                )}
            >
                {props.children}
                <svg
                    class="text-muted-foreground pointer-events-none size-4 shrink-0 translate-y-0.5 transition-transform duration-200"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <path d="m6 9 6 6 6-6" />
                </svg>
            </button>
        </h3>
    );
}
export function Content(props: JSX.HTMLAttributes<HTMLDivElement>) {
    const ctx = useAccordion(),
        val = useItem(),
        open = () => ctx.open(val);
    return (
        <div
            {...props}
            id={ctx.id + "-panel-" + val}
            aria-labelledby={ctx.id + "-trigger-" + val}
            role="region"
            data-slot="accordion-content"
            data-state={open() ? "open" : "closed"}
            hidden={!open()}
            class="data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down overflow-hidden text-sm"
        >
            <div class={cn("pt-0 pb-4", props.class)}>{props.children}</div>
        </div>
    );
}
export {
    Root as Accordion,
    Item as AccordionItem,
    Trigger as AccordionTrigger,
    Content as AccordionContent,
};
