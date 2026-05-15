use crate::{
    components::{
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
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameThread;

#[component]
pub fn ChatDropdown() -> impl IntoView {
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let chat_style = "absolute z-50 flex flex-col overflow-hidden w-full h-[80dvh] max-w-screen bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md left-0 p-2";
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
    let button_color = Memo::new(move |_| {
        if hamburger_show.get() {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        } else if unread.get() > 0 {
            "bg-ladybug-red"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    });

    let game_chat = use_embedded_game_chat_state();

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
            }
            id="chat"
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
