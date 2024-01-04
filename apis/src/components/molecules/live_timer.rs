use crate::common::time_control::TimeControl;
use crate::providers::timer::TimerSignal;
use hive_lib::color::Color;
use leptos::*;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn_with_options, UseIntervalFnOptions};
use std::time::Duration;

#[component]
pub fn LiveTimer(side: Color, parent_div: NodeRef<html::Div>) -> impl IntoView {
    let timer = expect_context::<TimerSignal>();
    let time = create_rw_signal(match side {
        Color::Black => timer.signal.get_untracked().black_time_left.unwrap(),
        Color::White => timer.signal.get_untracked().white_time_left.unwrap(),
    });
    let tick_rate = Duration::from_millis(100);
    // TODO: figure out time update bug when move is made but not confirmed
    // using a slice of the gamestate didn't fix it and lowering the tick_rate from 1s just hides the problem
    let Pausable {
        pause,
        resume,
        is_active,
    } = use_interval_fn_with_options(
        move || time.update(|t| *t -= tick_rate),
        100,
        UseIntervalFnOptions::default().immediate(false),
    );
    create_effect(move |_| {
        let timer = timer.signal.get();
        if timer.turn > 0 {
            if (side == Color::White) == (timer.turn % 2 == 0) && !timer.finished {
                resume();
            } else if is_active() {
                pause();
            }
        }
        // When time runs out declare winner and style timer that ran out
        if time() == Duration::from_secs(0) {
            pause();
            let class_list = parent_div()
                .expect("div_ref to be loaded by now")
                .class_list();
            class_list.add_1("bg-red-700").expect("Class added");
        }
    });
    view! {
        <div class="flex flex-grow resize h-full w-full select-none items-center justify-center text-[2vw] min-h-fit min-w-fit">
            {move || {
                TimeControl::RealTime(time(), timer.signal.get().time_increment.unwrap())
                    .to_string()
            }}

        </div>
    }
}
