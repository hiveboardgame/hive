use crate::{
    components::molecules::{live_timer::LiveTimer, user_with_rating::UserWithRating},
    providers::{auth_context::AuthContext, game_state::GameStateSignal, timer::TimerSignal},
};
use hive_lib::color::Color;
use leptos::*;
use leptos_icons::{BiIcon::BiInfiniteRegular, Icon};
use shared_types::time_mode::TimeMode;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub enum Placement {
    Top,
    Bottom,
}

#[component]
pub fn DisplayTimer(placement: Placement, vertical: bool) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    let player_is_black = create_memo(move |_| {
        user().map_or(false, |user| {
            let game_state = game_state.signal.get();
            Some(user.id) == game_state.black_id
        })
    });
    let side = move || match (player_is_black(), placement) {
        (true, Placement::Top) => Color::White,
        (true, Placement::Bottom) => Color::Black,
        (false, Placement::Top) => Color::Black,
        (false, Placement::Bottom) => Color::White,
    };
    let bg_color = move || match side() {
        Color::White => "bg-hive-black",
        Color::Black => "bg-hive-white",
    };
    let text_color = move || match side() {
        Color::White => "text-hive-white",
        Color::Black => "text-hive-black",
    };
    let css_grid_row = move || match placement {
        Placement::Top => "row-start-1 md:row-start-2",
        Placement::Bottom => "row-start-1",
    };
    let(outer_container_style, timer_container_style, user_container_style) = match vertical {
        false => ("grid grid-cols-2 grid-rows-2 col-span-2 row-span-1",
                "border-y-2 border-l-2 col-span-1 row-span-2 md:row-span-1 short:row-span-2 border-black dark:border-white duration-300",
                "h-full flex justify-center md:leading-5 row-span-2 md:row-span-1 short:row-span-2 short:text-xs items-center flex-col border-y-2 border-r-2 border-black dark:border-white select-none"),
        true => ("flex grow justify-end items-center", "w-14 h-14 grow-0 duration-300",""),
    };
    let timer = expect_context::<TimerSignal>();
    let active_side = create_memo(move |_| match timer.signal.get().finished {
        true => "bg-stone-200 dark:bg-gray-900",
        false => {
            if (side() == Color::White) == (timer.signal.get().turn % 2 == 0) {
                "bg-green-700"
            } else {
                "bg-stone-200 dark:bg-gray-900"
            }
        }
    });
    let classes = move || {
        if vertical {
            format!("{user_container_style} {}", bg_color())
        } else {
            format!("{user_container_style} {} {}", css_grid_row(), bg_color())
        }
    };

    view! {
        <div class=outer_container_style>
            <div class=move || {
                if vertical {
                    format!("{timer_container_style} {}", active_side())
                } else {
                    format!("{timer_container_style} {} {}", css_grid_row(), active_side())
                }
            }>

                {move || {
                    match TimeMode::from_str(&timer.signal.get().time_mode).expect("Valid TimeMode")
                    {
                        TimeMode::Untimed => {
                            view! {
                                <Icon icon=Icon::from(BiInfiniteRegular) class="h-full w-full"/>
                            }
                        }
                        TimeMode::Correspondence | TimeMode::RealTime => {
                            view! { <LiveTimer side=side()/> }
                        }
                    }
                }}

            </div>
            <Show when=move || !vertical>
                <div class=classes>
                    <UserWithRating side=side() text_color=text_color()/>
                </div>
            </Show>
        </div>
    }
}
