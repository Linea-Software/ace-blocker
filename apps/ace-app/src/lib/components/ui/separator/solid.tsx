import type {JSX} from "@solidjs/web";
import {cn} from "$lib/utils.js";
export function Root(props:JSX.HTMLAttributes<HTMLDivElement>&{orientation?:"horizontal"|"vertical";decorative?:boolean}){
 return <div {...props} data-slot="separator" role={props.decorative===false?"separator":"none"} aria-orientation={props.decorative===false?(props.orientation??"horizontal"):undefined} data-orientation={props.orientation??"horizontal"} class={cn("bg-border shrink-0 data-[orientation=horizontal]:h-px data-[orientation=horizontal]:w-full data-[orientation=vertical]:h-full data-[orientation=vertical]:w-px",props.class)} />;
}
export {Root as Separator};