import type {JSX} from "@solidjs/web";
import {createMemo,createSignal,createUniqueId,For,Show} from "solid-js";
import {CalendarDate} from "@internationalized/date";
import {cn} from "$lib/utils.js";
import {buttonVariants,type ButtonVariant} from "../button/solid";
type DateLike={year:number;month:number;day:number;};
type Div=JSX.HTMLAttributes<HTMLDivElement>;
type Props=Div&{type?:"single";value?:DateLike;defaultValue?:DateLike;onValueChange?:(v:CalendarDate)=>void;minValue?:DateLike;maxValue?:DateLike;placeholder?:DateLike;onPlaceholderChange?:(v:CalendarDate)=>void;locale?:string;monthFormat?:Intl.DateTimeFormatOptions["month"];yearFormat?:Intl.DateTimeFormatOptions["year"];weekdayFormat?:Intl.DateTimeFormatOptions["weekday"];captionLayout?:"label"|"dropdown"|"dropdown-months"|"dropdown-years";buttonVariant?:ButtonVariant;disableDaysOutsideMonth?:boolean;disabled?:boolean};
const cmp=(a:DateLike,b:DateLike)=>a.year!==b.year?a.year-b.year:a.month!==b.month?a.month-b.month:a.day-b.day;
const date=(d:DateLike)=>new CalendarDate(d.year,d.month,d.day);
const equal=(a:DateLike|undefined,b:DateLike)=>!!a&&cmp(a,b)===0;
const shift=(d:DateLike,n:number)=>date(d).add({days:n});
function cells(y:number,m:number){
 const first=new Date(y,m-1,1),lead=(first.getDay()+6)%7,days=new Date(y,m,0).getDate(),count=Math.ceil((lead+days)/7)*7;
 return Array.from({length:count},(_,i)=>{const d=new Date(y,m-1,1-lead+i);return new CalendarDate(d.getFullYear(),d.getMonth()+1,d.getDate())});
}
export function Root(props:Props){
 const locale=()=>props.locale??"en-US";
 const current=new Date();
 const [own,setOwn]=createSignal<DateLike|undefined>(props.defaultValue);
 const [view,setView]=createSignal({year:props.placeholder?.year??props.value?.year??current.getFullYear(),month:props.placeholder?.month??props.value?.month??current.getMonth()+1});
 const chosen=()=>props.value??own();
 const dates=createMemo(()=>cells(view().year,view().month));
 const weekdayNames=createMemo(()=>Array.from({length:7},(_,i)=>new Intl.DateTimeFormat(locale(),{weekday:props.weekdayFormat??"short"}).format(new Date(2024,0,1+i))));
 const label=createMemo(()=>new Intl.DateTimeFormat(locale(),{month:props.monthFormat??"long",year:props.yearFormat??"numeric"}).format(new Date(view().year,view().month-1,1)));
 const uid=createUniqueId(),isAllowed=(d:DateLike)=>!props.disabled&&(!props.minValue||cmp(d,props.minValue)>=0)&&(!props.maxValue||cmp(d,props.maxValue)<=0)&&(!props.disableDaysOutsideMonth||d.month===view().month);
 const navigate=(months:number)=>{const d=new Date(view().year,view().month-1+months,1);const next={year:d.getFullYear(),month:d.getMonth()+1};setView(next);props.onPlaceholderChange?.(new CalendarDate(next.year,next.month,1))};
 const select=(d:CalendarDate)=>{if(!isAllowed(d))return;setOwn(d);props.onValueChange?.(d)};
 const onDayKeyDown=(e:KeyboardEvent,d:CalendarDate)=>{let next:CalendarDate|undefined;if(e.key==="ArrowRight")next=shift(d,1);else if(e.key==="ArrowLeft")next=shift(d,-1);else if(e.key==="ArrowDown")next=shift(d,7);else if(e.key==="ArrowUp")next=shift(d,-7);else if(e.key==="Home")next=shift(d,-((new Date(d.year,d.month-1,d.day).getDay()+6)%7));else if(e.key==="End")next=shift(d,6-((new Date(d.year,d.month-1,d.day).getDay()+6)%7));else if(e.key==="PageDown")next=d.add({months:1});else if(e.key==="PageUp")next=d.subtract({months:1});if(!next)return;e.preventDefault();if(next.month!==view().month||next.year!==view().year){setView({year:next.year,month:next.month})}requestAnimationFrame(()=>document.querySelector<HTMLButtonElement>('[data-calendar-date="'+next!.year+'-'+next!.month+'-'+next!.day+'"]')?.focus())};
 return <div {...props} data-slot="calendar" role="group" aria-label={label()} class={cn("bg-background group/calendar p-3 [--cell-size:--spacing(8)] [[data-slot=card-content]_&]:bg-transparent [[data-slot=popover-content]_&]:bg-transparent",props.class)}>
  <div class="relative flex flex-col gap-4 md:flex-row"><div class="flex flex-col">
   <div class="flex h-(--cell-size) w-full items-center justify-center gap-1.5 text-sm font-medium">
    <Show when={props.captionLayout?.startsWith("dropdown")} fallback={<div class="px-(--cell-size) text-sm font-medium" aria-live="polite">{label()}</div>}>
     <select aria-label="Month" value={view().month} onChange={e=>navigate(Number(e.currentTarget.value)-view().month)} class="bg-transparent text-sm font-medium">{<For each={Array.from({length:12},(_,i)=>i+1)}>{m=><option value={m}>{new Intl.DateTimeFormat(locale(),{month:"long"}).format(new Date(2024,m-1,1))}</option>}</For>}</select>
     <select aria-label="Year" value={view().year} onChange={e=>navigate((Number(e.currentTarget.value)-view().year)*12)} class="bg-transparent text-sm font-medium">{<For each={Array.from({length:201},(_,i)=>current.getFullYear()-100+i)}>{y=><option value={y}>{y}</option>}</For>}</select>
    </Show>
   </div>
   <nav class="absolute inset-x-0 top-0 flex w-full items-center justify-between gap-1" aria-label="Calendar navigation">
    <button type="button" aria-label="Previous month" onClick={()=>navigate(-1)} class={cn(buttonVariants({variant:props.buttonVariant??"ghost"}),"size-(--cell-size) bg-transparent p-0 select-none disabled:opacity-50 rtl:rotate-180")}><svg class="size-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="m15 18-6-6 6-6"/></svg></button>
    <button type="button" aria-label="Next month" onClick={()=>navigate(1)} class={cn(buttonVariants({variant:props.buttonVariant??"ghost"}),"size-(--cell-size) bg-transparent p-0 select-none disabled:opacity-50 rtl:rotate-180")}><svg class="size-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="m9 18 6-6-6-6"/></svg></button>
   </nav>
   <table class="mt-4 w-full border-collapse" role="grid" aria-label={label()}><thead><tr class="flex select-none">{<For each={weekdayNames()}>{w=><th scope="col" class="text-muted-foreground w-(--cell-size) rounded-md text-sm font-normal">{w.slice(0,2)}</th>}</For>}</tr></thead>
    <tbody>{<For each={Array.from({length:Math.ceil(dates().length/7)},(_,i)=>i)}>{week=><tr class="mt-2 flex w-full">{<For each={dates().slice(week*7,(week+1)*7)}>{d=>{const selected=()=>equal(chosen(),d),today=new Date(),todayDate={year:today.getFullYear(),month:today.getMonth()+1,day:today.getDate()};return <td class="relative size-(--cell-size) p-0 text-center text-sm focus-within:z-20"><button type="button" id={uid+"-"+d.day} data-calendar-date={d.year+"-"+d.month+"-"+d.day} aria-label={new Intl.DateTimeFormat(locale(),{dateStyle:"full"}).format(new Date(d.year,d.month-1,d.day))} aria-selected={selected()?"true":"false"} data-bits-day data-selected={selected()||undefined} data-today={equal(todayDate,d)||undefined} data-outside-month={d.month!==view().month||undefined} data-disabled={!isAllowed(d)||undefined} disabled={!isAllowed(d)} onClick={()=>select(d)} onKeyDown={e=>onDayKeyDown(e,d)} class={cn(buttonVariants({variant:"ghost"}),"flex size-(--cell-size) flex-col items-center justify-center gap-1 p-0 leading-none font-normal whitespace-nowrap select-none [&[data-today]:not([data-selected])]:bg-accent [&[data-today]:not([data-selected])]:text-accent-foreground data-[selected]:bg-primary dark:data-[selected]:hover:bg-accent/50 data-[selected]:text-primary-foreground [&[data-outside-month]:not([data-selected])]:text-muted-foreground data-[disabled]:text-muted-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 dark:hover:text-accent-foreground focus:border-ring focus:ring-ring/50")}>{d.day}</button></td>}}</For>}</tr>}</For>}</tbody>
   </table>
  </div></div>
 </div>;
}
export const Calendar=Root;
export function Months(props:Div){return <div {...props} class={cn("relative flex flex-col gap-4 md:flex-row",props.class)}>{props.children}</div>}
export function Month(props:Div){return <div {...props} class={cn("flex flex-col",props.class)}>{props.children}</div>}
export function Header(props:Div){return <div {...props} class={cn("flex h-(--cell-size) w-full items-center justify-center gap-1.5 text-sm font-medium",props.class)}>{props.children}</div>}
export function Heading(props:Div){return <div {...props} class={cn("px-(--cell-size) text-sm font-medium",props.class)}>{props.children}</div>}
export function Nav(props:JSX.HTMLAttributes<HTMLElement>){return <nav {...props} class={cn("absolute inset-x-0 top-0 flex w-full items-center justify-between gap-1",props.class)}>{props.children}</nav>}
export function Grid(props:JSX.HTMLAttributes<HTMLTableElement>){return <table {...props} class={cn("mt-4 flex w-full border-collapse flex-col gap-1",props.class)}>{props.children}</table>}
export function GridHead(props:JSX.HTMLAttributes<HTMLTableSectionElement>){return <thead {...props}>{props.children}</thead>}
export function GridBody(props:JSX.HTMLAttributes<HTMLTableSectionElement>){return <tbody {...props}>{props.children}</tbody>}
export function GridRow(props:JSX.HTMLAttributes<HTMLTableRowElement>){return <tr {...props} class={cn("flex",props.class)}>{props.children}</tr>}
export function HeadCell(props:JSX.ThHTMLAttributes<HTMLTableCellElement>){return <th {...props} class={cn("text-muted-foreground w-(--cell-size) rounded-md text-sm font-normal",props.class)}>{props.children}</th>}
export function Cell(props:JSX.TdHTMLAttributes<HTMLTableCellElement>){return <td {...props} class={cn("relative size-(--cell-size) p-0 text-center text-sm focus-within:z-20",props.class)}>{props.children}</td>}
export function Day(props:JSX.ButtonHTMLAttributes<HTMLButtonElement>){return <button {...props} class={cn(buttonVariants({variant:"ghost"}),"flex size-(--cell-size) flex-col items-center justify-center gap-1 p-0 leading-none font-normal whitespace-nowrap select-none",props.class)}>{props.children}</button>}
export function PrevButton(props:JSX.ButtonHTMLAttributes<HTMLButtonElement>){return <button {...props} class={cn(buttonVariants({variant:"ghost"}),"size-(--cell-size) bg-transparent p-0 select-none disabled:opacity-50 rtl:rotate-180",props.class)}>{props.children??"‹"}</button>}
export function NextButton(props:JSX.ButtonHTMLAttributes<HTMLButtonElement>){return <button {...props} class={cn(buttonVariants({variant:"ghost"}),"size-(--cell-size) bg-transparent p-0 select-none disabled:opacity-50 rtl:rotate-180",props.class)}>{props.children??"›"}</button>}
export function MonthSelect(props:JSX.SelectHTMLAttributes<HTMLSelectElement>){return <select {...props}>{props.children}</select>}
export function YearSelect(props:JSX.SelectHTMLAttributes<HTMLSelectElement>){return <select {...props}>{props.children}</select>}
export function Caption(props:Div){return <div {...props}>{props.children}</div>}
