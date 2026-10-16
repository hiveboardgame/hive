use crate::{
    providers::tutorial::{Feedback, TutorialContext},
    tutorial::{
        chapter_of,
        group_of,
        find_lesson,
        lesson_count,
        lessons,
        neighbours,
        Lesson,
        Session,
    },
};
use hive_lib::Piece;
use leptos::{html, prelude::*};
use leptos_router::{components::A, hooks::use_navigate};

enum PromptPart {
    Text(&'static str),
    Piece(Piece),
}

fn prompt_parts(mut text: &'static str) -> Vec<PromptPart> {
    let mut parts = Vec::new();
    while let Some(start) = text.find("[[") {
        if start > 0 {
            parts.push(PromptPart::Text(&text[..start]));
        }
        let token_start = start + 2;
        let Some(end) = text[token_start..].find("]]") else {
            parts.push(PromptPart::Text(&text[start..]));
            return parts;
        };
        let token_end = token_start + end;
        let token = &text[token_start..token_end];
        match token.parse::<Piece>() {
            Ok(piece) => parts.push(PromptPart::Piece(piece)),
            Err(_) => parts.push(PromptPart::Text(&text[start..token_end + 2])),
        }
        text = &text[token_end + 2..];
    }
    if !text.is_empty() {
        parts.push(PromptPart::Text(text));
    }
    parts
}

fn piece_icon(piece: Piece) -> impl IntoView {
    let src = format!("/assets/tiles/flat/{}.svg", piece.bug().name());
    let alt = format!("{} {} piece", piece.color().name(), piece.bug().name());
    view! {
        <img
            src=src
            alt=alt
            class="inline-block mx-1 object-contain size-5 align-text-bottom"
        />
    }
}

fn piece_content(text: &'static str) -> impl IntoView {
    prompt_parts(text)
        .into_iter()
        .map(|part| match part {
            PromptPart::Text(text) => view! { <span>{text}</span> }.into_any(),
            PromptPart::Piece(piece) => piece_icon(piece).into_any(),
        })
        .collect_view()
}

fn prompt_line(text: &'static str) -> impl IntoView {
    text.split("**")
        .enumerate()
        .map(|(index, text)| {
            let content = piece_content(text);
            if index % 2 == 0 {
                view! { <span>{content}</span> }.into_any()
            } else {
                view! { <strong>{content}</strong> }.into_any()
            }
        })
        .collect_view()
}

fn tutorial_prompt(prompt: &'static str) -> impl IntoView {
    prompt
        .split('\n')
        .enumerate()
        .map(|(line_index, line)| {
            let line = prompt_line(line);
            if line_index == 0 {
                view! { {line} }.into_any()
            } else {
                view! { <br /> {line} }.into_any()
            }
        })
        .collect_view()
}

#[component]
pub fn LessonPanel(
    #[prop(into)] lesson_id: Signal<&'static str>,
    #[prop(optional)] compact: bool,
    #[prop(optional)] header_only: bool,
    #[prop(optional)] body_only: bool,
) -> impl IntoView {
    let tutorial = expect_context::<TutorialContext>();
    let lesson = move || find_lesson(lesson_id.get());
    let mobile_layout = move || compact;
    let chapter_title = move || chapter_of(lesson_id.get()).map_or("", |chapter| chapter.title);
    let title = move || lesson().map_or("", |lesson| lesson.title);
    let group_title = move || group_of(lesson_id.get()).map(|group| group.title);
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
    let instruction_content = NodeRef::<html::Div>::new();
    Effect::watch(
        move || (lesson_id.get(), prompt()),
        move |_, _, _| {
            if compact {
                if let Some(content) = instruction_content.get() {
                    content.set_scroll_top(0);
                }
            }
        },
        false,
    );
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
            Feedback::YourTurn => Some((
                "text-gray-700 dark:text-gray-200",
                "Your turn.".to_string(),
            )),
            Feedback::WellDone => Some((
                "text-green-700 dark:text-green-400",
                "Well done!".to_string(),
            )),
            Feedback::LessonComplete => Some((
                "text-lg font-bold text-green-700 xl:text-xl dark:text-green-400",
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
    let completion_tip = move || {
        tutorial.session.with(|session| {
            session
                .as_ref()
                .and_then(Session::current_step)
                .map_or("", |step| step.completion_tip)
        })
    };
    let written_moves = move || {
        tutorial
            .session
            .with(|session| session.as_ref().and_then(Session::written_moves))
    };
    let previous = move || neighbours(lesson_id.get()).0.map(|lesson| lesson.id);
    let next = move || neighbours(lesson_id.get()).1.map(|lesson| lesson.id);
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
        <div class=move || {
            if mobile_layout() {
                "flex overflow-hidden flex-col gap-2 p-3 min-h-0 h-full text-sm leading-5 text-gray-700 dark:text-gray-200 ui-panel"
            } else {
                "flex flex-col gap-3 p-3 text-sm leading-6 text-gray-700 xl:gap-4 xl:p-5 xl:text-base xl:leading-7 dark:text-gray-200 ui-panel"
            }
        }>
            <Show when=move || !body_only>
            <div class:shrink-0=mobile_layout>
                <div class=move || {
                    if mobile_layout() {
                        "flex gap-2 justify-between items-baseline pb-1 mb-1 border-b border-black/10 dark:border-white/10"
                    } else {
                        "flex gap-2 justify-between items-baseline"
                    }
                }>
                    <div class="text-xs font-bold tracking-wide text-gray-500 uppercase dark:text-gray-400">
                        {chapter_title} " · " {position}
                        <ShowLet some=group_title let:group>
                            " · " {group}
                        </ShowLet>
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
            </Show>
            <Show when=move || !header_only>
            <div
                node_ref=instruction_content
                role=move || compact.then_some("region")
                aria-label=move || compact.then_some("Lesson instructions")
                tabindex=move || compact.then_some(0)
                class=move || {
                    if mobile_layout() {
                        "flex overflow-y-auto flex-col gap-2 min-h-0 grow overscroll-contain"
                    } else {
                        "contents"
                    }
                }
            >
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
                        {tutorial_prompt(prompt)}
                    </div>
                </ShowLet>
                <ShowLet some=feedback let:feedback>
                    <div class=format!("font-medium {}", feedback.0) role="status">
                        {feedback.1}
                    </div>
                </ShowLet>
                    <Show when=move || {
                        tutorial
                            .session
                            .with(|session| session.as_ref().is_some_and(Session::needs_retry))
                    }>
                        <button
                            type="button"
                            class="self-start ui-button ui-button-primary ui-button-sm"
                            on:click=move |_| tutorial.try_again()
                        >
                            "Try again"
                        </button>
                    </Show>
                <Show when=move || finished() && !completion_tip().is_empty()>
                    <div class="font-medium text-gray-700 dark:text-gray-200">
                        {move || tutorial_prompt(completion_tip())}
                    </div>
                </Show>
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
            </div>
            <div class=move || {
                if mobile_layout() {
                    "flex flex-col gap-2 pt-2 border-t border-black/10 shrink-0 dark:border-white/10"
                } else {
                    "contents"
                }
            }>
                <div class="flex flex-wrap gap-2">
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
                <div class=move || {
                    if mobile_layout() {
                        "flex gap-3 justify-between items-center w-full"
                    } else {
                        "flex justify-between"
                    }
                }>
                    <ShowLet some=previous let:id>
                        <A href=format!("/tutorial?lesson={id}") attr:class="ui-text-link">
                            "← Previous"
                        </A>
                    </ShowLet>
                    <span class:hidden=mobile_layout></span>
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
            </div>

            </Show>
        </div>
    }
}

/// Reading drawer shared by mobile lessons, over the fixed board area.
#[component]
pub fn MobileLessonAbout(#[prop(into)] lesson_id: Signal<&'static str>) -> impl IntoView {
    let intro = move || {
        find_lesson(lesson_id.get())
            .map(|lesson| lesson.intro)
            .unwrap_or_default()
            .iter()
            .copied()
            .map(paragraph)
            .collect_view()
    };

    view! {
        <details class="group absolute inset-x-0 top-0 z-30 overflow-y-auto max-h-full overscroll-contain text-sm leading-5 text-gray-700 bg-even-light shadow-md dark:bg-surface-panel dark:text-gray-200">
            <summary class="sticky top-0 justify-between h-11 bg-even-light dark:bg-surface-panel ui-disclosure-summary">
                <span>"Read the lesson"</span>
                <svg
                    class="transition-transform size-5 shrink-0 group-open:rotate-180"
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    aria-hidden="true"
                >
                    <path
                        fill-rule="evenodd"
                        d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                        clip-rule="evenodd"
                    />
                </svg>
            </summary>
            <div class="flex flex-col gap-2 px-3 pt-1 pb-3">
                {intro}

            </div>
        </details>
    }
}

fn linked_text(text: &'static str) -> impl IntoView {
    let mut parts = Vec::new();
    let mut plain = String::new();
    for word in text.split_inclusive(' ') {
        match word.trim_end().strip_prefix("https://") {
            Some(shown) => {
                if !plain.is_empty() {
                    let text = std::mem::take(&mut plain);
                    parts.push(view! { <span>{text}</span> }.into_any());
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
        parts.push(view! { <span>{plain}</span> }.into_any());
    }
    parts
}

fn paragraph(text: &'static str) -> impl IntoView {
    let parts = text
        .split("**")
        .enumerate()
        .map(|(index, text)| {
            let content = prompt_parts(text)
                .into_iter()
                .map(|part| match part {
                    PromptPart::Text(text) => {
                        view! { <span>{linked_text(text)}</span> }.into_any()
                    }
                    PromptPart::Piece(piece) => piece_icon(piece).into_any(),
                })
                .collect_view();
            if index % 2 == 0 {
                view! { <span>{content}</span> }.into_any()
            } else {
                view! { <strong>{content}</strong> }.into_any()
            }
        })
        .collect_view();
    view! { <p>{parts}</p> }
}
