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
    online::CHAPTER,
];

/// Kept in the repository for later releases, excluded from navigation and progress.
#[allow(dead_code)]
pub const UNRELEASED_CHAPTERS: &[Chapter] = &[
    openings::CHAPTER,
    strategy::CHAPTER,
    tactics::CHAPTER,
];

pub fn lessons() -> impl Iterator<Item = &'static Lesson> {
    CHAPTERS.iter().flat_map(|chapter| chapter.lessons.iter())
}

#[cfg(test)]
pub fn find_authored_lesson(id: &str) -> Option<&'static Lesson> {
    CHAPTERS.iter().chain(UNRELEASED_CHAPTERS).flat_map(|chapter| chapter.lessons.iter())
        .find(|lesson| lesson.id == id)
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

/// Related lessons keep individual progress and numbering under a shared topic.
pub struct LessonGroup {
    pub id: &'static str,
    pub title: &'static str,
    pub lessons: &'static [&'static str],
}

pub const LESSON_GROUPS: &[LessonGroup] = &[
    LessonGroup {
        id: "bugs.queens",
        title: "The Queen Bee",
        lessons: &["bugs.queen", "bugs.freedom_to_move", "bugs.dont_break_the_hive"],
    },
    LessonGroup {
        id: "bugs.spiders",
        title: "The Spider",
        lessons: &["bugs.spider", "bugs.spider_dead_end", "bugs.spider_door", "bugs.spider_power"],
    },
    LessonGroup {
        id: "bugs.beetles",
        title: "The Beetle",
        lessons: &["bugs.beetle", "bugs.beetle_spawn", "bugs.beetle_gate"],
    },
    LessonGroup {
        id: "bugs.grasshoppers",
        title: "The Grasshopper",
        lessons: &["bugs.grasshopper", "bugs.grasshopper_place", "bugs.grasshopper_paths"],
    },
    LessonGroup {
        id: "bugs.ladybugs",
        title: "The Ladybug",
        lessons: &["bugs.ladybug", "bugs.ladybug_gate"],
    },
    LessonGroup {
        id: "bugs.pillbugs",
        title: "The Pillbug",
        lessons: &[
            "bugs.pillbug", "bugs.pillbug_save", "bugs.pillbug_enemy",
            "bugs.pillbug_gate", "bugs.pillbug_deactivate",
        ],
    },
    LessonGroup {
        id: "bugs.mosquitoes",
        title: "The Mosquito",
        lessons: &["bugs.mosquito", "bugs.mosquito_climb", "bugs.mosquito_ability", "bugs.mosquito_other"],
    },
    LessonGroup {
        id: "bugs.ants",
        title: "The Ant",
        lessons: &["bugs.ant_first_moves", "bugs.ant", "bugs.ant_attack"],
    },
];

pub fn group_of(id: &str) -> Option<&'static LessonGroup> {
    LESSON_GROUPS.iter().find(|group| group.lessons.contains(&id))
}
