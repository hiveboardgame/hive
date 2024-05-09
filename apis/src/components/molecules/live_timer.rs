use crate::providers::api_requests::ApiRequests;
use crate::providers::timer::TimerSignal;
use chrono::prelude::*;
use hive_lib::color::Color;
use lazy_static::lazy_static;
use leptos::*;
use leptos_router::RouterContext;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn_with_options, UseIntervalFnOptions};
use regex::Regex;

use std::time::Duration;
lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[component]
pub fn LiveTimer(side: Color) -> impl IntoView {
    let timer_signal = expect_context::<TimerSignal>();
    let timer = timer_signal.signal.get_untracked();
    let white_time = move || {
        let left = timer.white_time_left.unwrap();
        if timer.turn < 2
            || left == Duration::from_millis(0)
            || timer.turn % 2 == 1
            || timer.finished
        {
            left
        } else {
            let left = chrono::Duration::from_std(left).unwrap();
            let then = timer.last_interaction.unwrap();
            let future = then.checked_add_signed(left).unwrap();
            let now = Utc::now();
            if now > future {
                Duration::from_millis(0)
            } else {
                future.signed_duration_since(now).to_std().unwrap()
            }
        }
    };
    let black_time = move || {
        let left = timer.black_time_left.unwrap();
        if timer.turn < 2
            || left == Duration::from_millis(0)
            || timer.turn % 2 == 0
            || timer.finished
        {
            left
        } else {
            let left = chrono::Duration::from_std(left).unwrap();
            let then = timer.last_interaction.unwrap();
            let future = then.checked_add_signed(left).unwrap();
            let now = Utc::now();
            if now > future {
                Duration::from_millis(0)
            } else {
                future.signed_duration_since(now).to_std().unwrap()
            }
        }
    };
    let time_left = create_rw_signal({
        match side {
            Color::Black => black_time(),
            Color::White => white_time(),
        }
    });
    let time_is_red = Memo::new(move |_| {
        if time_left() == Duration::from_secs(0) {
            String::from("bg-ladybug-red")
        } else {
            String::new()
        }
    });
    let ticks = create_rw_signal(0);
    let tick_rate = Duration::from_millis(100);
    let Pausable {
        pause,
        resume,
        is_active,
    } = use_interval_fn_with_options(
        move || {
            ticks.update(|t| *t += 1);
            time_left.update(|t| {
                if ticks.get_untracked() > 10 {
                    ticks.update(|t| *t = 0);
                    *t = match side {
                        Color::Black => black_time(),
                        Color::White => white_time(),
                    };
                } else {
                    *t = t.checked_sub(tick_rate).unwrap_or(Duration::from_millis(0));
                }
            })
        },
        100,
        UseIntervalFnOptions::default().immediate(false),
    );

    // WARN: Might lead to problems, if we get  re-render loops, this could be the cause.
    create_isomorphic_effect(move |_| {
        let timer = timer_signal.signal.get();
        if timer.turn > 1 {
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
                    let nanoid = caps.name("nanoid").map_or("", |m| m.as_str());
                    if !nanoid.is_empty() {
                        api.game_check_time(nanoid);
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
