use hive_lib::{Board, Bug, Color, Direction, Piece, Position, State};

pub enum Goal {
    Continue,
    MoveAny,
    Move {
        piece: &'static str,
        to: &'static str,
    },
    Visit {
        piece: &'static str,
        hexes: &'static [&'static str],
    },
    Reach(Condition),
}

/// Piece specs name a piece in UHP (`wB1`, `bQ`). Leaving the number off (`wB`) matches any
/// piece of that bug, so lessons survive a HOP renumbering their pieces.
pub enum Condition {
    AnyPlaced(Color),
    Placed(&'static str),
    Touching(&'static str, &'static str),
    OnTop(&'static str, &'static str),
    Pinned(&'static str),
    Stuck(&'static str),
    Surrounded(&'static str),
    KillSpotsFilled { queen: Color, at_least: usize },
    DirectDrops { by: Color, at_least: usize },
    SpawnBlocked { around: &'static str, color: Color },
    Sandwiched(Color),
    ShutOut(Color),
    All(&'static [Condition]),
    Not(&'static Condition),
}

const OPPOSITE_SIDES: [(Direction, Direction); 3] = [
    (Direction::E, Direction::W),
    (Direction::NE, Direction::SW),
    (Direction::NW, Direction::SE),
];

impl Condition {
    pub fn holds(&self, state: &State) -> bool {
        let board = &state.board;
        match self {
            Condition::AnyPlaced(color) => board.played_by(*color) > 0,
            Condition::Placed(spec) => placed(board, spec).next().is_some(),
            Condition::Touching(a, b) => placed(board, a)
                .any(|(_, at)| placed(board, b).any(|(_, other)| at.is_neighbor(other))),
            Condition::OnTop(a, b) => placed(board, a).any(|(piece, at)| {
                board.top_piece(at) == Some(piece)
                    && placed(board, b).any(|(under, other)| other == at && under != piece)
            }),
            Condition::Pinned(spec) => placed(board, spec).any(|(piece, _)| board.is_pinned(piece)),
            Condition::Stuck(spec) => placed(board, spec).any(|(piece, _)| {
                !board
                    .moves(piece.color())
                    .iter()
                    .any(|((mover, _), targets)| *mover == piece && !targets.is_empty())
            }),
            Condition::Surrounded(spec) => placed(board, spec)
                .any(|(_, at)| at.positions_around().all(|around| board.occupied(around))),
            Condition::KillSpotsFilled { queen, at_least } => queen_position(board, *queen)
                .is_some_and(|at| {
                    at.positions_around()
                        .filter(|around| board.occupied(*around))
                        .count()
                        >= *at_least
                }),
            Condition::DirectDrops { by, at_least } => direct_drops(board, *by) >= *at_least,
            Condition::SpawnBlocked { around, color } => placed(board, around).any(|(_, at)| {
                !at.positions_around()
                    .any(|around| board.spawnable(*color, around))
            }),
            Condition::Sandwiched(color) => queen_position(board, *color).is_some_and(|at| {
                OPPOSITE_SIDES.iter().any(|(one, other)| {
                    owned_by(board, at.to(*one), *color) && owned_by(board, at.to(*other), *color)
                })
            }),
            Condition::ShutOut(color) => board.is_shutout(*color, state.game_type),
            Condition::All(conditions) => conditions.iter().all(|condition| condition.holds(state)),
            Condition::Not(condition) => !condition.holds(state),
        }
    }
}

pub fn direct_drops(board: &Board, by: Color) -> usize {
    queen_position(board, by.opposite_color()).map_or(0, |at| {
        at.positions_around()
            .filter(|around| board.spawnable(by, *around))
            .count()
    })
}

pub fn spec_pieces(spec: &str) -> Option<Vec<Piece>> {
    let mut chars = spec.chars();
    let color: Color = chars.next()?.to_string().parse().ok()?;
    let bug: Bug = chars.next()?.to_string().parse().ok()?;
    let order: String = chars.collect();
    if !bug.has_order() {
        return order
            .is_empty()
            .then(|| vec![Piece::new_from(bug, color, 0)]);
    }
    if order.is_empty() {
        return Some(
            (1..=3)
                .map(|order| Piece::new_from(bug, color, order))
                .collect(),
        );
    }
    let order: usize = order.parse().ok()?;
    (1..=3)
        .contains(&order)
        .then(|| vec![Piece::new_from(bug, color, order)])
}

pub fn spec_matches(spec: &str, piece: Piece) -> bool {
    spec_pieces(spec).is_some_and(|pieces| pieces.contains(&piece))
}

/// A hex in UHP's relative notation (`bQ-`, `/wA1`), or a piece's own hex when bare (`bQ`).
pub fn hex(board: &Board, notation: &str) -> Option<Position> {
    if notation.is_empty() || notation.starts_with('.') {
        return None;
    }
    Position::from_string(notation, board).ok()
}

pub fn placed<'a>(board: &'a Board, spec: &str) -> impl Iterator<Item = (Piece, Position)> + 'a {
    spec_pieces(spec)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|piece| board.position_of_piece(piece).map(|at| (piece, at)))
}

fn queen_position(board: &Board, color: Color) -> Option<Position> {
    board.position_of_piece(Piece::new_from(Bug::Queen, color, 0))
}

fn owned_by(board: &Board, at: Position, color: Color) -> bool {
    board
        .top_piece(at)
        .is_some_and(|piece| piece.is_color(color))
}
