import type {JSX} from "@solidjs/web";
import {makeModal} from "../solid-modal";
import {buttonVariants} from "../button/solid";
import {cn} from "$lib/utils.js";
const C=makeModal("alert-dialog");
export const Root=C.Root,Trigger=C.Trigger,Portal=C.Portal,Overlay=C.Overlay,Content=C.Content,Header=C.Header,Footer=C.Footer,Title=C.Title,Description=C.Description;
export function Action(props:JSX.ButtonHTMLAttributes<HTMLButtonElement>){return <C.Action {...props} class={cn(buttonVariants(),props.class)} />}
export function Cancel(props:JSX.ButtonHTMLAttributes<HTMLButtonElement>){return <C.Cancel {...props} class={cn(buttonVariants({variant:"outline"}),props.class)} />}
export {Root as AlertDialog,Trigger as AlertDialogTrigger,Portal as AlertDialogPortal,Overlay as AlertDialogOverlay,Content as AlertDialogContent,Header as AlertDialogHeader,Footer as AlertDialogFooter,Title as AlertDialogTitle,Description as AlertDialogDescription,Action as AlertDialogAction,Cancel as AlertDialogCancel};