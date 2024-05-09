use crate::{
    common::config_options::MoveConfirm,
    components::molecules::{live_timer::LiveTimer, user_with_rating::UserWithRating},
    providers::{
        auth_context::AuthContext, config::config::Config, game_state::GameStateSignal,
        timer::TimerSignal,
    },
};
use hive_lib::color::Color;
use leptos::*;
use leptos_icons::*;
use shared_types::time_mode::TimeMode;

#[derive(Clone, Copy, PartialEq)]
pub enum Placement {
    Top,
    Bottom,
}

#[component]
pub fn DisplayTimer(placement: Placement, vertical: bool) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>();
    let mut game_state_signal = expect_context::<GameStateSignal>();
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
        Color::White => "bg-hive-white",
        Color::Black => "bg-hive-black",
    };
    let text_color = move || match side() {
        Color::White => "text-hive-black",
        Color::Black => "text-hive-white",
    };
    let css_grid_row = move || match placement {
        Placement::Top => "row-start-1 md:row-start-2",
        Placement::Bottom => "row-start-1",
    };
    let(outer_container_style, timer_container_style, user_container_style) = match vertical {
        false => ("grid grid-cols-2 grid-rows-2 col-span-2 row-span-1",
                "border-y-2 border-l-2 col-span-1 row-span-2 md:row-span-1 short:row-span-2 border-black dark:border-white duration-300",
                "h-full flex justify-center md:leading-4 row-span-2 md:row-span-1 short:row-span-2 short:text-xs items-center flex-col border-y-2 border-r-2 border-black dark:border-white select-none"),
        true => ("flex grow justify-end items-center", "w-14 h-14 grow-0 duration-300",""),
    };
    let timer = expect_context::<TimerSignal>();
    let active_side = create_memo(move |_| match timer.signal.get().finished {
        true => "bg-stone-200 dark:bg-reserve-twilight",
        false => {
            if (side() == Color::White) == (timer.signal.get().turn % 2 == 0) {
                "bg-grasshopper-green"
            } else {
                "bg-stone-200 dark:bg-reserve-twilight"
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

    let is_button = move || {
        placement == Placement::Bottom
            && matches!(
                (config.confirm_mode.preferred_confirm)(),
                MoveConfirm::Clock
            )
            && game_state_signal.is_move_allowed()
    };

    let onclick = move |_| {
        if is_button() {
            game_state_signal.move_active();
        }
    };

    view! {
        <div class=outer_container_style>
            <button
                on:click=onclick
                class=move || {
                    if vertical {
                        format!(
                            "{timer_container_style} {} {}",
                            active_side(),
                            if !is_button() { "cursor-[unset]" } else { "cursor-pointer" },
                        )
                    } else {
                        format!(
                            "{timer_container_style} {} {} {}",
                            css_grid_row(),
                            active_side(),
                            if !is_button() { "cursor-[unset]" } else { "cursor-pointer" },
                        )
                    }
                }
            >

                {move || {
                    match timer.signal.get().time_mode {
                        TimeMode::Untimed => {
                            view! {
                                <Icon
                                    icon=icondata::BiInfiniteRegular
                                    class="h-full w-full bg-inherit"
                                />
                            }
                        }
                        TimeMode::Correspondence | TimeMode::RealTime => {
                            view! { <LiveTimer side=side()/> }
                        }
                    }
                }}

            </button>
            <Show when=move || !vertical>
                <div class=classes>
                    <UserWithRating side=side() text_color=text_color()/>
                </div>
            </Show>
        </div>
    }
}
