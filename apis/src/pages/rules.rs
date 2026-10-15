use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::empty_state::EmptyState,
    },
    i18n::{t_string, use_i18n},
};
use leptos::prelude::*;

#[component]
pub fn Rules() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <PageShell>
            <PageHeader title=move || t_string!(i18n, header.learn.rules) />
            <EmptyState
                title=move || t_string!(i18n, common.learn.rules.rules_page_coming_soon)
                message=move || {
                    t_string!(
                        i18n, common.learn.rules.use_the_official_rules_pdf_from_the_learn_menu
                    )
                }
            />
        </PageShell>
    }
}
