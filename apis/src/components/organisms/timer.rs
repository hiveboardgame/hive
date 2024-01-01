use crate::{
    common::time_control::TimeControl,
    components::molecules::{
        correspondence_timer::CorrespondenceTimer, live_timer::LiveTimer,
        user_with_rating::UserWithRating,
    },
    providers::game_state::GameStateSignal,
    responses::user::UserResponse,
};
use hive_lib::{color::Color, game_status::GameStatus};
use leptos::*;
use leptos_icons::{BiIcon::BiInfiniteRegular, Icon};

pub enum Placement {
    Top,
    Bottom,
}

#[component]
pub fn DisplayTimer(
    side: Color,
    #[prop(optional)] placement: Option<Placement>,
    player: StoredValue<UserResponse>,
    time_control: TimeControl,
    vertical: bool,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let (bg_color, text_color) = match side {
        Color::White => ("bg-hive-black", "text-hive-white"),
        Color::Black => ("bg-hive-white", "text-hive-black"),
    };
    let css_grid_row = match placement {
        Some(Placement::Top) => "row-start-1 md:row-start-2",
        Some(Placement::Bottom) => "row-start-1",
        _ => "",
    };
    let(outer_container_style, timer_container_style, user_container_style) = match vertical {
        false => ("grid grid-cols-2 grid-rows-2 col-span-2 row-span-1",
                "border-y-2 border-l-2 col-span-1 row-span-2 md:row-span-1 short:row-span-2 border-black dark:border-white",
                "h-full flex justify-center md:leading-5 row-span-2 md:row-span-1 short:row-span-2 short:text-xs items-center flex-col border-y-2 border-r-2 border-black dark:border-white select-none"),
        true => ("flex grow justify-end items-center", "w-14 h-14 grow-0 ",""),
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
        <div class=outer_container_style>
            <div
                ref=div_ref
                class=move || {
                    if vertical {
                        format!("{timer_container_style} {}", active_side())
                    } else {
                        format!("{timer_container_style} {css_grid_row} {}", active_side())
                    }
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
            <Show when=move || !vertical>
                <div class=move || format!("{user_container_style} {css_grid_row} {bg_color}")>
                    <UserWithRating player=player side=side text_color=text_color/>
                </div>
            </Show>

        </div>
    }
}
