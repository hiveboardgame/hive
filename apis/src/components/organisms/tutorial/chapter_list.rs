use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
    providers::tutorial::TutorialProgress,
    tutorial::{Chapter, CHAPTERS},
};
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn ChapterList(
    progress: TutorialProgress,
    #[prop(into)] current: Signal<&'static str>,
) -> impl IntoView {
    let chapters = CHAPTERS
        .iter()
        .map(|chapter| {
            let holds_current = move || {
                let id = current.get();
                chapter.lessons.iter().any(|lesson| lesson.id == id)
            };
            let toggled = RwSignal::new(None::<bool>);
            let is_open = move || toggled.get().unwrap_or_else(holds_current);
            view! {
                <div>
                    <button
                        type="button"
                        class="flex gap-2 justify-between items-center py-1 w-full text-xs font-bold tracking-wide text-gray-500 uppercase dark:text-gray-400"
                        on:click=move |_| toggled.set(Some(!is_open()))
                    >
                        <span class="flex gap-1 items-center">
                            <span class="inline-block transition-transform" class:rotate-90=is_open>
                                "›"
                            </span>
                            {chapter.title}
                        </span>
                        <span>{move || chapter_progress(progress, chapter)}</span>
                    </button>
                    <div class="flex flex-col gap-0.5 pt-1" class:hidden=move || !is_open()>
                        <LessonLinks chapter progress current />
                    </div>
                </div>
            }
        })
        .collect_view();

    view! {
        <nav class="flex flex-col gap-1 p-3 ui-panel" aria-label="Tutorial lessons">
            {chapters}
        </nav>
    }
}

#[component]
pub fn TutorialOverview(progress: TutorialProgress) -> impl IntoView {
    let next_lesson = move || progress.first_incomplete();
    let subtitle = move || {
        format!(
            "Learn to play Hive one board at a time. {} of {} lessons done.",
            progress.completed_count(),
            progress.total(),
        )
    };
    let cards = CHAPTERS
        .iter()
        .map(|chapter| {
            let width = move || {
                let done = chapter
                    .lessons
                    .iter()
                    .filter(|lesson| progress.is_complete(lesson.id))
                    .count();
                format!("width: {}%", done * 100 / chapter.lessons.len().max(1))
            };
            view! {
                <Panel title=chapter.title body_class="flex flex-col gap-3">
                    <div class="flex gap-3 items-center">
                        <div class="overflow-hidden h-1.5 rounded-full grow bg-black/10 dark:bg-white/10">
                            <div class="h-full rounded-full bg-orange-dawn" style=width></div>
                        </div>
                        <span class="text-xs font-bold text-gray-500 dark:text-gray-400">
                            {move || chapter_progress(progress, chapter)}
                        </span>
                    </div>
                    <p class="text-sm text-gray-600 dark:text-gray-300">{chapter.summary}</p>
                    <div class="flex flex-col">
                        <LessonLinks chapter progress current=Signal::stored("") />
                    </div>
                </Panel>
            }
        })
        .collect_view();

    view! {
        <PageShell>
            <div class="flex flex-wrap gap-4 justify-between items-end">
                <PageHeader title="Learn Hive" subtitle=subtitle />
                <ShowLet some=next_lesson let:id>
                    <A
                        href=format!("/tutorial?lesson={id}")
                        attr:class="ui-button ui-button-primary ui-button-md"
                    >
                        {move || {
                            if progress.completed_count() == 0 {
                                "Start learning"
                            } else {
                                "Continue"
                            }
                        }}
                    </A>
                </ShowLet>
            </div>
            <div class="grid grid-cols-1 gap-4 items-start md:grid-cols-2">{cards}</div>
        </PageShell>
    }
}

#[component]
fn LessonLinks(
    chapter: &'static Chapter,
    progress: TutorialProgress,
    current: Signal<&'static str>,
) -> impl IntoView {
    chapter
        .lessons
        .iter()
        .map(|lesson| {
            let id = lesson.id;
            let row_class = move || {
                if current.get() == id {
                    "flex gap-2 items-center py-1 px-2 -mx-2 text-sm xl:text-base font-bold rounded-md bg-orange-dawn/10"
                } else {
                    "flex gap-2 items-center py-1 px-2 -mx-2 text-sm xl:text-base rounded-md hover:bg-black/5 dark:hover:bg-white/5"
                }
            };
            let done = move || progress.is_complete(id);
            view! {
                <div class=row_class>
                    <span class=move || {
                        if done() {
                            "flex shrink-0 justify-center items-center w-4 text-xs font-bold text-orange-dawn"
                        } else {
                            "flex shrink-0 justify-center items-center w-4 text-xs text-gray-400 dark:text-gray-500"
                        }
                    }>{move || if done() { "✓" } else { "○" }}</span>
                    <A
                        href=format!("/tutorial?lesson={id}")
                        attr:class="text-gray-800 dark:text-gray-100 no-link-style hover:underline"
                    >
                        {lesson.title}
                    </A>
                </div>
            }
        })
        .collect_view()
}

fn chapter_progress(progress: TutorialProgress, chapter: &Chapter) -> String {
    let done = chapter
        .lessons
        .iter()
        .filter(|lesson| progress.is_complete(lesson.id))
        .count();
    format!("{done}/{}", chapter.lessons.len())
}
