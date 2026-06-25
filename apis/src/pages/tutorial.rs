use crate::components::{
    layouts::{page_header::PageHeader, page_shell::PageShell},
    molecules::empty_state::EmptyState,
};
use leptos::prelude::*;

#[component]
pub fn Tutorial() -> impl IntoView {
    view! {
        <PageShell>
            <PageHeader title="Tutorial" />
            <EmptyState
                title="Tutorial page coming soon"
                message="This section will introduce the game flow and core interactions."
            />
        </PageShell>
    }
}
