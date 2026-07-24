use crate::{
    components::{
        atoms::history_nav_button::HistoryNavButton,
        organisms::side_board::move_query_signal,
    },
    hooks::history_nav::{can_navigate_play_history, navigate_play_history, sync_play_move_query},
    providers::game_state::{GameStateStore, GameStateStoreFields},
};
use leptos::prelude::*;
use leptos_icons::Icon;

pub use crate::hooks::history_nav::PlayHistoryNavigation;

#[component]
pub fn HistoryButton(
    action: PlayHistoryNavigation,
    #[prop(optional)] post_action: Option<Callback<()>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let (_move, set_move) = move_query_signal();
    let board_view = game_state.board_view();
    let state = game_state.state();
    let is_disabled = Memo::new(move |_| {
        let board_view = board_view.get();
        state.with(|state| !can_navigate_play_history(board_view, state, action))
    });
    let on_press = Callback::new(move |()| {
        if navigate_play_history(action, game_state) {
            if let Some(post_action) = post_action {
                post_action.run(())
            }
            sync_play_move_query(game_state, &set_move);
        }
    });
    let icon = match action {
        PlayHistoryNavigation::First => icondata_ai::AiFastBackwardFilled,
        PlayHistoryNavigation::Last => icondata_ai::AiFastForwardFilled,
        PlayHistoryNavigation::Next => icondata_ai::AiStepForwardFilled,
        PlayHistoryNavigation::Previous => icondata_ai::AiStepBackwardFilled,
    };

    view! {
        <HistoryNavButton disabled=is_disabled on_press=on_press>
            <Icon icon=icon />
        </HistoryNavButton>
    }
}
