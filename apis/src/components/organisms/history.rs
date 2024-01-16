use crate::components::{
    atoms::history_button::{HistoryButton, HistoryNavigation},
    organisms::reserve::{Alignment, Reserve},
};
use crate::providers::game_state::GameStateSignal;
use hive_lib::{color::Color, game_status::GameStatus};
use leptos::*;
use leptos_use::use_window;

#[component]
pub fn HistoryMove(
    turn: usize,
    piece: String,
    position: String,
    parent_div: NodeRef<html::Div>,
) -> impl IntoView {
    let mut game_state_signal = expect_context::<GameStateSignal>();
    let div_ref = create_node_ref::<html::Div>();
    // scrolls history when new move is made
    // TODO: find a nicer way to do it, maybe do it just on_load and add div_height to scroll_heigt
    div_ref.on_load(move |_| {
        let _ = div_ref
            .get_untracked()
            .expect("div to be loaded")
            .on_mount(move |_| {
                let parent_div = parent_div.get_untracked().expect("div to be loaded");
                parent_div.set_scroll_top(parent_div.scroll_height())
            });
    });
    let onclick = move |_| {
        game_state_signal.show_history_turn(turn);
    };
    let get_class = move || {
        let mut class = "ml-3 hover:bg-blue-300 col-span-2 leading-6 h-auto max-h-6 transform transition-transform duration-300 active:scale-95";
        if let Some(history_turn) = (game_state_signal.signal)().history_turn {
            if turn == history_turn {
                class = "ml-3 hover:bg-blue-300 col-span-2 bg-orange-300 leading-6 h-auto max-h-6 transform transition-transform duration-300 active:scale-95"
            }
        }
        class
    };
    view! {
        <div ref=div_ref class=get_class on:click=onclick>
            {format!("{}. {piece} {position}", turn + 1)}
        </div>
    }
}

#[component]
pub fn History(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let history_moves = move || {
        let mut his = Vec::new();
        for (i, (piece, pos)) in (game_state_signal.signal)()
            .state
            .history
            .moves
            .into_iter()
            .enumerate()
        {
            if i == 0 {
                his.push((i, piece, String::new()));
            } else {
                his.push((i, piece, pos));
            }
        }
        his
    };

    let parent = create_node_ref::<html::Div>();
    let is_finished = move || {
        matches!(
            (game_state_signal.signal)().state.game_status,
            GameStatus::Finished(_)
        )
    };

    let game_result = move || match (game_state_signal.signal)().state.game_status {
        GameStatus::Finished(result) => result.to_string(),
        _ => "".to_string(),
    };

    let window = use_window();
    let active = Signal::derive(move || {
        if window.is_some() {
            window
                .as_ref()
                .expect("window to exist")
                .document()
                .expect("window to have a document")
                .query_selector(".bg-orange-300")
                .expect("to have an Element")
        } else {
            None
        }
    });

    let focus = Callback::new(move |()| {
        if let Some(elem) = active.get_untracked() {
            elem.scroll_into_view_with_bool(false);
        }
    });

    let nav_buttons_style =
        "flex justify-center h-fit hover:bg-green-400 dark:hover:bg-green-500 transform transition-transform duration-300 active:scale-95 mt-6 rounded-md border-cyan-500 border-2 drop-shadow-lg";
    let white_black_styles = "col-span-2";
    view! {
        <div class=format!("h-[90%] grid grid-cols-4 grid-rows-6 gap-1 pb-4 {extend_tw_classes}")>
            <div class="col-span-4 grid grid-cols-4 gap-1 dark:bg-dark bg-light min-h-0 row-span-3 row-start-1">
                <HistoryButton
                    nav_buttons_style=nav_buttons_style
                    action=HistoryNavigation::First
                    post_action=focus
                />

                <HistoryButton
                    nav_buttons_style=nav_buttons_style
                    action=HistoryNavigation::Previous
                    post_action=focus
                />
                <HistoryButton
                    nav_buttons_style=nav_buttons_style
                    action=HistoryNavigation::Next
                    post_action=focus
                />

                <HistoryButton
                    nav_buttons_style=nav_buttons_style
                    action=HistoryNavigation::Last
                    post_action=focus
                />
                <div class=white_black_styles>
                    <Reserve alignment=Alignment::DoubleRow color=Color::White/>
                </div>
                <div class=white_black_styles>
                    <Reserve alignment=Alignment::DoubleRow color=Color::Black/>
                </div>
            </div>
            <div
                ref=parent
                class="col-span-4 grid grid-cols-4 gap-1 overflow-auto max-h-[inherit] min-h-[inherit] mt-8 row-span-3 row-start-4"
            >
                <For each=history_moves key=|history_move| (history_move.0) let:history_move>

                    <HistoryMove
                        turn=history_move.0
                        piece=history_move.1
                        position=history_move.2
                        parent_div=parent
                    />
                </For>

                <Show when=is_finished>
                    <div class="col-span-4 text-center">{game_result}</div>
                </Show>
            </div>
        </div>
    }
}
