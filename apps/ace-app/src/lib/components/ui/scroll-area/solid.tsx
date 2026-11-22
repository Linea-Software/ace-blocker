import type {JSX} from "@solidjs/web";
import {cn} from "$lib/utils.js";
type Props=JSX.HTMLAttributes<HTMLDivElement>&{orientation?:"vertical"|"horizontal"|"both";scrollbarXClasses?:string;scrollbarYClasses?:string;viewportRef?:HTMLDivElement|((v:HTMLDivElement)=>void)};
export function Scrollbar(props:JSX.HTMLAttributes<HTMLDivElement>&{orientation?:"vertical"|"horizontal"}){
 return <div {...props} data-slot="scroll-area-scrollbar" data-orientation={props.orientation??"vertical"} class={cn("flex touch-none p-px transition-colors select-none",props.orientation==="horizontal"?"h-2.5 flex-col border-t border-t-transparent":"h-full w-2.5 border-s border-s-transparent",props.class)}><div data-slot="scroll-area-thumb" class="bg-border relative flex-1 rounded-full"/></div>;
}
export function Root(props:Props){
 return <div {...props} data-slot="scroll-area" class={cn("relative",props.class)}>
 <div ref={props.viewportRef} data-slot="scroll-area-viewport" tabindex={0} class={cn("ring-ring/10 dark:ring-ring/20 dark:outline-ring/40 outline-ring/50 size-full rounded-[inherit] transition-[color,box-shadow] focus-visible:ring-4 focus-visible:outline-1",props.orientation==="horizontal"?"overflow-x-auto overflow-y-hidden":props.orientation==="both"?"overflow-auto":"overflow-y-auto overflow-x-hidden")}>{props.children}</div>
 </div>;
}
export {Root as ScrollArea,Scrollbar as ScrollAreaScrollbar};