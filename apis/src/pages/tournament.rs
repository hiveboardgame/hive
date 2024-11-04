use crate::common::{ScheduleAction, TournamentAction};
use crate::components::{
    atoms::progress_bar::ProgressBar,
    molecules::{
        game_previews::GamePreviews, myschedules::MySchedules, pending_game_row::PendingGameRow,
        time_row::TimeRow, user_row::UserRow,
    },
    organisms::{
        chat::ChatWindow, standings::Standings, tournament_admin::TournamentAdminControls,
    },
};
use crate::providers::{
    navigation_controller::NavigationControllerSignal, schedules::SchedulesContext,
    tournaments::TournamentStateContext, ApiRequests, AuthContext,
};
use crate::responses::{GameResponse, TournamentResponse};
use chrono::{DateTime, Duration, Local, Utc};
use hive_lib::GameStatus;
use leptos::*;
use leptos_router::use_navigate;
use shared_types::{GameId, PrettyString};
use shared_types::{GameSpeed, TimeInfo, TournamentStatus};
use std::collections::HashMap;

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";

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
    let account = move || match (auth_context.user)() {
        Some(Ok(Some(account))) => Some(account),
        _ => None,
    };
    let user_id = Signal::derive(move || account().map(|a| a.user.uid));
    let schedules_signal = expect_context::<SchedulesContext>();
    let tournament_schedules = move || schedules_signal.tournament.get();
    let time_info = Signal::derive(move || {
        let tournament = tournament();
        TimeInfo {
            mode: tournament.time_mode,
            base: tournament.time_base,
            increment: tournament.time_increment,
        }
    });
    let tournament_id = Memo::new(move |_| tournament().tournament_id);
    create_effect(move |_| {
        if tournament().status != TournamentStatus::NotStarted {
            let api = ApiRequests::new();
            api.schedule_action(ScheduleAction::TournamentPublic(tournament_id()));
            if user_id().is_some() {
                api.schedule_action(ScheduleAction::TournamentOwn(tournament_id()));
            }
        }
    });

    let games_hashmap = Memo::new(move |_| {
        if tournament().status != TournamentStatus::NotStarted {
            let mut games_hashmap = HashMap::new();
            for game in tournament().games {
                games_hashmap.insert(game.game_id.clone(), game);
            }
            games_hashmap
        } else {
            HashMap::new()
        }
    });

    let number_of_players = Memo::new(move |_| tournament().players.len() as i32);
    let user_joined = move || {
        if let Some(account) = account() {
            tournament().players.iter().any(|(id, _)| *id == account.id)
        } else {
            false
        }
    };

    let user_is_organizer = move || {
        if let Some(account) = account() {
            tournament().organizers.iter().any(|p| p.uid == account.id)
        } else {
            false
        }
    };
    let delete = move |_| {
        if user_is_organizer() {
            let action = TournamentAction::Delete(tournament_id());
            let api = ApiRequests::new();
            api.tournament(action);
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let start = move |_| {
        if user_is_organizer() {
            let action = TournamentAction::Start(tournament_id());
            let api = ApiRequests::new();
            api.tournament(action);
        }
    };
    let leave = move |_| {
        let api = ApiRequests::new();
        api.tournament(TournamentAction::Leave(tournament_id()));
    };
    let join = move |_| {
        let api = ApiRequests::new();
        api.tournament(TournamentAction::Join(tournament_id()));
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

    let game_previews = Callback::new(move |_| {
        games_hashmap
            .get()
            .iter()
            .filter_map(|(_, g)| match g.game_status {
                GameStatus::NotStarted => None,
                _ => Some(g.clone()),
            })
            .collect::<Vec<GameResponse>>()
    });

    let pending_games = move || {
        let mut result: HashMap<GameId, (Option<DateTime<Utc>>, GameResponse)> = HashMap::new();
        games_hashmap().iter().for_each(|(game_id, game)| {
            if game.game_status == GameStatus::NotStarted {
                let mut should_insert_none = true;
                if let Some(schedules) = tournament_schedules().get(game_id) {
                    for schedule in schedules.values() {
                        if (schedule.start_t + Duration::hours(1)) > Utc::now() && schedule.agreed {
                            result.insert(game_id.clone(), (Some(schedule.start_t), game.clone()));
                            should_insert_none = false;
                        }
                    }
                }

                if should_insert_none {
                    result
                        .entry(game_id.clone())
                        .or_insert((None, game.clone()));
                }
            }
        });

        result
    };
    let pending_games_string = move || {
        let nr = pending_games().len();
        if nr == 1 {
            String::from("1 Pending game")
        } else {
            format!("{nr} Pending games")
        }
    };
    let tournament_style = move || {
        if not_started() {
            "flex flex-col gap-1 w-full items-center"
        } else {
            "flex flex-col gap-1 w-full sm:flex-row sm:flex-wrap"
        }
    };

    view! {
        <div class="flex flex-col justify-center p-2 w-full">
            <h1 class="w-full max-w-full text-3xl font-bold text-center whitespace-normal break-words">
                {move || tournament().name}
            </h1>
            <div class="w-full text-center whitespace-normal break-words">
                {move || tournament().description}
            </div>
        </div>
        <div class=tournament_style>

            <div class="m-2 h-fit">
                <div class="m-2 h-6 text-lg font-bold">Tournament Info</div>
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
                                <UserRow actions=vec![] user=store_value(user) />
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
                                        on:click=join
                                    >
                                        Join
                                    </button>
                                }
                            }
                        >

                            <button class=BUTTON_STYLE on:click=leave>
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
            </div>

            <Show when=move || !not_started()>
                <Standings tournament />
                <MySchedules games_hashmap user_id />
                <Show when=move || !pending_games().is_empty()>
                    <details class="m-2 min-w-[320px]">
                        <summary class="m-2 h-6 text-lg font-bold">{pending_games_string}</summary>
                        <For
                            each=move || pending_games().into_values()

                            key=|(time, game)| (time.to_owned(), game.game_id.clone())

                            let:tuple
                        >

                            {
                                let (schedule, game) = tuple;
                                view! { <PendingGameRow schedule game /> }
                            }

                        </For>
                    </details>
                </Show>
            </Show>
            <Show when=move || user_is_organizer() || user_joined()>
                <details class="flex items-center min-w-[320px] m-2 w-full">
                    <summary class="m-2 h-6 text-lg font-bold">Tournament chat</summary>
                    <div class="m-2 w-full h-60 whitespace-normal break-words bg-even-light dark:bg-even-dark">
                        <ChatWindow destination=shared_types::SimpleDestination::Tournament />
                    </div>
                </details>
            </Show>
            <Show when=move || !game_previews(()).is_empty()>
                <details class="min-w-[320px] m-2">
                    <summary class="m-2 h-6 text-lg font-bold">Finished or ongoing games:</summary>
                    <GamePreviews games=game_previews />
                </details>
            </Show>
        </div>
    }
}
