use crate::{
    common::time_control::TimeControl,
    components::molecules::{correspondence_timer::CorrespondenceTimer, live_timer::LiveTimer},
    providers::game_state::GameStateSignal,
};
use hive_lib::color::Color;
use leptos::*;
use leptos_icons::{BiIcon::BiInfiniteRegular, Icon};

#[component]
pub fn DisplayTimer(side: Color, time_control: TimeControl) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let css_grid_row: i8 = match side {
        Color::White => 0,
        Color::Black => 9,
    };
    let div_ref = create_node_ref::<html::Div>();
    let active_side = create_memo(move |_| {
        if (side == Color::White) == ((game_state_signal.signal)().state.turn % 2 == 0) {
            "bg-green-700"
        } else {
            ""
        }
    });

    view! {
        <div
            ref=div_ref
            class=move || {
                format!(
                    "h-full w-full col-start-9 col-span-2 border-2 row-span-1 row-start-{css_grid_row} {}",
                    active_side(),
                )
            }
        >

            {move || {
                match time_control {
                    TimeControl::Untimed => {
                        view! { <Icon icon=Icon::from(BiInfiniteRegular) class="h-full w-full"/> }
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
    }
}

