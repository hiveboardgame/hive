use crate::{
    chat::SimpleDestination,
    components::{molecules::hamburger::Hamburger, organisms::chat::ChatWindow},
    i18n::*,
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_params_map;
use shared_types::GameId;

#[component]
pub fn ChatDropdown(destination: SimpleDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let hamburger_show = RwSignal::new(false);
    let chat_style = "absolute z-50 flex flex-col overflow-hidden w-full h-[80dvh] max-w-screen bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md left-0 p-2";
    let destination_for_toggle = destination.clone();
    let destination_for_fallback = StoredValue::new(destination);
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let unread = Memo::new(move |_| chat.unread_count_for_game(&game_id()));
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
            chat.seen_messages(game_id());
        }
    });

    // For game chat on mobile: show Players | Spectators toggle when user is a player (same as desktop side_board).
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let show_players_spectators_toggle = Memo::new(move |_| {
        matches!(&destination_for_toggle, SimpleDestination::Game)
            && auth_context.user.with(|u| {
                u.as_ref().is_some_and(|user| {
                    game_state.signal.with(|gs| {
                        gs.white_id == Some(user.user.uid) || gs.black_id == Some(user.user.uid)
                    })
                })
            })
    });
    let game_finished = create_read_slice(game_state.signal, |gs| {
        gs.game_response.as_ref().is_some_and(|gr| gr.finished)
    });
    let game_show_players = RwSignal::new(true);

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
                <Show when=move || unread.get().gt(&0)>
                    <span class="flex justify-center items-center px-1 h-5 text-xs font-bold leading-none text-white rounded-full min-w-5 bg-black/30 dark:bg-white/30">
                        {move || {
                            let count = unread.get();
                            if count > 99 { "99+".to_string() } else { count.to_string() }
                        }}
                    </span>
                </Show>
            }
            id="chat"
        >
            <Show
                when=move || show_players_spectators_toggle.get()
                fallback=move || {
                    view! { <ChatWindow destination=destination_for_fallback.get_value() /> }
                }
            >
                <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                    <div class="flex gap-0.5 p-1 mb-1 bg-gray-100 rounded-t border-b border-gray-300 dark:border-gray-600 shrink-0 dark:bg-gray-800/50">
                        <button
                            type="button"
                            class=move || {
                                format!(
                                    "flex-1 px-3 py-2 text-sm font-medium rounded border border-transparent transition-colors {}",
                                    if game_show_players.get() {
                                        "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                                    } else {
                                        "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                                    },
                                )
                            }
                            on:click=move |_| game_show_players.set(true)
                        >
                            {t!(i18n, messages.chat.players)}
                        </button>
                        <button
                            type="button"
                            disabled=move || !game_finished()
                            class=move || {
                                format!(
                                    "flex-1 px-3 py-2 text-sm font-medium rounded border border-transparent transition-colors {}",
                                    if !game_show_players.get() {
                                        "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                                    } else if game_finished() {
                                        "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                                    } else {
                                        "bg-transparent text-gray-500 dark:text-gray-500 cursor-not-allowed"
                                    },
                                )
                            }
                            on:click=move |_| game_show_players.set(false)
                        >
                            {t!(i18n, messages.chat.spectators)}
                        </button>
                    </div>
                    <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                        <ChatWindow
                            destination=SimpleDestination::Game
                            game_channel_override=Signal::from(game_show_players)
                        />
                    </div>
                </div>
            </Show>
        </Hamburger>
    }
}
