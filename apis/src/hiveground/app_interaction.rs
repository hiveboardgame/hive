use super::interaction::{
    HivegroundAction,
    HivegroundActions,
    HivegroundCapabilities,
    HivegroundInteraction,
};
use crate::{
    common::{CurrentConfirm, MoveConfirm, PieceType},
    providers::{
        analysis::AnalysisSignal,
        config::ConfigOpts,
        game_state::{GameStateStore, GameStateStoreFields},
        ApiRequestsProvider,
        AuthContext,
        Config,
    },
    responses::AccountResponse,
};
use hive_lib::{Color, GameStatus, Piece, Position};
use leptos::prelude::*;
use uuid::Uuid;

pub fn live_hiveground_interaction() -> HivegroundInteraction {
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let user = auth_context.user;
    let capabilities = Signal::derive(move || live_capabilities(game_state, user, config));
    let handler = HivegroundActionHandler {
        game_state,
        analysis: None,
        api,
        current_confirm,
        config,
        user,
    };

    HivegroundInteraction::new(capabilities, hiveground_actions(handler))
}

pub fn analysis_hiveground_interaction() -> HivegroundInteraction {
    let analysis = expect_context::<AnalysisSignal>();
    let game_state = expect_context::<GameStateStore>();
    let api = expect_context::<ApiRequestsProvider>();
    let current_confirm = expect_context::<CurrentConfirm>().0;
    let config = expect_context::<Config>().0;
    let user = expect_context::<AuthContext>().user;
    let handler = HivegroundActionHandler {
        game_state,
        analysis: Some(analysis),
        api,
        current_confirm,
        config,
        user,
    };

    HivegroundInteraction::new(
        HivegroundCapabilities::analysis_selection(),
        hiveground_actions(handler),
    )
}

struct HivegroundActionHandler {
    game_state: GameStateStore,
    analysis: Option<AnalysisSignal>,
    api: ApiRequestsProvider,
    current_confirm: Memo<MoveConfirm>,
    config: Signal<ConfigOpts>,
    user: Signal<Option<AccountResponse>>,
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
                game_state.move_active(self.analysis.clone(), self.api.0.get_untracked());
            }
        }
    }

    fn reset_selection(&self) {
        let game_state = self.game_state;
        game_state.reset();
    }

    fn preselect_piece(&self, piece: Piece, position: Position, piece_type: PieceType) {
        if self.analysis.is_none() {
            preselect_piece(
                self.game_state,
                self.config,
                self.user,
                piece,
                position,
                piece_type,
            );
        }
    }
}

fn live_capabilities(
    game_state: GameStateStore,
    user: Signal<Option<AccountResponse>>,
    config: Signal<ConfigOpts>,
) -> HivegroundCapabilities {
    let user_id = user.with(|user| user.as_ref().map(|user| user.id));
    let allow_preselect = config.with(|config| config.allow_preselect);
    let white_id = game_state.white_id().get();
    let black_id = game_state.black_id().get();
    let turn_color = game_state.state().with(|state| state.turn_color);
    let game_status = game_state.state().with(|state| state.game_status.clone());
    let is_player = user_is_player(user_id, white_id, black_id);
    let current_player_id = match turn_color {
        Color::White => white_id,
        Color::Black => black_id,
    };
    let is_current_player = user_id.is_some() && user_id == current_player_id;
    let is_finished = matches!(
        game_status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    );

    if is_current_player && !is_finished {
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
    user: Signal<Option<AccountResponse>>,
    piece: Piece,
    position: Position,
    piece_type: PieceType,
) {
    let allow_preselect = config.with_untracked(|config| config.allow_preselect);
    let user_id = user.with_untracked(|user| user.as_ref().map(|user| user.id));
    let white_id = game_state.white_id().get_untracked();
    let black_id = game_state.black_id().get_untracked();
    let is_player = user_is_player(user_id, white_id, black_id);
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

fn user_is_player(user_id: Option<Uuid>, white_id: Option<Uuid>, black_id: Option<Uuid>) -> bool {
    user_id.is_some() && (user_id == white_id || user_id == black_id)
}
