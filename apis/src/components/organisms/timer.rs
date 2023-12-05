use crate::{
    common::time_control::TimeControl,
    components::molecules::{correspondence_timer::CorrespondenceTimer, live_timer::LiveTimer},
    functions::users::user_response::UserResponse,
    providers::game_state::GameStateSignal,
};
use hive_lib::{color::Color, game_status::GameStatus};
use leptos::*;
use leptos_icons::{BiIcon::BiInfiniteRegular, Icon};

#[component]
pub fn DisplayTimer(
    side: Color,
    player: StoredValue<UserResponse>,
    time_control: TimeControl,
) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let (css_grid_row, bg_color, text_color) = match side {
        Color::White => ("row-start-2", "bg-[#f0ead6]", "text-[#3a3a3a]"),
        Color::Black => ("row-start-1", "bg-[#3a3a3a]", "text-[#f0ead6]"),
    };

    let div_ref = create_node_ref::<html::Div>();
    let active_side =
        create_memo(
            move |_| match game_state_signal.signal.get_untracked().state.game_status {
                GameStatus::Finished(_) => "",
                _ => {
                    if (side == Color::White) == ((game_state_signal.signal)().state.turn % 2 == 0)
                    {
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
                <p class=format!("ml-4 text-[1vw] {text_color}")>{player().rating}</p>
            </div>

        </div>
    }
}
