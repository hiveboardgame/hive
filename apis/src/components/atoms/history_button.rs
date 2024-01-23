use crate::providers::game_state::GameStateSignal;
use leptos::{leptos_dom::helpers::debounce, *};
use leptos_icons::{
    AiIcon::{
        AiFastBackwardFilled, AiFastForwardFilled, AiStepBackwardFilled, AiStepForwardFilled,
    },
    Icon,
};
#[derive(Clone)]
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
    let cloned_action = action.clone();
    let icon = match action {
        HistoryNavigation::First => leptos_icons::Icon::Ai(AiFastBackwardFilled),
        HistoryNavigation::Last | HistoryNavigation::MobileLast => {
            leptos_icons::Icon::Ai(AiFastForwardFilled)
        }
        HistoryNavigation::Next => leptos_icons::Icon::Ai(AiStepForwardFilled),
        HistoryNavigation::Previous => leptos_icons::Icon::Ai(AiStepBackwardFilled),
    };
    let is_disabled = move || {
        let game_state_signal = expect_context::<GameStateSignal>();
        let game_state = game_state_signal.signal.get();
        match cloned_action {
            HistoryNavigation::Last | HistoryNavigation::MobileLast | HistoryNavigation::Next => {
                game_state.is_last_turn()
            }
            HistoryNavigation::Previous | HistoryNavigation::First => {
                game_state.history_turn == None || game_state.history_turn == Some(0)
            }
        }
    };
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        send_action(&action);
        if let Some(post_action) = post_action {
            post_action(())
        }
    });

    view! {
        <button class=nav_buttons_style prop:disabled=is_disabled on:click=debounced_action>

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
        HistoryNavigation::MobileLast => {
            if game_state_signal.signal.get_untracked().state.turn > 0 {
                game_state_signal
                    .signal
                    .update_untracked(|s| s.history_turn = Some(s.state.turn - 1));
            }
            game_state_signal.view_game()
        }
    }
}
