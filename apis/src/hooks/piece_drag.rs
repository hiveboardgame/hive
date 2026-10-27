use crate::hiveground::HivegroundInteraction;
use hive_lib::Position;
use leptos::ev::{pointercancel, pointerdown, pointermove, pointerup};
use leptos_use::{
    use_event_listener,
    use_event_listener_with_options,
    use_window,
    UseEventListenerOptions,
};
use web_sys::PointerEvent;

// Listens on the window, not the board, so a piece can be dragged out of the reserve's SVG.
pub fn use_piece_drag(
    interaction: HivegroundInteraction,
    drop_position: impl Fn(&PointerEvent) -> Option<Position> + 'static,
) {
    let window = use_window();
    _ = use_event_listener_with_options(
        window.clone(),
        pointerdown,
        move |_| interaction.pointer_down(),
        UseEventListenerOptions::default().capture(true),
    );
    _ = use_event_listener(window.clone(), pointermove, move |evt| {
        interaction.drag_to(evt.pointer_id(), (evt.client_x(), evt.client_y()));
    });
    _ = use_event_listener(window.clone(), pointerup, move |evt| {
        let drop = if interaction.is_dragging_untracked() {
            drop_position(&evt)
        } else {
            None
        };
        interaction.release_drag(evt.pointer_id(), drop);
    });
    _ = use_event_listener(window, pointercancel, move |evt| {
        interaction.cancel_drag_of(evt.pointer_id());
    });
}
