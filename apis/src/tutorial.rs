mod basics;
mod bugs;
mod curriculum;
mod goal;
mod lesson;
mod online;
mod openings;
mod session;
mod sources;
mod strategy;
mod tactics;
#[cfg(test)]
mod tests;

pub use curriculum::{chapter_of, find_lesson, lesson_count, lessons, neighbours, CHAPTERS};
pub use lesson::{Chapter, Lesson, Setup};
pub use session::{Outcome, Session, SessionError, Target};
pub use sources::source_game;
