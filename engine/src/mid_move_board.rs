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

    pub(crate) fn scan_crawl_negative_space_while(
        &self,
        position: Position,
        keep_scanning: &mut impl FnMut(Position) -> bool,
    ) -> bool {
        let [nw, se, ne, sw, w, e] = position.neighbors();

        let nw_level = self.level(nw);
        let se_level = self.level(se);
        let ne_level = self.level(ne);
        let sw_level = self.level(sw);
        let w_level = self.level(w);
        let e_level = self.level(e);

        // Each candidate is the empty neighbor we might crawl into, plus the two
        // cells it shares with `position`. The two shared cells are its gates.
        let candidates = [
            (nw, nw_level, w_level, ne_level),
            (se, se_level, e_level, sw_level),
            (ne, ne_level, nw_level, e_level),
            (sw, sw_level, se_level, w_level),
            (w, w_level, sw_level, nw_level),
            (e, e_level, ne_level, se_level),
        ];
        for (cell, cell_level, gate_a_level, gate_b_level) in candidates {
            if self.can_crawl_to(cell, cell_level, gate_a_level, gate_b_level)
                && !keep_scanning(cell)
            {
                return false;
            }
        }
        true
    }

    // A crawl target is empty negative space reachable by a ground slide: the
    // move is not gated, meaning exactly one of the two shared gates is occupied
    // (a piece to pivot on, but not a wall on both sides).
    #[inline(always)]
    fn can_crawl_to(
        &self,
        cell: Position,
        cell_level: usize,
        gate_a_level: usize,
        gate_b_level: usize,
    ) -> bool {
        cell_level == 0
            && *self.neighbor_count.get(cell) > 0
            && (gate_a_level > 0) != (gate_b_level > 0)
    }

    #[inline(always)]
    fn level(&self, position: Position) -> usize {
        let mut level = self.board.board.get(position).size as usize;
        if position == self.position_in_flight {
            level = level
                .checked_sub(1)
                .expect("Position in flight must contain a piece");
        }
        level
    }
}
