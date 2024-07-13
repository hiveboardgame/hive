use crate::providers::{game_state::GameStateSignal, timer::TimerSignal, ApiRequests};
use hive_lib::{Color, GameStatus};
use lazy_static::lazy_static;
use leptos::*;
use leptos_router::RouterContext;
use leptos_use::{use_interval_fn_with_options, utils::Pausable, UseIntervalFnOptions};
use regex::Regex;
use shared_types::GameId;
use std::time::Duration;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[component]
pub fn LiveTimer(side: Color) -> impl IntoView {
    let timer_signal = expect_context::<TimerSignal>();
    let mut game_state = expect_context::<GameStateSignal>();
    let in_progress = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map_or(false, |gr| gr.game_status == GameStatus::InProgress)
    });
    let timer = timer_signal.signal.get_untracked();
    let time_for_color = move |color: Color| match color {
        Color::White => timer.white_time_left.unwrap(),
        Color::Black => timer.black_time_left.unwrap(),
    };

    let time_left = create_rw_signal(time_for_color(side));
    let time_is_red = Memo::new(move |_| {
        if time_left() == Duration::from_secs(0) {
            String::from("bg-ladybug-red")
        } else {
            String::new()
        }
    });
    let tick_rate = Duration::from_millis(100);
    let Pausable {
        pause,
        resume,
        is_active,
    } = use_interval_fn_with_options(
        move || {
            batch(move || {
                time_left.update(|t| {
                    *t = t.checked_sub(tick_rate).unwrap_or(Duration::from_millis(0));
                    if t.as_nanos() == 0 {
                        game_state.reset();
                    };
                })
            })
        },
        100,
        UseIntervalFnOptions::default().immediate(false),
    );

    // WARN: Might lead to problems, if we get  re-render loops, this could be the cause.
    create_isomorphic_effect(move |_| {
        let timer = timer_signal.signal.get();
        if in_progress() {
            if (side == Color::White) == (timer.turn % 2 == 0) && !timer.finished {
                resume();
            } else if is_active() {
                pause();
            }
        }
        // When time runs out declare winner and style timer that ran out
        if time_left() == Duration::from_secs(0) {
            pause();
            if !timer.finished {
                let api = ApiRequests::new();
                let router = expect_context::<RouterContext>();
                if let Some(caps) = NANOID.captures(&router.pathname().get_untracked()) {
                    if let Some(nanoid) = caps.name("nanoid") {
                        let game_id = GameId(nanoid.as_str().to_string());
                        api.game_check_time(&game_id);
                    }
                }
            }
        }
    });

    view! {
        <div class=move || {
            format!(
                "flex resize h-full select-none items-center justify-center text-xl md:text-2xl lg:text-4xl {}",
                time_is_red(),
            )
        }>{move || { timer.time_mode.time_remaining(time_left.get()) }}</div>
    }
}
