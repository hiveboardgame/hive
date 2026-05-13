use crate::{board::Board, position::Position, torus_array::TorusArray};

pub struct MidMoveBoard<'this> {
    pub board: &'this Board,
    pub position_in_flight: Position,
    pub neighbor_count: TorusArray<u8>,
}

impl<'this> MidMoveBoard<'this> {
    pub fn new(board: &'this Board, position: Position) -> Self {
        let mut neighbor_count = board.neighbor_count.clone();
        debug_assert_eq!(board.level(position), 1);

        for pos in position.positions_around() {
            *neighbor_count.get_mut(pos) -= 1;
        }

        Self {
            board,
            position_in_flight: position,
            neighbor_count,
        }
    }

    pub fn is_negative_space(&self, position: Position) -> bool {
        *self.neighbor_count.get(position) > 0 && self.level(position) == 0
    }

    pub fn gated(&self, level: usize, from: Position, to: Position) -> bool {
        let (pos1, pos2) = from.common_adjacent_positions(to);
        let level1 = self.level(pos1);
        let level2 = self.level(pos2);
        if level1 == 0 || level2 == 0 {
            return false;
        }
        level1 >= level && level2 >= level
    }

    pub fn occupied(&self, position: Position) -> bool {
        self.level(position) > 0
    }

    fn level(&self, position: Position) -> usize {
        let mut level = self.board.level(position);
        if position == self.position_in_flight {
            level = level
                .checked_sub(1)
                .expect("Position in flight must contain a piece");
        }
        level
    }
}
