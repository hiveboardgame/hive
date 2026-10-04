use hive_lib::{Board, Position};
use std::collections::BTreeSet;

/// The empty hexes within two steps of the hive, or around the first hex on an empty board.
pub fn grid_positions(board: &Board) -> Vec<Position> {
    let mut anchors: Vec<Position> = board.all_taken_positions().collect();
    if anchors.is_empty() {
        anchors.push(Position::initial_spawn_position());
    }
    let near: BTreeSet<Position> = anchors
        .iter()
        .flat_map(|anchor| anchor.positions_around())
        .flat_map(|ring| std::iter::once(ring).chain(ring.positions_around()))
        .chain(anchors.iter().copied())
        .collect();
    near.into_iter()
        .filter(|position| !board.occupied(*position))
        .collect()
}
