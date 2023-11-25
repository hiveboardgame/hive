use crate::{common::time_control::TimeControl, providers::game_state::GameStateSignal};
use hive_lib::{color::Color, game_result::GameResult, game_status::GameStatus};
use leptos::*;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn_with_options, UseIntervalFnOptions};
use std::time::Duration;

#[component]
pub fn LiveTimer(
    side: Color,
    parent_div: NodeRef<html::Div>,
    starting_time: Duration,
    increment: Duration,
) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let time = create_rw_signal(starting_time);
    let tick_rate = Duration::from_millis(100);
    let (turn, _set_turn) = create_slice(
        // we take a slice *from* `state`
        game_state_signal.signal,
        // our getter returns a "slice" of the data
        |gamestate| gamestate.state.turn,
        // our setter describes how to mutate that slice, given a new value
        |gamestate, n| gamestate.state.turn = n,
    );
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
        if turn() > 0 {
            if (side == Color::White) == (turn() % 2 == 0) {
                resume();
            } else {
                if is_active() {
                    pause();
                    time.update(|t| *t += increment);
                }
            }
        }
        // When time runs out declare winner and style timer that ran out
        if time() == Duration::from_secs(0) {
            pause();
            let class_list = parent_div()
                .expect("div_ref to be loaded by now")
                .class_list();
            class_list.add_1("bg-red-700").unwrap();

            match side {
                Color::White => {
                    (game_state_signal.signal).update(|s| {
                        s.state.game_status =
                            GameStatus::Finished(GameResult::Winner(Color::Black));
                    });
                }
                Color::Black => {
                    (game_state_signal.signal).update(|s| {
                        s.state.game_status =
                            GameStatus::Finished(GameResult::Winner(Color::White));
                    });
                }
            }
        }
    });
    view! {
        <div class="flex flex-grow resize h-full w-full select-none items-center justify-center text-[4vw] min-h-fit min-w-fit">
            {move || TimeControl::RealTime(time(), increment).to_string()}
        </div>
    }
}

