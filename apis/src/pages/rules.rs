use crate::components::{
    layouts::{page_header::PageHeader, page_shell::PageShell},
    molecules::empty_state::EmptyState,
};
use leptos::prelude::*;

#[component]
pub fn Rules() -> impl IntoView {
    view! {
        <PageShell>
            <PageHeader title="Rules" />
            <EmptyState
                title="Rules page coming soon"
                message="Use the official rules PDF from the Learn menu while this page is being built."
            />
        </PageShell>
    }
}
