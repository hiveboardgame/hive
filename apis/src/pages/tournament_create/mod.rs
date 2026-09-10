mod arena;
mod common;
mod elimination;
mod round_robin;
mod swiss;

use crate::{common::render_text_prop, components::layouts::page_shell::PageShell, i18n::*};
use arena::ArenaCreationForm;
use elimination::EliminationCreationForm;
use leptos::prelude::*;
use round_robin::RoundRobinCreationForm;
use swiss::SwissCreationForm;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TournamentCreationKind {
    Arena,
    Swiss,
    RoundRobin,
    Elimination,
}

#[component]
pub fn TournamentCreate(kind: TournamentCreationKind) -> impl IntoView {
    match kind {
        TournamentCreationKind::Arena => view! { <ArenaCreationForm /> }.into_any(),
        TournamentCreationKind::Swiss => view! { <SwissCreationForm /> }.into_any(),
        TournamentCreationKind::RoundRobin => view! { <RoundRobinCreationForm /> }.into_any(),
        TournamentCreationKind::Elimination => view! { <EliminationCreationForm /> }.into_any(),
    }
}

#[component]
pub fn TournamentCreateChooser() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <PageShell>
            <div class="mx-auto space-y-6 w-full max-w-4xl">
                <div class="flex flex-col gap-1 text-center">
                    <h1 class="ui-page-title">{t!(i18n, tournaments.creation.title)}</h1>
                    <p class="ui-page-subtitle">{t!(i18n, tournaments.creation.choose_format)}</p>
                </div>
                <div class="grid gap-4 sm:grid-cols-2">
                    <CreationTypeCard
                        href="/tournaments/create/arena"
                        title=move || t_string!(i18n, tournaments.format.arena)
                        description=move || t_string!(i18n, tournaments.creation.arena_description)
                    />
                    <CreationTypeCard
                        href="/tournaments/create/swiss"
                        title=move || t_string!(i18n, tournaments.creation.swiss_title)
                        description=move || t_string!(i18n, tournaments.creation.swiss_description)
                    />
                    <CreationTypeCard
                        href="/tournaments/create/round-robin"
                        title=move || t_string!(i18n, tournaments.format.round_robin)
                        description=move || {
                            t_string!(i18n, tournaments.creation.round_robin_description)
                        }
                    />
                    <CreationTypeCard
                        href="/tournaments/create/elimination"
                        title=move || t_string!(i18n, tournaments.creation.elimination_title)
                        description=move || {
                            t_string!(i18n, tournaments.creation.elimination_description)
                        }
                    />
                </div>
            </div>
        </PageShell>
    }
}

#[component]
fn CreationTypeCard(
    href: &'static str,
    #[prop(into)] title: TextProp,
    #[prop(into)] description: TextProp,
) -> impl IntoView {
    view! {
        <a href=href class="block p-6 space-y-2 no-link-style ui-card-row">
            <h2 class="text-xl font-bold text-gray-900 dark:text-gray-100">
                {render_text_prop(title)}
            </h2>
            <p class="text-sm text-gray-600 dark:text-gray-300">{render_text_prop(description)}</p>
        </a>
    }
}
