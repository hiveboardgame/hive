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

/// Marks the moments that matter on the board: a buried target Queen, the surrounded Queen, the
/// square of a move that failed, and the spot the last hint points to.
pub fn use_puzzle_marks(puzzle: PuzzleContext, annotations: AnnotationsSignal) {
    let state = puzzle.game_state.state();
    Effect::new(move |_| {
        let phase = puzzle.phase.get();
        let mut marks = AnnotationSet::default();
        state.with(|state| {
            let queen = Piece::new_from(Bug::Queen, puzzle.solver().opposite_color(), 0);
            if let Some(at) = state.board.position_of_piece(queen) {
                if state.board.top_piece(at) != Some(queen) && !phase.is_over() {
                    marks.highlights.push(Highlight {
                        position: at,
                        color: AnnotationColor::Red,
                    });
                }
                let neighbours: Vec<_> = at.positions_around().collect();
                if neighbours.iter().all(|cell| state.board.occupied(*cell)) {
                    marks.highlights.push(Highlight {
                        position: at,
                        color: AnnotationColor::Red,
                    });
                }
            }
            if phase == PuzzlePhase::Solving && puzzle.hint_level.get() == 1 {
                if let Some(hinted) = puzzle.hint_piece.get() {
                    let solver = puzzle.solver();
                    let bugs = if hinted.color() != solver {
                        vec![Bug::Pillbug, Bug::Mosquito]
                    } else {
                        vec![hinted.bug()]
                    };
                    marks.highlights.extend(
                        state
                            .board
                            .all_taken_positions()
                            .filter(|cell| {
                                state.board.top_piece(*cell).is_some_and(|top| {
                                    top.color() == solver && bugs.contains(&top.bug())
                                })
                            })
                            .map(|position| Highlight {
                                position,
                                color: AnnotationColor::Green,
                            }),
                    );
                }
            }
            if phase == PuzzlePhase::Failed {
                if let Some(position) = puzzle.wrong_at.get() {
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
