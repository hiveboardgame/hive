mod basics;
mod bugs;
mod curriculum;
mod goal;
mod lesson;
mod online;
#[allow(dead_code)]
mod openings;
mod session;
mod sources;
#[allow(dead_code)]
mod strategy;
#[allow(dead_code)]
mod tactics;
#[cfg(test)]
mod tests;

pub use curriculum::{group_of, LessonGroup, LESSON_GROUPS, chapter_of, find_lesson, lesson_count, lessons, neighbours, CHAPTERS};
pub use lesson::{Chapter, GuidePage, Lesson, Setup};
pub use session::{Outcome, Session, SessionError, Target};
pub use sources::source_game;
