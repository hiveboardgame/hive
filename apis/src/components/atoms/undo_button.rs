use crate::providers::game_state::GameStateSignal;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn UndoButton() -> impl IntoView {
    let undo = move |_| {
        let mut game_state = expect_context::<GameStateSignal>();
        game_state.undo_move();
    };

    view! {
        <button
            class="aspect-square hover:bg-pillbug-teal rounded-sm transform transition-transform duration-300 active:scale-95 flex items-center justify-center"
            on:click=undo
        >
            <Icon icon=icondata::BiUndoRegular class="h-6 w-6 lg:h-8 lg:w-8"/>
        </button>
    }
}
