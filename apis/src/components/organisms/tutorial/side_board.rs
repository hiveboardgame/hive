use crate::{
    components::{
        atoms::gc_button::GameControlButton,
        organisms::{
            display_timer::{DisplayTimer, Placement},
            history::History,
            reserve::{Alignment, Reserve},
        },
    },
    hiveground::HivegroundInteraction,
    providers::{
        config::Config,
        tutorial::{Feedback, TutorialContext},
    },
};
use hive_lib::{Board as HiveBoard, Color, GameControl};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Game,
    History,
    Chat,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Game => "Game",
            Tab::History => "History",
            Tab::Chat => "Chat",
        }
    }
}

const CONTROLS: [(GameControl, &str); 3] = [
    (
        GameControl::TakebackRequest(Color::White),
        "Takeback requested. Your opponent sees the request and can accept, which undoes your last move, or decline it.",
    ),
    (
        GameControl::DrawOffer(Color::White),
        "Draw offered. If your opponent accepts, the game ends and nobody wins. If not, play goes on.",
    ),
    (
        GameControl::Resign(Color::White),
        "You resigned. In a real game that ends it at once and your opponent wins. Here nothing happened, so carry on.",
    ),
];

#[component]
pub fn TutorialSideboard(
    learner: Signal<Color>,
    opponent: Signal<Color>,
    interaction: HivegroundInteraction,
    history_board: Memo<HiveBoard>,
) -> impl IntoView {
    let config = expect_context::<Config>().0;
    let background_style = Signal::derive(move || {
        let bg = config.with(|c| c.tile.get_effective_background_color(c.prefers_dark));
        format!("background-color: {bg}")
    });
    let tab = RwSignal::new(Tab::Game);
    let tab_button = move |name: Tab| {
        view! {
            <button
                type="button"
                on:click=move |_| tab.set(name)
                class=move || {
                    format!(
                        "ui-board-tab-trigger cursor-pointer {}",
                        if tab.get() == name {
                            "ui-segmented-active hover:bg-button-dawn dark:hover:bg-button-twilight"
                        } else {
                            "hover:bg-blue-light/70 dark:hover:bg-pillbug-teal/15"
                        },
                    )
                }
            >
                {name.label()}
            </button>
        }
    };
    view! {
        <div
            class="grid grid-cols-2 col-span-2 col-start-9 grid-rows-6 row-span-full row-start-1 gap-2 p-1"
            style=background_style
        >
            <DisplayTimer placement=Placement::Top vertical=false name="Opponent" />
            <div class="flex relative flex-col col-span-2 row-span-4 row-start-2 h-full min-h-0 select-none ui-board-side-panel">
                <div>
                    <div class="sticky top-0 z-10 ui-board-tab-list">
                        {tab_button(Tab::Game)} {tab_button(Tab::History)} {tab_button(Tab::Chat)}
                    </div>
                </div>
                <div
                    class="flex flex-col h-full min-h-0"
                    class:hidden=move || tab.get() != Tab::Game
                >
                    <Reserve
                        color=opponent
                        alignment=Alignment::DoubleRow
                        interaction
                        history_board
                    />
                    <div class="flex justify-center items-start">
                        <TutorialGameControls />
                    </div>
                    <Reserve
                        color=learner
                        alignment=Alignment::DoubleRow
                        interaction
                        history_board
                    />
                </div>
                <div class="h-full min-h-0" class:hidden=move || tab.get() != Tab::History>
                    <History interaction history_board />
                </div>
                <div class="p-2 h-full min-h-0" class:hidden=move || tab.get() != Tab::Chat>
                    <div class="ui-empty-state">
                        "During a game you can chat with your opponent here, and spectators have their own channel."
                    </div>
                </div>
            </div>
            <DisplayTimer placement=Placement::Bottom vertical=false name="You" />
        </div>
    }
}

#[component]
pub fn TutorialGameControls() -> impl IntoView {
    let tutorial = expect_context::<TutorialContext>();
    let on_confirm = Callback::new(move |control: GameControl| {
        let outcome = CONTROLS
            .iter()
            .find(|(known, _)| *known == control)
            .map_or("", |(_, outcome)| *outcome);
        tutorial.feedback.set(Some(Feedback::Note(outcome)));
    });
    let note = move || match tutorial.feedback.get() {
        Some(Feedback::Note(note)) => note,
        _ => "",
    };
    let buttons = CONTROLS
        .into_iter()
        .map(|(game_control, _)| view! { <GameControlButton game_control on_confirm /> })
        .collect_view();
    view! {
        <div class="flex flex-col items-center grow shrink">
            <div class="flex justify-around items-center w-full">{buttons}</div>
            <p class="px-2 text-sm text-center" class:hidden=move || note().is_empty()>
                {note}
            </p>
        </div>
    }
}
