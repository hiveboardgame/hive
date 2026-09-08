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
    view! {
        <LeaderboardPage
            title="Top Rated Players"
            subtitle="Highest rated players by speed."
            kind=LeaderboardKind::Humans
        />
    }
}

#[component]
pub fn TopBots() -> impl IntoView {
    view! {
        <LeaderboardPage
            title="Top Rated Bots"
            subtitle="Highest rated bots by speed."
            kind=LeaderboardKind::Bots
        />
    }
}
