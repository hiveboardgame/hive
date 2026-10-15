use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{empty_state::EmptyState, page_card::PageCard},
    },
    i18n::*,
};
use leptos::prelude::*;

const ALLOWED_LOCALES: [Locale; 6] = [
    Locale::en,
    Locale::es,
    Locale::ca,
    Locale::it,
    Locale::fr,
    Locale::pt,
];
#[component]
pub fn RulesSummary() -> impl IntoView {
    let i18n = use_i18n();
    let is_allowed = Signal::derive(move || ALLOWED_LOCALES.contains(&i18n.get_locale()));
    view! {
        <PageShell>
            <PageHeader title=move || t_string!(i18n, header.learn.rules_summary) />
            <Show
                when=is_allowed
                fallback=move || {
                    view! {
                        <EmptyState
                            title=move || {
                                t_string!(
                                    i18n, common.learn.rules_summary.rules_summary_unavailable
                                )
                            }
                            message=move || {
                                t_string!(
                                    i18n, common.learn.rules_summary.this_language_does_not_have_a_rules_summary_yet
                                )
                            }
                        />
                    }
                }
            >

                <PageCard class="overflow-hidden p-2">
                    <img
                        src=Signal::derive(move || {
                            let locale = i18n.get_locale().to_string();
                            format!("/assets/rules_summary/{locale}.png")
                        })

                        alt=move || t_string!(i18n, header.learn.rules_summary)
                        class="w-full"
                    />
                </PageCard>
            </Show>
        </PageShell>
    }
}
