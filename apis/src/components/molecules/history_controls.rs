use crate::{
    components::{
        atoms::history_button::{HistoryButton, HistoryNavigation},
        organisms::reserve::{Alignment, Reserve},
    },
    hiveground::HivegroundInteraction,
    providers::game_state::GameStateStore,
};
use hive_lib::{Color, State};
use leptos::{html, prelude::*};
use leptos_use::use_window;

pub fn scroll_active_history_move_into_view() {
    let active = use_window()
        .as_ref()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(".bg-orange-twilight").ok())
        .flatten();
    if let Some(elem) = active {
        elem.scroll_into_view_with_bool(false);
    }
}

#[component]
pub fn HistoryControls(
    interaction: HivegroundInteraction,
    history_state: Memo<State>,
    #[prop(optional)] parent: MaybeProp<NodeRef<html::Div>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let focus = Callback::new(move |()| {
        scroll_active_history_move_into_view();
    });

    let scroll_to_end = Callback::new(move |()| {
        if let Some(parent) = parent.get() {
            let parent = parent.get_untracked().expect("div to be loaded");
            parent.set_scroll_top(parent.scroll_height());
        }
    });
    let if_last_go_to_end = Callback::new(move |()| {
        focus.run(());
        if game_state.is_last_turn_untracked() {
            scroll_to_end.run(());
        }
    });
    view! {
        <div>
            <div class="flex gap-1 min-h-0 [&>*]:grow">
                <HistoryButton action=HistoryNavigation::First post_action=focus />
                <HistoryButton action=HistoryNavigation::Previous post_action=focus />
                <HistoryButton action=HistoryNavigation::Next post_action=if_last_go_to_end />
                <HistoryButton action=HistoryNavigation::Last post_action=scroll_to_end />
            </div>
            <div class="flex p-2">
                <Reserve
                    alignment=Alignment::DoubleRow
                    color=Color::White
                    interaction
                    history_state
                />
                <Reserve
                    alignment=Alignment::DoubleRow
                    color=Color::Black
                    interaction
                    history_state
                />
            </div>
        </div>
    }
}
