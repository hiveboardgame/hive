use crate::{
    components::organisms::{
        tournament_completion_summary::TournamentCompletionSummary,
        tournament_inspector::TournamentSelection,
        tournament_tv::TournamentTv,
    },
    providers::{
        games::GamesSignal,
        ArenaStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatStore,
        TournamentState,
    },
};
use leptos::prelude::*;
use shared_types::TournamentStatus;

#[component]
pub(crate) fn TournamentOverviewRail(
    tournament: TournamentState,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let games = expect_context::<GamesSignal>();
    view! {
        {move || {
            match tournament.common.lifecycle().get().status {
                TournamentStatus::NotStarted => ().into_any(),
                TournamentStatus::InProgress => {
                    match tournament.format {
                        TournamentFormatStore::Arena(arena) => {
                            view! {
                                <div class="min-w-0">
                                    <TournamentTv common=tournament.common arena games selection />
                                </div>
                            }
                                .into_any()
                        }
                        _ => ().into_any(),
                    }
                }
                TournamentStatus::Finished => {
                    let continuing_games = match tournament.format {
                        TournamentFormatStore::Arena(arena) => {
                            arena
                                .games()
                                .get()
                                .into_values()
                                .filter(|game| !game.game.finished)
                                .collect::<Vec<_>>()
                        }
                        _ => Vec::new(),
                    };
                    view! {
                        <section class="p-4 min-w-0 ui-panel">
                            <TournamentCompletionSummary tournament />
                            {(!continuing_games.is_empty())
                                .then(|| {
                                    view! {
                                        <div class="pt-4 mt-4 space-y-2 border-t border-black/10 dark:border-white/10">
                                            // TODO: i18n once copy is approved.
                                            <p class="text-sm text-gray-600 dark:text-gray-300">
                                                "Standings are frozen. Games still in progress no longer affect them."
                                            </p>
                                            <div class="flex flex-wrap gap-2">
                                                {continuing_games
                                                    .into_iter()
                                                    .map(|game| {
                                                        let memberships = tournament.common.memberships().get();
                                                        let names = game
                                                            .game
                                                            .participants
                                                            .map(|id| {
                                                                memberships
                                                                    .players
                                                                    .get(&id)
                                                                    .map(|player| player.username.clone())
                                                                    .unwrap_or_else(|| id.to_string())
                                                            });
                                                        view! {
                                                            // TODO: i18n once copy is approved.
                                                            <a
                                                                class="ui-button ui-button-secondary ui-button-sm"
                                                                href=format!("/game/{}", game.game.game_id.0)
                                                            >
                                                                {format!("{} vs {}", names[0], names[1])}
                                                            </a>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </div>
                                        </div>
                                    }
                                })}
                        </section>
                    }
                        .into_any()
                }
            }
        }}
    }
}
