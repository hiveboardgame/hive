use crate::{
    common::{CurrentConfirm, MoveConfirm},
    components::{
        layouts::base_layout::OrientationSignal,
        organisms::{
            board::Board,
            display_timer::{DisplayTimer, Placement},
            reserve::{Alignment, Reserve, MOBILE_RESERVE_VIEWBOX},
            tutorial::{
                ChapterList,
                LessonPanel,
                TutorialGameControls,
                TutorialOverview,
                TutorialSideboard,
            },
        },
    },
    hiveground::{selected_history_board, tutorial_hiveground_interaction},
    hooks::tutorial_clock::use_tutorial_clock,
    providers::{
        annotations::AnnotationsSignal,
        game_state::GameStateStore,
        timer::TimerSignal,
        tutorial::{TutorialContext, TutorialProgress},
    },
    tutorial::{find_lesson, Setup},
};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[component]
pub fn Tutorial() -> impl IntoView {
    let game_state = GameStateStore::new();
    provide_context(game_state);
    provide_context(CurrentConfirm(Memo::new(move |_| MoveConfirm::Single)));
    let timer = TimerSignal::new();
    provide_context(timer);
    let progress = TutorialProgress::synced();
    let tutorial = TutorialContext::new(game_state, progress);
    provide_context(tutorial);
    provide_context(AnnotationsSignal::fixed(tutorial.annotations()));
    let interaction = tutorial_hiveground_interaction();
    let history_board = selected_history_board(game_state);
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;

    let queries = use_query_map();
    let chosen = Memo::new(move |_| {
        queries
            .with(|queries| queries.get("lesson"))
            .and_then(|id| find_lesson(&id))
            .map(|lesson| lesson.id)
    });
    let lesson_id = Signal::derive(move || chosen.get().unwrap_or_default());
    // Loading during setup lets the server render the lesson exactly as the browser will.
    if let Some(lesson) = chosen.get_untracked().and_then(find_lesson) {
        tutorial.load(lesson);
    }
    Effect::watch(
        move || chosen.get(),
        move |id, _, _| {
            if let Some(lesson) = id.and_then(find_lesson) {
                tutorial.load(lesson);
            }
        },
        false,
    );
    use_tutorial_clock(tutorial, timer);
    let clocked =
        move || find_lesson(lesson_id.get()).is_some_and(|lesson| lesson.clock().is_some());

    let has_board = move || {
        tutorial.session.with(Option::is_some)
            && find_lesson(lesson_id.get()).is_some_and(|lesson| lesson.has_board())
    };
    let learner = Signal::derive(move || {
        tutorial.session.with(|session| {
            session
                .as_ref()
                .map_or(Color::White, |session| session.learner())
        })
    });
    let opponent = Signal::derive(move || learner.get().opposite_color());
    let placeholder_text = move || {
        find_lesson(lesson_id.get()).and_then(|lesson| match lesson.setup {
            Setup::Pending => Some("The board for this lesson is still being built."),
            Setup::TextOnly => Some("No board needed for this one: just read along."),
            Setup::Position(_) | Setup::Clocked { .. } => None,
        })
    };
    let board_placeholder = move || {
        view! {
            <div class="flex col-span-full row-span-full justify-center items-center p-6 w-full h-full">
                <ShowLet some=placeholder_text let:text>
                    <div class="max-w-sm text-center ui-empty-state">{text}</div>
                </ShowLet>
            </div>
        }
    };

    view! {
        <Show
            when=move || chosen.get().is_some()
            fallback=move || {
                view! { <TutorialOverview progress /> }
            }
        >
            <div class=move || {
                if vertical() {
                    "ui-board-page-surface flex flex-col overflow-hidden h-[calc(100svh-2.5rem)] standalone:min-h-[var(--app-height)]"
                } else {
                    "ui-board-page-surface grid overflow-hidden min-h-[100dvh] max-h-[100dvh] grid-cols-10 grid-rows-6 pr-1 standalone:min-h-[var(--app-height)] standalone:max-h-[var(--app-height)]"
                }
            }>
                <Show
                    when=vertical
                    fallback=move || {
                        view! {
                            <div class="flex overflow-y-auto flex-col col-span-2 col-start-1 row-span-6 row-start-1 gap-2 my-1 ml-1 min-h-0 xl:col-span-3">
                                <LessonPanel lesson_id />
                                <ChapterList progress current=lesson_id />
                            </div>
                            <div class="grid relative grid-cols-8 col-span-6 col-start-3 grid-rows-6 row-span-6 row-start-1 min-w-0 min-h-0 xl:col-span-5 xl:col-start-4">
                                <Show when=has_board fallback=board_placeholder>
                                    <Board interaction history_board fit_on=tutorial.generation />
                                </Show>
                            </div>
                            <Show
                                when=clocked
                                fallback=move || {
                                    view! {
                                        <div class="flex flex-col col-span-2 col-start-9 row-span-6 row-start-1 justify-between p-1 my-1 mr-1 min-h-0 ui-board-side-panel">
                                            <Reserve
                                                color=opponent
                                                alignment=Alignment::DoubleRow
                                                interaction
                                                history_board
                                            />
                                            <Reserve
                                                color=learner
                                                alignment=Alignment::DoubleRow
                                                interaction
                                                history_board
                                            />
                                        </div>
                                    }
                                }
                            >
                                <TutorialSideboard learner opponent interaction history_board />
                            </Show>
                        }
                    }
                >
                    <div class="overflow-y-auto shrink-0 h-[40%]">
                        <LessonPanel lesson_id compact=true />
                    </div>
                    <Show when=clocked>
                        <div class="flex shrink-0 ui-board-reserve">
                            <TutorialGameControls />
                        </div>
                    </Show>
                    <div class="flex justify-between ml-1 h-12 max-h-12 shrink-0 ui-board-reserve">
                        <Reserve
                            color=opponent
                            alignment=Alignment::SingleRow
                            viewbox_str=MOBILE_RESERVE_VIEWBOX
                            interaction
                            history_board
                        />
                        <Show when=clocked>
                            <div class="flex w-16 min-h-0 shrink-0">
                                <DisplayTimer vertical=true placement=Placement::Top />
                            </div>
                        </Show>
                    </div>
                    <div class="flex relative min-h-0 grow">
                        <Show when=has_board fallback=board_placeholder>
                            <Board interaction history_board fit_on=tutorial.generation />
                        </Show>
                    </div>
                    <div class="flex justify-between ml-1 h-20 max-h-20 shrink-0 ui-board-reserve">
                        <Reserve
                            color=learner
                            alignment=Alignment::SingleRow
                            viewbox_str=MOBILE_RESERVE_VIEWBOX
                            interaction
                            history_board
                        />
                        <Show when=clocked>
                            <div class="flex w-16 min-h-0 shrink-0">
                                <DisplayTimer vertical=true placement=Placement::Bottom />
                            </div>
                        </Show>
                    </div>
                </Show>
            </div>
        </Show>
    }
}
