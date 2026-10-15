use crate::{
    components::organisms::{
        tournament_completion_summary::TournamentCompletionSummary,
        tournament_inspector::TournamentSelection,
        tournament_tv::TournamentTv,
    },
    i18n::*,
    providers::{
        games::GamesSignal,
        ArenaStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatState,
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
                        TournamentFormatState::Arena(arena) => {
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
                    let i18n = use_i18n();
                    let continuing_games = match tournament.format {
                        TournamentFormatState::Arena(arena) => {
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

                                            <p class="text-sm text-gray-600 dark:text-gray-300">
                                                {move || {
                                                    t_string!(
                                                        i18n, tournaments.ui.standings_are_frozen_games_still_in_progress_no_longer_affect_them
                                                    )
                                                        .to_string()
                                                }}
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
                                                            <a
                                                                class="ui-button ui-button-secondary ui-button-sm"
                                                                href=format!("/game/{}", game.game.game_id.0)
                                                            >
                                                                {move || {
                                                                    t_string!(
                                                                        i18n, common.versus_players, white = names[0].clone(), black = names[1].clone()
                                                                    )
                                                                        .to_string()
                                                                }}
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
