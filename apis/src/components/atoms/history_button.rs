use crate::providers::game_state::GameStateSignal;
use leptos::{leptos_dom::helpers::debounce, *};
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
    let cloned_action = action.clone();
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-grasshopper-green transform transition-transform duration-300 active:scale-95 m-1 h-6 rounded-md border-cyan-500 border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let icon = match action {
        HistoryNavigation::First => icondata::AiFastBackwardFilled,
        HistoryNavigation::Last | HistoryNavigation::MobileLast => icondata::AiFastForwardFilled,
        HistoryNavigation::Next => icondata::AiStepForwardFilled,
        HistoryNavigation::Previous => icondata::AiStepBackwardFilled,
    };
    let is_disabled = move || {
        let game_state_signal = expect_context::<GameStateSignal>();
        let game_state = game_state_signal.signal.get();
        match cloned_action {
            HistoryNavigation::Last | HistoryNavigation::MobileLast | HistoryNavigation::Next => {
                game_state.is_last_turn()
            }
            HistoryNavigation::Previous | HistoryNavigation::First => {
                game_state.history_turn.is_none() || game_state.history_turn == Some(0)
            }
        }
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

            <Icon icon=icon/>
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
}
