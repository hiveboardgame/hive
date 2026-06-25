use leptos::prelude::*;
use shared_types::GameSpeed;

use crate::components::{
    layouts::{
        page_header::PageHeader,
        page_shell::{PageShell, PageShellVariant},
    },
    organisms::leaderboard::Leaderboard,
};

#[component]
pub fn TopPlayers() -> impl IntoView {
    let leaderboards = GameSpeed::all_rated_games()
        .into_iter()
        .map(|speed| {
            view! { <Leaderboard speed=speed /> }
        })
        .collect_view();
    view! {
        <PageShell variant=PageShellVariant::Dashboard>
            <div class="flex flex-col gap-6 mx-auto w-full max-w-[114rem]">
                <PageHeader title="Top Rated Players" subtitle="Highest rated players by speed." />
                <div class="flex flex-col flex-wrap gap-3 items-center w-full md:flex-row md:items-start">
                    {leaderboards}
                </div>
            </div>
        </PageShell>
    }
}
