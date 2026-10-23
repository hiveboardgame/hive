use hive_lib::{Board, Bug, Position};

/// Reconstruct the three individual steps of a legal Spider or Ladybug move.
/// Gate heights and hive contact use the same checks as the engine. These
/// paths explain a move; move legality is still decided by the engine.
pub fn movement_path(bug: Bug, board: &Board, from: Position, to: Position) -> Vec<(Position, Position)> {
    if board.level(from) != 1 { return Vec::new(); }
    if board.top_piece(from).is_some_and(|piece| piece.bug()==Bug::Mosquito)
        && !board.neighbors(from).any(|stack| stack.top_piece().is_some_and(|piece| piece.bug()==bug)) {
        return Vec::new();
    }
    match bug {
        Bug::Spider => {
            let mut remaining=board.clone();
            remaining.remove(from);
            let crawls=|at: Position| at.positions_around().filter(|next| {
                let (a,b)=at.common_adjacent_positions(*next);
                (remaining.occupied(a)||remaining.occupied(b))
                    && remaining.is_negative_space(*next) && !remaining.gated(1,at,*next)
            }).collect::<Vec<_>>();
            for first in crawls(from) {
                for second in crawls(first).into_iter().filter(|at| *at!=from) {
                    if to!=from && to!=first && crawls(second).contains(&to) {
                        return vec![(from,first),(first,second),(second,to)];
                    }
                }
            }
        }
        Bug::Ladybug => {
            for first in board.positions_taken_around(from).filter(|at|
                !board.gated(board.level(*at)+1,from,*at)) {
                for second in board.positions_taken_around(first).filter(|at|
                    *at!=from && !board.gated((board.level(first)+1).max(board.level(*at)+1),first,*at)) {
                    if second.is_neighbor(to) && !board.occupied(to)
                        && !board.gated(board.level(second)+1,second,to) {
                        return vec![(from,first),(first,second),(second,to)];
                    }
                }
            }
        }
        _ => {}
    }
    Vec::new()
}
