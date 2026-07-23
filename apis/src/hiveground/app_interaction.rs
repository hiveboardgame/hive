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
        game_state::{color_for_user, live_move_allowed, GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        Config,
    },
};
use hive_lib::{Piece, Position};
use leptos::prelude::*;

pub fn live_hiveground_interaction() -> HivegroundInteraction {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let identity = auth_context.identity;
    // Annotate mode suppresses piece selection so board taps annotate.
    let annotations = use_context::<AnnotationsSignal>();
    let capabilities = Signal::derive(move || {
        if annotations.is_some_and(|a| a.mode.get()) {
            HivegroundCapabilities::none()
        } else {
            live_capabilities(game_state, identity, config)
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
            HivegroundCapabilities::analysis_selection()
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

fn live_capabilities(
    game_state: GameStateStore,
    identity: Signal<Option<AuthIdentity>>,
    config: Signal<ConfigOpts>,
) -> HivegroundCapabilities {
    let user_id = identity.get().and_then(AuthIdentity::user_id);
    let allow_preselect = config.with(|config| config.allow_preselect);
    let white_id = game_state.white_id().get();
    let black_id = game_state.black_id().get();
    let user_color = color_for_user(user_id, white_id, black_id);
    let is_player = user_color.is_some();
    let can_move = game_state
        .state()
        .with(|state| live_move_allowed(user_color, state.turn_color, &state.game_status));

    if can_move {
        let mut capabilities = HivegroundCapabilities::live_selection();
        capabilities.preselect_piece = false;
        capabilities
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
    let allow_preselect = config.with_untracked(|config| config.allow_preselect);
    let user_id = identity.get_untracked().and_then(AuthIdentity::user_id);
    let is_player = game_state.user_color_untracked(user_id).is_some();
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
