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
        let nw = Position::new(position.q, position.r - 1);
        let se = Position::new(position.q, position.r + 1);
        let ne = Position::new(position.q + 1, position.r - 1);
        let sw = Position::new(position.q - 1, position.r + 1);
        let w = Position::new(position.q - 1, position.r);
        let e = Position::new(position.q + 1, position.r);

        let nw_level = self.level(nw);
        let se_level = self.level(se);
        let ne_level = self.level(ne);
        let sw_level = self.level(sw);
        let w_level = self.level(w);
        let e_level = self.level(e);

        if nw_level == 0
            && *self.neighbor_count.get(nw) > 0
            && (w_level > 0) != (ne_level > 0)
            && !keep_scanning(nw)
        {
            return false;
        }
        if se_level == 0
            && *self.neighbor_count.get(se) > 0
            && (e_level > 0) != (sw_level > 0)
            && !keep_scanning(se)
        {
            return false;
        }
        if ne_level == 0
            && *self.neighbor_count.get(ne) > 0
            && (nw_level > 0) != (e_level > 0)
            && !keep_scanning(ne)
        {
            return false;
        }
        if sw_level == 0
            && *self.neighbor_count.get(sw) > 0
            && (se_level > 0) != (w_level > 0)
            && !keep_scanning(sw)
        {
            return false;
        }
        if w_level == 0
            && *self.neighbor_count.get(w) > 0
            && (sw_level > 0) != (nw_level > 0)
            && !keep_scanning(w)
        {
            return false;
        }
        if e_level == 0
            && *self.neighbor_count.get(e) > 0
            && (ne_level > 0) != (se_level > 0)
            && !keep_scanning(e)
        {
            return false;
        }
        true
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
