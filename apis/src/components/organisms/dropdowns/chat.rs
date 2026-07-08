use crate::{
    components::{
        layouts::base_layout::OrientationSignal,
        molecules::{
            game_thread_toggle::{
                use_embedded_game_chat_state,
                GameThreadToggle,
                GameThreadToggleSize,
            },
            hamburger::Hamburger,
        },
        organisms::chat::GameChatWindow,
    },
    providers::{chat::Chat, game_state::GameStateSignal},
};
use leptos::{
    leptos_dom::helpers::{set_timeout_with_handle, TimeoutHandle},
    prelude::*,
};
use leptos_icons::*;
use shared_types::GameThread;
use std::time::Duration;

const HEIGHT_LOCK_SETTLE: Duration = Duration::from_millis(450);

#[component]
pub fn ChatDropdown() -> impl IntoView {
    let chat = expect_context::<Chat>();
    let orientation = expect_context::<OrientationSignal>();
    let vertical = orientation.orientation_vertical;
    let height_lock = orientation.height_lock;
    let hamburger_show = RwSignal::new(false);
    let unlock_timer = StoredValue::new(None::<TimeoutHandle>);
    let game_state = expect_context::<GameStateSignal>();
    let current_game_id =
        Signal::derive(move || game_state.signal.with(|state| state.game_id.clone()));
    let unread = Memo::new(move |_| {
        current_game_id
            .get()
            .as_ref()
            .map(|game_id| chat.unread_count_for_game(game_id))
            .unwrap_or(0)
    });
    let chat_style =
        "absolute z-50 flex flex-col overflow-hidden w-full h-[80dvh] max-w-screen left-0 p-2";
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
    on_cleanup(move || {
        clear_unlock_timer(unlock_timer);
        height_lock.set(None);
    });

    let game_chat = use_embedded_game_chat_state();

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=Signal::derive(move || {
                if unread.get() > 0 {
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
            <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                <Show when=move || game_chat.access.get().can_toggle_embedded_threads()>
                    <GameThreadToggle
                        selected=game_chat.selected_thread
                        spectators_enabled=Signal::derive(move || {
                            game_chat.access.get().can_read(GameThread::Spectators)
                        })
                        size=GameThreadToggleSize::Roomy
                    />
                </Show>
                <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                    <GameChatWindow explicit_thread=game_chat.explicit_thread />
                </div>
            </div>
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
