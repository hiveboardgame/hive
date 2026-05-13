use crate::{
    components::organisms::side_board::move_query_signal,
    providers::game_state::GameStateSignal,
};
use leptos::{html, leptos_dom::helpers::debounce, prelude::*};
use leptos_icons::*;

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
        send_action(&action, game_state_signal);
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

fn send_action(action: &HistoryNavigation, mut game_state_signal: GameStateSignal) {
    match action {
        HistoryNavigation::First => game_state_signal.first_history_turn(),
        HistoryNavigation::Last => {
            game_state_signal.signal.update(|game_state| {
                game_state.view_history();
                game_state.history_turn = game_state.state.turn.checked_sub(1);
            });
        }
        HistoryNavigation::Next => game_state_signal.next_history_turn(),
        HistoryNavigation::Previous => game_state_signal.previous_history_turn(),
        HistoryNavigation::MobileLast => game_state_signal.view_game(),
    }
}
