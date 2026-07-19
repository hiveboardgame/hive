use crate::providers::{
    analysis::{AnalysisSignal, AnalysisTree},
    game_state::{BoardView, GameStateStore, GameStateStoreFields},
};
use leptos::{ev::keydown, prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_use::{use_event_listener, use_window};
use wasm_bindgen::JsCast;
use web_sys::{Element, KeyboardEvent};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AnalysisHistoryNavigation {
    First,
    Next,
    Previous,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayHistoryNavigation {
    First,
    Last,
    Next,
    Previous,
}

impl AnalysisHistoryNavigation {
    fn target_node_id(self, analysis: &AnalysisTree) -> Option<i32> {
        match self {
            Self::First => analysis.first_history_target_node_id(),
            Self::Next => analysis.next_history_target_node_id(),
            Self::Previous => analysis.previous_history_target_node_id(),
        }
    }
}

pub(crate) fn can_navigate_analysis_history(
    analysis: &AnalysisTree,
    action: AnalysisHistoryNavigation,
) -> bool {
    action.target_node_id(analysis).is_some()
}

pub(crate) fn navigate_analysis_history(
    action: AnalysisHistoryNavigation,
    analysis: RwSignal<AnalysisTree>,
    game_state: GameStateStore,
) -> bool {
    let updated_node_id = analysis.with_untracked(|analysis| action.target_node_id(analysis));
    let Some(updated_node_id) = updated_node_id else {
        return false;
    };

    analysis.update(|analysis| {
        analysis.update_node(updated_node_id, Some(game_state));
    });
    true
}

pub(crate) fn use_analysis_history_keyboard_navigation(
    active_analysis: impl Fn() -> Option<AnalysisSignal> + 'static,
    scroll_on_navigate: impl Fn() -> bool + 'static,
    before_navigate: impl Fn(AnalysisSignal) + 'static,
) {
    let game_state = expect_context::<GameStateStore>();
    use_history_arrow_keyboard_navigation(
        AnalysisHistoryNavigation::Previous,
        AnalysisHistoryNavigation::Next,
        move |action| {
            let Some(analysis) = active_analysis() else {
                return;
            };
            if !analysis
                .tree
                .with_untracked(|analysis| can_navigate_analysis_history(analysis, action))
            {
                return;
            }
            before_navigate(analysis);
            if navigate_analysis_history(action, analysis.tree, game_state) && scroll_on_navigate()
            {
                scroll_move_into_view();
            }
        },
    );
}

pub(crate) fn sync_play_move_query(
    game_state: GameStateStore,
    set_move: &SignalSetter<Option<usize>>,
) {
    let move_param = match game_state.board_view().get_untracked() {
        BoardView::Live => None,
        BoardView::History { turn } => turn.map(|turn| turn + 1),
    };

    set_move.set(move_param);
}

pub(crate) fn can_navigate_play_history(
    view: &BoardView,
    state_turn: usize,
    action: PlayHistoryNavigation,
) -> bool {
    let history_turn = match view {
        BoardView::Live => state_turn.checked_sub(1),
        BoardView::History { turn } => *turn,
    };
    let is_last_turn = view.is_last_turn(state_turn);
    let is_first_history_turn = history_turn.is_none() || history_turn == Some(0);

    match action {
        PlayHistoryNavigation::Last => !is_last_turn,
        PlayHistoryNavigation::Next => view.is_history() && !is_last_turn,
        PlayHistoryNavigation::First => !is_first_history_turn,
        PlayHistoryNavigation::Previous => {
            if matches!(view, BoardView::Live) {
                state_turn > 0
            } else {
                !is_first_history_turn
            }
        }
    }
}

pub(crate) fn navigate_play_history(
    action: PlayHistoryNavigation,
    game_state: GameStateStore,
) -> bool {
    let can_navigate = game_state.with_untracked(|game_state| {
        can_navigate_play_history(&game_state.board_view, game_state.state.turn, action)
    });
    if !can_navigate {
        return false;
    }

    match action {
        PlayHistoryNavigation::First => game_state.first_history_turn(),
        PlayHistoryNavigation::Last => game_state.last_history_turn(),
        PlayHistoryNavigation::Next => game_state.next_history_turn(),
        PlayHistoryNavigation::Previous => {
            if matches!(game_state.board_view().get_untracked(), BoardView::Live) {
                game_state.last_history_turn();
            }

            if matches!(
                game_state.board_view().get_untracked(),
                BoardView::History { turn: Some(turn) } if turn > 0
            ) {
                game_state.previous_history_turn();
            }
        }
    }

    true
}

pub(crate) fn use_play_history_keyboard_navigation(
    game_state: GameStateStore,
    set_move: SignalSetter<Option<usize>>,
    on_navigate: Callback<PlayHistoryNavigation>,
) {
    use_history_arrow_keyboard_navigation(
        PlayHistoryNavigation::Previous,
        PlayHistoryNavigation::Next,
        move |action| {
            if navigate_play_history(action, game_state) {
                sync_play_move_query(game_state, &set_move);
                on_navigate.run(action);
            }
        },
    );
}

pub(crate) fn scroll_move_into_view() {
    let active = use_window()
        .as_ref()
        .and_then(|window| window.document())
        .and_then(|document| {
            document
                .query_selector("[data-history-current='true']")
                .ok()
        })
        .flatten();
    if let Some(element) = active {
        element.scroll_into_view_with_bool(false);
    }
}

fn use_history_arrow_keyboard_navigation<Action: Copy + 'static>(
    previous_action: Action,
    next_action: Action,
    navigate: impl Fn(Action) + 'static,
) {
    let body = use_window().document().body();

    _ = use_event_listener(body, keydown, move |evt: KeyboardEvent| {
        let action = match evt.key().as_str() {
            "ArrowLeft" => previous_action,
            "ArrowRight" => next_action,
            _ => return,
        };

        if should_ignore_history_key_event(&evt) {
            return;
        }
        evt.prevent_default();
        navigate(action);
    });
}

fn should_ignore_history_key_event(evt: &KeyboardEvent) -> bool {
    evt.alt_key()
        || evt.ctrl_key()
        || evt.meta_key()
        || evt.shift_key()
        || key_event_target_uses_arrows(evt)
}

fn key_event_target_uses_arrows(evt: &KeyboardEvent) -> bool {
    let Some(target) = evt.target() else {
        return false;
    };
    let Some(element) = target.dyn_ref::<Element>() else {
        return false;
    };

    matches!(
        element.tag_name().to_uppercase().as_str(),
        "INPUT" | "TEXTAREA" | "SELECT"
    ) || element
        .closest("[contenteditable='true']")
        .ok()
        .flatten()
        .is_some()
        || element
            .get_attribute("contenteditable")
            .is_some_and(|value| value != "false")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hive_lib::{History, State};
    use leptos::prelude::Owner;

    #[test]
    fn previous_from_live_enters_history_before_the_live_edge() {
        let owner = Owner::new();
        owner.with(|| {
            let source_history = History::from_pgn_str(
                include_str!("../../../engine/test_pgns/valid/p_game.pgn").to_string(),
            )
            .expect("valid history");
            let mut history = History::new();
            history.game_type = source_history.game_type;
            history.moves = source_history.moves[..5].to_vec();

            let game_state = GameStateStore::new();
            game_state.reset_with_state(
                State::new_from_history(&history).expect("history reconstructs a valid state"),
            );

            assert!(navigate_play_history(
                PlayHistoryNavigation::Previous,
                game_state,
            ));
            assert_eq!(
                game_state.board_view().get_untracked(),
                BoardView::History { turn: Some(3) },
            );
        });
    }
}
