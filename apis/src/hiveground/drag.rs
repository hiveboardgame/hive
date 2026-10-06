use crate::common::PieceType;
use hive_lib::{Piece, Position};

pub const DRAG_SLOP_PX: f64 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraggedPiece {
    pub piece: Piece,
    pub position: Position,
    pub piece_type: PieceType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragPhase {
    Pressed { origin: (i32, i32) },
    Dragging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PieceDrag {
    pub dragged: DraggedPiece,
    pub phase: DragPhase,
    pub pointer: (i32, i32),
    pub pointer_id: i32,
}

impl PieceDrag {
    pub fn pressed(dragged: DraggedPiece, pointer: (i32, i32), pointer_id: i32) -> Self {
        Self {
            dragged,
            phase: DragPhase::Pressed { origin: pointer },
            pointer,
            pointer_id,
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.phase == DragPhase::Dragging
    }

    pub fn left_slop(&self, pointer: (i32, i32)) -> bool {
        match self.phase {
            DragPhase::Pressed { origin } => {
                f64::from(pointer.0 - origin.0).hypot(f64::from(pointer.1 - origin.1))
                    > DRAG_SLOP_PX
            }
            DragPhase::Dragging => false,
        }
    }

    pub fn dropped_on_origin(&self, drop: Position) -> bool {
        self.dragged.piece_type == PieceType::Board && self.dragged.position == drop
    }
}
