use crate::providers::{
    analysis::{AnalysisSignal, AnalysisTree},
    game_state::{GameState, GameStateSignal, View},
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
    game_state: GameStateSignal,
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
    let game_state = expect_context::<GameStateSignal>();
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
    game_state_signal: GameStateSignal,
    set_move: &SignalSetter<Option<usize>>,
) {
    game_state_signal.signal.with_untracked(|game_state| {
        let move_param = match game_state.view {
            View::Game => None,
            View::History => game_state.history_turn.map(|turn| turn + 1),
        };

        set_move.set(move_param);
    });
}

pub(crate) fn can_navigate_play_history(
    game_state: &GameState,
    action: PlayHistoryNavigation,
) -> bool {
    match action {
        PlayHistoryNavigation::Last => !game_state.is_last_turn(),
        PlayHistoryNavigation::Next => {
            matches!(game_state.view, View::History) && !game_state.is_last_turn()
        }
        PlayHistoryNavigation::First => !game_state.is_first_history_turn(),
        PlayHistoryNavigation::Previous => {
            if matches!(game_state.view, View::Game) {
                game_state.state.turn > 0
            } else {
                !game_state.is_first_history_turn()
            }
        }
    }
}

pub(crate) fn navigate_play_history(
    action: PlayHistoryNavigation,
    mut game_state_signal: GameStateSignal,
) -> bool {
    if !game_state_signal
        .signal
        .with_untracked(|game_state| can_navigate_play_history(game_state, action))
    {
        return false;
    }

    match action {
        PlayHistoryNavigation::First => game_state_signal.first_history_turn(),
        PlayHistoryNavigation::Last => game_state_signal.last_history_turn(),
        PlayHistoryNavigation::Next => game_state_signal.next_history_turn(),
        PlayHistoryNavigation::Previous => {
            if game_state_signal
                .signal
                .with_untracked(|game_state| matches!(game_state.view, View::Game))
            {
                game_state_signal.last_history_turn();
            }

            if game_state_signal.signal.with_untracked(
                |game_state| matches!(game_state.history_turn, Some(turn) if turn > 0),
            ) {
                game_state_signal.previous_history_turn();
            }
        }
    }

    true
}

pub(crate) fn use_play_history_keyboard_navigation(
    game_state: GameStateSignal,
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
