use leptos::prelude::*;
use shared_types::Grade;

pub fn grade_name(grade: Grade) -> &'static str {
    match grade {
        Grade::Inaccuracy => "inaccuracy",
        Grade::Mistake => "mistake",
        Grade::Blunder => "blunder",
    }
}

/// "blunder" for one, "blunders" for any other count.
pub fn grade_plural(grade: Grade, count: usize) -> &'static str {
    match (grade, count) {
        (_, 1) => grade_name(grade),
        (Grade::Inaccuracy, _) => "inaccuracies",
        (Grade::Mistake, _) => "mistakes",
        (Grade::Blunder, _) => "blunders",
    }
}

pub fn grade_text_class(grade: Grade) -> &'static str {
    match grade {
        Grade::Inaccuracy => "text-yellow-700 dark:text-yellow-300",
        Grade::Mistake => "text-orange-600 dark:text-orange-400",
        Grade::Blunder => "text-red-600 dark:text-red-400",
    }
}

pub fn grade_fill_class(grade: Grade) -> &'static str {
    match grade {
        Grade::Inaccuracy => "bg-yellow-400",
        Grade::Mistake => "bg-orange-500",
        Grade::Blunder => "bg-red-600",
    }
}

#[component]
pub fn GradeMark(grade: Grade) -> impl IntoView {
    let name = grade_name(grade);
    view! {
        <abbr
            class=format!("font-bold no-underline {}", grade_text_class(grade))
            title=name
            aria-label=name
        >
            {grade.glyph()}
        </abbr>
    }
}
