use crate::{
    components::{
        atoms::unread_badge::{UnreadBadge, UnreadBadgeVariant},
        molecules::{
            game_thread_toggle::{GameThreadToggle, GameThreadToggleSize},
            hamburger::Hamburger,
        },
        organisms::chat::GameChatWindow,
    },
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{GameChatCapabilities, GameThread};

#[component]
pub fn ChatDropdown() -> impl IntoView {
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let chat_style = "absolute z-50 flex flex-col overflow-hidden w-full h-[80dvh] max-w-screen bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md left-0 p-2";
    let game_state = expect_context::<GameStateSignal>();
    let current_game_id = Signal::derive(move || game_state.signal.with(|state| state.game_id.clone()));
    let unread = Memo::new(move |_| {
        current_game_id
            .get()
            .as_ref()
            .map(|game_id| chat.unread_count_for_game(game_id))
            .unwrap_or(0)
    });
    let button_color = Memo::new(move |_| {
        if hamburger_show.get() {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        } else if unread.get() > 0 {
            "bg-ladybug-red"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    });

    Effect::new(move |_| {
        if hamburger_show() {
            if let Some(game_id) = current_game_id.get() {
                chat.seen_messages(game_id);
            }
        }
    });

    let auth_context = expect_context::<AuthContext>();
    let game_chat_access = Signal::derive(move || {
        let is_player = auth_context.user.with(|u| {
            u.as_ref().is_some_and(|user| {
                game_state.signal.with(|gs| {
                    gs.white_id == Some(user.user.uid) || gs.black_id == Some(user.user.uid)
                })
            })
        });
        let finished = game_state
            .signal
            .with(|gs| gs.game_response.as_ref().is_some_and(|gr| gr.finished));
        GameChatCapabilities::new(is_player, finished)
    });
    let selected_game_thread = RwSignal::new(GameThread::Players);
    let explicit_game_thread = Signal::derive(move || {
        game_chat_access
            .get()
            .can_toggle_embedded_threads()
            .then_some(selected_game_thread.get())
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=Signal::derive(move || {
                format!(
                    "{} transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 m-1 px-4 rounded flex items-center gap-1",
                    button_color.get(),
                )
            })
            extend_tw_classes="static"
            dropdown_style=chat_style
            content=view! {
                <Icon icon=icondata_bi::BiChatRegular attr:class="size-4" />
                <UnreadBadge count=Signal::derive(move || unread.get()) variant=UnreadBadgeVariant::Overlay />
            }
            id="chat"
        >
            <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                <Show when=move || game_chat_access.get().can_toggle_embedded_threads()>
                    <GameThreadToggle
                        selected=selected_game_thread
                        spectators_enabled=Signal::derive(move || {
                            game_chat_access.get().can_read(GameThread::Spectators)
                        })
                        size=GameThreadToggleSize::Roomy
                    />
                </Show>
                <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                    <GameChatWindow explicit_thread=explicit_game_thread />
                </div>
            </div>
        </Hamburger>
    }
}
