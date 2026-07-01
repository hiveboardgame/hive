use crate::{
    common::{with_class, TournamentAction},
    components::atoms::profile_link::ProfileLink,
    providers::{schedules::SchedulesContext, ApiRequestsProvider},
    responses::GameResponse,
};
use chrono::{DateTime, Duration, Local, Utc};
use hudsoni::Color;
use leptos::prelude::*;
use shared_types::{GameStart, PrettyString, TournamentGameResult};

#[component]
pub fn UnplayedGameRow(
    game: GameResponse,
    user_is_organizer: Signal<bool>,
    tournament_finished: Signal<bool>,
) -> impl IntoView {
    let schedules_signal = expect_context::<SchedulesContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
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
            let api = api.get();
            api.tournament(action);
            show_adjudicate_menu.update(|b| *b = !*b);
        }
    };
    let buttons_container_class = move || {
        if user_is_organizer() {
            "flex flex-wrap gap-2 items-center h-fit justify-center md:grow md:justify-end"
        } else {
            "flex flex-wrap gap-2 items-center h-fit w-fit"
        }
    };
    view! {
        <div class=with_class(
            "ui-card-row",
            "flex min-w-fit w-full flex-col items-center gap-3 p-3 md:flex-row md:justify-between",
        )>
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
                                        deleted=game.white_player.deleted
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
                                        deleted=game.black_player.deleted
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
                                    <a
                                        class="ui-button ui-button-primary ui-button-md no-link-style min-w-fit"
                                        href=format!("/game/{}", &game.game_id)
                                    >
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
                    <Show when=move || {
                        user_is_organizer() && !tournament_finished()
                            && game.with_value(|game| game.organizer_can_adjudicate())
                    }>
                        <button
                            class="ui-button ui-button-secondary ui-button-md min-w-fit"
                            on:click=toggle_adjudicate
                        >
                            {"Adjudicate"}
                        </button>
                    </Show>
                </Show>
                <Show when=move || show_adjudicate_menu() && !tournament_finished()>
                    <button
                        class="ui-button ui-button-primary ui-button-md min-w-fit"
                        on:click=move |_| adjudicate(TournamentGameResult::Winner(Color::White))
                    >
                        {"White won"}
                    </button>
                    <button
                        class="ui-button ui-button-primary ui-button-md min-w-fit"
                        on:click=move |_| adjudicate(TournamentGameResult::Winner(Color::Black))
                    >
                        {"Black won"}
                    </button>
                    <button
                        class="ui-button ui-button-danger ui-button-md min-w-fit"
                        on:click=move |_| adjudicate(TournamentGameResult::DoubeForfeit)
                    >
                        {"Double forfeit"}
                    </button>
                    <button
                        class="ui-button ui-button-secondary ui-button-md min-w-fit"
                        on:click=move |_| adjudicate(TournamentGameResult::Draw)
                    >
                        {"Draw"}
                    </button>
                    <Show
                        when=move || game.with_value(|game| game.finished)
                        fallback=move || {
                            view! {
                                <button
                                    class="ui-button ui-button-danger ui-button-md min-w-fit"
                                    on:click=toggle_adjudicate
                                >
                                    {"Cancel"}
                                </button>
                            }
                        }
                    >
                        <button
                            class="ui-button ui-button-danger ui-button-md min-w-fit"
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
