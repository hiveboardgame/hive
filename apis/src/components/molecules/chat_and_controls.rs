use crate::{
    components::{
        atoms::toggle_controls::ToggleControls, layouts::base_layout::OrientationSignal,
        organisms::dropdowns::chat::ChatDropdown,
    },
    providers::game_state::GameStateSignal,
};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, SimpleDestination};

#[component]
pub fn ChatAndControls() -> impl IntoView {
    let params = use_params_map();
    let game_id = move || params.get().get("nanoid").map(|s| GameId(s.to_owned()));
    let gamestate = expect_context::<GameStateSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let is_finished = gamestate.is_finished();
    let in_mobile_game =
        move || orientation_signal.orientation_vertical.get() && game_id().is_some();
    view! {
        <Show when=in_mobile_game>
            <Show when=move || !is_finished()>
                <ToggleControls />
            </Show>
            <ChatDropdown destination=SimpleDestination::Game />
        </Show>
    }
}
