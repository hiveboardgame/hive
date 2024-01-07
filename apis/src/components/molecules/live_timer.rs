use crate::providers::api_requests::ApiRequests;
use crate::providers::timer::TimerSignal;
use hive_lib::color::Color;
use lazy_static::lazy_static;
use leptos::*;
use leptos_router::RouterContext;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn_with_options, UseIntervalFnOptions};
use regex::Regex;
use shared_types::time_mode::TimeMode;
use std::time::Duration;
lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[component]
pub fn LiveTimer(side: Color, parent_div: NodeRef<html::Div>) -> impl IntoView {
    let timer = expect_context::<TimerSignal>();
    let time_left = create_rw_signal(match side {
        Color::Black => timer.signal.get_untracked().black_time_left.unwrap(),
        Color::White => timer.signal.get_untracked().white_time_left.unwrap(),
    });
    let tick_rate = Duration::from_millis(100);
    let Pausable {
        pause,
        resume,
        is_active,
    } = use_interval_fn_with_options(
        move || time_left.update(|t| *t -= tick_rate),
        100,
        UseIntervalFnOptions::default().immediate(false),
    );
    create_effect(move |_| {
        let timer = timer.signal.get();
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
            let api = ApiRequests::new();
            let router = expect_context::<RouterContext>();
            if let Some(caps) = NANOID.captures(&router.pathname().get_untracked()) {
                let nanoid = caps.name("nanoid").map_or("", |m| m.as_str());
                if !nanoid.is_empty() {
                    api.game_check_time(nanoid);
                }
            }
            // WARN: THIS IS HACKY
            let class_list = parent_div()
                .expect("div_ref to be loaded by now")
                .class_list();
            class_list.add_1("bg-red-700").expect("Class added");
        }
    });
    view! {
        <div class="flex flex-grow resize h-full w-full select-none items-center justify-center text-[2vw] min-h-fit min-w-fit">
            {move || {
                TimeMode::RealTime.time_remaining(time_left.get())
            }}

        </div>
    }
}
