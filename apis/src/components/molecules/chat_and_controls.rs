use crate::{
    components::{
        atoms::toggle_controls::ToggleControls, layouts::base_layout::OrientationSignal,
        organisms::dropdowns::ChatDropdown,
    },
    providers::{game_state::GameStateSignal, navigation_controller::NavigationControllerSignal},
};
use leptos::*;
use shared_types::SimpleDestination;

#[component]
pub fn ChatAndControls() -> impl IntoView {
    let gamestate = expect_context::<GameStateSignal>();
    let navi = expect_context::<NavigationControllerSignal>();
    let orientation_signal = expect_context::<OrientationSignal>();
    let is_finished = gamestate.is_finished();
    let in_mobile_game = move || {
        orientation_signal.orientation_vertical.get() && navi.signal.get().game_id.is_some()
    };
    view! {
        <Show when=in_mobile_game>
            <Show when=move || !is_finished()>
                <ToggleControls/>
            </Show>
            <ChatDropdown destination=SimpleDestination::Game/>
        </Show>
    }
}
