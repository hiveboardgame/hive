use crate::providers::websocket::{ConnectionReadyState, WebsocketContext};
use leptos::prelude::*;

/// Resource consumers can skip the mount callback because they fetch on creation.
pub fn use_websocket_refresh(refresh: impl Fn() + 'static, run_on_mount: bool) {
    let websocket = expect_context::<WebsocketContext>();
    Effect::watch(
        move || {
            (
                websocket.ready_state.get(),
                websocket.wake_resync_epoch.get(),
            )
        },
        move |(ready, wake), previous, _| {
            if *ready == ConnectionReadyState::Open
                && previous.is_none_or(|(previous_ready, previous_wake)| {
                    *previous_ready != ConnectionReadyState::Open || previous_wake != wake
                })
            {
                refresh();
            }
        },
        run_on_mount,
    );
}
