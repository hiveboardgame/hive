use super::{
    config::ReserveLayout,
    model::{
        ActiveMarkerState,
        DrawingBounds,
        HivegroundRenderModel,
        LastMoveDirection,
        PieceShadow,
        RenderLayer,
        RenderLayerKind,
        RenderStack,
    },
};
use crate::common::{MoveInfo, PieceType};
use hudsoni::{Board, Bug, BugStack, Color, GameStatus, Piece, Position, State};
use std::{collections::HashMap, str::FromStr};

const DOUBLE_ROW_RESERVE_COLUMNS: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewSource {
    Board,
    Reserve,
}

#[derive(Clone, Debug, PartialEq)]
struct HivegroundConfig {
    stacks: Vec<DisplayStack>,
    overlays: OverlaySet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DisplayStack {
    position: Position,
    pieces: Vec<DisplayPiece>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DisplayPiece {
    piece: Piece,
    piece_type: PieceType,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct OverlaySet {
    last_move: Option<LastMoveOverlay>,
    active_marker: Option<ActiveMarkerOverlay>,
    target_positions: Vec<Position>,
    ghost_piece: Option<GhostPieceOverlay>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LastMoveOverlay {
    from: Option<Position>,
    to: Option<Position>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveMarkerOverlay {
    position: Position,
    remove_source_top_piece: bool,
    suppress_source_shadow: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GhostPieceOverlay {
    position: Position,
    piece: Piece,
    piece_type: PieceType,
}

impl OverlaySet {
    fn positions(&self) -> Vec<Position> {
        let mut positions = Vec::new();
        positions.extend(self.target_positions.iter().copied());
        positions.extend(self.ghost_piece.as_ref().map(|ghost| ghost.position));
        positions.extend(self.active_marker.as_ref().map(|active| active.position));
        if let Some(last_move) = &self.last_move {
            positions.extend(last_move.from);
            positions.extend(last_move.to);
        }
        positions
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReserveRenderOptions {
    pub reserve_color: Color,
    pub alignment: ReserveLayout,
    pub interactivity: ReserveInteractivity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReserveInteractivity {
    pub viewing_past_turn: bool,
    pub status: GameStatus,
    pub user_color: Option<Color>,
    pub tournament: bool,
    pub analysis: bool,
}

fn static_display_stacks(board: &Board) -> Vec<DisplayStack> {
    display_stacks(board, |_, _| PieceType::History)
}

fn board_display_stacks(board: &Board) -> Vec<DisplayStack> {
    display_stacks(board, base_piece_type)
}

fn display_stacks(
    board: &Board,
    piece_type_for_level: impl Fn(usize, usize) -> PieceType,
) -> Vec<DisplayStack> {
    occupied_board_stacks(board)
        .map(|(position, stack)| {
            let top_index = stack.len().saturating_sub(1);
            let pieces = stack.pieces[..stack.len()]
                .iter()
                .enumerate()
                .map(|(level, piece)| {
                    let piece_type = piece_type_for_level(level, top_index);
                    DisplayPiece {
                        piece: *piece,
                        piece_type,
                    }
                })
                .collect();

            DisplayStack { position, pieces }
        })
        .collect()
}

fn occupied_board_stacks(board: &Board) -> impl Iterator<Item = (Position, &BugStack)> + '_ {
    let mut positions = board.all_taken_positions().collect::<Vec<_>>();
    positions.sort_unstable_by_key(|position| (position.r, position.q));
    positions
        .into_iter()
        .map(move |position| (position, board.board.get(position)))
}

pub fn build_board_render_model(state: &State, move_info: &MoveInfo) -> HivegroundRenderModel {
    let config = HivegroundConfig {
        stacks: board_display_stacks(&state.board),
        overlays: board_overlay_set(state, move_info),
    };
    build_hiveground_render_model(&config)
}

pub fn build_static_render_model(board: &Board) -> HivegroundRenderModel {
    let config = HivegroundConfig {
        stacks: static_display_stacks(board),
        overlays: OverlaySet::default(),
    };
    build_hiveground_render_model(&config)
}

pub fn build_reserve_render_model(
    state: &State,
    reserve_board: &Board,
    move_info: &MoveInfo,
    options: ReserveRenderOptions,
) -> HivegroundRenderModel {
    let config = HivegroundConfig {
        stacks: reserve_display_stacks(state, reserve_board, &options),
        overlays: reserve_overlay_set(
            options.reserve_color,
            move_info,
            options.interactivity.user_color,
            options.interactivity.analysis,
        ),
    };
    build_hiveground_render_model(&config)
}

fn build_hiveground_render_model(config: &HivegroundConfig) -> HivegroundRenderModel {
    let positions = render_positions(config);
    let stacks_by_position = config
        .stacks
        .iter()
        .map(|stack| (stack.position, stack))
        .collect::<HashMap<_, _>>();
    let stacks = positions
        .iter()
        .filter_map(|position| {
            build_stack_model(
                stacks_by_position.get(position).copied(),
                &config.overlays,
                *position,
            )
        })
        .collect::<Vec<_>>();

    HivegroundRenderModel {
        bounds: DrawingBounds::from_positions(&positions),
        stacks,
    }
}

fn board_overlay_set(state: &State, move_info: &MoveInfo) -> OverlaySet {
    let active_piece = move_info.active.map(|(piece, _)| piece);
    let preview_source = preview_source(move_info);
    let has_active_preview = active_piece.is_some() && preview_source.is_some();
    let active_marker = match preview_source {
        Some((PreviewSource::Board, position)) => Some(ActiveMarkerOverlay {
            position,
            remove_source_top_piece: move_info.target_position.is_some(),
            suppress_source_shadow: active_piece.is_some(),
        }),
        Some((PreviewSource::Reserve, _)) | None => None,
    };
    let target_positions = if has_active_preview {
        move_info.target_positions.clone()
    } else {
        Vec::new()
    };
    let ghost_piece = active_piece
        .zip(preview_source)
        .and_then(|(piece, (source, _))| {
            move_info.target_position.map(|position| GhostPieceOverlay {
                position,
                piece,
                piece_type: ghost_piece_type_for_source(source),
            })
        });

    OverlaySet {
        last_move: (!has_active_preview).then_some(LastMoveOverlay {
            from: state.board.last_move.0,
            to: state.board.last_move.1,
        }),
        active_marker,
        target_positions,
        ghost_piece,
    }
}

fn reserve_overlay_set(
    reserve_color: Color,
    move_info: &MoveInfo,
    user_color: Option<Color>,
    analysis: bool,
) -> OverlaySet {
    OverlaySet {
        active_marker: reserve_active_position(reserve_color, move_info, user_color, analysis).map(
            |position| ActiveMarkerOverlay {
                position,
                remove_source_top_piece: move_info.target_position.is_some(),
                suppress_source_shadow: move_info.active.is_some(),
            },
        ),
        ..OverlaySet::default()
    }
}

fn preview_source(move_info: &MoveInfo) -> Option<(PreviewSource, Position)> {
    if let Some(position) = move_info.current_position {
        Some((PreviewSource::Board, position))
    } else {
        move_info
            .reserve_position
            .map(|position| (PreviewSource::Reserve, position))
    }
}

fn render_positions(config: &HivegroundConfig) -> Vec<Position> {
    let mut positions = config
        .stacks
        .iter()
        .map(|stack| stack.position)
        .collect::<Vec<_>>();

    positions.extend(config.overlays.positions());

    sort_dedup_positions(positions)
}

fn reserve_display_stacks(
    state: &State,
    reserve_board: &Board,
    options: &ReserveRenderOptions,
) -> Vec<DisplayStack> {
    let reserve = reserve_board.reserve(options.reserve_color, state.game_type);
    let mut reserve_slot_index = 0;
    let mut stacks = Vec::new();

    for bug in Bug::all() {
        let reserve_pieces = reserve.get(&bug);
        let has_display_stack = reserve_pieces.is_some();

        if let Some(piece_strings) = reserve_pieces {
            let position = reserve_position_for_slot(reserve_slot_index, options.alignment);
            let pieces = piece_strings
                .iter()
                .rev()
                .map(|piece_str| {
                    let piece = Piece::from_str(piece_str).expect("Parsed piece");
                    let piece_type = if reserve_piece_active(state, &options.interactivity, piece) {
                        PieceType::Reserve
                    } else {
                        PieceType::Inactive
                    };

                    DisplayPiece { piece, piece_type }
                })
                .collect();

            stacks.push(DisplayStack { position, pieces });
        }

        if has_display_stack || options.alignment == ReserveLayout::DoubleRow {
            reserve_slot_index += 1;
        }
    }

    stacks
}

fn build_stack_model(
    display_stack: Option<&DisplayStack>,
    overlays: &OverlaySet,
    position: Position,
) -> Option<RenderStack> {
    let mut layers = display_stack.map(piece_layers).unwrap_or_default();
    apply_overlays(&mut layers, overlays, position);

    (!layers.is_empty()).then_some(RenderStack { position, layers })
}

fn apply_overlays(layers: &mut Vec<RenderLayer>, overlays: &OverlaySet, position: Position) {
    if let Some(last_move) = &overlays.last_move {
        if last_move.to == Some(position) {
            add_last_move_to(layers);
        }
        if last_move.from == Some(position) {
            add_last_move_from(layers);
        }
    }

    if let Some(active_marker) = &overlays.active_marker {
        if active_marker.position == position {
            add_active_marker(
                layers,
                active_marker.remove_source_top_piece,
                active_marker.suppress_source_shadow,
            );
        }
    }

    if overlays.target_positions.contains(&position) {
        add_target(layers);
    }

    if let Some(ghost_piece) = &overlays.ghost_piece {
        if ghost_piece.position == position {
            add_ghost(layers, ghost_piece);
        }
    }
}

fn piece_layers(stack: &DisplayStack) -> Vec<RenderLayer> {
    stack
        .pieces
        .iter()
        .enumerate()
        .map(|(level, piece)| RenderLayer {
            level,
            kind: RenderLayerKind::Piece {
                piece: piece.piece,
                piece_type: piece.piece_type,
                shadow: PieceShadow::for_piece_type(piece.piece_type),
            },
        })
        .collect()
}

fn base_piece_type(level: usize, top_index: usize) -> PieceType {
    if level == top_index {
        PieceType::Board
    } else {
        PieceType::Covered
    }
}

fn add_last_move_to(layers: &mut Vec<RenderLayer>) {
    let top = layers.pop();
    layers.push(RenderLayer {
        level: layers.len(),
        kind: RenderLayerKind::LastMove {
            direction: LastMoveDirection::To,
        },
    });
    if let Some(piece) = top {
        layers.push(piece.with_shadow(PieceShadow::None));
    }
}

fn add_last_move_from(layers: &mut Vec<RenderLayer>) {
    layers.push(RenderLayer {
        level: layers.len(),
        kind: RenderLayerKind::LastMove {
            direction: LastMoveDirection::From,
        },
    });
}

fn add_active_marker(
    layers: &mut Vec<RenderLayer>,
    remove_source_top_piece: bool,
    suppress_source_shadow: bool,
) {
    let len = layer_count_for_tile(layers);
    if remove_source_top_piece {
        layers.pop();
    } else if suppress_source_shadow {
        suppress_top_piece_shadow(layers);
    }
    let state = active_state(layers);
    layers.push(RenderLayer {
        level: len.saturating_sub(1),
        kind: RenderLayerKind::Active { state },
    });
}

fn suppress_top_piece_shadow(layers: &mut [RenderLayer]) {
    if let Some(RenderLayer {
        kind: RenderLayerKind::Piece { shadow, .. },
        ..
    }) = layers.last_mut()
    {
        *shadow = PieceShadow::None;
    }
}

fn add_target(layers: &mut Vec<RenderLayer>) {
    layers.push(RenderLayer {
        level: layer_count_for_tile(layers),
        kind: RenderLayerKind::Target,
    });
}

fn add_ghost(layers: &mut Vec<RenderLayer>, ghost_piece: &GhostPieceOverlay) {
    let level = layer_count_for_tile(layers);
    layers.push(RenderLayer {
        level,
        kind: RenderLayerKind::Piece {
            piece: ghost_piece.piece,
            piece_type: ghost_piece.piece_type,
            shadow: PieceShadow::for_piece_type(ghost_piece.piece_type),
        },
    });
}

fn layer_count_for_tile(layers: &[RenderLayer]) -> usize {
    layers
        .iter()
        .filter(|layer| !matches!(&layer.kind, RenderLayerKind::LastMove { .. }))
        .count()
}

fn active_state(layers: &[RenderLayer]) -> ActiveMarkerState {
    match layers.last().map(|layer| &layer.kind) {
        Some(RenderLayerKind::Target) => ActiveMarkerState::Board,
        Some(RenderLayerKind::Piece { piece_type, .. }) => match piece_type {
            PieceType::Board | PieceType::Spawn => ActiveMarkerState::Board,
            PieceType::Reserve => ActiveMarkerState::Reserve,
            _ => ActiveMarkerState::None,
        },
        _ => ActiveMarkerState::None,
    }
}

fn ghost_piece_type_for_source(source: PreviewSource) -> PieceType {
    match source {
        PreviewSource::Board => PieceType::Move,
        PreviewSource::Reserve => PieceType::Spawn,
    }
}

fn reserve_active_position(
    reserve_color: Color,
    move_info: &MoveInfo,
    user_color: Option<Color>,
    analysis: bool,
) -> Option<Position> {
    let active_color = move_info.active.as_ref().map(|(piece, _)| piece.color());
    if active_color == Some(reserve_color) || (!analysis && user_color == Some(reserve_color)) {
        move_info.reserve_position
    } else {
        None
    }
}

fn reserve_piece_active(state: &State, interactivity: &ReserveInteractivity, piece: Piece) -> bool {
    if interactivity.viewing_past_turn {
        return false;
    }
    if interactivity.tournament && matches!(&interactivity.status, GameStatus::NotStarted) {
        return false;
    }
    if !piece.is_color(state.turn_color) {
        return false;
    };
    if state.tournament && piece.bug() == Bug::Queen && state.turn < 2 {
        return false;
    };
    if state.board.queen_required(state.turn, state.turn_color) && piece.bug() != Bug::Queen {
        return false;
    };
    if matches!(
        &interactivity.status,
        GameStatus::Finished(_) | GameStatus::Adjudicated
    ) {
        return interactivity.analysis;
    }
    true
}

fn reserve_position_for_slot(slot_index: i32, alignment: ReserveLayout) -> Position {
    if alignment == ReserveLayout::SingleRow {
        Position::new(slot_index, 0)
    } else {
        Position::new(
            slot_index % DOUBLE_ROW_RESERVE_COLUMNS,
            slot_index / DOUBLE_ROW_RESERVE_COLUMNS,
        )
    }
}

fn sort_dedup_positions(mut positions: Vec<Position>) -> Vec<Position> {
    positions.sort_unstable_by_key(|position| (position.r, position.q));
    positions.dedup();
    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use hudsoni::{Bug, Color, GameResult, GameStatus, GameType, Piece};

    fn piece(piece: &str) -> Piece {
        piece.parse().expect("test piece parses")
    }

    fn state_with_stacks(stacks: Vec<(Position, Vec<&str>)>) -> State {
        let mut state = State::new(GameType::MLP, false);
        state.game_status = GameStatus::InProgress;
        for (position, pieces) in stacks {
            for piece in pieces {
                state.board.insert(position, self::piece(piece), true);
            }
        }
        state
    }

    fn stack_at(model: &HivegroundRenderModel, position: Position) -> &RenderStack {
        model
            .stacks
            .iter()
            .find(|stack| stack.position == position)
            .expect("stack exists")
    }

    fn layer_summary(stack: &RenderStack) -> Vec<String> {
        stack
            .layers
            .iter()
            .map(|layer| match &layer.kind {
                RenderLayerKind::Piece {
                    piece, piece_type, ..
                } => format!("piece:{piece}:{piece_type:?}:{}", layer.level),
                RenderLayerKind::Target => format!("target:{}", layer.level),
                RenderLayerKind::Active { state, .. } => {
                    format!("active:{state:?}:{}", layer.level)
                }
                RenderLayerKind::LastMove { direction, .. } => {
                    format!("last:{direction:?}:{}", layer.level)
                }
            })
            .collect()
    }

    fn piece_summary(stack: &RenderStack) -> Vec<(Piece, PieceType, usize)> {
        stack
            .layers
            .iter()
            .filter_map(|layer| match layer {
                RenderLayer {
                    level,
                    kind:
                        RenderLayerKind::Piece {
                            piece, piece_type, ..
                        },
                } => Some((*piece, *piece_type, *level)),
                _ => None,
            })
            .collect()
    }

    fn piece_shadow_summary(stack: &RenderStack) -> Vec<(Piece, PieceType, PieceShadow, usize)> {
        stack
            .layers
            .iter()
            .filter_map(|layer| match layer {
                RenderLayer {
                    level,
                    kind:
                        RenderLayerKind::Piece {
                            piece,
                            piece_type,
                            shadow,
                        },
                } => Some((*piece, *piece_type, *shadow, *level)),
                _ => None,
            })
            .collect()
    }

    fn piece_types(model: &HivegroundRenderModel) -> Vec<PieceType> {
        model
            .stacks
            .iter()
            .flat_map(piece_summary)
            .map(|(_, piece_type, _)| piece_type)
            .collect()
    }

    fn assert_all_model_pieces_have_type(model: &HivegroundRenderModel, expected: PieceType) {
        let piece_types = piece_types(model);
        assert!(!piece_types.is_empty());
        assert!(
            piece_types.iter().all(|piece_type| *piece_type == expected),
            "piece types: {piece_types:?}"
        );
    }

    fn model_has_active_layer(model: &HivegroundRenderModel) -> bool {
        model
            .stacks
            .iter()
            .flat_map(|stack| stack.layers.iter())
            .any(|layer| matches!(&layer.kind, RenderLayerKind::Active { .. }))
    }

    fn reserve_options() -> ReserveRenderOptions {
        ReserveRenderOptions {
            reserve_color: Color::White,
            alignment: ReserveLayout::SingleRow,
            interactivity: reserve_interactivity(),
        }
    }

    fn reserve_interactivity() -> ReserveInteractivity {
        ReserveInteractivity {
            viewing_past_turn: false,
            status: GameStatus::InProgress,
            user_color: Some(Color::White),
            tournament: false,
            analysis: false,
        }
    }

    fn reserve_model(state: &State, options: ReserveRenderOptions) -> HivegroundRenderModel {
        let move_info = MoveInfo::new();
        build_reserve_render_model(state, &state.board, &move_info, options)
    }

    fn reserve_model_with_board(
        state: &State,
        reserve_board: &Board,
        options: ReserveRenderOptions,
    ) -> HivegroundRenderModel {
        let move_info = MoveInfo::new();
        build_reserve_render_model(state, reserve_board, &move_info, options)
    }

    fn reserve_model_with_move_info(
        state: &State,
        move_info: &MoveInfo,
        options: ReserveRenderOptions,
    ) -> HivegroundRenderModel {
        build_reserve_render_model(state, &state.board, move_info, options)
    }

    #[test]
    fn board_model_single_piece_renders_board_piece() {
        let position = Position::new(3, 2);
        let state = state_with_stacks(vec![(position, vec!["wQ"])]);

        let model = build_board_render_model(&state, &MoveInfo::new());

        assert_eq!(model.stacks.len(), 1);
        assert_eq!(
            piece_summary(stack_at(&model, position)),
            vec![(piece("wQ"), PieceType::Board, 0)]
        );
    }

    #[test]
    fn board_model_marks_covered_and_top_piece() {
        let position = Position::new(3, 2);
        let state = state_with_stacks(vec![(position, vec!["wQ", "bB1"])]);

        let model = build_board_render_model(&state, &MoveInfo::new());

        assert_eq!(
            layer_summary(stack_at(&model, position)),
            vec!["piece:wQ:Covered:0", "piece:bB1:Board:1"]
        );
        assert_eq!(model.bounds.min_q, position.q);
        assert_eq!(model.bounds.max_q, position.q);
    }

    #[test]
    fn static_model_marks_all_pieces_as_history() {
        let position = Position::new(3, 2);
        let state = state_with_stacks(vec![(position, vec!["wQ", "bB1"])]);

        let model = build_static_render_model(&state.board);

        assert_eq!(
            layer_summary(stack_at(&model, position)),
            vec!["piece:wQ:History:0", "piece:bB1:History:1"]
        );
    }

    #[test]
    fn board_model_keeps_last_move_to_under_top_piece() {
        let from = Position::new(2, 2);
        let position = Position::new(3, 2);
        let mut state = state_with_stacks(vec![(position, vec!["wQ", "bB1"])]);
        state.board.last_move = (Some(from), Some(position));

        let model = build_board_render_model(&state, &MoveInfo::new());

        assert_eq!(
            layer_summary(stack_at(&model, position)),
            vec!["piece:wQ:Covered:0", "last:To:1", "piece:bB1:Board:1",]
        );
        assert_eq!(
            piece_shadow_summary(stack_at(&model, position)),
            vec![
                (piece("wQ"), PieceType::Covered, PieceShadow::Design, 0),
                (piece("bB1"), PieceType::Board, PieceShadow::None, 1),
            ]
        );
    }

    #[test]
    fn board_model_renders_last_move_from_on_empty_origin() {
        let from = Position::new(2, 2);
        let to = Position::new(3, 2);
        let mut state = state_with_stacks(vec![(to, vec!["wQ"])]);
        state.board.last_move = (Some(from), Some(to));

        let model = build_board_render_model(&state, &MoveInfo::new());

        assert_eq!(layer_summary(stack_at(&model, from)), vec!["last:From:0"]);
    }

    #[test]
    fn board_model_renders_targets_on_empty_and_occupied_positions() {
        let origin = Position::new(2, 2);
        let occupied = Position::new(3, 2);
        let empty = Position::new(4, 2);
        let state = state_with_stacks(vec![(origin, vec!["bB1"]), (occupied, vec!["wQ"])]);
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("bB1"), PieceType::Board));
        move_info.current_position = Some(origin);
        move_info.target_positions = vec![empty, occupied];

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(layer_summary(stack_at(&model, empty)), vec!["target:0"]);
        assert_eq!(
            layer_summary(stack_at(&model, occupied)),
            vec!["piece:wQ:Board:0", "target:1"]
        );
    }

    #[test]
    fn board_model_active_piece_without_target_keeps_origin_piece() {
        let origin = Position::new(3, 2);
        let mut state = state_with_stacks(vec![(origin, vec!["wQ", "bB1"])]);
        state.turn_color = Color::Black;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("bB1"), PieceType::Board));
        move_info.current_position = Some(origin);

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(
            layer_summary(stack_at(&model, origin)),
            vec!["piece:wQ:Covered:0", "piece:bB1:Board:1", "active:Board:1",]
        );
        assert_eq!(
            piece_shadow_summary(stack_at(&model, origin)),
            vec![
                (piece("wQ"), PieceType::Covered, PieceShadow::Design, 0),
                (piece("bB1"), PieceType::Board, PieceShadow::None, 1),
            ]
        );
    }

    #[test]
    fn board_model_clicked_piece_without_active_move_keeps_marker() {
        let origin = Position::new(3, 2);
        let state = state_with_stacks(vec![(origin, vec!["wQ"])]);
        let mut move_info = MoveInfo::new();
        move_info.current_position = Some(origin);

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(
            layer_summary(stack_at(&model, origin)),
            vec!["piece:wQ:Board:0", "active:Board:0"]
        );
        assert_eq!(
            piece_shadow_summary(stack_at(&model, origin)),
            vec![(piece("wQ"), PieceType::Board, PieceShadow::Design, 0)]
        );
    }

    #[test]
    fn board_model_lifts_active_origin_and_adds_move_ghost() {
        let origin = Position::new(3, 2);
        let target = Position::new(4, 2);
        let mut state = state_with_stacks(vec![(origin, vec!["wQ", "bB1"])]);
        state.turn_color = Color::Black;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("bB1"), PieceType::Board));
        move_info.current_position = Some(origin);
        move_info.target_position = Some(target);
        move_info.target_positions = vec![target];

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(
            layer_summary(stack_at(&model, origin)),
            vec!["piece:wQ:Covered:0", "active:None:1"]
        );
        assert_eq!(
            layer_summary(stack_at(&model, target)),
            vec!["target:0", "piece:bB1:Move:1"]
        );
    }

    #[test]
    fn board_model_places_move_ghost_above_target_marker_on_stack() {
        let origin = Position::new(3, 2);
        let target = Position::new(4, 2);
        let mut state = state_with_stacks(vec![(origin, vec!["bB1"]), (target, vec!["wQ", "bA1"])]);
        state.turn_color = Color::Black;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("bB1"), PieceType::Board));
        move_info.current_position = Some(origin);
        move_info.target_position = Some(target);
        move_info.target_positions = vec![target];

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(
            layer_summary(stack_at(&model, target)),
            vec![
                "piece:wQ:Covered:0",
                "piece:bA1:Board:1",
                "target:2",
                "piece:bB1:Move:3",
            ]
        );
    }

