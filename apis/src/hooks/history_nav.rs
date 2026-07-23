use crate::providers::{
    analysis::{AnalysisContext, AnalysisStore, NodeId},
    game_state::{BoardView, GameStateStore, GameStateStoreFields},
};
use hive_lib::State;
use leptos::{ev::keydown, prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_use::{use_event_listener, use_window};
use wasm_bindgen::JsCast;
use web_sys::{Element, KeyboardEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisHistoryNavigation {
    First,
    Next,
    Previous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayHistoryNavigation {
    First,
    Last,
    Next,
    Previous,
}

impl AnalysisHistoryNavigation {
    fn target_node_id(self, analysis: AnalysisStore) -> Option<NodeId> {
        match self {
            Self::First => analysis.first_history_target_node_id(),
            Self::Next => analysis.next_history_target_node_id(),
            Self::Previous => analysis.previous_history_target_node_id(),
        }
    }
}

pub(crate) fn can_navigate_analysis_history(
    analysis: AnalysisStore,
    action: AnalysisHistoryNavigation,
) -> bool {
    action.target_node_id(analysis).is_some()
}

pub(crate) fn navigate_analysis_history(
    action: AnalysisHistoryNavigation,
    analysis: AnalysisStore,
    game_state: GameStateStore,
) -> bool {
    let updated_node_id = action.target_node_id(analysis);
    let Some(updated_node_id) = updated_node_id else {
        return false;
    };

    analysis.select_node(updated_node_id, game_state)
}

pub(crate) fn use_analysis_history_keyboard_navigation(
    active_analysis: impl Fn() -> Option<AnalysisContext> + 'static,
    before_navigate: impl Fn(AnalysisContext) + 'static,
) {
    let game_state = expect_context::<GameStateStore>();
    use_history_arrow_keyboard_navigation(
        AnalysisHistoryNavigation::Previous,
        AnalysisHistoryNavigation::Next,
        move |action| {
            let Some(analysis) = active_analysis() else {
                return;
            };
            let Some(target) = action.target_node_id(analysis.store) else {
                return;
            };
            before_navigate(analysis);
            analysis.store.select_node(target, game_state);
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
    view: BoardView,
    state: &State,
    action: PlayHistoryNavigation,
) -> bool {
    play_history_target(view, state, action).is_some()
}

pub(crate) fn play_history_target(
    view: BoardView,
    state: &State,
    action: PlayHistoryNavigation,
) -> Option<BoardView> {
    let displayed_turn = view.displayed_turn(state.turn);
    let turn = match action {
        PlayHistoryNavigation::First => {
            if displayed_turn.is_none() || displayed_turn == Some(0) {
                return None;
            }
            Some(0)
        }
        PlayHistoryNavigation::Last => {
            if view.is_last_turn(state.turn) {
                return None;
            }
            state.turn.checked_sub(1)
        }
        PlayHistoryNavigation::Next => {
            if !view.is_history() || view.is_last_turn(state.turn) {
                return None;
            }
            let last = state.turn.checked_sub(1)?;
            let mut next = displayed_turn.map_or(0, |turn| turn.saturating_add(1).min(last));
            if state.history.move_is_pass(next) {
                next = next.saturating_add(1).min(last);
            }
            Some(next)
        }
        PlayHistoryNavigation::Previous => match view {
            BoardView::Live => {
                let last = state.turn.checked_sub(1)?;
                if last == 0 {
                    Some(0)
                } else {
                    previous_turn(state, last)
                }
            }
            BoardView::History { turn } => {
                if turn.is_none() || turn == Some(0) {
                    return None;
                }
                previous_turn(state, turn?)
            }
        },
    };
    Some(BoardView::History { turn })
}

fn previous_turn(state: &State, current: usize) -> Option<usize> {
    let mut previous = current.checked_sub(1);
    if previous.is_some_and(|turn| state.history.move_is_pass(turn)) {
        previous = previous.and_then(|turn| turn.checked_sub(1));
    }
    previous
}

pub(crate) fn navigate_play_history(
    action: PlayHistoryNavigation,
    game_state: GameStateStore,
) -> bool {
    let target = game_state.with_untracked(|game_state| {
        play_history_target(game_state.board_view, &game_state.state, action)
    });
    let Some(target) = target else {
        return false;
    };
    game_state.board_view().set(target);
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
    use hive_lib::History;

    #[test]
    fn play_history_targets_cover_live_and_history_edges() {
        let source_history =
            History::from_pgn_str(include_str!("../../../engine/test_pgns/valid/p_game.pgn"))
                .expect("valid history");
        let mut history = History::new();
        history.game_type = source_history.game_type;
        history.moves = source_history.moves[..5].to_vec();
        let state = State::new_from_history(&history).expect("valid state");

        let cases = [
            (
                BoardView::Live,
                PlayHistoryNavigation::First,
                Some(BoardView::History { turn: Some(0) }),
            ),
            (BoardView::Live, PlayHistoryNavigation::Last, None),
            (
                BoardView::Live,
                PlayHistoryNavigation::Previous,
                Some(BoardView::History { turn: Some(3) }),
            ),
            (
                BoardView::History { turn: None },
                PlayHistoryNavigation::First,
                None,
            ),
            (
                BoardView::History { turn: None },
                PlayHistoryNavigation::Next,
                Some(BoardView::History { turn: Some(0) }),
            ),
            (
                BoardView::History { turn: Some(0) },
                PlayHistoryNavigation::Previous,
                None,
            ),
            (
                BoardView::History { turn: Some(2) },
                PlayHistoryNavigation::First,
                Some(BoardView::History { turn: Some(0) }),
            ),
            (
                BoardView::History { turn: Some(2) },
                PlayHistoryNavigation::Last,
                Some(BoardView::History { turn: Some(4) }),
            ),
        ];
        for (view, action, expected) in cases {
            assert_eq!(
                play_history_target(view, &state, action),
                expected,
                "unexpected target for {action:?} from {view:?}",
            );
        }

        let mut one_move = History::new();
        one_move.game_type = source_history.game_type;
        one_move.moves = source_history.moves[..1].to_vec();
        let one_move = State::new_from_history(&one_move).expect("valid one-move state");
        assert_eq!(
            play_history_target(BoardView::Live, &one_move, PlayHistoryNavigation::Previous,),
            Some(BoardView::History { turn: Some(0) }),
        );
    }

    #[test]
    fn play_history_targets_skip_passes() {
        let history =
            History::from_pgn_str(include_str!("../../../engine/test_pgns/valid/pass2.pgn"))
                .expect("valid pass history");
        let pass_turn = history
            .moves
            .iter()
            .position(|(piece, _)| piece == "pass")
            .expect("fixture contains a pass");
        let turn_before_pass = pass_turn.checked_sub(1).expect("pass follows a move");
        let turn_after_pass = pass_turn
            .checked_add(1)
            .filter(|turn| *turn < history.moves.len())
            .expect("pass precedes a move");
        let state = State::new_from_history(&history).expect("valid pass state");

        assert_eq!(
            play_history_target(
                BoardView::History {
                    turn: Some(turn_before_pass),
                },
                &state,
                PlayHistoryNavigation::Next,
            ),
            Some(BoardView::History {
                turn: Some(turn_after_pass),
            }),
        );
        assert_eq!(
            play_history_target(
                BoardView::History {
                    turn: Some(turn_after_pass),
                },
                &state,
                PlayHistoryNavigation::Previous,
            ),
            Some(BoardView::History {
                turn: Some(turn_before_pass),
            }),
        );
    }
}
