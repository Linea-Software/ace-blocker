import type { JSX } from "@solidjs/web";
import { Portal } from "@solidjs/web";
import {
    createContext,
    createSignal,
    onSettled,
    useContext,
    Show,
    omit,
} from "solid-js";
import { tv, type VariantProps } from "tailwind-variants";
import { cn } from "$lib/utils.js";
import * as m from "$lib/paraglide/messages.js";
import { buttonVariants } from "../button/solid";
import { Root as InputPrimitive } from "../input/solid";
import { Root as SeparatorPrimitive } from "../separator/solid";
import { Root as SkeletonPrimitive } from "../skeleton/solid";
import * as Sheet from "../sheet/solid";
import {
    SIDEBAR_COOKIE_NAME,
    SIDEBAR_COOKIE_MAX_AGE,
    SIDEBAR_WIDTH,
    SIDEBAR_WIDTH_MOBILE,
    SIDEBAR_WIDTH_ICON,
    SIDEBAR_KEYBOARD_SHORTCUT,
} from "./constants.js";
type Div = JSX.HTMLAttributes<HTMLDivElement>;
type ButtonProps = JSX.ButtonHTMLAttributes<HTMLButtonElement>;
type SidebarContext = {
    open: () => boolean;
    setOpen: (v: boolean) => void;
    openMobile: () => boolean;
    setOpenMobile: (v: boolean) => void;
    isMobile: () => boolean;
    state: () => "collapsed" | "expanded";
    toggle: () => void;
};
const Ctx = createContext<SidebarContext>();
export function useSidebar() {
    const v = useContext(Ctx);
    if (!v) throw Error("Sidebar.Provider must wrap Sidebar");
    return v;
}
export function Provider(
    props: Div & {
        open?: boolean;
        defaultOpen?: boolean;
        onOpenChange?: (v: boolean) => void;
    },
) {
    const [own, setOwn] = createSignal(props.defaultOpen ?? true),
        [openMobile, setOpenMobile] = createSignal(false),
        [isMobile, setMobile] = createSignal(false);
    const open = () => props.open ?? own(),
        setOpen = (v: boolean) => {
            setOwn(v);
            props.onOpenChange?.(v);
            if (typeof document !== "undefined")
                document.cookie =
                    SIDEBAR_COOKIE_NAME +
                    "=" +
                    v +
                    "; path=/; max-age=" +
                    SIDEBAR_COOKIE_MAX_AGE;
        };
    const ctx: SidebarContext = {
        open,
        setOpen,
        openMobile,
        setOpenMobile,
        isMobile,
        state: () => (open() ? "expanded" : "collapsed"),
        toggle: () =>
            isMobile() ? setOpenMobile(!openMobile()) : setOpen(!open()),
    };
    onSettled(() => {
        const mq = window.matchMedia("(max-width: 767px)"),
            change = () => setMobile(mq.matches),
            keydown = (e: KeyboardEvent) => {
                if (
                    e.key === SIDEBAR_KEYBOARD_SHORTCUT &&
                    (e.metaKey || e.ctrlKey)
                ) {
                    e.preventDefault();
                    ctx.toggle();
                }
            };
        change();
        mq.addEventListener("change", change);
        window.addEventListener("keydown", keydown);
        return () => {
            mq.removeEventListener("change", change);
            window.removeEventListener("keydown", keydown);
        };
    });
    return (
        <Ctx value={ctx}>
            <div
                {...props}
                data-slot="sidebar-wrapper"
                class={cn(
                    "group/sidebar-wrapper has-data-[variant=inset]:bg-sidebar flex min-h-svh w-full",
                    props.class,
                )}
                style={{
                    "--sidebar-width": SIDEBAR_WIDTH,
                    "--sidebar-width-icon": SIDEBAR_WIDTH_ICON,
                }}
            >
                {props.children}
            </div>
        </Ctx>
    );
}
export function Root(
    props: Div & {
        side?: "left" | "right";
        variant?: "sidebar" | "floating" | "inset";
        collapsible?: "offcanvas" | "icon" | "none";
    },
) {
    const sidebar = useSidebar(),
        side = () => props.side ?? "left",
        variant = () => props.variant ?? "sidebar",
        collapsible = () => props.collapsible ?? "offcanvas",
        floating = () => variant() === "floating" || variant() === "inset";
    return (
        <Show
            when={collapsible() !== "none"}
            fallback={
                <div
                    {...props}
                    class={cn(
                        "bg-sidebar text-sidebar-foreground flex h-full w-(--sidebar-width) flex-col",
                        props.class,
                    )}
                >
                    {props.children}
                </div>
            }
        >
            <Show
                when={sidebar.isMobile()}
                fallback={
                    <div
                        class="text-sidebar-foreground group peer hidden md:block"
                        data-state={sidebar.state()}
                        data-collapsible={
                            sidebar.state() === "collapsed" ? collapsible() : ""
                        }
                        data-variant={variant()}
                        data-side={side()}
                        data-slot="sidebar"
                    >
                        <div
                            data-slot="sidebar-gap"
                            class={cn(
                                "relative w-(--sidebar-width) bg-transparent transition-[width] duration-200 ease-linear group-data-[collapsible=offcanvas]:w-0 group-data-[side=right]:rotate-180",
                                floating()
                                    ? "group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+(--spacing(4))+2px)]"
                                    : "group-data-[collapsible=icon]:w-(--sidebar-width-icon)",
                            )}
                        />
                        <div
                            {...props}
                            data-slot="sidebar-container"
                            class={cn(
                                "fixed inset-y-0 z-10 hidden h-svh w-(--sidebar-width) transition-[left,right,width] duration-200 ease-linear md:flex",
                                side() === "left"
                                    ? "start-0 group-data-[collapsible=offcanvas]:start-[calc(var(--sidebar-width)*-1)]"
                                    : "end-0 group-data-[collapsible=offcanvas]:end-[calc(var(--sidebar-width)*-1)]",
                                floating()
                                    ? "p-2 group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+(--spacing(4))+2px)]"
                                    : "group-data-[collapsible=icon]:w-(--sidebar-width-icon) group-data-[side=left]:border-e group-data-[side=right]:border-s",
                                props.class,
                            )}
                        >
                            <div
                                data-sidebar="sidebar"
                                data-slot="sidebar-inner"
                                class="bg-sidebar group-data-[variant=floating]:border-sidebar-border flex h-full w-full flex-col group-data-[variant=floating]:rounded-lg group-data-[variant=floating]:border group-data-[variant=floating]:shadow-sm"
                            >
                                {props.children}
                            </div>
                        </div>
                    </div>
                }
            >
                <Sheet.Root
                    open={sidebar.openMobile()}
                    onOpenChange={sidebar.setOpenMobile}
                >
                    <Sheet.Content
                        side={side()}
                        showCloseButton={false}
                        class="bg-sidebar text-sidebar-foreground w-(--sidebar-width) p-0 [&>button]:hidden"
                        data-sidebar="sidebar"
                        data-slot="sidebar"
                        data-mobile="true"
                        style={{ "--sidebar-width": SIDEBAR_WIDTH_MOBILE }}
                    >
                        <Sheet.Header class="sr-only">
                            <Sheet.Title>{m.ui_sidebar()}</Sheet.Title>
                            <Sheet.Description>
                                {m.ui_sidebar_description()}
                            </Sheet.Description>
                        </Sheet.Header>
                        <div class="flex h-full w-full flex-col">
                            {props.children}
                        </div>
                    </Sheet.Content>
                </Sheet.Root>
            </Show>
        </Show>
    );
}
export function Trigger(props: ButtonProps) {
    const ctx = useSidebar();
    return (
        <button
            {...omit(props, "ref")}
            data-sidebar="trigger"
            data-slot="sidebar-trigger"
            type="button"
            onClick={(e: MouseEvent) => {
                (props.onClick as ((e: MouseEvent) => void) | undefined)?.(e);
                if (!e.defaultPrevented) ctx.toggle();
            }}
            class={cn(
                buttonVariants({ variant: "ghost", size: "icon" }),
                "size-7",
                props.class,
            )}
        >
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="size-4"
            >
                <rect x="3" y="3" width="18" height="18" rx="2" />
                <path d="M9 3v18" />
            </svg>
            <span class="sr-only">{m.ui_toggle_sidebar()}</span>
        </button>
    );
}
export function Rail(props: ButtonProps) {
    const ctx = useSidebar();
    return (
        <button
            {...props}
            type="button"
            data-sidebar="rail"
            data-slot="sidebar-rail"
            aria-label={m.ui_toggle_sidebar()}
            tabindex={-1}
            title={m.ui_toggle_sidebar()}
            onClick={() => ctx.toggle()}
            class={cn(
                "hover:after:bg-sidebar-border absolute inset-y-0 z-20 hidden w-4 -translate-x-1/2 transition-all ease-linear group-data-[side=left]:-end-4 group-data-[side=right]:start-0 after:absolute after:inset-y-0 after:start-[calc(1/2*100%-1px)] after:w-[2px] sm:flex",
                "in-data-[side=left]:cursor-w-resize in-data-[side=right]:cursor-e-resize",
                "[[data-side=left][data-state=collapsed]_&]:cursor-e-resize [[data-side=right][data-state=collapsed]_&]:cursor-w-resize",
                "hover:group-data-[collapsible=offcanvas]:bg-sidebar group-data-[collapsible=offcanvas]:translate-x-0 group-data-[collapsible=offcanvas]:after:start-full",
                "[[data-side=left][data-collapsible=offcanvas]_&]:-end-2",
                "[[data-side=right][data-collapsible=offcanvas]_&]:-start-2",
                props.class,
            )}
        >
            {props.children}
        </button>
    );
}
export function Header(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-header"
            data-sidebar="header"
            class={cn("flex flex-col gap-2 p-2", props.class)}
        >
            {props.children}
        </div>
    );
}
export function Footer(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-footer"
            data-sidebar="footer"
            class={cn("flex flex-col gap-2 p-2", props.class)}
        >
            {props.children}
        </div>
    );
}
export function Content(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-content"
            data-sidebar="content"
            class={cn(
                "flex min-h-0 flex-1 flex-col gap-2 overflow-auto group-data-[collapsible=icon]:overflow-hidden",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function Group(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-group"
            data-sidebar="group"
            class={cn("relative flex w-full min-w-0 flex-col p-2", props.class)}
        >
            {props.children}
        </div>
    );
}
export function GroupContent(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-group-content"
            data-sidebar="group-content"
            class={cn("w-full text-sm", props.class)}
        >
            {props.children}
        </div>
    );
}
export function GroupLabel(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-group-label"
            data-sidebar="group-label"
            class={cn(
                "text-sidebar-foreground/70 ring-sidebar-ring flex h-8 shrink-0 items-center rounded-md px-2 text-xs font-medium outline-hidden transition-[margin,opacity] duration-200 ease-linear focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0 group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function GroupAction(props: ButtonProps) {
    return (
        <button
            {...props}
            data-slot="sidebar-group-action"
            data-sidebar="group-action"
            class={cn(
                "text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground absolute end-3 top-3.5 flex aspect-square w-5 items-center justify-center rounded-md p-0 outline-hidden transition-transform focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0 after:absolute after:-inset-2 md:after:hidden group-data-[collapsible=icon]:hidden",
                props.class,
            )}
        >
            {props.children}
        </button>
    );
}
export function Input(props: JSX.InputHTMLAttributes<HTMLInputElement>) {
    return (
        <InputPrimitive
            {...props}
            data-slot="sidebar-input"
            data-sidebar="input"
            class={cn("bg-background h-8 w-full shadow-none", props.class)}
        />
    );
}
export function Inset(props: JSX.HTMLAttributes<HTMLElement>) {
    return (
        <main
            {...props}
            data-slot="sidebar-inset"
            class={cn(
                "bg-background relative flex w-full flex-1 flex-col",
                "md:peer-data-[variant=inset]:m-2 md:peer-data-[variant=inset]:ms-0 md:peer-data-[variant=inset]:rounded-xl md:peer-data-[variant=inset]:shadow-sm md:peer-data-[variant=inset]:peer-data-[state=collapsed]:ms-2",
                props.class,
            )}
        >
            {props.children}
        </main>
    );
}
export function Menu(props: JSX.HTMLAttributes<HTMLUListElement>) {
    return (
        <ul
            {...props}
            data-slot="sidebar-menu"
            data-sidebar="menu"
            class={cn("flex w-full min-w-0 flex-col gap-1", props.class)}
        >
            {props.children}
        </ul>
    );
}
export function MenuItem(props: JSX.LiHTMLAttributes<HTMLLIElement>) {
    return (
        <li
            {...props}
            data-slot="sidebar-menu-item"
            data-sidebar="menu-item"
            class={cn("group/menu-item relative", props.class)}
        >
            {props.children}
        </li>
    );
}
export const sidebarMenuButtonVariants = tv({
    base: "peer/menu-button ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground data-[state=open]:hover:bg-sidebar-accent data-[state=open]:hover:text-sidebar-accent-foreground flex w-full items-center gap-2 overflow-hidden rounded-md p-2 text-start text-sm outline-hidden transition-[width,height,padding] group-has-data-[sidebar=menu-action]/menu-item:pe-8 group-data-[collapsible=icon]:size-8! group-data-[collapsible=icon]:p-2! focus-visible:ring-2 disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-[active=true]:font-medium [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0",
    variants: {
        variant: {
            default:
                "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
            outline:
                "bg-background hover:bg-sidebar-accent hover:text-sidebar-accent-foreground shadow-[0_0_0_1px_var(--sidebar-border)] hover:shadow-[0_0_0_1px_var(--sidebar-accent)]",
        },
        size: {
            default: "h-8 text-sm",
            sm: "h-7 text-xs",
            lg: "h-12 text-sm group-data-[collapsible=icon]:p-0!",
        },
    },
    defaultVariants: { variant: "default", size: "default" },
});
export type SidebarMenuButtonVariant = VariantProps<
    typeof sidebarMenuButtonVariants
>["variant"];
export type SidebarMenuButtonSize = VariantProps<
    typeof sidebarMenuButtonVariants
>["size"];
type MenuButtonProps = ButtonProps & {
    isActive?: boolean;
    variant?: SidebarMenuButtonVariant;
    size?: SidebarMenuButtonSize;
    tooltipContent?: JSX.Element | string;
    tooltipContentProps?: Record<string, unknown>;
};
export function MenuButton(props: MenuButtonProps) {
    const ctx = useSidebar(),
        [hover, setHover] = createSignal(false);
    let el!: HTMLButtonElement;
    return (
        <>
            <button
                ref={el}
                {...props}
                data-slot="sidebar-menu-button"
                data-sidebar="menu-button"
                data-size={props.size ?? "default"}
                data-active={props.isActive ?? false}
                type="button"
                onMouseEnter={() => setHover(true)}
                onMouseLeave={() => setHover(false)}
                onFocus={() => setHover(true)}
                onBlur={() => setHover(false)}
                class={cn(
                    sidebarMenuButtonVariants({
                        variant: props.variant,
                        size: props.size,
                    }),
                    props.class,
                )}
            >
                {props.children}
            </button>
            <Show
                when={
                    !!props.tooltipContent &&
                    hover() &&
                    ctx.state() === "collapsed" &&
                    !ctx.isMobile()
                }
            >
                <Portal>
                    <div
                        role="tooltip"
                        data-slot="tooltip-content"
                        class="bg-foreground text-background fixed z-50 w-fit rounded-md px-3 py-1.5 text-xs"
                        style={{
                            top: (el?.getBoundingClientRect().top ?? 0) + "px",
                            left:
                                (el?.getBoundingClientRect().right ?? 0) +
                                8 +
                                "px",
                        }}
                    >
                        {props.tooltipContent}
                    </div>
                </Portal>
            </Show>
        </>
    );
}
export function MenuAction(props: ButtonProps & { showOnHover?: boolean }) {
    return (
        <button
            {...props}
            data-slot="sidebar-menu-action"
            data-sidebar="menu-action"
            class={cn(
                "text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground peer-hover/menu-button:text-sidebar-accent-foreground absolute end-1 top-1.5 flex aspect-square w-5 items-center justify-center rounded-md p-0 outline-hidden transition-transform focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0 after:absolute after:-inset-2 md:after:hidden peer-data-[size=sm]/menu-button:top-1 peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5 group-data-[collapsible=icon]:hidden",
                props.showOnHover &&
                    "peer-data-[active=true]/menu-button:text-sidebar-accent-foreground group-focus-within/menu-item:opacity-100 group-hover/menu-item:opacity-100 data-[state=open]:opacity-100 md:opacity-0",
                props.class,
            )}
        >
            {props.children}
        </button>
    );
}
export function MenuBadge(props: Div) {
    return (
        <div
            {...props}
            data-slot="sidebar-menu-badge"
            data-sidebar="menu-badge"
            class={cn(
                "text-sidebar-foreground pointer-events-none absolute end-1 flex h-5 min-w-5 items-center justify-center rounded-md px-1 text-xs font-medium tabular-nums select-none peer-hover/menu-button:text-sidebar-accent-foreground peer-data-[active=true]/menu-button:text-sidebar-accent-foreground peer-data-[size=sm]/menu-button:top-1 peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5 group-data-[collapsible=icon]:hidden",
                props.class,
            )}
        >
            {props.children}
        </div>
    );
}
export function MenuSkeleton(props: Div & { showIcon?: boolean }) {
    const width = () => Math.floor(Math.random() * 40) + 50 + "%";
    return (
        <div
            {...props}
            data-slot="sidebar-menu-skeleton"
            data-sidebar="menu-skeleton"
            class={cn(
                "flex h-8 items-center gap-2 rounded-md px-2",
                props.class,
            )}
        >
            <Show when={props.showIcon}>
                <SkeletonPrimitive
                    class="size-4 rounded-md"
                    data-sidebar="menu-skeleton-icon"
                />
            </Show>
            <SkeletonPrimitive
                class="h-4 max-w-(--skeleton-width) flex-1"
                data-sidebar="menu-skeleton-text"
                style={{ "--skeleton-width": width() }}
            />
            {props.children}
        </div>
    );
}
export function MenuSub(props: JSX.HTMLAttributes<HTMLUListElement>) {
    return (
        <ul
            {...props}
            data-slot="sidebar-menu-sub"
            data-sidebar="menu-sub"
            class={cn(
                "border-sidebar-border mx-3.5 flex min-w-0 translate-x-px flex-col gap-1 border-s px-2.5 py-0.5 group-data-[collapsible=icon]:hidden",
                props.class,
            )}
        >
            {props.children}
        </ul>
    );
}
export function MenuSubItem(props: JSX.LiHTMLAttributes<HTMLLIElement>) {
    return (
        <li
            {...props}
            data-slot="sidebar-menu-sub-item"
            data-sidebar="menu-sub-item"
            class={cn("group/menu-sub-item relative", props.class)}
        >
            {props.children}
        </li>
    );
}
export function MenuSubButton(
    props: JSX.AnchorHTMLAttributes<HTMLAnchorElement> & {
        size?: "sm" | "md";
        isActive?: boolean;
    },
) {
    return (
        <a
            {...props}
            data-slot="sidebar-menu-sub-button"
            data-sidebar="menu-sub-button"
            data-size={props.size ?? "md"}
            data-active={props.isActive ?? false}
            class={cn(
                "text-sidebar-foreground ring-sidebar-ring hover:bg-sidebar-accent hover:text-sidebar-accent-foreground active:bg-sidebar-accent active:text-sidebar-accent-foreground [&>svg]:text-sidebar-accent-foreground flex h-7 min-w-0 -translate-x-px items-center gap-2 overflow-hidden rounded-md px-2 outline-hidden focus-visible:ring-2 disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 data-[active=true]:bg-sidebar-accent data-[active=true]:text-sidebar-accent-foreground group-data-[collapsible=icon]:hidden",
                props.size === "sm" ? "text-xs" : "text-sm",
                props.class,
            )}
        >
            {props.children}
        </a>
    );
}
export function Separator(props: JSX.HTMLAttributes<HTMLDivElement>) {
    return (
        <SeparatorPrimitive
            {...props}
            data-slot="sidebar-separator"
            data-sidebar="separator"
            class={cn("bg-sidebar-border", props.class)}
        />
    );
}
export {
    Root as Sidebar,
    Provider as SidebarProvider,
    Trigger as SidebarTrigger,
    Rail as SidebarRail,
    Header as SidebarHeader,
    Footer as SidebarFooter,
    Content as SidebarContent,
    Group as SidebarGroup,
    GroupContent as SidebarGroupContent,
    GroupLabel as SidebarGroupLabel,
    GroupAction as SidebarGroupAction,
    Input as SidebarInput,
    Inset as SidebarInset,
    Menu as SidebarMenu,
    MenuItem as SidebarMenuItem,
    MenuButton as SidebarMenuButton,
    MenuAction as SidebarMenuAction,
    MenuBadge as SidebarMenuBadge,
    MenuSkeleton as SidebarMenuSkeleton,
    MenuSub as SidebarMenuSub,
    MenuSubItem as SidebarMenuSubItem,
    MenuSubButton as SidebarMenuSubButton,
    Separator as SidebarSeparator,
};
