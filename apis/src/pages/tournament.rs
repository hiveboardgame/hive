use crate::common::{markdown_to_html, ScheduleAction, TournamentAction};
use crate::components::{
    atoms::progress_bar::ProgressBar,
    molecules::{
        game_previews::GamePreviews, my_schedules::MySchedules, time_row::TimeRow,
        unplayed_game_row::UnplayedGameRow, user_row::UserRow,
    },
    organisms::{
        chat::ChatWindow, standings::Standings, tournament_admin::TournamentAdminControls,
    },
};
use crate::providers::ApiRequestsProvider;
use crate::providers::{
    navigation_controller::NavigationControllerSignal, tournaments::TournamentStateContext, AuthContext,
};
use crate::responses::{GameResponse, TournamentResponse};
use chrono::Local;
use hive_lib::GameStatus;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use shared_types::{
    Conclusion, GameSpeed, PrettyString, TimeInfo, TournamentGameResult, TournamentStatus,
};
use std::collections::HashMap;

const DETAILS_STYLE: &str = "m-2 min-w-[320px]";
pub const INFO_STYLE: &str = "m-2 h-6 text-lg font-bold sm:place-self-center";
pub const BUTTON_STYLE: &str = "flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";

#[component]
pub fn Tournament() -> impl IntoView {
    let navi = expect_context::<NavigationControllerSignal>();
    let tournaments = expect_context::<TournamentStateContext>();
    let tournament_id = move || navi.tournament_signal.get().tournament_id;
    let current_tournament = move || {
        tournament_id().and_then(|tournament_id| {
            tournaments
                .full
                .get()
                .tournaments
                .get(&tournament_id)
                .cloned()
        })
    };

    view! {
        <div class="flex flex-col justify-center items-center pt-20 w-full">
            <div class="container flex flex-col items-center w-full">
                <Show when=move || current_tournament().is_some()>
                    <LoadedTournament tournament=Signal::derive(move || {
                        current_tournament().expect("Current tournament is some")
                    }) />
                </Show>
            </div>
        </div>
    }
}

