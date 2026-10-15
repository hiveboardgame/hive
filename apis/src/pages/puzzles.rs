use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::empty_state::EmptyState,
    },
    i18n::{t_string, use_i18n},
};
use leptos::prelude::*;

#[component]
pub fn Puzzles() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <PageShell>
            <PageHeader title=move || t_string!(i18n, common.learn.puzzles.puzzles) />
            <EmptyState
                title=move || t_string!(i18n, common.learn.puzzles.puzzles_page_coming_soon)
                message=move || {
                    t_string!(
                        i18n, common.learn.puzzles.this_section_will_host_hive_tactics_and_training_positions
                    )
                }
            />
        </PageShell>
    }
}
