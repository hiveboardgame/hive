use crate::{board::Board, bug_stack::BugStack, position::Position, torus_array::TorusArray};

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
        *self.neighbor_count.get(position) > 0 && self.get(position).size == 0
    }

    pub fn gated(&self, level: usize, from: Position, to: Position) -> bool {
        let (pos1, pos2) = from.common_adjacent_positions(to);
        let p1 = self.get(pos1);
        let p2 = self.get(pos2);
        if p1.is_empty() || p2.is_empty() {
            return false;
        }
        p1.len() >= level && p2.len() >= level
    }

    pub fn get(&self, position: Position) -> BugStack {
        let mut bug_stack = self.board.board.get(position).clone();
        if position == self.position_in_flight {
            bug_stack.pop_piece();
        }
        bug_stack
    }
}
