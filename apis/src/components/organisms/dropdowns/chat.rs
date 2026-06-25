use crate::{
    components::{
        layouts::base_layout::OrientationSignal,
        molecules::hamburger::Hamburger,
        organisms::chat::ChatWindow,
    },
    providers::chat::Chat,
};
use leptos::{
    leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle},
    prelude::*,
};
use leptos_icons::*;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, SimpleDestination};
use std::time::Duration;

const HEIGHT_LOCK_SETTLE: Duration = Duration::from_millis(450);

#[component]
pub fn ChatDropdown(destination: SimpleDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let orientation = expect_context::<OrientationSignal>();
    let vertical = orientation.orientation_vertical;
    let height_lock = orientation.height_lock;
    let hamburger_show = RwSignal::new(false);
    let unlock_timer = StoredValue::new(None::<TimeoutHandle>);
    let chat_style = "absolute z-50 flex-col w-full h-[80dvh] max-w-screen left-0 p-2";
    let params = use_params_map();
    let game_id = Signal::derive(move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    });
    Effect::watch(
        move || hamburger_show.get(),
        move |show, _, _| {
            if *show {
                clear_unlock_timer(unlock_timer);
                if vertical.get_untracked() {
                    height_lock.set(viewport_height());
                }
            } else {
                clear_unlock_timer(unlock_timer);
                if let Ok(timer) = set_timeout_with_handle(
                    move || {
                        if !hamburger_show.get_untracked() {
                            height_lock.set(None);
                        }
                        unlock_timer.set_value(None);
                    },
                    HEIGHT_LOCK_SETTLE,
                ) {
                    unlock_timer.set_value(Some(timer));
                }
            }
        },
        false,
    );
    Effect::watch(
        move || (hamburger_show.get(), game_id.get()),
        move |(show, game_id), _, _| {
            if *show {
                chat.seen_messages(game_id.clone());
            }
        },
        false,
    );
    on_cleanup(move || {
        clear_unlock_timer(unlock_timer);
        height_lock.set(None);
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=Signal::derive(move || {
                if chat.has_messages(game_id.get()) {
                    "ui-header-icon-button ui-header-action-alert".to_string()
                } else {
                    "ui-header-icon-button".to_string()
                }
            })
            extend_tw_classes="static h-full"
            dropdown_style=chat_style
            content=view! { <Icon icon=icondata_bi::BiChatRegular attr:class="size-4" /> }
            id="chat"
            aria_label="Open chat"
        >
            <ChatWindow destination=destination.clone() />
        </Hamburger>
    }
}

fn viewport_height() -> Option<f64> {
    web_sys::window()?.inner_height().ok()?.as_f64()
}

fn clear_unlock_timer(timer: StoredValue<Option<TimeoutHandle>>) {
    timer.update_value(|timer| {
        if let Some(timer) = timer.take() {
            timer.clear();
        }
    });
}
