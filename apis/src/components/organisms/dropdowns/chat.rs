use crate::{
    components::{molecules::hamburger::Hamburger, organisms::chat::ChatWindow},
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, SimpleDestination};

#[component]
pub fn ChatDropdown(destination: SimpleDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let chat_style = "absolute z-50 flex flex-col overflow-hidden w-full h-[80dvh] max-w-screen bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md left-0 p-2";
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let unread = move || {
        let _ = chat.unread_counts.get();
        chat.unread_count_for_game(&game_id())
    };
    let button_color = move || {
        let _ = chat.unread_counts.get();
        if hamburger_show() {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        } else if chat.has_messages(game_id()) {
            "bg-ladybug-red"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    };

    Effect::new(move |_| {
        hamburger_show();
        chat.seen_messages(game_id());
    });

    // For game chat on mobile: show Players | Spectators toggle when user is a player (same as desktop side_board).
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let destination_stored = StoredValue::new(destination);
    let show_players_spectators_toggle = Signal::derive(move || {
        matches!(destination_stored.get_value(), SimpleDestination::Game)
            && auth_context
                .user
                .with_untracked(|u| u.as_ref().is_some_and(|user| {
                    game_state.signal.with(|gs| {
                        gs.white_id == Some(user.user.uid) || gs.black_id == Some(user.user.uid)
                    })
                }))
    });
    let game_finished =
        create_read_slice(game_state.signal, |gs| gs.game_response.as_ref().map_or(false, |gr| gr.finished));
    let game_show_players = RwSignal::new(true);

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=Signal::derive(move || {
                format!(
                    "{} transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 m-1 px-4 rounded flex items-center gap-1",
                    button_color(),
                )
            })
            extend_tw_classes="static"
            dropdown_style=chat_style
            content=view! {
                <Icon icon=icondata_bi::BiChatRegular attr:class="size-4" />
                {move || (unread() > 0).then(|| view! {
                    <span class="h-5 min-w-5 flex items-center justify-center px-1 text-xs font-bold leading-none text-white bg-black/30 dark:bg-white/30 rounded-full">
                        {if unread() > 99 { "99+".to_string() } else { unread().to_string() }}
                    </span>
                })}
            }
            id="chat"
        >
            {move || {
                if show_players_spectators_toggle() {
                    let finished = game_finished();
                    view! {
                        <div class="flex flex-col flex-1 min-h-0 overflow-hidden">
                            <div class="shrink-0 flex border-b border-gray-300 dark:border-gray-600 p-1 gap-0.5 rounded-t bg-gray-100 dark:bg-gray-800/50 mb-1">
                                <button
                                    type="button"
                                    class=move || format!(
                                        "flex-1 px-3 py-2 text-sm font-medium rounded border border-transparent transition-colors {}",
                                        if game_show_players.get() {
                                            "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                                        } else {
                                            "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                                        }
                                    )
                                    on:click=move |_| game_show_players.set(true)
                                >
                                    "Players"
                                </button>
                                <button
                                    type="button"
                                    disabled=move || !game_finished()
                                    class=move || format!(
                                        "flex-1 px-3 py-2 text-sm font-medium rounded border border-transparent transition-colors {}",
                                        if !game_show_players.get() {
                                            "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                                        } else if finished {
                                            "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                                        } else {
                                            "bg-transparent text-gray-500 dark:text-gray-500 cursor-not-allowed"
                                        }
                                    )
                                    on:click=move |_| game_show_players.set(false)
                                >
                                    "Spectators"
                                </button>
                            </div>
                            <div class="flex-1 min-h-0 overflow-hidden flex flex-col">
                                <ChatWindow
                                    destination=SimpleDestination::Game
                                    game_channel_override=Signal::derive(move || game_show_players.get())
                                />
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    view! { <ChatWindow destination=destination_stored.get_value() /> }.into_any()
                }
            }}
        </Hamburger>
    }
}
