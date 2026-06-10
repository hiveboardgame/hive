use crate::{
    components::organisms::side_board::move_query_signal,
    providers::game_state::{GameStateStore, GameStateStoreFields, View},
};
use leptos::{
    html,
    leptos_dom::helpers::debounce,
    prelude::*,
    reactive::wrappers::write::SignalSetter,
};
use leptos_icons::*;

#[derive(Clone)]
pub enum HistoryNavigation {
    First,
    Last,
    Next,
    Previous,
    MobileLast,
}

pub fn sync_play_move_query(game_state: GameStateStore, set_move: &SignalSetter<Option<usize>>) {
    let move_param = match game_state.view().get_untracked() {
        View::Game => None,
        View::History => game_state
            .history_turn()
            .get_untracked()
            .map(|turn| turn + 1),
    };
    set_move.set(move_param);
}

#[component]
pub fn HistoryButton(
    action: HistoryNavigation,
    #[prop(optional)] post_action: Option<Callback<()>>,
    #[prop(optional)] node_ref: Option<NodeRef<html::Button>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let (_move, set_move) = move_query_signal();
    let is_last_turn = game_state.is_last_turn_as_signal();
    let is_first_turn = game_state.is_first_turn_as_signal();
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
        send_action(&action, game_state);
        if let Some(post_action) = post_action {
            post_action.run(())
        }
        sync_play_move_query(game_state, &set_move);
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

fn send_action(action: &HistoryNavigation, game_state: GameStateStore) {
    match action {
        HistoryNavigation::First => game_state.first_history_turn(),
        HistoryNavigation::Last => {
            game_state.view_history_at_last_turn();
        }
        HistoryNavigation::Next => game_state.next_history_turn(),
        HistoryNavigation::Previous => game_state.previous_history_turn(),
        HistoryNavigation::MobileLast => game_state.view_game(),
    }
}
