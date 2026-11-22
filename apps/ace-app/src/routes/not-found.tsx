import { ArrowLeft } from "@lucide/solid";
import PageLayout from "$lib/components/PageLayout";
import PageHeader from "$lib/components/PageHeader";

/** Keep unmatched desktop routes recoverable rather than rendering a blank shell. */
export default function NotFoundPage() {
    return (
        <PageLayout
            width="narrow"
            header={
                <PageHeader
                    title="Page not found"
                    subtitle="The requested Ace Blocker page does not exist."
                />
            }
        >
            <a
                href="/"
                class="inline-flex items-center gap-2 rounded-sm border border-border bg-card px-4 py-2.5 text-sm font-semibold text-foreground hover:bg-muted focus-visible:outline-2 focus-visible:outline-ring"
            >
                <ArrowLeft size={16} aria-hidden="true" />
                Back to Control
            </a>
        </PageLayout>
    );
}
