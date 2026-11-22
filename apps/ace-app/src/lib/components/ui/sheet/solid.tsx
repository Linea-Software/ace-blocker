import {makeModal,sheetVariants,type Side} from "../solid-modal";
const C=makeModal("sheet");
export const Root=C.Root,Trigger=C.Trigger,Close=C.Close,Portal=C.Portal,Overlay=C.Overlay,Content=C.Content,Header=C.Header,Footer=C.Footer,Title=C.Title,Description=C.Description;
export {sheetVariants,type Side,Root as Sheet,Trigger as SheetTrigger,Close as SheetClose,Portal as SheetPortal,Overlay as SheetOverlay,Content as SheetContent,Header as SheetHeader,Footer as SheetFooter,Title as SheetTitle,Description as SheetDescription};