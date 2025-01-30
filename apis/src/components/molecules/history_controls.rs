use crate::components::atoms::history_button::{HistoryButton, HistoryNavigation};
use crate::components::organisms::reserve::{Alignment, Reserve};
use crate::providers::game_state::GameStateSignal;
use leptos::ev::keydown;
use hive_lib::Color;
use leptos::{prelude::*, html};
use leptos_use::{use_event_listener, use_window};

#[component]
pub fn HistoryControls(#[prop(optional)] parent: MaybeProp<NodeRef<html::Div>>) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let window = use_window();
    let active = Signal::derive(move || {
        window.as_ref().and_then(|w| {
            w.document()
                .expect("window to have a document")
                .query_selector(".bg-orange-twilight")
                .expect("to have an Element")
        })
    });
    let focus = Callback::new(move |()| {
        if let Some(elem) = active.get_untracked() {
            elem.scroll_into_view_with_bool(false);
        }
    });
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

    let go_to_end = Callback::new(move |()| {
        if let Some(parent) = parent.get() {
            let parent = parent.get_untracked().expect("div to be loaded");
            parent.set_scroll_top(parent.scroll_height());
        }
        game_state.show_history_turn(game_state.signal.get_untracked().state.turn - 1);
    });
    let if_last_go_to_end = Callback::new(move |()| {
        focus.run(());
        let gamestate = game_state.signal.get_untracked();
        {
            if let Some(turn) = gamestate.history_turn {
                if turn == gamestate.state.turn - 1 {
                    go_to_end.run(());
                }
            }
        }
    });
    view! {
        <div>
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
                <Reserve alignment=Alignment::DoubleRow color=Color::White analysis=false />
                <Reserve alignment=Alignment::DoubleRow color=Color::Black analysis=false />
            </div>
        </div>
    }
}
