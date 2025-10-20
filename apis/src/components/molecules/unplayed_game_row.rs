use crate::providers::schedules::SchedulesContext;
use crate::responses::GameResponse;
use crate::websocket::new_style::client::ClientApi;
use crate::{common::TournamentAction, components::atoms::profile_link::ProfileLink};
use chrono::{DateTime, Duration, Local, Utc};
use hive_lib::Color;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared_types::{GameStart, PrettyString, TournamentGameResult};

pub const BUTTON_STYLE: &str = "no-link-style flex justify-center items-center min-w-fit px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";

#[component]
pub fn UnplayedGameRow(
    game: GameResponse,
    user_is_organizer: Signal<bool>,
    tournament_finished: Signal<bool>,
) -> impl IntoView {
    let schedules_signal = expect_context::<SchedulesContext>();
    let client_api = expect_context::<ClientApi>();
    let game = StoredValue::new(game);
    let schedule: Signal<Option<DateTime<Utc>>> = Signal::derive(move || {
        schedules_signal.tournament.with(|tournament| {
            tournament
                .get(&game.with_value(|game| game.game_id.clone()))
                .and_then(|schedules| {
                    schedules
                        .values()
                        .find(|s| s.agreed && (s.start_t + Duration::hours(1)) > Utc::now())
                        .map(|s| s.start_t)
                })
        })
    });
    let progress_info = move || {
        game.with_value(|game| {
            if game.game_start != GameStart::Ready {
                "In progress".to_owned()
            } else if let Some(time) = schedule() {
                format!(
                    "Scheduled at {}",
                    time.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
                )
            } else {
                "Not yet scheduled".to_owned()
            }
        })
    };
    let show_adjudicate_menu = RwSignal::new(false);
    let toggle_adjudicate = move |_| {
        show_adjudicate_menu.update(|b| *b = !*b);
    };
    let adjudicate = move |result| {
        if user_is_organizer() {
            let action = game.with_value(|game| {
                TournamentAction::AdjudicateResult(game.game_id.clone(), result)
            });
            let api = client_api;
            spawn_local(async move {
                api.tournament(action).await;
            });
            show_adjudicate_menu.update(|b| *b = !*b);
        }
    };
    let buttons_container_class = move || {
        if user_is_organizer() {
            "flex flex-wrap gap-1 items-center m-1 h-fit md:min-w-[405px] justify-center md:grow md:justify-end"
        } else {
            "flex flex-wrap gap-1 items-center m-1 h-fit w-fit"
        }
    };
    view! {
        <div class="flex flex-col items-center p-3 w-full md:flex-row md:justify-between min-w-fit dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <div class="flex flex-col flex-wrap gap-1 justify-between items-center sm:flex-row h-fit grow">
                <div class="flex gap-1 items-center p-1">
                    <div class="flex shrink">
                        {game
                            .with_value(|game| {
                                view! {
                                    <ProfileLink
                                        patreon=game.white_player.patreon
                                        bot=game.white_player.bot
                                        username=game.white_player.username.clone()
                                        extend_tw_classes="truncate max-w-[120px]"
                                    />
                                    {format!("({})", game.white_rating())}
                                }
                            })}
                    </div>
                    vs.
                    <div class="flex shrink">
                        {game
                            .with_value(|game| {
                                view! {
                                    <ProfileLink
                                        patreon=game.black_player.patreon
                                        bot=game.black_player.bot
                                        username=game.black_player.username.clone()
                                        extend_tw_classes="truncate max-w-[120px]"
                                    />
                                    {format!("({})", game.black_rating())}
                                }
                            })}
                    </div>
                </div>
                <div class=move || {
                    format!("flex p-1 {}", if schedule().is_some() { "font-bold" } else { "" })
                }>
                    <Show
                        when=move || game.with_value(|game| !game.finished)
                        fallback=move || {
                            game.with_value(|game| {
                                view! {
                                    {format!(
                                        "{} {}",
                                        game.conclusion.pretty_string(),
                                        game.tournament_game_result,
                                    )}
                                }
                            })
                        }
                    >
                        {progress_info}
                    </Show>
                </div>
            </div>

            <div class=buttons_container_class>
                <Show when=move || !show_adjudicate_menu()>
                    <Show when=move || {
                        !tournament_finished()
                    }>
                        {game
                            .with_value(|game| {
                                view! {
                                    <a class=BUTTON_STYLE href=format!("/game/{}", &game.game_id)>
                                        {if game.tournament_game_result
                                            == TournamentGameResult::Unknown
                                        {
                                            "Join Game"
                                        } else {
                                            "View Game"
                                        }}
                                    </a>
                                }
                            })}
                    </Show>
                    <Show when=tournament_finished>
                        {game
                            .with_value(|game| {
                                format!("Adjudicated result: {}", game.tournament_game_result)
                            })}
                    </Show>
                    <Show when=move || user_is_organizer() && !tournament_finished()>
                        <button class=BUTTON_STYLE on:click=toggle_adjudicate>
                            {"Adjudicate"}
                        </button>
                    </Show>
                </Show>
                <Show when=move || show_adjudicate_menu() && !tournament_finished()>
                    <button
                        class=BUTTON_STYLE
                        on:click=move |_| adjudicate(TournamentGameResult::Winner(Color::White))
                    >
                        {"White won"}
                    </button>
                    <button
                        class=BUTTON_STYLE
                        on:click=move |_| adjudicate(TournamentGameResult::Winner(Color::Black))
                    >
                        {"Black won"}
                    </button>
                    <button
                        class=BUTTON_STYLE
                        on:click=move |_| adjudicate(TournamentGameResult::DoubeForfeit)
                    >
                        {"Double forfeit"}
                    </button>
                    <button
                        class=BUTTON_STYLE
                        on:click=move |_| adjudicate(TournamentGameResult::Draw)
                    >
                        {"Draw"}
                    </button>
                    <Show
                        when=move || game.with_value(|game| game.finished)
                        fallback=move || {
                            view! {
                                <button
                                    class="flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-ladybug-red hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
                                    on:click=toggle_adjudicate
                                >
                                    {"Cancel"}
                                </button>
                            }
                        }
                    >
                        <button
                            class="flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-ladybug-red hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
                            on:click=move |_| adjudicate(TournamentGameResult::Unknown)
                        >
                            {"Delete"}
                        </button>
                    </Show>
                </Show>
            </div>
        </div>
    }
}
