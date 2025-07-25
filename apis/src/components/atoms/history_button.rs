use crate::{
    components::organisms::side_board::move_query_signal,
    providers::{
        game_state::GameStateSignal,
        timer::{Timer, TimerSignal},
    },
};
use hive_lib::{Color, GameResult, GameStatus};
use leptos::{html, leptos_dom::helpers::debounce, prelude::*};
use leptos_icons::*;
use shared_types::{Conclusion, TimeMode};
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
    let timer = expect_context::<TimerSignal>();
    let (_move, set_move) = move_query_signal();
    let is_last_turn = game_state_signal.is_last_turn_as_signal();
    let is_first_turn = game_state_signal.is_first_turn_as_signal();
    let cloned_action = action.clone();
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 m-1 h-7 rounded-md border-cyan-500 dark:border-button-twilight border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let icon = match action {
        HistoryNavigation::First => icondata_ai::AiFastBackwardFilled,
        HistoryNavigation::Last | HistoryNavigation::MobileLast => icondata_ai::AiFastForwardFilled,
        HistoryNavigation::Next => icondata_ai::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata_ai::AiStepBackwardFilled,
    };

    let is_disabled = move || match cloned_action {
        HistoryNavigation::Last | HistoryNavigation::MobileLast | HistoryNavigation::Next => {
            is_last_turn()
        }

        HistoryNavigation::Previous | HistoryNavigation::First => is_first_turn(),
    };
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        send_action(&action, game_state_signal, timer);
        if let Some(post_action) = post_action {
            post_action.run(())
        }
        let turn = game_state_signal.signal.with_untracked(|gs| match action {
            HistoryNavigation::Last => Some(gs.state.turn),
            HistoryNavigation::MobileLast => None,
            _ => gs.history_turn.map(|v| v + 1),
        });
        set_move.set(turn);
    });
    let _definite_node_ref = node_ref.unwrap_or_default();

    view! {
        <button
            node_ref=_definite_node_ref
            class=nav_buttons_style
            prop:disabled=is_disabled
            on:click=debounced_action
        >

            <Icon icon=icon />
        </button>
    }
}

fn send_action(
    action: &HistoryNavigation,
    mut game_state_signal: GameStateSignal,
    timer: TimerSignal,
) {
    match action {
        HistoryNavigation::First => game_state_signal.first_history_turn(),
        HistoryNavigation::Last => game_state_signal.view_history(),
        HistoryNavigation::Next => game_state_signal.next_history_turn(),
        HistoryNavigation::Previous => game_state_signal.previous_history_turn(),
        HistoryNavigation::MobileLast => {
            if game_state_signal
                .signal
                .with_untracked(|gs| gs.state.turn > 0)
            {
                game_state_signal
                    .signal
                    .update_untracked(|s| s.history_turn = Some(s.state.turn - 1));
            }
            game_state_signal.view_game()
        }
    }
    set_timer_from_response(game_state_signal, timer);
}

pub fn set_timer_from_response(game_state_signal: GameStateSignal, timer: TimerSignal) {
    game_state_signal.signal.with(|gs| {
        if !matches!(
            gs.state.game_status,
            GameStatus::Finished(_) | GameStatus::Adjudicated
        ) {
            return;
        }
        let turn = gs.history_turn.unwrap_or_default();
        let response = gs.game_response.as_ref();
        let timed_out = turn == gs.state.turn - 1
            && response.is_some_and(|r| r.conclusion == Conclusion::Timeout);
        if let Some(response) = response {
            if !matches!(response.time_mode, TimeMode::RealTime) {
                return;
            }

            let set_timeout_flags = |t: &mut Timer| {
                if timed_out {
                    if let GameStatus::Finished(GameResult::Winner(color)) = response.game_status {
                        t.white_timed_out = color == Color::Black;
                        t.black_timed_out = color == Color::White;
                    }
                } else {
                    t.white_timed_out = false;
                    t.black_timed_out = false;
                }
            };

            if turn == 0 {
                let base = response.time_base.unwrap_or(0);
                timer.signal.update(|t| {
                    t.white_time_left = Some(Duration::from_secs(base as u64));
                    t.black_time_left = Some(Duration::from_secs(base as u64));
                    set_timeout_flags(t);
                });
            } else if let (Some(Some(curr_time_left)), Some(Some(prev_time_left))) = (
                response.move_times.get(turn),
                response.move_times.get(turn - 1),
            ) {
                let curr_time_left = Duration::from_nanos(*curr_time_left as u64);
                let prev_time_left = Duration::from_nanos(*prev_time_left as u64);
                timer.signal.update(|t| {
                    if turn.is_multiple_of(2) {
                        t.white_time_left = Some(curr_time_left);
                        t.black_time_left = Some(prev_time_left);
                    } else {
                        t.black_time_left = Some(curr_time_left);
                        t.white_time_left = Some(prev_time_left);
                    }
                    set_timeout_flags(t);
                });
            }
        }
    });
}