    #[test]
    fn board_model_adds_spawn_ghost_for_reserve_origin() {
        let target = Position::new(4, 2);
        let mut state = state_with_stacks(vec![(Position::new(3, 2), vec!["wQ"])]);
        state.turn_color = Color::Black;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("bA1"), PieceType::Reserve));
        move_info.reserve_position = Some(Position::new(0, 0));
        move_info.target_position = Some(target);
        move_info.target_positions = vec![target];

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(
            layer_summary(stack_at(&model, target)),
            vec!["target:0", "piece:bA1:Spawn:1"]
        );
    }

    #[test]
    fn board_model_bounds_include_empty_overlay_positions() {
        let occupied = Position::new(3, 2);
        let target = Position::new(4, 2);
        let state = state_with_stacks(vec![(occupied, vec!["wQ"])]);
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("wQ"), PieceType::Board));
        move_info.current_position = Some(occupied);
        move_info.target_positions = vec![target];

        let model = build_board_render_model(&state, &move_info);

        assert_eq!(model.bounds.min_q, occupied.q);
        assert_eq!(model.bounds.max_q, target.q);
        assert_eq!(
            model
                .stacks
                .iter()
                .map(|stack| stack.position)
                .collect::<Vec<_>>(),
            vec![occupied, target]
        );
    }

    #[test]
    fn board_model_positions_are_sorted_and_deduped_across_overlays() {
        let occupied = Position::new(3, 2);
        let last_move_from = Position::new(1, 2);
        let target = Position::new(4, 2);
        let mut state = state_with_stacks(vec![(occupied, vec!["wQ"])]);
        state.board.last_move = (Some(last_move_from), Some(occupied));
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("wQ"), PieceType::Board));
        move_info.current_position = Some(occupied);
        move_info.target_positions = vec![target, target, occupied];

        let positions = build_board_render_model(&state, &move_info)
            .stacks
            .into_iter()
            .map(|stack| stack.position)
            .collect::<Vec<_>>();

        assert_eq!(positions, vec![occupied, target]);
    }

    #[test]
    fn reserve_model_single_row_positions_follow_bug_order() {
        let state = State::new(GameType::MLP, false);
        let model = reserve_model(&state, reserve_options());

        let positions = model
            .stacks
            .iter()
            .map(|stack| stack.position)
            .collect::<Vec<_>>();

        assert_eq!(
            positions,
            (0..8).map(|q| Position::new(q, 0)).collect::<Vec<_>>()
        );
        assert_eq!(
            model
                .stacks
                .iter()
                .filter_map(|stack| {
                    piece_summary(stack)
                        .into_iter()
                        .next()
                        .map(|(piece, _, _)| piece.bug())
                })
                .collect::<Vec<_>>(),
            Bug::all().to_vec()
        );
        assert_eq!(
            piece_summary(&model.stacks[0]),
            vec![
                (piece("wA3"), PieceType::Reserve, 0),
                (piece("wA2"), PieceType::Reserve, 1),
                (piece("wA1"), PieceType::Reserve, 2),
            ]
        );
    }

    #[test]
    fn reserve_model_double_row_layout_preserves_gaps_for_missing_expansions() {
        let state = State::new(GameType::Base, false);
        let model = reserve_model(
            &state,
            ReserveRenderOptions {
                alignment: ReserveLayout::DoubleRow,
                ..reserve_options()
            },
        );

        let positions = model
            .stacks
            .iter()
            .map(|stack| stack.position)
            .collect::<Vec<_>>();

        assert_eq!(
            positions,
            vec![
                Position::new(0, 0),
                Position::new(1, 0),
                Position::new(2, 0),
                Position::new(2, 1),
                Position::new(3, 1),
            ]
        );
    }

    #[test]
    fn reserve_model_wrong_color_pieces_are_inactive() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;

        let model = reserve_model(
            &state,
            ReserveRenderOptions {
                reserve_color: Color::Black,
                ..reserve_options()
            },
        );

        assert_all_model_pieces_have_type(&model, PieceType::Inactive);
    }

    #[test]
    fn reserve_model_history_is_inactive_when_not_at_last_turn() {
        let state = State::new_from_str("wA1; bG1 wA1-", &GameType::Base.to_string())
            .expect("test history parses");

        let model = reserve_model(
            &state,
            ReserveRenderOptions {
                interactivity: ReserveInteractivity {
                    viewing_past_turn: true,
                    ..reserve_interactivity()
                },
                ..reserve_options()
            },
        );

        assert_all_model_pieces_have_type(&model, PieceType::Inactive);
    }

    #[test]
    fn reserve_model_uses_supplied_board_for_history_view() {
        let mut state =
            State::new_from_str("wA1", &GameType::Base.to_string()).expect("test history parses");
        state.turn_color = Color::White;
        let selected_state = State::new(GameType::Base, false);

        let model = reserve_model_with_board(&state, &selected_state.board, reserve_options());

        assert_eq!(piece_summary(&model.stacks[0]).len(), 3);
    }

    #[test]
    fn reserve_model_tournament_not_started_is_inactive() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;

        let model = reserve_model(
            &state,
            ReserveRenderOptions {
                interactivity: ReserveInteractivity {
                    status: GameStatus::NotStarted,
                    tournament: true,
                    ..reserve_interactivity()
                },
                ..reserve_options()
            },
        );

        assert_all_model_pieces_have_type(&model, PieceType::Inactive);
    }

    #[test]
    fn reserve_model_tournament_queen_restriction_marks_queen_inactive_early() {
        let mut state = State::new(GameType::Base, true);
        state.turn_color = Color::White;
        state.turn = 0;

        let model = reserve_model(
            &state,
            ReserveRenderOptions {
                interactivity: ReserveInteractivity {
                    tournament: true,
                    ..reserve_interactivity()
                },
                ..reserve_options()
            },
        );

        assert_eq!(
            piece_summary(stack_at(&model, Position::new(3, 0))),
            vec![(piece("wQ"), PieceType::Inactive, 0)]
        );
    }

    #[test]
    fn reserve_model_queen_required_disables_non_queen_pieces() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;
        state.turn = 6;

        let model = reserve_model(&state, reserve_options());

        let pieces = model
            .stacks
            .iter()
            .flat_map(piece_summary)
            .collect::<Vec<_>>();
        assert!(pieces.iter().all(|(piece, piece_type, _)| {
            if piece.bug() == Bug::Queen {
                *piece_type == PieceType::Reserve
            } else {
                *piece_type == PieceType::Inactive
            }
        }));
        assert_eq!(
            piece_summary(stack_at(&model, Position::new(3, 0))),
            vec![(piece("wQ"), PieceType::Reserve, 0)]
        );
    }

    #[test]
    fn reserve_model_finished_is_inactive_unless_analysis_allows_interaction() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;

        let inactive = reserve_model(
            &state,
            ReserveRenderOptions {
                interactivity: ReserveInteractivity {
                    status: GameStatus::Finished(GameResult::Draw),
                    ..reserve_interactivity()
                },
                ..reserve_options()
            },
        );
        let analysis = reserve_model(
            &state,
            ReserveRenderOptions {
                interactivity: ReserveInteractivity {
                    status: GameStatus::Finished(GameResult::Draw),
                    analysis: true,
                    ..reserve_interactivity()
                },
                ..reserve_options()
            },
        );

        assert_all_model_pieces_have_type(&inactive, PieceType::Inactive);
        assert_all_model_pieces_have_type(&analysis, PieceType::Reserve);
    }

    #[test]
    fn reserve_model_active_marker_appears_on_clicked_stack() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("wA1"), PieceType::Reserve));
        move_info.reserve_position = Some(Position::new(0, 0));

        let model = reserve_model_with_move_info(&state, &move_info, reserve_options());

        assert_eq!(
            layer_summary(&model.stacks[0]),
            vec![
                "piece:wA3:Reserve:0",
                "piece:wA2:Reserve:1",
                "piece:wA1:Reserve:2",
                "active:Reserve:2",
            ]
        );
        assert_eq!(
            piece_shadow_summary(&model.stacks[0]),
            vec![
                (piece("wA3"), PieceType::Reserve, PieceShadow::Design, 0),
                (piece("wA2"), PieceType::Reserve, PieceShadow::Design, 1),
                (piece("wA1"), PieceType::Reserve, PieceShadow::None, 2),
            ]
        );
    }

    #[test]
    fn reserve_model_active_marker_does_not_leak_to_opposite_reserve() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("wA1"), PieceType::Reserve));
        move_info.reserve_position = Some(Position::new(0, 0));

        let model = reserve_model_with_move_info(
            &state,
            &move_info,
            ReserveRenderOptions {
                reserve_color: Color::Black,
                ..reserve_options()
            },
        );

        assert!(!model_has_active_layer(&model));
    }

    #[test]
    fn reserve_model_active_with_selected_board_target_lifts_selected_piece() {
        let mut state = State::new(GameType::Base, false);
        state.turn_color = Color::White;
        let mut move_info = MoveInfo::new();
        move_info.active = Some((piece("wA1"), PieceType::Reserve));
        move_info.reserve_position = Some(Position::new(0, 0));
        move_info.target_position = Some(Position::new(2, 2));

        let model = reserve_model_with_move_info(&state, &move_info, reserve_options());

        assert_eq!(
            layer_summary(&model.stacks[0]),
            vec![
                "piece:wA3:Reserve:0",
                "piece:wA2:Reserve:1",
                "active:Reserve:2",
            ]
        );
    }
}
