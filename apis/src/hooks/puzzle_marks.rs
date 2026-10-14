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
use hive_lib::Bug;
use leptos::prelude::*;

/// Marks the moments that matter on the board: the pieces a first hint points at, the square of a
/// move that failed, and the spot the last hint points to. The Queens glow from underneath instead.
pub fn use_puzzle_marks(puzzle: PuzzleContext, annotations: AnnotationsSignal) {
    let state = puzzle.game_state.state();
    Effect::new(move |_| {
        let phase = puzzle.phase.get();
        let mut marks = AnnotationSet::default();
        state.with(|state| {
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
