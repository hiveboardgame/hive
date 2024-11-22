use crate::providers::{game_state::GameStateSignal, timer::TimerSignal};
use hive_lib::GameStatus;
use leptos::{leptos_dom::helpers::debounce, *};
use leptos_icons::*;
use shared_types::TimeMode;
use std::time::Duration;
#[derive(Clone)]
pub enum HistoryNavigation {
    First,
    Last,
    Next,
    Previous,
    MobileLast,
}

#[component]
pub fn HistoryButton(
    action: HistoryNavigation,
    #[prop(optional)] post_action: Option<Callback<()>>,
    #[prop(optional)] node_ref: Option<NodeRef<html::Button>>,
) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let is_last_turn = game_state_signal.is_last_turn_as_signal();
    let is_first_turn = game_state_signal.is_first_turn_as_signal();
    let cloned_action = action.clone();
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 m-1 h-7 rounded-md border-cyan-500 dark:border-button-twilight border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let icon = match action {
        HistoryNavigation::First => icondata::AiFastBackwardFilled,
        HistoryNavigation::Last | HistoryNavigation::MobileLast => icondata::AiFastForwardFilled,
        HistoryNavigation::Next => icondata::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata::AiStepBackwardFilled,
    };

    let is_disabled = move || match cloned_action {
        HistoryNavigation::Last | HistoryNavigation::MobileLast | HistoryNavigation::Next => {
            is_last_turn()
        }

        HistoryNavigation::Previous | HistoryNavigation::First => is_first_turn(),
    };
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        send_action(&action);
        if let Some(post_action) = post_action {
            post_action(())
        }
    });
    let _definite_node_ref = node_ref.unwrap_or(create_node_ref::<html::Button>());

    view! {
        <button
            ref=_definite_node_ref
            class=nav_buttons_style
            prop:disabled=is_disabled
            on:click=debounced_action
        >

            <Icon icon=icon />
        </button>
    }
}

fn send_action(action: &HistoryNavigation) {
    let mut game_state_signal = expect_context::<GameStateSignal>();
    match action {
        HistoryNavigation::First => game_state_signal.first_history_turn(),
        HistoryNavigation::Last => game_state_signal.view_history(),
        HistoryNavigation::Next => game_state_signal.next_history_turn(),
        HistoryNavigation::Previous => game_state_signal.previous_history_turn(),
        HistoryNavigation::MobileLast => {
            if game_state_signal.signal.get_untracked().state.turn > 0 {
                game_state_signal
                    .signal
                    .update_untracked(|s| s.history_turn = Some(s.state.turn - 1));
            }
            game_state_signal.view_game()
        }
    }
    set_timer_from_response();
}

pub fn set_timer_from_response() {
    let game_state_signal = expect_context::<GameStateSignal>();
    let timer = expect_context::<TimerSignal>();
    game_state_signal.signal.with(|gs| {
        if !matches!(gs.state.game_status, GameStatus::Finished(_)) {
            return;
        }
        let turn = gs.history_turn.unwrap_or_default();
        let response = gs.game_response.as_ref();
        if let Some(response) = response {
            if !matches!(response.time_mode, TimeMode::RealTime) {
                return;
            }
            if turn == 0 {
                let base = response.time_base.unwrap_or(0);
                timer.signal.update(|t| {
                    t.white_time_left = Some(Duration::from_secs(base as u64));
                    t.black_time_left = Some(Duration::from_secs(base as u64));
                });
            } else {
                let curr_time_left = response.move_times[turn].expect("Turn is finished");
                let prev_time_left = response.move_times[turn - 1].expect("Turn is finished");
                let curr_time_left = Duration::from_nanos(curr_time_left as u64);
                let prev_time_left = Duration::from_nanos(prev_time_left as u64);
                timer.signal.update(|t| {
                    if turn % 2 == 0 {
                        t.white_time_left = Some(curr_time_left);
                        t.black_time_left = Some(prev_time_left);
                    } else {
                        t.black_time_left = Some(curr_time_left);
                        t.white_time_left = Some(prev_time_left);
                    }
                });
            }
        }
    });
}
