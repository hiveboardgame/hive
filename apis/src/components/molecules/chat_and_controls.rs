use crate::{
    components::{
        atoms::toggle_controls::ToggleControls, layouts::base_layout::OrientationSignal,
        organisms::dropdowns::chat::ChatDropdown,
    },
    providers::game_state::GameStateSignal,
};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use shared_types::SimpleDestination;

#[component]
pub fn ChatAndControls() -> impl IntoView {
    let location = use_location();
    let gamestate = expect_context::<GameStateSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let is_finished = gamestate.is_finished();
    let in_mobile_game = move || {
        orientation_signal.orientation_vertical.get()
            && location.pathname.with(|l| l.starts_with("/game/"))
    };
    view! {
        <Show when=in_mobile_game>
            <Show when=move || !is_finished()>
                <ToggleControls />
            </Show>
            <ChatDropdown destination=SimpleDestination::Game />
        </Show>
    }
}
