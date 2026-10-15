use crate::i18n::{t_string, use_i18n};
use leptos::prelude::*;
use shared_types::{GameSpeed, LeaderboardKind};

use crate::components::{
    layouts::{
        page_header::PageHeader,
        page_shell::{PageShell, PageShellVariant},
    },
    organisms::leaderboard::Leaderboard,
};

#[component]
fn LeaderboardPage(
    #[prop(into)] title: TextProp,
    #[prop(into)] subtitle: TextProp,
    kind: LeaderboardKind,
) -> impl IntoView {
    let boards = GameSpeed::all_rated_games()
        .into_iter()
        .map(|speed| {
            view! { <Leaderboard speed=speed kind=kind /> }
        })
        .collect_view();
    view! {
        <PageShell variant=PageShellVariant::Dashboard>
            <div class="flex flex-col gap-6 mx-auto w-full max-w-[114rem]">
                <PageHeader title=title subtitle=subtitle />
                <div class="flex flex-col flex-wrap gap-3 items-center w-full md:flex-row md:items-start">
                    {boards}
                </div>
            </div>
        </PageShell>
    }
}

#[component]
pub fn TopPlayers() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <LeaderboardPage
            title=move || t_string!(i18n, profile.leaderboards.top_rated_players)
            subtitle=move || t_string!(i18n, profile.leaderboards.highest_rated_players_by_speed)
            kind=LeaderboardKind::Humans
        />
    }
}

#[component]
pub fn TopBots() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <LeaderboardPage
            title=move || t_string!(i18n, profile.leaderboards.top_rated_bots)
            subtitle=move || t_string!(i18n, profile.leaderboards.highest_rated_bots_by_speed)
            kind=LeaderboardKind::Bots
        />
    }
}
