use crate::components::{
    atoms::history_button::{HistoryButton, HistoryNavigation},
    organisms::reserve::{Alignment, Reserve},
};
use crate::providers::game_state::GameStateSignal;
use hive_lib::{color::Color, game_status::GameStatus};
use leptos::{ev::keydown, *};
use leptos_use::{use_event_listener, use_window};
use shared_types::conclusion::Conclusion;

#[component]
pub fn HistoryMove(
    turn: usize,
    piece: String,
    position: String,
    repetition: bool,
    parent_div: NodeRef<html::Div>,
) -> impl IntoView {
    let mut game_state_signal = expect_context::<GameStateSignal>();
    let div_ref = create_node_ref::<html::Div>();
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
        let mut class = "ml-3 hover:bg-pillbug-teal col-span-2 leading-6 h-auto max-h-6 transform transition-transform duration-300 active:scale-95";
        if let Some(history_turn) = (game_state_signal.signal)().history_turn {
            if turn == history_turn {
                class = "ml-3 hover:bg-pillbug-teal col-span-2 bg-orange-twilight leading-6 h-auto max-h-6 transform transition-transform duration-300 active:scale-95"
            }
        }
        class
    };
    let rep = if repetition {
        String::from(" ↺")
    } else {
        String::new()
    };
    view! {
        <div ref=div_ref class=get_class on:click=onclick>
            {format!("{}. {piece} {position}{}", turn + 1, rep)}
        </div>
    }
}

#[component]
pub fn History(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let game_state_signal = expect_context::<GameStateSignal>();
    let history_moves = move || {
        (game_state_signal.signal)()
            .state
            .history
            .moves
            .into_iter()
            .enumerate()
            .map(|(i, (piece, pos))| (i, piece, pos))
            .collect::<Vec<(usize, String, String)>>()
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

    let conclusion = move || {
        if let Some(game) = (game_state_signal.signal)().game_response {
            match game.conclusion {
                Conclusion::Board => String::from("Finished on board"),
                Conclusion::Draw => String::from("Draw agreed"),
                Conclusion::Resigned => String::from("Resigned"),
                Conclusion::Timeout => String::from("Timeout"),
                Conclusion::Repetition => String::from("3 move repetition"),
                Conclusion::Unknown => String::from("Unknown"),
            }
        } else {
            String::from("No data")
        }
    };

    let window = use_window();
    let active = Signal::derive(move || {
        if window.is_some() {
            window
                .as_ref()
                .expect("window to exist")
                .document()
                .expect("window to have a document")
                .query_selector(".bg-orange-twilight")
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
    let go_to_end = Callback::new(move |()| {
        let parent = parent.get_untracked().expect("div to be loaded");
        parent.set_scroll_top(parent.scroll_height());
    });

    let if_last_go_to_end = Callback::new(move |()| {
        focus(());
        let gamestate = (game_state_signal.signal)();
        {
            if let Some(turn) = gamestate.history_turn {
                if turn == gamestate.state.turn - 1 {
                    go_to_end(());
                }
            }
        }
    });

    let is_repetition = move |turn: usize| {
        if let Some(game) = (game_state_signal.signal)().game_response {
            game.repetitions.contains(&turn)
        } else {
            false
        }
    };
    let prev_button = create_node_ref::<html::Button>();
    let next_button = create_node_ref::<html::Button>();
    _ = use_event_listener(document().body(), keydown, move |evt| {
        if evt.key() == "ArrowLeft" {
            evt.prevent_default();
            if let Some(prev) = prev_button.get_untracked() {
                prev.click()
            };
        } else if evt.key() == "ArrowRight" {
            evt.prevent_default();
            if let Some(next) = next_button.get_untracked() {
                next.click()
            };
        }
    });
    view! {
        <div class=format!("h-full flex flex-col pb-4 {extend_tw_classes}")>
            <div class="flex gap-1 min-h-0 [&>*]:grow">
                <HistoryButton

                    action=HistoryNavigation::First
                    post_action=focus
                />

                <HistoryButton
                    node_ref=prev_button
                    action=HistoryNavigation::Previous
                    post_action=focus
                />
                <HistoryButton
                    node_ref=next_button
                    action=HistoryNavigation::Next
                    post_action=if_last_go_to_end
                />

                <HistoryButton

                    action=HistoryNavigation::Last
                    post_action=go_to_end
                />
            </div>
            <div class="flex p-2">
                <Reserve alignment=Alignment::DoubleRow color=Color::White/>
                <Reserve alignment=Alignment::DoubleRow color=Color::Black/>
            </div>
            <div ref=parent class="grid grid-cols-4 gap-1 overflow-auto mb-8 max-h-full h-fit">
                <For each=history_moves key=|history_move| (history_move.0) let:history_move>

                    <HistoryMove
                        turn=history_move.0
                        piece=history_move.1
                        position=history_move.2
                        parent_div=parent
                        repetition=is_repetition(history_move.0)
                    />
                </For>

                <Show when=is_finished>
                    <div class="col-span-4 text-center">{game_result}</div>
                    <div class="col-span-4 text-center">{conclusion}</div>
                </Show>
            </div>
        </div>
    }
}
