use crate::providers::schedules::SchedulesContext;
use crate::providers::ApiRequestsProvider;
use crate::responses::GameResponse;
use crate::{common::TournamentAction, components::atoms::profile_link::ProfileLink};
use chrono::{DateTime, Duration, Local, Utc};
use hive_lib::Color;
use leptos::prelude::*;
use shared_types::{PrettyString, TournamentGameResult};

pub const BUTTON_STYLE: &str = "flex justify-center items-center min-w-fit px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";

#[component]
pub fn UnplayedGameRow(
    game: GameResponse,
    user_is_organizer: Signal<bool>,
    tournament_finished: Signal<bool>,
) -> impl IntoView {
    let schedules_signal = expect_context::<SchedulesContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let game = Signal::derive(move || game.clone());
    let schedule: Signal<Option<DateTime<Utc>>> = Signal::derive(move || {
        schedules_signal
            .tournament
            .get()
            .get(&game().game_id)
            .and_then(|schedules| {
                schedules
                    .values()
                    .find(|s| s.agreed && (s.start_t + Duration::hours(1)) > Utc::now())
                    .map(|s| s.start_t)
            })
    });

    let date_str = move || {
        if let Some(time) = schedule() {
            format!(
                "Scheduled at {}",
                time.with_timezone(&Local).format("%Y-%m-%d %H:%M"),
            )
        } else {
            "Not yet scheduled".to_owned()
        }
    };
    let show_adjudicate_menu = RwSignal::new(false);
    let toggle_adjudicate = move |_| {
        show_adjudicate_menu.update(|b| *b = !*b);
    };
    let adjudicate = move |result| {
        if user_is_organizer() {
            let action = TournamentAction::AdjudicateResult(game().game_id.clone(), result);
            let api = api.get_value();
            api.tournament(action);
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
                        <ProfileLink
                            patreon=game().white_player.patreon
                            username=game().white_player.username.clone()
                            extend_tw_classes="truncate max-w-[120px]"
                        />
                        {format!("({})", game().white_rating())}
                    </div>
                    vs.
                    <div class="flex shrink">
                        <ProfileLink
                            patreon=game().black_player.patreon
                            username=game().black_player.username.clone()
                            extend_tw_classes="truncate max-w-[120px]"
                        />
                        {format!("({})", game().black_rating())}
                    </div>
                </div>
                <div class=move || {
                    format!("flex p-1 {}", if schedule().is_some() { "font-bold" } else { "" })
                }>
                    <Show
                        when=move || !game().finished
                        fallback=move || {
                            view! {
                                {format!(
                                    "{} {}",
                                    game().conclusion.pretty_string(),
                                    game().tournament_game_result,
                                )}
                            }
                        }
                    >
                        {date_str}
                    </Show>
                </div>
            </div>

            <div class=buttons_container_class>
                <Show when=move || !show_adjudicate_menu()>
                    <Show when=move || !tournament_finished()>
                        <a class=BUTTON_STYLE href=format!("/game/{}", &game().game_id)>
                            {if game().tournament_game_result == TournamentGameResult::Unknown {
                                "Join Game"
                            } else {
                                "View Game"
                            }}
                        </a>
                    </Show>
                    <Show when=tournament_finished>
                        {format!("Adjudicated result: {}", game().tournament_game_result)}
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
                    <Show
                        when=move || game().finished
                        fallback=move || {
                            view! {
                                <button
                                    class="flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-ladybug-red hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                                    on:click=toggle_adjudicate
                                >
                                    {"Cancel"}
                                </button>
                            }
                        }
                    >
                        <button
                            class="flex justify-center items-center px-4 py-2 font-bold text-white rounded bg-ladybug-red hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
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
