use crate::{
    common::{CurrentConfirm, MoveConfirm},
    components::molecules::{live_timer::LiveTimer, user_with_rating::UserWithRating},
    providers::{game_state::GameStateStore, timer::TimerSignal, ApiRequestsProvider, AuthContext},
};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TimeMode;

const BOARD_TIMER_IDLE_CLASS: &str = "bg-stone-200 dark:bg-[#222b35]";

#[derive(Clone, Copy, PartialEq)]
pub enum Placement {
    Top,
    Bottom,
}

#[component]
pub fn DisplayTimer(placement: Placement, vertical: bool) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let user_color = game_state.user_color_as_signal(auth_context.identity);
    let player_is_black = Signal::derive(move || user_color.get() == Some(Color::Black));
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
    let css_grid_row = "row-start-1";
    let (outer_container_style, timer_container_style, user_container_style) = match (vertical, placement) {
        (false, _) => ("grid h-16 grid-cols-2 col-span-2 row-span-1 overflow-hidden ui-board-side-panel short:h-full short:grid-rows-2",
                "col-span-1 row-span-2 md:row-span-1 short:row-span-2 overflow-hidden transition-colors duration-300",
                "h-full flex justify-center md:leading-4 row-span-2 md:row-span-1 short:row-span-2 short:text-xs items-center flex-col select-none"),
        (true, _) => (
            "flex h-full grow justify-end items-stretch",
            "h-full w-16 shrink-0 grow-0 overflow-hidden rounded-none transition-colors duration-300",
            "",
        ),
    };
    let timer = expect_context::<TimerSignal>().signal;
    let active_side = Memo::new(move |_| {
        let timer = timer();
        match timer.finished {
            true => BOARD_TIMER_IDLE_CLASS,
            false => {
                if (side() == Color::White) == timer.turn.is_multiple_of(2) {
                    "bg-grasshopper-green"
                } else {
                    BOARD_TIMER_IDLE_CLASS
                }
            }
        }
    });
    let classes = move || {
        if vertical {
            format!("{user_container_style} {}", bg_color())
        } else {
            format!("{user_container_style} {css_grid_row} {}", bg_color())
        }
    };

    let is_button = move || {
        placement == Placement::Bottom
            && current_confirm() == MoveConfirm::Clock
            && game_state.is_move_allowed(false)
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
                css_grid_row,
                active_side(),
                cursor_style,
            )
        }
    };
    let api = expect_context::<ApiRequestsProvider>().0;
    let onclick = move |_| {
        if is_button() {
            game_state.move_active(None, api());
        }
    };
    let outer_container_class = move || {
        if vertical {
            outer_container_style.to_string()
        } else {
            let alignment = match placement {
                Placement::Top => "self-end",
                Placement::Bottom => "self-start",
            };
            format!("{outer_container_style} {alignment}")
        }
    };

    view! {
        <div class=outer_container_class>
            <button on:click=onclick class=button_class id="timer">
                <Show
                    when=move || {
                        matches!(timer().time_mode, TimeMode::Correspondence | TimeMode::RealTime)
                    }

                    fallback=|| {
                        view! {
                            <Icon
                                icon=icondata_bi::BiInfiniteRegular
                                attr:class="size-full bg-inherit"
                            />
                        }
                    }
                >

                    <LiveTimer side compact=vertical />
                </Show>
            </button>
            <Show when=move || !vertical>
                <div class=classes>
                    <UserWithRating side=side() text_color=text_color() />
                </div>
            </Show>
        </div>
    }
}
