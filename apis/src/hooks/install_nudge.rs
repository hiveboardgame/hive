use crate::pwa;
use leptos::{ev::Custom, prelude::*};
use leptos_use::{use_event_listener, use_window};

pub fn use_install_nudge_active() -> RwSignal<bool> {
    let active = RwSignal::new(false);

    Effect::new(move |_| {
        active.set(pwa::install_nudge_should_show());
    });
    let _ = use_event_listener(
        use_window(),
        Custom::<web_sys::Event>::new("hive-installable"),
        move |_| {
            active.set(pwa::install_nudge_should_show());
        },
    );

    active
}
