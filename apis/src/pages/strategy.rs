use crate::components::{
    layouts::{page_header::PageHeader, page_shell::PageShell},
    molecules::empty_state::EmptyState,
};
use leptos::prelude::*;

#[component]
pub fn Strategy() -> impl IntoView {
    view! {
        <PageShell>
            <PageHeader title="Strategy" />
            <EmptyState
                title="Strategy page coming soon"
                message="This section will collect learning material and practical Hive guidance."
            />
        </PageShell>
    }
}
