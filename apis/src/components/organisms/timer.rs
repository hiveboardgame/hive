use crate::{
    common::time_control::TimeControl,
    components::{
        atoms::profile_link::ProfileLink,
        molecules::{
            correspondence_timer::CorrespondenceTimer, live_timer::LiveTimer,
            rating_and_change::RatingAndChangeDynamic,
        },
    },
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
    let game_state = expect_context::<GameStateSignal>();
    let (css_grid_row, bg_color, text_color) = match side {
        Color::White => (
            "row-start-1 md:row-start-2",
            "bg-[#f0ead6]",
            "text-[#3a3a3a]",
        ),
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
                GameStatus::Finished(_) => "bg-stone-200 dark:bg-gray-900",
                _ => {
                    if (side == Color::White) == ((game_state.signal)().state.turn % 2 == 0) {
                        "bg-green-700"
                    } else {
                        "bg-stone-200 dark:bg-gray-900"
                    }
                }
            },
        );

    view! {
        <div class="grid grid-cols-2 grid-rows-2 h-full w-full col-start-9 col-span-2 row-span-1">
            <div
                ref=div_ref
                class=move || {
                    format!(
                        "border-y-2 border-l-2 col-span-1 row-span-2 md:row-span-1 short:row-span-2 border-black dark:border-white {css_grid_row} {}",
                        active_side(),
                    )
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
                    "min-h-fit min-w-fit flex justify-center leading-5 row-span-2 md:row-span-1 short:row-span-2 items-center flex-col border-y-2 border-r-2 border-black dark:border-white select-none {css_grid_row} {bg_color}",
                )
            }>
                <ProfileLink username=player().username extend_tw_classes=text_color/>
                <Show
                    when=is_finished
                    fallback=move || {
                        view! { <p class=format!("{text_color}")>{player().rating}</p> }
                    }
                >

                    <RatingAndChangeDynamic extend_tw_classes=text_color side=side/>
                </Show>
            </div>

        </div>
    }
}
