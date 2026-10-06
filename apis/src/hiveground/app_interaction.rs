use super::interaction::{
    HivegroundAction,
    HivegroundActions,
    HivegroundCapabilities,
    HivegroundInteraction,
};
use crate::{
    common::{CurrentConfirm, MoveConfirm, PieceType},
    providers::{
        analysis::AnalysisContext,
        annotations::AnnotationsSignal,
        config::ConfigOpts,
        game_state::{live_move_allowed, GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        Config,
    },
};
use hive_lib::{Color, GameStatus, Piece, Position};
use leptos::prelude::*;

pub fn live_hiveground_interaction() -> HivegroundInteraction {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let identity = auth_context.identity;
    let user_color = game_state.user_color_as_signal(identity);
    // Annotate mode suppresses piece selection so board taps annotate.
    let annotations = use_context::<AnnotationsSignal>();
    let capabilities = Signal::derive(move || {
        if annotations.is_some_and(|a| a.mode.get()) {
            HivegroundCapabilities::none()
        } else {
            live_capabilities(game_state, user_color.get(), config)
        }
    });
    let handler = HivegroundActionHandler {
        game_state,
        analysis: None,
        api,
        current_confirm,
        config,
        identity,
    };

    HivegroundInteraction::new(capabilities, hiveground_actions(handler))
}

pub fn analysis_hiveground_interaction() -> HivegroundInteraction {
    let analysis = expect_context::<AnalysisContext>();
    let game_state = expect_context::<GameStateStore>();
    let api = expect_context::<ApiRequestsProvider>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let identity = expect_context::<AuthContext>().identity;
    let handler = HivegroundActionHandler {
        game_state,
        analysis: Some(analysis),
        api,
        current_confirm,
        config,
        identity,
    };

    let annotations = use_context::<AnnotationsSignal>();
    let capabilities = Signal::derive(move || {
        if annotations.is_some_and(|a| a.mode.get()) {
            HivegroundCapabilities::none()
        } else {
            let drag_color = config
                .with(|config| config.drag_and_drop)
                .then(|| game_state.state().with(|state| state.turn_color));
            HivegroundCapabilities {
                drag_color,
                ..HivegroundCapabilities::analysis_selection()
            }
        }
    });

    HivegroundInteraction::new(capabilities, hiveground_actions(handler))
}

struct HivegroundActionHandler {
    game_state: GameStateStore,
    analysis: Option<AnalysisContext>,
    api: ApiRequestsProvider,
    current_confirm: Memo<MoveConfirm>,
    config: Signal<ConfigOpts>,
    identity: Signal<Option<AuthIdentity>>,
}

fn hiveground_actions(handler: HivegroundActionHandler) -> HivegroundActions {
    HivegroundActions {
        dispatch: Some(Callback::new(move |action| {
            handler.handle(action);
        })),
    }
}

impl HivegroundActionHandler {
    fn handle(&self, action: HivegroundAction) {
        match action {
            HivegroundAction::SelectBoardPiece { piece, position } => {
                self.select_board_piece(piece, position);
            }
            HivegroundAction::SelectReservePiece { piece, position } => {
                self.select_reserve_piece(piece, position);
            }
            HivegroundAction::SelectTarget { position } => {
                self.select_target(position);
            }
            HivegroundAction::DropOnTarget {
                position,
                dragged,
                dragged_type,
            } => {
                self.drop_on_target(position, (dragged, dragged_type));
            }
            HivegroundAction::ResetSelection => {
                self.reset_selection();
            }
            HivegroundAction::PreselectPiece {
                piece,
                position,
                piece_type,
            } => {
                self.preselect_piece(piece, position, piece_type);
            }
        }
    }

    fn select_board_piece(&self, piece: Piece, position: Position) {
        let game_state = self.game_state;
        if game_state.is_move_allowed(self.analysis.is_some()) {
            game_state.show_moves(piece, position);
        }
    }

    fn select_reserve_piece(&self, piece: Piece, position: Position) {
        let game_state = self.game_state;
        if game_state.is_move_allowed(self.analysis.is_some()) {
            game_state.show_spawns(piece, position);
        }
    }

    fn select_target(&self, position: Position) {
        let game_state = self.game_state;
        if game_state.is_move_allowed(self.analysis.is_some()) {
            let was_selected = game_state
                .move_info()
                .with_untracked(|move_info| move_info.target_position == Some(position));
            game_state.set_target(position);
            let confirm = self.current_confirm.get_untracked();
            if confirm == MoveConfirm::Single || (confirm == MoveConfirm::Double && was_selected) {
                game_state.move_active(self.analysis, self.api.0.get_untracked());
            }
        } else {
            self.set_premove(position);
        }
    }

    fn drop_on_target(&self, position: Position, dragged: (Piece, PieceType)) {
        let game_state = self.game_state;
        let (is_target, drags_selection) = game_state.move_info().with_untracked(|move_info| {
            (
                move_info.target_positions.contains(&position),
                move_info
                    .active
                    .is_some_and(|selected| is_dragged_selection(selected, dragged)),
            )
        });
        if !is_target || !drags_selection {
            self.reset_selection();
        } else if game_state.is_move_allowed(self.analysis.is_some()) {
            game_state.set_target(position);
            game_state.move_active(self.analysis, self.api.0.get_untracked());
        } else {
            self.set_premove(position);
        }
    }

    fn set_premove(&self, position: Position) {
        let allow_premove = self.config.with_untracked(|config| config.allow_premove);
        let (is_target, queued_target) = self.game_state.move_info().with_untracked(|move_info| {
            (
                move_info.target_positions.contains(&position),
                move_info.queued_premove().map(|(_, target)| target),
            )
        });
        if queued_target == Some(position) {
            self.reset_selection();
        } else if queued_target.is_none() && self.analysis.is_none() && allow_premove && is_target {
            self.game_state.set_premove(position);
        }
    }

    fn reset_selection(&self) {
        self.game_state.clear_selection();
    }

    fn preselect_piece(&self, piece: Piece, position: Position, piece_type: PieceType) {
        if self.analysis.is_none() {
            preselect_piece(
                self.game_state,
                self.config,
                self.identity,
                piece,
                position,
                piece_type,
            );
        }
    }
}

// Selecting a reserve piece picks the lowest-numbered one of that bug, so match on bug there.
fn is_dragged_selection(
    (selected, selected_type): (Piece, PieceType),
    (dragged, dragged_type): (Piece, PieceType),
) -> bool {
    match (
        selected_type == PieceType::Board,
        dragged_type == PieceType::Board,
    ) {
        (true, true) => selected == dragged,
        (false, false) => selected.color() == dragged.color() && selected.bug() == dragged.bug(),
        _ => false,
    }
}

fn live_capabilities(
    game_state: GameStateStore,
    user_color: Option<Color>,
    config: Signal<ConfigOpts>,
) -> HivegroundCapabilities {
    let (allow_preselect, allow_premove, drag_and_drop) = config.with(|config| {
        (
            config.allow_preselect,
            config.allow_premove,
            config.drag_and_drop,
        )
    });
    let is_player = user_color.is_some();
    let (can_move, game_over) = game_state.state().with(|state| {
        (
            live_move_allowed(user_color, state.turn_color, &state.game_status),
            matches!(
                state.game_status,
                GameStatus::Finished(_) | GameStatus::Adjudicated
            ),
        )
    });
    let drag_color = user_color.filter(|_| drag_and_drop);
    let state_turn = game_state.state().with(|state| state.turn);
    let viewing_past_turn = game_state
        .board_view()
        .with(|view| view.is_history() && !view.is_last_turn(state_turn));

    // A selection made on a past position would act on the live one, unseen.
    if viewing_past_turn {
        HivegroundCapabilities::board_inspection()
    } else if can_move {
        HivegroundCapabilities {
            preselect_piece: false,
            drag_color,
            ..HivegroundCapabilities::live_selection()
        }
    } else if is_player && allow_premove && !game_over {
        HivegroundCapabilities {
            select_target: true,
            preselect_piece: true,
            inspect_stacks: true,
            drag_color,
            ..HivegroundCapabilities::none()
        }
    } else if is_player && allow_preselect {
        HivegroundCapabilities {
            preselect_piece: true,
            inspect_stacks: true,
            ..HivegroundCapabilities::none()
        }
    } else {
        HivegroundCapabilities::board_inspection()
    }
}

fn preselect_piece(
    game_state: GameStateStore,
    config: Signal<ConfigOpts>,
    identity: Signal<Option<AuthIdentity>>,
    piece: Piece,
    position: Position,
    piece_type: PieceType,
) {
    let (allow_preselect, allow_premove) =
        config.with_untracked(|config| (config.allow_preselect, config.allow_premove));
    let user_id = identity.get_untracked().and_then(AuthIdentity::user_id);
    let user_color = game_state.user_color_untracked(user_id);
    if allow_premove {
        if let Some(color) = user_color {
            game_state.show_premove_targets(piece, position, piece_type, color);
        }
        return;
    }
    let is_player = user_color.is_some();
    let current_turn_color = game_state.state().with_untracked(|state| state.turn_color);
    let is_selectable_piece = allow_preselect
        && match piece_type {
            PieceType::Board => true,
            PieceType::Inactive | PieceType::Reserve => !piece.is_color(current_turn_color),
            _ => false,
        };

    if !is_player || !is_selectable_piece {
        return;
    }

    game_state.move_info().update(|move_info| {
        move_info.active = Some((piece, piece_type));
        if piece_type == PieceType::Board {
            move_info.current_position = Some(position);
        } else {
            move_info.reserve_position = Some(position);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    #[test]
    fn a_drop_only_plays_the_piece_that_was_dragged() {
        let reserve = |name| (piece(name), PieceType::Inactive);
        let board = |name| (piece(name), PieceType::Board);

        let queen = (piece("wQ"), PieceType::Reserve);
        assert!(!is_dragged_selection(queen, reserve("wB1")));
        assert!(is_dragged_selection(queen, reserve("wQ")));

        let first_ant = (piece("wA1"), PieceType::Reserve);
        assert!(is_dragged_selection(first_ant, reserve("wA3")));
        assert!(!is_dragged_selection(first_ant, reserve("bA1")));
        assert!(!is_dragged_selection(first_ant, board("wA1")));

        let board_ant = board("wA1");
        assert!(is_dragged_selection(board_ant, board("wA1")));
        assert!(!is_dragged_selection(board_ant, board("wA2")));
        assert!(!is_dragged_selection(board_ant, reserve("wA1")));
    }
}
