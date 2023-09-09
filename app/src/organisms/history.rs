use crate::common::game_state::GameState;
use leptos::*;

#[component]
pub fn History(cx: Scope) -> impl IntoView {
    let game_state =
        use_context::<RwSignal<GameState>>(cx).expect("there to be a `GameState` signal provided");
    let history = move || game_state().state.get().history.to_string();
    view! { cx,
        <p style="position:absolute; max-width:80%; left:10%">
            History:
            {history}
        </p>
    }
}
