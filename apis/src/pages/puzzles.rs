use crate::components::{
    layouts::{page_header::PageHeader, page_shell::PageShell},
    molecules::empty_state::EmptyState,
};
use leptos::prelude::*;

#[component]
pub fn Puzzles() -> impl IntoView {
    view! {
        <PageShell>
            <PageHeader title="Puzzles" />
            <EmptyState
                title="Puzzles page coming soon"
                message="This section will host Hive tactics and training positions."
            />
        </PageShell>
    }
}
