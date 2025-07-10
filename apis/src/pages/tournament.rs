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
    update_from_event::update_from_input,
};
use crate::functions::tournaments::{get_complete, UpdateDescription};
use crate::providers::AuthContext;
use crate::providers::{ApiRequestsProvider, UpdateNotifier, websocket::WebsocketContext};
use crate::responses::{GameResponse, TournamentResponse};
use chrono::Local;
use hive_lib::GameStatus;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_use::core::ConnectionReadyState;
use shared_types::{
    Conclusion, GameSpeed, PrettyString, TimeInfo, TournamentGameResult, TournamentId,
    TournamentStatus,
};
use std::collections::HashMap;

const DETAILS_STYLE: &str = "m-2 min-w-[320px]";
const BUTTON_STYLE: &str = "flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
pub const INFO_STYLE: &str = "m-2 h-6 text-lg font-bold sm:place-self-center";

#[component]
pub fn Tournament() -> impl IntoView {
    let use_params = use_params_map();
    let update_notification = expect_context::<UpdateNotifier>().tournament_update;
    let tournament_id = move || {
        use_params
            .get_untracked()
            .get("nanoid")
            .map(|s| TournamentId(s.to_string()))
    };
    let current_tournament = Action::new(move |_: &()| async move {
        get_complete(tournament_id().unwrap().to_string())
            .await
            .ok()
    });
    Effect::watch(
        update_notification,
        move |needs_update, _, _| {
            if Some(needs_update) == tournament_id().as_ref() {
                current_tournament.dispatch(());
            }
        },
        true,
    );
    Effect::new(move |_| {
        current_tournament.dispatch(());
    });
    view! {
        <div class="flex flex-col justify-center items-center pt-20 w-full">
            <div class="container flex flex-col items-center w-full">
                <Show when=move || current_tournament.value().get().is_some()>
                    <LoadedTournament tournament=current_tournament
                        .value()
                        .get()
                        .flatten()
                        .expect("Current tournament is some") />
                </Show>
            </div>
        </div>
    }
}

#[component]
fn LoadedTournament(tournament: TournamentResponse) -> impl IntoView {
    let tournament = Signal::derive(move || tournament.clone());
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let websocket = expect_context::<WebsocketContext>();
    let account = auth_context.user;
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
        let ready_state = websocket.ready_state.get();
        if tournament().status != TournamentStatus::NotStarted 
            && ready_state == ConnectionReadyState::Open {
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

    let user_is_organizer_or_admin = Signal::derive(move || {
        if let Some(account) = account() {
            account.user.admin || tournament().organizers.iter().any(|p| p.uid == account.id)
        } else {
            false
        }
    });
    let send_action = move |action: TournamentAction| {
        let api = api.get();
        api.tournament(action);
    };
    let delete = move |_| {
        if user_is_organizer_or_admin() {
            send_action(TournamentAction::Delete(tournament_id()));
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let finish = move |_| {
        if user_is_organizer_or_admin() {
            send_action(TournamentAction::Finish(tournament_id()));
        }
    };
    let start = move |_| {
        if user_is_organizer_or_admin() {
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
    let current_description = RwSignal::new(String::new());
    let markdown_desc = move || markdown_to_html(&current_description());
    let editing_description = RwSignal::new(false);
    let previewing_description = RwSignal::new(false);
    let description_text = RwSignal::new(String::new());
    let update_desc_action = ServerAction::<UpdateDescription>::new();

    // Initialize current description from tournament data
    Effect::new(move |_| {
        current_description.set(tournament().description.clone());
    });

    // Close edit mode and optimistically update description on successful submission
    Effect::new(move |_| {
        if let Some(Ok(())) = update_desc_action.value().get() {
            current_description.set(description_text.get_untracked());
            editing_description.set(false);
            previewing_description.set(false);
        }
    });
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
            <Show
                when=move || editing_description()
                fallback=move || {
                    view! {
                        <div
                            class="p-4 w-full break-words prose dark:prose-invert"
                            inner_html=markdown_desc
                        />
                        <Show when=user_is_organizer_or_admin>
                            <button
                                class="px-4 py-2 mb-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
                                on:click=move |_| {
                                    description_text.set(current_description.get_untracked());
                                    editing_description.set(true);
                                    previewing_description.set(false);
                                }
                            >
                                "Edit Description"
                            </button>
                        </Show>
                    }
                }
            >
                <ActionForm action=update_desc_action>
                    <input
                        type="hidden"
                        name="tournament_id"
                        value=move || tournament().tournament_id.0.clone()
                    />
                    <Show
                        when=move || previewing_description()
                        fallback=move || {
                            view! {
                                <textarea
                                    class="px-3 py-2 m-2 w-full h-32 leading-tight rounded border shadow appearance-none focus:outline-none"
                                    name="description"
                                    prop:value=description_text
                                    on:input=update_from_input(description_text)
                                    maxlength="2000"
                                    placeholder="At least 50 characters required"
                                ></textarea>
                            }
                        }
                    >
                        <div
                            class="p-4 m-2 w-full break-words rounded border prose dark:prose-invert"
                            inner_html=move || markdown_to_html(&description_text())
                        />
                    </Show>
                    <div class="flex flex-row gap-2 p-2">
                        <button
                            type="submit"
                            class=BUTTON_STYLE
                            prop:disabled=move || {
                                !(50..=2000).contains(&description_text().len())
                            }
                        >
                            "Update Description"
                        </button>
                        <button
                            type="button"
                            class=BUTTON_STYLE
                            on:click=move |_| {
                                editing_description.set(false);
                                previewing_description.set(false);
                            }
                        >
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class=BUTTON_STYLE
                            on:click=move |_| previewing_description.update(|b| *b = !*b)
                        >
                            {move || if previewing_description() { "Edit" } else { "Preview" }}
                        </button>
                        <a
                            class="font-bold text-blue-500 hover:underline"
                            href="https://commonmark.org/help/"
                            target="_blank"
                        >
                            "Markdown Cheat Sheet"
                        </a>
                    </div>
                </ActionForm>
            </Show>
        </div>
        <div class=tournament_style>
            <div class="m-2 h-fit">
                <div class=INFO_STYLE>Tournament Info</div>
                <div class="flex gap-1 m-2">
                    <span class="font-bold">"Time control: "</span>
                    <TimeRow time_info=time_info />
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
                                <UserRow actions=vec![] user />
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
                        <Show when=user_is_organizer_or_admin>
                            <button class=BUTTON_STYLE on:click=delete>
                                {"Delete"}
                            </button>
                            <button prop:disabled=start_disabled class=BUTTON_STYLE on:click=start>
                                {"Start"}
                            </button>
                        </Show>
                    </div>
                    <TournamentAdminControls
                        user_is_organizer=user_is_organizer_or_admin()
                        tournament=tournament()
                    />
                </Show>
                <Show when=user_is_organizer_or_admin>
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
                            user_is_organizer=user_is_organizer_or_admin
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
        <Show when=move || user_is_organizer_or_admin() || user_joined()>
            <div class="p-3 m-2 w-full max-w-full h-60 whitespace-normal break-words sm:w-2/3 bg-even-light dark:bg-even-dark">
                <ChatWindow destination=shared_types::SimpleDestination::Tournament(
                    tournament().tournament_id,
                ) />
            </div>
        </Show>
    }
}
