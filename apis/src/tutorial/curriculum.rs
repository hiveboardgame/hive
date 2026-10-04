use super::{
    basics,
    bugs,
    lesson::{Chapter, Lesson},
    online,
    openings,
    strategy,
    tactics,
};

pub const CHAPTERS: &[Chapter] = &[
    basics::CHAPTER,
    bugs::CHAPTER,
    openings::CHAPTER,
    strategy::CHAPTER,
    tactics::CHAPTER,
    online::CHAPTER,
];

pub fn lessons() -> impl Iterator<Item = &'static Lesson> {
    CHAPTERS.iter().flat_map(|chapter| chapter.lessons.iter())
}

pub fn lesson_count() -> usize {
    lessons().count()
}

pub fn find_lesson(id: &str) -> Option<&'static Lesson> {
    lessons().find(|lesson| lesson.id == id)
}

pub fn chapter_of(id: &str) -> Option<&'static Chapter> {
    CHAPTERS
        .iter()
        .find(|chapter| chapter.lessons.iter().any(|lesson| lesson.id == id))
}

pub fn neighbours(id: &str) -> (Option<&'static Lesson>, Option<&'static Lesson>) {
    let all: Vec<&'static Lesson> = lessons().collect();
    let Some(index) = all.iter().position(|lesson| lesson.id == id) else {
        return (None, None);
    };
    let previous = index
        .checked_sub(1)
        .and_then(|index| all.get(index).copied());
    (previous, all.get(index + 1).copied())
}
