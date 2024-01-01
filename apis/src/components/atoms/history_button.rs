use crate::providers::game_state::GameStateSignal;
use leptos::*;
use leptos_icons::{
    AiIcon::{
        AiFastBackwardFilled, AiFastForwardFilled, AiStepBackwardFilled, AiStepForwardFilled,
    },
    Icon,
};

pub enum HistoryNavigation {
    First,
    Last,
    Next,
    Previous,
    MobileLast,
}

#[component]
pub fn HistoryButton(
    nav_buttons_style: &'static str,
    action: HistoryNavigation,
    #[prop(optional)] post_action: Option<Callback<()>>,
) -> impl IntoView {
    let icon = match action {
        HistoryNavigation::First => leptos_icons::Icon::Ai(AiFastBackwardFilled),
        HistoryNavigation::Last => leptos_icons::Icon::Ai(AiFastForwardFilled),
        HistoryNavigation::Next => leptos_icons::Icon::Ai(AiStepForwardFilled),
        HistoryNavigation::Previous => leptos_icons::Icon::Ai(AiStepBackwardFilled),
        HistoryNavigation::MobileLast => leptos_icons::Icon::Ai(AiFastForwardFilled),
    };
    view! {
        <button
            class=nav_buttons_style
            on:click=move |_| {
                send_action(&action);
                if let Some(post_action) = post_action {
                    post_action(())
                }
            }
        >

            <Icon icon=icon/>
        </button>
    }
}

fn send_action(action: &HistoryNavigation) {
    let mut game_state_signal = expect_context::<GameStateSignal>();
    match action {
        HistoryNavigation::First => game_state_signal.first_history_turn(),
        HistoryNavigation::Last => game_state_signal.view_history(),
        HistoryNavigation::Next => game_state_signal.next_history_turn(),
        HistoryNavigation::Previous => game_state_signal.previous_history_turn(),
        HistoryNavigation::MobileLast => game_state_signal.view_game(),
    }
}
