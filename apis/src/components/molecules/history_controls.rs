use crate::{
    components::{
        molecules::{
            annotation_toolbar::AnnotationToggle,
            play_history_button::{HistoryButton, PlayHistoryNavigation as HistoryNavigation},
        },
        organisms::reserve::{Alignment, Reserve},
    },
    hiveground::HivegroundInteraction,
    hooks::history_nav::scroll_move_into_view,
    providers::game_state::GameStateSignal,
};
use hudsoni::{Color, State};
use leptos::{html, prelude::*};

#[component]
pub fn HistoryControls(
    interaction: HivegroundInteraction,
    history_state: Memo<State>,
    #[prop(optional)] parent: MaybeProp<NodeRef<html::Div>>,
) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let focus = Callback::new(move |()| {
        scroll_move_into_view();
    });

    let scroll_to_end = Callback::new(move |()| {
        if let Some(parent) = parent.get() {
            let parent = parent.get_untracked().expect("div to be loaded");
            parent.set_scroll_top(parent.scroll_height());
        }
    });
    let if_last_go_to_end = Callback::new(move |()| {
        focus.run(());
        if game_state.signal.with_untracked(|gs| gs.is_last_turn()) {
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
                <AnnotationToggle
                    class="ui-board-nav-button"
                    active_tw_classes="ui-segmented-active"
                />
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
