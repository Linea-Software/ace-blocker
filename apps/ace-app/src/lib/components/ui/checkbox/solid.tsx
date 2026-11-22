import type {JSX} from "@solidjs/web";
import {createSignal,Show} from "solid-js";
import {cn} from "$lib/utils.js";
type Props=Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>,"onChange"> & {checked?:boolean; defaultChecked?:boolean;indeterminate?:boolean;onCheckedChange?:(v:boolean)=>void;onIndeterminateChange?:(v:boolean)=>void};
export function Root(props:Props){
 const [own,setOwn]=createSignal(props.defaultChecked??false);
 const checked=()=>props.checked??own();
 const toggle=()=>{if(props.disabled)return;const next=!checked();setOwn(next);props.onCheckedChange?.(next);if(props.indeterminate)props.onIndeterminateChange?.(false)};
 return <button {...props} type="button" data-slot="checkbox" role="checkbox" aria-checked={props.indeterminate?"mixed":checked()?"true":"false"} data-state={props.indeterminate?"indeterminate":checked()?"checked":"unchecked"} onClick={toggle} class={cn("border-input dark:bg-input/30 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground dark:data-[state=checked]:bg-primary data-[state=checked]:border-primary focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive peer flex size-4 shrink-0 items-center justify-center rounded-[4px] border shadow-xs transition-shadow outline-none focus-visible:ring-[3px] disabled:cursor-not-allowed disabled:opacity-50",props.class)}>
 <span data-slot="checkbox-indicator" class="text-current transition-none">
 <Show when={checked() && !props.indeterminate}><svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m20 6-11 11-5-5"/></svg></Show>
 <Show when={props.indeterminate}><svg class="size-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12h14"/></svg></Show>
 </span></button>;
}
export {Root as Checkbox};