#[component]
fn LoadedTournament(tournament: Signal<TournamentResponse>) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let account = move || match auth_context.user.get() {
        Some(Ok(account)) => Some(account),
        _ => None,
    };
    let user_id = Signal::derive(move || account().map(|a| a.user.uid));
    let time_info = Signal::derive(move || {
        let tournament = tournament();
        TimeInfo {
            mode: tournament.time_mode,
            base: tournament.time_base,
            increment: tournament.time_increment,
        }
    });
    let tournament_id = Memo::new(move |_| tournament().tournament_id);
    Effect::new(move |_| {
        if tournament().status != TournamentStatus::NotStarted {
            let api = api.get();
            api.schedule_action(ScheduleAction::TournamentPublic(tournament_id()));
            if user_id().is_some() {
                api.schedule_action(ScheduleAction::TournamentOwn(tournament_id()));
            }
        }
    });

    let games_hashmap = Memo::new(move |_| {
        let mut games_hashmap = HashMap::new();
        if tournament().status != TournamentStatus::NotStarted {
            for game in tournament().games {
                games_hashmap.insert(game.game_id.clone(), game);
            }
        }
        games_hashmap
    });

    let number_of_players = Memo::new(move |_| tournament().players.len() as i32);
    let user_joined = move || {
        if let Some(account) = account() {
            tournament().players.iter().any(|(id, _)| *id == account.id)
        } else {
            false
        }
    };

    let user_is_organizer = Signal::derive(move || {
        if let Some(account) = account() {
            tournament().organizers.iter().any(|p| p.uid == account.id)
        } else {
            false
        }
    });
    let send_action = move |action: TournamentAction| {
        let api = api.get();
        api.tournament(action);
    };
    let delete = move |_| {
        if user_is_organizer() {
            send_action(TournamentAction::Delete(tournament_id()));
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let finish = move |_| {
        if user_is_organizer() {
            send_action(TournamentAction::Finish(tournament_id()));
        }
    };
    let start = move |_| {
        if user_is_organizer() {
            send_action(TournamentAction::Start(tournament_id()));
        }
    };
    let start_disabled = move || tournament().min_seats > number_of_players();
    let join_disabled = move || {
        let tournament = tournament();
        if tournament.seats <= number_of_players() {
            return true;
        }
        if let Some(account) = account() {
            let user = account.user;
            if tournament.invite_only {
                if tournament
                    .invitees
                    .iter()
                    .any(|invitee| invitee.uid == user.uid)
                {
                    return false;
                }
                if tournament
                    .organizers
                    .iter()
                    .any(|organizer| organizer.uid == user.uid)
                {
                    return false;
                }
                return true;
            }
            let game_speed =
                GameSpeed::from_base_increment(tournament.time_base, tournament.time_increment);
            let rating = user.rating_for_speed(&game_speed) as i32;
            match (tournament.band_lower, tournament.band_upper) {
                (None, None) => false,
                (None, Some(upper)) => rating >= upper,
                (Some(lower), None) => rating <= lower,
                (Some(lower), Some(upper)) => rating <= lower || rating >= upper,
            }
        } else {
            true
        }
    };

    let starts = move || {
        let tournament = tournament();
        if matches!(tournament.status, TournamentStatus::NotStarted) {
            match tournament.starts_at {
                None => "Start up to organizer".to_string(),
                Some(time) => time
                    .with_timezone(&Local)
                    .format("Starts: %d/%m/%Y %H:%M")
                    .to_string(),
            }
        } else {
            let pretty = tournament.status.pretty_string();
            if let Some(started_at) = tournament.started_at {
                let start = started_at
                    .with_timezone(&Local)
                    .format("started: %d/%m/%Y %H:%M")
                    .to_string();
                format!("{pretty}, {start}")
            } else {
                pretty
            }
        }
    };
    let total_games = move || tournament().games.len();
    let finished_games =
        Signal::derive(move || tournament().games.iter().filter(|g| g.finished).count());
    let not_started = Memo::new(move |_| tournament().status == TournamentStatus::NotStarted);
    let inprogress = Memo::new(move |_| tournament().status == TournamentStatus::InProgress);
    let finished = Memo::new(move |_| tournament().status == TournamentStatus::Finished);

    let game_previews = Callback::new(move |()| {
        games_hashmap
            .get()
            .iter()
            .filter_map(|(_, g)| match g.game_status {
                GameStatus::NotStarted => None,
                _ => Some(g.clone()),
            })
            .collect::<Vec<GameResponse>>()
    });
    let markdown_desc = move || markdown_to_html(&tournament().description);
    let unplayed_games = Memo::new(move |_| {
        let mut result = tournament()
            .games
            .into_iter()
            .filter(|g| {
                g.conclusion == Conclusion::Unknown
                    || g.conclusion == Conclusion::Committee
                    || g.conclusion == Conclusion::Forfeit
            })
            .collect::<Vec<GameResponse>>();
        result.sort();
        result
    });
    let tournament_lacks_results = move || {
        tournament()
            .games
            .iter()
            .any(|e| (e.tournament_game_result == TournamentGameResult::Unknown))
    };
    let unplayed_games_string = move || {
        let nr = unplayed_games().len();
        if nr == 1 {
            String::from("1 unplayed game")
        } else {
            format!("{nr} unplayed games")
        }
    };
    let tournament_style = move || {
        if not_started() {
            "flex flex-col gap-1 w-full items-center justify-center"
        } else {
            "flex flex-col gap-1 w-full sm:flex-row sm:flex-wrap justify-center"
        }
    };
    view! {
        <div class="flex flex-col items-center p-2 w-full">
            <h1 class="w-full max-w-full text-3xl font-bold text-center whitespace-normal break-words">
                {move || tournament().name}
            </h1>
            <div class="p-4 w-full break-words prose dark:prose-invert" inner_html=markdown_desc />
        </div>
        <div class=tournament_style>
            <div class="m-2 h-fit">
                <div class=INFO_STYLE>Tournament Info</div>
                <div class="flex gap-1 m-2">
                    <span class="font-bold">"Time control: "</span>
                    <TimeRow time_info=time_info.into() />
                </div>
                <div class="m-2">
                    <span class="font-bold">"Players: "</span>
                    {number_of_players}
                    /
                    {move || tournament().seats}
                </div>
                <Show when=not_started>
                    <div class="m-2">
                        <span class="font-bold">"Minimum players: "</span>
                        {move || tournament().min_seats}
                    </div>
                </Show>
                <div class="m-2 font-bold">{starts}</div>
                <div class="flex flex-col m-2">
                    <div class="flex flex-col items-center mb-2">
                        <p class="font-bold">Organized by:</p>
                        <For
                            each=move || { tournament().organizers }

                            key=|users| (users.uid)
                            let:user
                        >
                            <div>
                                <UserRow actions=vec![] user=StoredValue::new(user) />
                            </div>
                        </For>
                    </div>
                </div>
                <ProgressBar current=finished_games total=total_games() />
                <Show when=not_started>
                    <div class="flex gap-1 justify-center items-center pb-2">
                        <Show
                            when=user_joined
                            fallback=move || {
                                view! {
                                    <button
                                        prop:disabled=join_disabled
                                        class=BUTTON_STYLE
                                        on:click=move |_| send_action(
                                            TournamentAction::Join(tournament_id()),
                                        )
                                    >
                                        Join
                                    </button>
                                }
                            }
                        >

                            <button
                                class=BUTTON_STYLE
                                on:click=move |_| send_action(
                                    TournamentAction::Leave(tournament_id()),
                                )
                            >
                                Leave
                            </button>
                        </Show>
                        <Show when=user_is_organizer>
                            <button class=BUTTON_STYLE on:click=delete>
                                {"Delete"}
                            </button>
                            <button prop:disabled=start_disabled class=BUTTON_STYLE on:click=start>
                                {"Start"}
                            </button>
                        </Show>
                    </div>
                    <TournamentAdminControls
                        user_is_organizer=user_is_organizer()
                        tournament=tournament()
                    />
                </Show>
                <Show when=user_is_organizer>
                    <div class="flex gap-1 justify-center items-center p-2">
                        <Show when=inprogress>
                            <button
                                class=BUTTON_STYLE
                                on:click=finish
                                prop:disabled=tournament_lacks_results
                            >
                                {"Finish"}
                            </button>
                        </Show>
                    </div>
                </Show>
            </div>
            <Show when=move || !not_started()>
                <Standings tournament />
            </Show>
        </div>
        <div class="flex flex-col flex-wrap gap-1 justify-center mx-auto w-full sm:flex-row">
            <MySchedules games_hashmap user_id />
            <Show when=move || !unplayed_games().is_empty()>
                <details class=DETAILS_STYLE>
                    <summary class=INFO_STYLE>{unplayed_games_string}</summary>
                    <For
                        each=move || unplayed_games()

                        key=|game| (
                            game.game_id.clone(),
                            game.tournament_game_result.clone(),
                            game.finished,
                        )
                        let:game
                    >

                        <UnplayedGameRow
                            game
                            user_is_organizer=user_is_organizer.into()
                            tournament_finished=finished.into()
                        />

                    </For>
                </details>
            </Show>
            <Show when=move || !game_previews.run(()).is_empty()>
                <details class=DETAILS_STYLE>
                    <summary class=INFO_STYLE>Finished or ongoing games:</summary>
                    <GamePreviews games=game_previews />
                </details>
            </Show>
        </div>
        <Show when=move || user_is_organizer() || user_joined()>
            <div class="p-3 m-2 w-full max-w-full h-60 whitespace-normal break-words sm:w-2/3 bg-even-light dark:bg-even-dark">
                <ChatWindow destination=shared_types::SimpleDestination::Tournament />
            </div>
        </Show>
    }
}
