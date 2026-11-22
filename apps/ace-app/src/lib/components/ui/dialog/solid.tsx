import { makeModal } from "../solid-modal";
const C = makeModal("dialog");
export const Root = C.Root,
    Trigger = C.Trigger,
    Close = C.Close,
    Portal = C.Portal,
    Overlay = C.Overlay,
    Content = C.Content,
    Header = C.Header,
    Footer = C.Footer,
    Title = C.Title,
    Description = C.Description;
export {
    Root as Dialog,
    Trigger as DialogTrigger,
    Close as DialogClose,
    Portal as DialogPortal,
    Overlay as DialogOverlay,
    Content as DialogContent,
    Header as DialogHeader,
    Footer as DialogFooter,
    Title as DialogTitle,
    Description as DialogDescription,
};
