use crate::{
    common::MoveConfirm,
    components::molecules::{live_timer::LiveTimer, user_with_rating::UserWithRating},
    pages::play::CurrentConfirm,
    providers::{game_state::GameStateSignal, timer::TimerSignal, AuthContext},
};
use hive_lib::Color;
use leptos::*;
use leptos_icons::*;
use shared_types::TimeMode;

#[derive(Clone, Copy, PartialEq)]
pub enum Placement {
    Top,
    Bottom,
}

#[component]
pub fn DisplayTimer(placement: Placement, vertical: bool) -> impl IntoView {
    let mut game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let user = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };
    let black_id = create_read_slice(game_state.signal, |gs| gs.black_id);
    let player_is_black =
        create_memo(move |_| user().map_or(false, |user| Some(user.id) == black_id()));
    let side = Signal::derive(move || match (player_is_black(), placement) {
        (true, Placement::Top) => Color::White,
        (true, Placement::Bottom) => Color::Black,
        (false, Placement::Top) => Color::Black,
        (false, Placement::Bottom) => Color::White,
    });
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
    let active_side = create_memo(move |_| {
        let timer = timer.signal.get();
        match timer.finished {
            true => "bg-stone-200 dark:bg-reserve-twilight",
            false => {
                if (side() == Color::White) == (timer.turn % 2 == 0) {
                    "bg-grasshopper-green"
                } else {
                    "bg-stone-200 dark:bg-reserve-twilight"
                }
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
            && current_confirm() == MoveConfirm::Clock
            && game_state.is_move_allowed()
    };

    let button_class = move || {
        let cursor_style = if !is_button() {
            "cursor-[unset]"
        } else {
            "cursor-pointer"
        };
        if vertical {
            format!("{timer_container_style} {} {}", active_side(), cursor_style,)
        } else {
            format!(
                "{timer_container_style} {} {} {}",
                css_grid_row(),
                active_side(),
                cursor_style,
            )
        }
    };

    let onclick = move |_| {
        if is_button() {
            game_state.move_active();
        }
    };

    view! {
        <div class=outer_container_style>
            <button on:click=onclick class=button_class>
                <Show
                    when=move || {
                        matches!(
                            timer.signal.get().time_mode,
                            TimeMode::Correspondence | TimeMode::RealTime
                        )
                    }
                    fallback=|| {
                        view! {
                            <Icon
                                icon=icondata::BiInfiniteRegular
                                class="w-full h-full bg-inherit"
                            />
                        }
                    }
                >
                    <LiveTimer side/>
                </Show>
            </button>
            <Show when=move || !vertical>
                <div class=classes>
                    <UserWithRating side=side() text_color=text_color()/>
                </div>
            </Show>
        </div>
    }
}
