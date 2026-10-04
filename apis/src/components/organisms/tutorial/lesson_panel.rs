use crate::{
    providers::tutorial::{Feedback, TutorialContext},
    tutorial::{
        chapter_of,
        find_lesson,
        lesson_count,
        lessons,
        neighbours,
        source_game,
        Lesson,
        Session,
    },
};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_navigate};

#[component]
pub fn LessonPanel(
    #[prop(into)] lesson_id: Signal<&'static str>,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let tutorial = expect_context::<TutorialContext>();
    let lesson = move || find_lesson(lesson_id.get());
    let chapter_title = move || chapter_of(lesson_id.get()).map_or("", |chapter| chapter.title);
    let title = move || lesson().map_or("", |lesson| lesson.title);
    let position = move || {
        let id = lesson_id.get();
        let index = lessons().position(|lesson| lesson.id == id).unwrap_or(0);
        format!("Lesson {} of {}", index + 1, lesson_count())
    };
    let intro = move || {
        lesson()
            .map(|lesson| lesson.intro)
            .unwrap_or_default()
            .iter()
            .copied()
            .map(paragraph)
            .collect_view()
    };
    let finished = move || {
        tutorial
            .session
            .with(|session| session.as_ref().is_some_and(Session::is_finished))
    };
    let prompt = move || {
        tutorial.session.with(|session| {
            session
                .as_ref()
                .and_then(Session::current_step)
                .map(|step| step.prompt)
        })
    };
    let waits_for_continue = move || {
        tutorial
            .session
            .with(|session| session.as_ref().is_some_and(Session::awaits_continue))
    };
    let feedback = move || {
        tutorial.feedback.get().and_then(|feedback| match feedback {
            Feedback::Note(_) => None,
            Feedback::Hint(hint) => Some((
                "text-red-600 dark:text-red-400",
                format!("Not quite. {hint}"),
            )),
            Feedback::Refuted(hint) => Some((
                "text-red-600 dark:text-red-400",
                format!("Black struck back. {hint}"),
            )),
            Feedback::KeepGoing => Some((
                "text-green-700 dark:text-green-400",
                "Good! It's back where it started, so try the next one.".to_string(),
            )),
            Feedback::WellDone => Some((
                "text-green-700 dark:text-green-400",
                "Well done!".to_string(),
            )),
            Feedback::LessonComplete => Some((
                "text-green-700 dark:text-green-400",
                "Lesson complete!".to_string(),
            )),
            Feedback::ContinueFirst => Some((
                "text-gray-600 dark:text-gray-300",
                "Look around as much as you like. Press Continue when you're ready to move."
                    .to_string(),
            )),
        })
    };
    let outro = move || lesson().map_or("", |lesson| lesson.outro);
    let written_moves = move || {
        tutorial
            .session
            .with(|session| session.as_ref().and_then(Session::written_moves))
    };
    let previous = move || neighbours(lesson_id.get()).0.map(|lesson| lesson.id);
    let next = move || neighbours(lesson_id.get()).1.map(|lesson| lesson.id);
    let game_link = move || source_game(lesson_id.get());
    let restart = move |_| {
        if let Some(lesson) = lesson() {
            tutorial.load(lesson);
        }
    };
    let text_only = move || lesson().is_some_and(Lesson::is_text_only);
    let navigate = StoredValue::new_local(use_navigate());
    let mark_read = move |_| {
        let id = lesson_id.get_untracked();
        tutorial.progress.complete(id);
        let destination = neighbours(id).1.map_or_else(
            || "/tutorial".to_string(),
            |next| format!("/tutorial?lesson={}", next.id),
        );
        navigate.with_value(|navigate| navigate(&destination, Default::default()));
    };

    view! {
        <div class="flex flex-col gap-3 p-3 text-sm leading-6 text-gray-700 xl:gap-4 xl:p-5 xl:text-base xl:leading-7 dark:text-gray-200 ui-panel">
            <div>
                <div class="flex gap-2 justify-between items-baseline">
                    <div class="text-xs font-bold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                        {chapter_title} " · " {position}
                    </div>
                    <A
                        href="/tutorial"
                        attr:class=move || {
                            if compact {
                                "text-xs whitespace-nowrap ui-text-link"
                            } else {
                                "hidden"
                            }
                        }
                    >
                        "All lessons"
                    </A>
                </div>
                <h1 class="text-lg font-bold text-gray-900 xl:text-2xl dark:text-gray-100">
                    {title}
                </h1>
            </div>
            <div class="flex flex-col gap-2" class:hidden=move || compact>
                {intro}
            </div>
            <ShowLet some=move || tutorial.load_error.get() let:error>
                <div class="ui-empty-state" role="status">
                    {if lesson().is_some_and(|lesson| lesson.is_pending()) {
                        "This board is still being set up. Check back soon!".to_string()
                    } else {
                        error
                    }}
                </div>
            </ShowLet>
            <Show when=move || finished() && !outro().is_empty()>{move || paragraph(outro())}</Show>
            <ShowLet some=prompt let:prompt>
                <div class="p-2 font-medium rounded-lg border xl:p-3 border-orange-dawn/40 bg-orange-dawn/10">
                    {prompt}
                </div>
            </ShowLet>
            <ShowLet some=feedback let:feedback>
                <div class=format!("font-medium {}", feedback.0) role="status">
                    {feedback.1}
                </div>
            </ShowLet>
            <ShowLet some=written_moves let:moves>
                <div class="flex flex-col gap-1">
                    <div class="text-xs font-bold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                        "Moves"
                    </div>
                    <ol class="pl-6 font-mono text-xs list-decimal">
                        {if moves.is_empty() {
                            view! { <li class="list-none">"No moves yet."</li> }.into_any()
                        } else {
                            moves
                                .into_iter()
                                .map(|written| view! { <li>{written}</li> })
                                .collect_view()
                                .into_any()
                        }}
                    </ol>
                </div>
            </ShowLet>
            <div class="flex flex-wrap gap-2">
                <Show when=move || {
                    tutorial
                        .session
                        .with(|session| session.as_ref().is_some_and(Session::is_refuted))
                }>
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        on:click=move |_| tutorial.try_again()
                    >
                        "Try again"
                    </button>
                </Show>
                <Show when=move || waits_for_continue() && !text_only()>
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        on:click=move |_| tutorial.advance()
                    >
                        "Continue"
                    </button>
                </Show>
                <Show when=move || tutorial.session.with(Option::is_some) && !text_only()>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=restart
                    >
                        "Restart lesson"
                    </button>
                </Show>
                <Show when=text_only>
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        on:click=mark_read
                    >
                        "Mark as read"
                    </button>
                </Show>
            </div>
            <div class="flex justify-between">
                <ShowLet some=previous let:id>
                    <A href=format!("/tutorial?lesson={id}") attr:class="ui-text-link">
                        "← Previous"
                    </A>
                </ShowLet>
                <span></span>
                <ShowLet some=next let:id>
                    <A
                        href=format!("/tutorial?lesson={id}")
                        attr:class=move || {
                            if finished() {
                                "ui-button ui-button-primary ui-button-sm"
                            } else {
                                "ui-text-link"
                            }
                        }
                    >
                        "Next lesson →"
                    </A>
                </ShowLet>
            </div>
            {move || {
                game_link()
                    .map(|href| {
                        view! {
                            <a
                                href=href
                                target="_blank"
                                rel="noopener"
                                class="text-xs text-gray-500 dark:text-gray-400 hover:underline no-link-style"
                            >
                                "From a real game ↗"
                            </a>
                        }
                    })
            }}
            <details class="group" class:hidden=move || !compact>
                <summary class="justify-between ui-disclosure-summary">
                    <span>"About this lesson"</span>
                    <span class="flex-shrink-0 ml-1.5 transition-transform group-open:rotate-180">
                        <svg class="size-5" viewBox="0 0 20 20" fill="currentColor">
                            <path
                                fill-rule="evenodd"
                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                clip-rule="evenodd"
                            />
                        </svg>
                    </span>
                </summary>
                <div class="flex flex-col gap-2 pt-2">{intro}</div>
            </details>
        </div>
    }
}

fn paragraph(text: &'static str) -> impl IntoView {
    let mut parts = Vec::new();
    let mut plain = String::new();
    for word in text.split_inclusive(' ') {
        match word.trim_end().strip_prefix("https://") {
            Some(shown) => {
                if !plain.is_empty() {
                    parts.push(std::mem::take(&mut plain).into_any());
                }
                parts.push(
                    view! {
                        <a href=word.trim_end() target="_blank" rel="noopener noreferrer">
                            {shown}
                        </a>
                    }
                    .into_any(),
                );
                if word.ends_with(' ') {
                    plain.push(' ');
                }
            }
            None => plain.push_str(word),
        }
    }
    if !plain.is_empty() {
        parts.push(plain.into_any());
    }
    view! { <p>{parts}</p> }
}
