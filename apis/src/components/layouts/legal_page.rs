use crate::{
    common::{GITHUB_ISSUES_URL, HIVEGAME_DISCORD_URL},
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
};
use leptos::prelude::*;

#[component]
pub fn LegalPage(
    title: &'static str,
    last_updated: &'static str,
    summary: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <PageShell attr:lang="en">
            <PageHeader title=title subtitle=format!("Last updated {last_updated}") />
            <p class="ui-notice">{summary}</p>
            {children()}
        </PageShell>
    }
}

#[component]
pub fn LegalSection(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <Panel
            title=title
            body_class="space-y-3 text-sm leading-6 text-gray-700 dark:text-gray-300"
        >
            {children()}
        </Panel>
    }
}

#[component]
pub fn LegalList(children: Children) -> impl IntoView {
    view! { <ul class="pl-5 space-y-2 list-disc">{children()}</ul> }
}

#[component]
pub fn ContactMaintainers() -> impl IntoView {
    view! {
        "message leex or iongler on our "
        <a class="ui-text-link" href=HIVEGAME_DISCORD_URL rel="external" target="_blank">
            "Discord server"
        </a>
        " or open an issue on "
        <a class="ui-text-link" href=GITHUB_ISSUES_URL rel="external" target="_blank">
            "GitHub"
        </a>
    }
}
