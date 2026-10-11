use crate::providers::{
    annotations::{
        AnnotationColor,
        AnnotationSet,
        AnnotationsSignal,
        Highlight,
        Marker,
        MarkerShape,
    },
    game_state::GameStateStoreFields,
    puzzle::{PuzzleContext, PuzzlePhase},
};
use hive_lib::{Bug, Piece};
use leptos::prelude::*;

/// Marks the moments that matter on the board: the surrounded Queen, the
/// square of a move that failed, and the spot the last hint points to.
pub fn use_puzzle_marks(puzzle: PuzzleContext, annotations: AnnotationsSignal) {
    let state = puzzle.game_state.state();
    Effect::new(move |_| {
        let phase = puzzle.phase.get();
        let mut marks = AnnotationSet::default();
        state.with(|state| {
            let queen = Piece::new_from(Bug::Queen, puzzle.solver().opposite_color(), 0);
            if let Some(at) = state.board.position_of_piece(queen) {
                let neighbours: Vec<_> = at.positions_around().collect();
                if neighbours.iter().all(|cell| state.board.occupied(*cell)) {
                    marks.highlights.push(Highlight {
                        position: at,
                        color: AnnotationColor::Red,
                    });
                }
            }
            if phase == PuzzlePhase::Failed {
                if let Some(position) = state.board.last_move.1 {
                    marks.markers.push(Marker {
                        position,
                        shape: MarkerShape::Cross,
                        color: AnnotationColor::Red,
                    });
                }
            }
        });
        if phase == PuzzlePhase::Solving && puzzle.hint_level.get() >= 3 {
            if let Some(position) = puzzle.hint_target.get() {
                marks.highlights.push(Highlight {
                    position,
                    color: AnnotationColor::Green,
                });
            }
        }
        annotations.replace_current(marks);
    });
}
