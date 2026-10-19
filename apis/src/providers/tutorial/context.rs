use super::progress::TutorialProgress;
use crate::{
    providers::{
        annotations::{
            AnnotationColor,
            AnnotationSet,
            Arrow,
            Highlight,
            Label,
            Marker,
            MarkerShape,
        },
        game_state::{GameStateStore, GameStateStoreFields},
    },
    tutorial::{Lesson, Outcome, Session},
};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    Hint(&'static str),
    Refuted(&'static str),
    KeepGoing,
    WellDone,
    YourTurn,
    LessonComplete,
    ContinueFirst,
    Note(&'static str),
}

#[derive(Clone, Copy)]
pub struct TutorialContext {
    pub session: RwSignal<Option<Session>>,
    pub load_error: RwSignal<Option<String>>,
    pub feedback: RwSignal<Option<Feedback>>,
    pub generation: RwSignal<u64>,
    pub progress: TutorialProgress,
    completed_positions: RwSignal<std::collections::HashMap<&'static str, hive_lib::State>>,
    game_state: GameStateStore,
}

impl TutorialContext {
    pub fn new(game_state: GameStateStore, progress: TutorialProgress) -> Self {
        Self {
            session: RwSignal::new(None),
            load_error: RwSignal::new(None),
            feedback: RwSignal::new(None),
            generation: RwSignal::new(0),
            progress,
            completed_positions: RwSignal::new(std::collections::HashMap::new()),
            game_state,
        }
    }

    pub fn load(&self, lesson: &'static Lesson) {
        self.feedback.set(None);
        let starting = match lesson.setup {
            crate::tutorial::Setup::AfterLesson { lesson: source, .. } => {
                self.completed_positions.with_untracked(|positions| positions.get(source).cloned())
            }
            _ => None,
        };
        match Session::with_starting_state(lesson, starting) {
            Ok(session) => {
                self.load_error.set(None);
                self.game_state.reset_with_state(session.state().clone());
                self.session.set(Some(session));
                self.generation.update(|generation| *generation += 1);
            }
            Err(error) => {
                self.session.set(None);
                self.load_error.set(Some(error.to_string()));
            }
        }
    }

    pub fn allows_selection(&self) -> bool {
        self.session
            .with(|session| session.as_ref().is_some_and(Session::is_learners_turn))
    }

    pub fn play_selected(&self) {
        let (active, target) = self.game_state.move_info().with_untracked(|move_info| {
            (
                move_info.active.map(|(piece, _)| piece),
                move_info.target_position,
            )
        });
        self.game_state.clear_selection();
        let (Some(piece), Some(position)) = (active, target) else {
            return;
        };
        self.play_move(piece, position);
    }

    pub fn try_forbidden(&self, position: hive_lib::Position) {
        let allowed = self.session.with_untracked(|session| {
            session.as_ref().is_some_and(|session| {
                session.accepts_moves() && session.forbidden().contains(&position)
            })
        });
        if !allowed {
            return;
        }
        let active = self.game_state
            .move_info()
            .with_untracked(|move_info| move_info.active.map(|(piece, _)| piece));
        if let Some(piece) = active {
            self.game_state.clear_selection();
            self.play_move(piece, position);
        }
    }

    fn play_move(&self, piece: hive_lib::Piece, position: hive_lib::Position) {
        if self
            .session
            .with_untracked(|session| session.as_ref().is_some_and(Session::awaits_continue))
        {
            self.feedback.set(Some(Feedback::ContinueFirst));
            return;
        }
        let your_turn = self.session.with_untracked(|session| {
            session
                .as_ref()
                .and_then(Session::current_step)
                .is_some_and(|step| step.your_turn)
        });
        let hint = self.session.with_untracked(|session| {
            session
                .as_ref()
                .and_then(Session::current_step)
                .map_or("", |step| step.hint)
        });
        let outcome = self
            .session
            .try_update(|session| {
                session
                    .as_mut()
                    .map(|session| session.play(piece, position))
            })
            .flatten();
        let Some(outcome) = outcome else {
            return;
        };
        self.sync_board();
        self.feedback.set(match outcome {
            Outcome::Illegal => None,
            Outcome::Wrong => Some(Feedback::Hint(hint)),
            Outcome::Refuted => Some(Feedback::Refuted(hint)),
            Outcome::Visited => Some(Feedback::KeepGoing),
            Outcome::Progress => Some(Feedback::YourTurn),
            Outcome::StepDone => Some(if your_turn {
                Feedback::YourTurn
            } else {
                Feedback::WellDone
            }),
            Outcome::LessonDone => Some(Feedback::LessonComplete),
        });
        if outcome == Outcome::LessonDone {
            self.complete_lesson();
        }
    }

    pub fn try_again(&self) {
        self.session.update(|session| {
            if let Some(session) = session {
                session.retry();
            }
        });
        self.game_state.clear_selection();
        self.sync_board();
        self.feedback.set(None);
        if self.session.with_untracked(|session| {
            session.as_ref().is_some_and(|session| session.lesson().clock().is_some())
        }) {
            self.generation.update(|generation| *generation += 1);
        }
    }

    pub fn advance(&self) {
        let outcome = self
            .session
            .try_update(|session| session.as_mut().map(Session::advance))
            .flatten();
        self.sync_board();
        if outcome == Some(Outcome::LessonDone) {
            self.feedback.set(Some(Feedback::LessonComplete));
            self.complete_lesson();
        } else {
            self.feedback.set(None);
        }
    }

    pub fn annotations(&self) -> Signal<AnnotationSet> {
        let session = self.session;
        Signal::derive(move || {
            session.with(|session| {
                let Some(session) = session else {
                    return AnnotationSet::default();
                };
                let stars = session.stars();
                let marks = session.marks();
                let labels = stars
                    .iter()
                    .map(|star| (star, AnnotationColor::Green))
                    .chain(marks.iter().map(|mark| (mark, AnnotationColor::Orange)))
                    .filter_map(|(target, color)| {
                        target.number.map(|number| Label {
                            position: target.position,
                            number,
                            color,
                        })
                    })
                    .collect();
                AnnotationSet {
                    highlights: stars
                        .iter()
                        .map(|star| star.position)
                        .chain(session.highlights())
                        .map(|position| Highlight {
                            position,
                            color: AnnotationColor::Green,
                        })
                        .chain(
                            session.forbidden().into_iter().map(|position| Highlight {
                                position,
                                color: AnnotationColor::Red,
                            }),
                        )
                        .collect(),
                    markers: session
                        .grid()
                        .into_iter()
                        .map(|position| Marker {
                            position,
                            shape: MarkerShape::Grid,
                            color: AnnotationColor::White,
                        })
                        .chain(marks.iter().map(|mark| Marker {
                            position: mark.position,
                            shape: MarkerShape::Ring,
                            color: AnnotationColor::Orange,
                        }))
                        .chain(session.red_marks().into_iter().map(|mark| Marker {
                            position: mark.position,
                            shape: MarkerShape::Ring,
                            color: AnnotationColor::Red,
                        }))
                        .chain(session.placement_crosses().into_iter().map(|position| Marker {
                            position,
                            shape: MarkerShape::Cross,
                            color: AnnotationColor::Red,
                        }))
                        .collect(),
                    arrows: session
                        .arrows()
                        .into_iter()
                        .map(|(from, to)| Arrow {
                            from,
                            to,
                            color: AnnotationColor::Green,
                        })
                        .chain(session.red_arrows().into_iter().map(|(from, to)| Arrow {
                            from,
                            to,
                            color: AnnotationColor::Red,
                        }))
                        .collect(),
                    labels,
                }
            })
        })
    }

    /// Only the position changes: replacing the whole store makes the board treat it as a new
    /// game and throw away its zoom.
    fn sync_board(&self) {
        if let Some(state) = self
            .session
            .with_untracked(|session| session.as_ref().map(|session| session.state().clone()))
        {
            self.game_state.state().set(state);
        }
    }

    fn complete_lesson(&self) {
        if let Some(id) = self
            .session
            .with_untracked(|session| session.as_ref().map(|session| session.lesson().id))
        {
            self.progress.complete(id);
            if let Some(state) = self.session.with_untracked(|session| {
                session.as_ref().map(|session| session.state().clone())
            }) {
                self.completed_positions.update(|positions| { positions.insert(id, state); });
            }
        }
    }
}
