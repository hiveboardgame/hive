use crate::{
    common::TournamentAction,
    hooks::arena_clock::{format_time_left, use_arena_time_left},
    providers::{ApiRequestsProvider, AuthContext},
    responses::TournamentResponse,
};
use leptos::prelude::*;
use shared_types::{ArenaEventKind, Tiebreaker, TournamentStatus};

/// Joining, stepping out of, and leaving a running arena.
///
/// An arena is the one format a player manages while it is under way: it admits
/// newcomers for as long as its clock runs, and pairs whoever is in the pool on
/// every tick, so "I need a break" has to be expressible without forfeiting.
#[component]
pub fn ArenaControls(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;

    let user_id = Signal::derive(move || auth_context.user.with(|a| a.as_ref().map(|u| u.id)));
    let is_running_arena = Signal::derive(move || {
        tournament.with(|t| t.mode == "Arena" && t.status == TournamentStatus::InProgress)
    });
    let is_entrant = Signal::derive(move || match user_id.get() {
        Some(id) => tournament.with(|t| t.players.contains_key(&id)),
        None => false,
    });

    let time_left = use_arena_time_left(
        Signal::derive(move || tournament.with(|t| t.started_at)),
        Signal::derive(move || tournament.with(|t| t.arena_duration_seconds)),
    );

    // What the player has to show for the arena so far. Read from the standings
    // the engine already computed rather than counted here.
    let own_points = Signal::derive(move || {
        let id = user_id.get()?;
        tournament.with(|t| t.standings.score(id, Tiebreaker::RawPoints))
    });
    let own_games = Signal::derive(move || {
        let id = user_id.get()?;
        tournament.with(|t| {
            t.standings
                .players()
                .find(|standing| standing.player == id)
                .map(|standing| standing.games_played)
        })
    });
    let own_position = Signal::derive(move || {
        let id = user_id.get()?;
        tournament.with(|t| t.standings.position_of(id))
    });

    let send = move |action: TournamentAction| {
        move |_| {
            api.get().tournament(action.clone());
        }
    };
    let tournament_id = Signal::derive(move || tournament.with(|t| t.tournament_id.clone()));

    view! {
        <Show when=is_running_arena>
            <div class="flex flex-wrap gap-y-2 gap-x-4 justify-between items-center p-3 w-full ui-setting-group">
                <div class="flex flex-col">
                    <span class="text-xs tracking-tight text-gray-600 uppercase dark:text-gray-400">
                        "Time left"
                    </span>
                    <span class="text-lg font-bold tabular-nums text-gray-900 dark:text-gray-100">
                        {move || {
                            time_left.get().map(format_time_left).unwrap_or_else(|| "over".into())
                        }}
                    </span>
                </div>

                <Show when=move || is_entrant.get() && own_games.get().is_some()>
                    <div class="flex flex-col">
                        <span class="text-xs tracking-tight text-gray-600 uppercase dark:text-gray-400">
                            "You"
                        </span>
                        <span class="text-sm text-gray-900 dark:text-gray-100">
                            {move || {
                                let position = own_position
                                    .get()
                                    .map(|position| format!("#{position}"))
                                    .unwrap_or_default();
                                let points = own_points.get().unwrap_or(0.0);
                                let games = own_games.get().unwrap_or(0);
                                format!("{position} · {points} pts · {games} games")
                            }}
                        </span>
                    </div>
                </Show>

                <div class="flex gap-1 items-center">
                    <Show
                        when=is_entrant
                        fallback=move || {
                            view! {
                                <button
                                    class="ui-button ui-button-primary ui-button-sm"
                                    on:click=send(
                                        TournamentAction::JoinArena(tournament_id.get_untracked()),
                                    )
                                >
                                    "Join"
                                </button>
                            }
                        }
                    >
                        // Pause and resume are both offered: the engine refuses
                        // whichever does not apply, and the player's current
                        // state is not carried on the tournament response.
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            title="Stop being paired. A game already under way still counts."
                            on:click=send(
                                TournamentAction::ArenaBreak(
                                    tournament_id.get_untracked(),
                                    ArenaEventKind::Pause,
                                ),
                            )
                        >
                            "Pause"
                        </button>
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            title="Go back into the pairing pool"
                            on:click=send(
                                TournamentAction::ArenaBreak(
                                    tournament_id.get_untracked(),
                                    ArenaEventKind::Resume,
                                ),
                            )
                        >
                            "Resume"
                        </button>
                        <button
                            class="ui-button ui-button-danger ui-button-sm"
                            title="Leave for good. What you have scored still counts."
                            on:click=send(
                                TournamentAction::ArenaBreak(
                                    tournament_id.get_untracked(),
                                    ArenaEventKind::Withdraw,
                                ),
                            )
                        >
                            "Leave"
                        </button>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
