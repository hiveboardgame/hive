use crate::{
    common::time_control::TimeControl,
    components::molecules::{
        correspondence_timer::CorrespondenceTimer, finished_rating::FinishedRating,
        live_timer::LiveTimer,
    },
    functions::{games::game_response::GameStateResponse, users::user_response::UserResponse},
    providers::game_state::GameStateSignal,
};
use hive_lib::{color::Color, game_status::GameStatus};
use leptos::*;
use leptos_icons::{BiIcon::BiInfiniteRegular, Icon};

#[component]
pub fn DisplayTimer(
    side: Color,
    game: StoredValue<GameStateResponse>,
    player: StoredValue<UserResponse>,
    time_control: TimeControl,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let (css_grid_row, bg_color, text_color) = match side {
        Color::White => ("row-start-2", "bg-[#f0ead6]", "text-[#3a3a3a]"),
        Color::Black => ("row-start-1", "bg-[#3a3a3a]", "text-[#f0ead6]"),
    };

    let is_finished = move || match (game_state.signal)().state.game_status {
        GameStatus::Finished(_) => true,
        _ => false,
    };

    let div_ref = create_node_ref::<html::Div>();
    let active_side =
        create_memo(
            move |_| match game_state.signal.get_untracked().state.game_status {
                GameStatus::Finished(_) => "",
                _ => {
                    if (side == Color::White) == ((game_state.signal)().state.turn % 2 == 0) {
                        "bg-green-700"
                    } else {
                        ""
                    }
                }
            },
        );

    view! {
        <div class="grid grid-cols-2 grid-rows-2 h-full w-full col-start-9 col-span-2 row-span-1 ">
            <div
                ref=div_ref
                class=move || {
                    format!("border-2 col-span-1 row-span-1 {css_grid_row} {}", active_side())
                }
            >

                {move || {
                    match time_control {
                        TimeControl::Untimed => {
                            view! {
                                <Icon icon=Icon::from(BiInfiniteRegular) class="h-full w-full"/>
                            }
                        }
                        TimeControl::Correspondence(max_time_to_move) => {
                            view! {
                                <CorrespondenceTimer
                                    side=side
                                    parent_div=div_ref
                                    max_time_to_move=max_time_to_move
                                />
                            }
                        }
                        TimeControl::RealTime(starting_time, increment) => {
                            view! {
                                <LiveTimer
                                    side=side
                                    parent_div=div_ref
                                    starting_time=starting_time
                                    increment=increment
                                />
                            }
                        }
                    }
                }}

            </div>
            <div class=move || {
                format!(
                    "flex justify-center items-start flex-col border-2 col-span-1 row-span-1 col-start-2 {css_grid_row} {bg_color}",
                )
            }>
                <p class=format!("ml-4 text-[1vw] {text_color}")>{player().username}</p>
                <Show
                    when=is_finished
                    fallback=move || {
                        view! {
                            <p class=format!("ml-4 text-[1vw] {text_color}")>{player().rating}</p>
                        }
                    }
                >

                    <FinishedRating extend_tw_classes=text_color game=game() side=side/>
                </Show>
            </div>

        </div>
    }
}
