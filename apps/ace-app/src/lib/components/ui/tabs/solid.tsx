import type {JSX} from "@solidjs/web";
import {createContext,createSignal,useContext,Show} from "solid-js";
import {cn} from "$lib/utils.js";
type Common=JSX.HTMLAttributes<HTMLDivElement>;
type TabContext={value:()=>string;select:(v:string)=>void;id:string};
const Context=createContext<TabContext>();
function useTabs(){const ctx=useContext(Context);if(!ctx)throw Error("Tabs must be inside Tabs.Root");return ctx}
let serial=0;
export function Root(props:Common&{value?:string;defaultValue?:string;onValueChange?:(value:string)=>void;orientation?:"horizontal"|"vertical"}){
 const [own,setOwn]=createSignal(props.defaultValue??"");
 const ctx:TabContext={id:"ace-tabs-"+(++serial),value:()=>props.value??own(),select:v=>{setOwn(v);props.onValueChange?.(v)}};
 return <Context value={ctx}><div {...props} data-slot="tabs" data-orientation={props.orientation??"horizontal"} class={cn("flex flex-col gap-2",props.class)}>{props.children}</div></Context>;
}
export function List(props:Common){return <div {...props} role="tablist" data-slot="tabs-list" class={cn("bg-muted text-muted-foreground inline-flex h-9 w-fit items-center justify-center rounded-lg p-[3px]",props.class)} onKeyDown={e=>{if(!["ArrowLeft","ArrowRight","ArrowUp","ArrowDown","Home","End"].includes(e.key))return;const tabs=Array.from(e.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]:not(:disabled)'));const idx=tabs.indexOf(document.activeElement as HTMLButtonElement);const next=e.key==="Home"?0:e.key==="End"?tabs.length-1:(idx+(e.key==="ArrowLeft"||e.key==="ArrowUp"?tabs.length-1:1))%tabs.length;e.preventDefault();tabs[next]?.focus();tabs[next]?.click()}}>{props.children}</div>}
export function Trigger(props:JSX.ButtonHTMLAttributes<HTMLButtonElement>&{value:string}){
 const ctx=useTabs();const active=()=>ctx.value()===props.value;
 return <button {...props} type="button" role="tab" id={ctx.id+"-trigger-"+props.value} aria-controls={ctx.id+"-panel-"+props.value} aria-selected={active()?"true":"false"} tabindex={active()?0:-1} data-slot="tabs-trigger" data-state={active()?"active":"inactive"} onClick={()=>ctx.select(props.value)} class={cn("data-[state=active]:bg-background dark:data-[state=active]:text-foreground focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:outline-ring dark:data-[state=active]:border-input dark:data-[state=active]:bg-input/30 text-foreground dark:text-muted-foreground inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-2 py-1 text-sm font-medium whitespace-nowrap transition-[color,box-shadow] focus-visible:ring-[3px] focus-visible:outline-1 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:shadow-sm [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",props.class)}>{props.children}</button>;
}
export function Content(props:Common&{value:string;forceMount?:boolean}){
 const ctx=useTabs();const active=()=>ctx.value()===props.value;
 return <Show when={props.forceMount||active()}><div {...props} id={ctx.id+"-panel-"+props.value} role="tabpanel" aria-labelledby={ctx.id+"-trigger-"+props.value} data-slot="tabs-content" data-state={active()?"active":"inactive"} tabindex={0} class={cn("flex-1 outline-none",props.class)}>{props.children}</div></Show>;
}
export {Root as Tabs,List as TabsList,Trigger as TabsTrigger,Content as TabsContent};