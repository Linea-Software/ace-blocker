import type { JSX } from "@solidjs/web";
import {cn} from "$lib/utils.js";
export function Root(props:JSX.HTMLAttributes<HTMLDivElement>){
 return <div {...props} data-slot="skeleton" class={cn("bg-accent animate-pulse rounded-md",props.class)} />;
}
export { Root as Skeleton };
