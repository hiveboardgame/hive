use crate::common::PieceType;
use hudsoni::{Piece, Position};

#[derive(Clone, Debug, PartialEq)]
pub struct HivegroundRenderModel {
    pub stacks: Vec<RenderStack>,
    pub bounds: DrawingBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderStack {
    pub position: Position,
    pub layers: Vec<RenderLayer>,
}

impl RenderStack {
    pub fn key(&self) -> Position {
        self.position
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawingBounds {
    pub min_q: i32,
    pub max_q: i32,
    pub min_r: i32,
    pub max_r: i32,
}

impl DrawingBounds {
    pub fn from_positions(positions: &[Position]) -> Self {
        let Some(first) = positions.first() else {
            return Self {
                min_q: 0,
                max_q: 0,
                min_r: 0,
                max_r: 0,
            };
        };

        positions.iter().fold(
            Self {
                min_q: first.q,
                max_q: first.q,
                min_r: first.r,
                max_r: first.r,
            },
            |bounds, position| Self {
                min_q: bounds.min_q.min(position.q),
                max_q: bounds.max_q.max(position.q),
                min_r: bounds.min_r.min(position.r),
                max_r: bounds.max_r.max(position.r),
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandedStackLevel {
    Fixed,
    Separated,
    Attached,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LastMoveDirection {
    From,
    To,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum ActiveMarkerState {
    Board,
    #[default]
    Reserve,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RenderLayer {
    pub level: usize,
    pub kind: RenderLayerKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RenderLayerKind {
    Piece {
        piece: Piece,
        piece_type: PieceType,
        shadow: PieceShadow,
    },
    Target,
    Active {
        state: ActiveMarkerState,
    },
    LastMove {
        direction: LastMoveDirection,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PieceShadow {
    Design,
    None,
}

impl PieceShadow {
    pub fn for_piece_type(piece_type: PieceType) -> Self {
        if matches!(piece_type, PieceType::Move | PieceType::Spawn) {
            Self::None
        } else {
            Self::Design
        }
    }
}

impl RenderLayer {
    pub fn with_shadow(mut self, next_shadow: PieceShadow) -> Self {
        if let RenderLayerKind::Piece { shadow, .. } = &mut self.kind {
            *shadow = next_shadow;
        }
        self
    }

    pub fn base_level(&self) -> usize {
        self.level
    }

    pub fn expanded_stack_level(&self) -> ExpandedStackLevel {
        match &self.kind {
            RenderLayerKind::Piece { piece_type, .. } => match piece_type {
                PieceType::Board | PieceType::Covered | PieceType::History => {
                    ExpandedStackLevel::Separated
                }
                PieceType::Move => ExpandedStackLevel::Attached,
                _ => ExpandedStackLevel::Fixed,
            },
            RenderLayerKind::Target => {
                if self.level == 0 {
                    ExpandedStackLevel::Fixed
                } else {
                    ExpandedStackLevel::Attached
                }
            }
            RenderLayerKind::Active { state, .. } => {
                if self.level == 0 || *state == ActiveMarkerState::Board {
                    ExpandedStackLevel::Separated
                } else {
                    ExpandedStackLevel::Attached
                }
            }
            RenderLayerKind::LastMove { direction, .. } => match direction {
                LastMoveDirection::To => ExpandedStackLevel::Separated,
                LastMoveDirection::From => {
                    if self.level == 0 {
                        ExpandedStackLevel::Fixed
                    } else {
                        ExpandedStackLevel::Attached
                    }
                }
            },
        }
    }

    pub fn is_stack_expandable(&self) -> bool {
        match &self.kind {
            RenderLayerKind::Piece { piece_type, .. } => {
                !matches!(piece_type, PieceType::Reserve | PieceType::Inactive) && self.level != 0
            }
            RenderLayerKind::Active { state, .. } => *state == ActiveMarkerState::Board,
            RenderLayerKind::Target => self.level != 0,
            RenderLayerKind::LastMove { .. } => false,
        }
    }
}
