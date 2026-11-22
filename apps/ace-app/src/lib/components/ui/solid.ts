/**
 * Solid 2.0 UI entry points.
 *
 * Explicit Solid imports coexist with the legacy Svelte-only index.ts modules
 * until the route/component migration replaces their consumers.
 * Do not import a Svelte index.ts from Solid TSX.
 */
export * as Accordion from "./accordion/solid";
export * as Alert from "./alert/solid";
export * as AlertDialog from "./alert-dialog/solid";
export * as Card from "./card/solid";
export * as Dialog from "./dialog/solid";
export * as DropdownMenu from "./dropdown-menu/solid";
export * as Popover from "./popover/solid";
export * as Select from "./select/solid";
export * as Sheet from "./sheet/solid";
export * as Sidebar from "./sidebar/solid";
export * as Tabs from "./tabs/solid";
export * as Tooltip from "./tooltip/solid";
export * as CalendarParts from "./calendar/solid";
export { Calendar } from "./calendar/solid";
export { Badge, badgeVariants } from "./badge/solid";
export { Button, buttonVariants } from "./button/solid";
export { Checkbox } from "./checkbox/solid";
export { Input } from "./input/solid";
export { Label } from "./label/solid";
export { Progress } from "./progress/solid";
export { ScrollArea } from "./scroll-area/solid";
export { Separator } from "./separator/solid";
export { Skeleton } from "./skeleton/solid";
export { Spinner } from "./spinner/solid";
export { Switch } from "./switch/solid";
export { Textarea } from "./textarea/solid";
export { Toaster, toast } from "./sonner/solid";
