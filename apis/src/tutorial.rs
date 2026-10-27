mod basics;
mod bugs;
mod curriculum;
mod goal;
mod lesson;
mod online;
#[allow(dead_code)]
mod openings;
mod paths;
mod session;
mod sources;
#[allow(dead_code)]
mod strategy;
#[allow(dead_code)]
mod tactics;
#[cfg(test)]
mod tests;
pub mod text;

pub use curriculum::{
    chapter_of,
    find_lesson,
    group_of,
    lesson_count,
    lessons,
    neighbours,
    LessonGroup,
    CHAPTERS,
    LESSON_GROUPS,
};
pub use lesson::{Chapter, GuidePage, Lesson, Setup};
pub use session::{Outcome, Session, SessionError, Target};
pub use sources::source_game;
