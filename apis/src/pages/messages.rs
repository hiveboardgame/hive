//! Messages hub: /message — DMs, Tournaments, Games, and recent announcements.
//! Canonical routes under /message are the source of truth for the open thread.

use crate::{
    chat::ChannelKey,
    components::{
        atoms::block_toggle_button::BlockToggleButton,
        organisms::chat::{GameChatThread, ResolvedChatWindow},
    },
    functions::{
        blocks_mutes::{mute_tournament_chat, unmute_tournament_chat},
        chat::{
            get_game_chat_route_data,
            get_messages_hub_data,
            get_tournament_route_data,
            DmConversation,
            GameChannel,
            MessagesHubData,
            MyConversations,
            TournamentChannel,
        },
        users::resolve_username,
    },
    i18n::*,
    providers::{chat::Chat, AuthContext},
};
use leptos::{logging::log, prelude::*, task::spawn_local};
use leptos_router::{
    components::{Outlet, A},
    hooks::{use_location, use_params_map},
};
use shared_types::{ChannelType, ChatDestination, GameId, TournamentId};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
enum MessagesThreadKey {
    Root,
    Global,
    Dm(String),
    Tournament(String),
    Game {
        channel_id: String,
        channel_type: ChannelType,
    },
}

fn messages_root_href() -> &'static str {
    "/message"
}

fn messages_global_href() -> &'static str {
    "/message/global"
}

fn messages_dm_href(username: &str) -> String {
    format!("/message/dm/{username}")
}

fn messages_tournament_href(nanoid: &str) -> String {
    format!("/message/tournament/{nanoid}")
}

fn game_thread_slug(thread: GameChatThread) -> &'static str {
    match thread {
        GameChatThread::Players => "players",
        GameChatThread::Spectators => "spectators",
    }
}

fn game_thread_from_channel_type(channel_type: ChannelType) -> Option<GameChatThread> {
    match channel_type {
        ChannelType::GamePlayers => Some(GameChatThread::Players),
        ChannelType::GameSpectators => Some(GameChatThread::Spectators),
        _ => None,
    }
}

fn messages_game_href(nanoid: &str, channel_type: ChannelType) -> Option<String> {
    game_thread_from_channel_type(channel_type)
        .map(|thread| format!("/message/game/{nanoid}/{}", game_thread_slug(thread)))
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_messages_thread_key(path: &str) -> MessagesThreadKey {
    let normalized = normalize_path(path);
    if normalized == messages_root_href() {
        return MessagesThreadKey::Root;
    }
    if normalized == messages_global_href() {
        return MessagesThreadKey::Global;
    }
    if let Some(username) = normalized.strip_prefix("/message/dm/") {
        return MessagesThreadKey::Dm(username.to_string());
    }
    if let Some(nanoid) = normalized.strip_prefix("/message/tournament/") {
        return MessagesThreadKey::Tournament(nanoid.to_string());
    }
    if let Some(rest) = normalized.strip_prefix("/message/game/") {
        let mut parts = rest.split('/');
        if let (Some(channel_id), Some(thread), None) = (parts.next(), parts.next(), parts.next()) {
            let channel_type = match thread {
                "players" => Some(ChannelType::GamePlayers),
                "spectators" => Some(ChannelType::GameSpectators),
                _ => None,
            };
            if let Some(channel_type) = channel_type {
                return MessagesThreadKey::Game {
                    channel_id: channel_id.to_string(),
                    channel_type,
                };
            }
        }
    }
    MessagesThreadKey::Root
}

fn tournament_route_can_send(is_participant: bool, is_admin: bool) -> bool {
    is_participant || is_admin
}

fn game_channel_matches_route(
    channel_id: &str,
    channel_type: ChannelType,
    finished: bool,
    current_route: &MessagesThreadKey,
) -> bool {
    match current_route {
        MessagesThreadKey::Game {
            channel_id: current_channel_id,
            channel_type: current_channel_type,
        } => {
            current_channel_id == channel_id && (finished || *current_channel_type == channel_type)
        }
        _ => false,
    }
}

fn refresh_open_dm_thread(chat: Chat, current_user_id: Option<Uuid>, other_id: Uuid) {
    let Some(current_user_id) = current_user_id else {
        return;
    };
    let key = ChannelKey::direct(current_user_id, other_id);
    spawn_local(async move {
        match chat.fetch_channel_history(&key).await {
            Ok(messages) => chat.replace_history(&key, messages),
            Err(error) => log!("Failed to refresh DM history for {other_id}: {error}"),
        }
    });
}

#[component]
pub fn MessagesLayout() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let location = use_location();
    let hub_data = Resource::new(
        move || chat.conversation_list_version.get(),
        move |_| async move { get_messages_hub_data().await },
    );
    Effect::new(move |_| {
        hub_data.with(|result| {
            if let Some(Ok(data)) = result.as_ref() {
                chat.apply_server_unread_counts(data.unread_counts.clone());
            }
        });
    });
    let current_route = Signal::derive(move || parse_messages_thread_key(&location.pathname.get()));
    let mobile_list_visible =
        Signal::derive(move || matches!(current_route.get(), MessagesThreadKey::Root));
    let mobile_thread_visible =
        Signal::derive(move || !matches!(current_route.get(), MessagesThreadKey::Root));
    let current_user_id = auth.user.with_untracked(|a| a.as_ref().map(|a| a.user.uid));

    view! {
        <div class="flex overflow-hidden fixed right-0 bottom-0 left-0 top-12 z-0 flex-col bg-gray-100 sm:flex-row dark:bg-gray-950">
            <aside class=move || {
                format!(
                    "w-full sm:w-72 flex-shrink-0 flex flex-col min-h-0 overflow-hidden bg-white dark:bg-gray-900 shadow-lg sm:rounded-r-xl border-l border-gray-200 dark:border-gray-700 {} sm:!flex",
                    if mobile_list_visible.get() { "" } else { "hidden " },
                )
            }>
                <div class="py-3 px-4 bg-white border-b border-gray-200 dark:bg-gray-900 dark:border-gray-700">
                    <h1 class="text-xl font-bold tracking-tight text-gray-800 dark:text-gray-100">
                        {t!(i18n, messages.page.title)}
                    </h1>
                </div>
                <div class="overflow-y-auto flex-1 p-2 pb-6 min-h-0 sm:pb-2">
                    <MessagesSidebar hub_data current_route me=current_user_id />
                </div>
            </aside>
            <main class=move || {
                format!(
                    "flex-1 flex flex-col min-w-0 min-h-0 overflow-hidden {} sm:!flex",
                    if mobile_thread_visible.get() { "" } else { "hidden " },
                )
            }>
                <Outlet />
            </main>
        </div>
    }
}

#[component]
fn MessagesSidebar(
    hub_data: Resource<Result<MessagesHubData, ServerFnError>>,
    current_route: Signal<MessagesThreadKey>,
    me: Option<Uuid>,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();

    view! {
        <ShowLet
            some=move || hub_data.get()
            let:hub_result
            fallback=move || {
                view! {
                    <p class="p-3 text-sm text-gray-500 animate-pulse dark:text-gray-400">
                        {t!(i18n, messages.page.loading)}
                    </p>
                }
            }
        >
            <ShowLet
                some=move || hub_result.as_ref().ok().cloned()
                let:hub_page_data
                fallback=move || {
                    view! {
                        <p class="p-3 text-sm text-red-600 dark:text-red-400">
                            {t!(i18n, messages.page.failed_conversations)}
                        </p>
                    }
                }
            >
                <ChannelLists
                    conv=hub_page_data.conversations
                    me=me
                    current_route=current_route
                    chat=chat
                />
            </ShowLet>
        </ShowLet>
    }
}

#[component]
pub fn MessagesIndex() -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="hidden sm:flex overflow-hidden flex-col flex-1 min-h-0 bg-white border-r border-gray-200 shadow-inner sm:rounded-l-xl dark:bg-gray-900 dark:border-gray-700">
            <div class="flex flex-col flex-1 gap-2 justify-center items-center p-8 text-gray-500 dark:text-gray-400">
                <span class="text-4xl opacity-50">"💬"</span>
                <p class="font-medium text-center">
                    {t!(i18n, messages.page.select_conversation)}
                </p>
                <p class="max-w-xs text-sm text-center">
                    {t!(i18n, messages.page.choose_conversation)}
                </p>
            </div>
        </div>
    }
}

#[component]
pub fn MessagesGlobalThread() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let title =
        Signal::derive(move || t_string!(i18n, messages.sections.recent_announcements).to_string());
    let destination = ChatDestination::Global;
    let input_disabled = Signal::derive(move || {
        auth.user
            .with(|user| !user.as_ref().is_some_and(|account| account.user.admin))
    });

    view! {
        <MessagesThreadFrame title>
            <div class="overflow-hidden flex-1 min-h-0">
                <ResolvedChatWindow destination input_disabled />
            </div>
        </MessagesThreadFrame>
    }
}

#[component]
pub fn MessagesDmThread() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let params = use_params_map();
    let route_username = Signal::derive(move || {
        params
            .get()
            .get("username")
            .map(|username| username.to_string())
    });

    view! {
        <ShowLet some=move || route_username.get() let:route_username>
            <Transition fallback=move || {
                view! {
                    <MessagesThreadFrame title=Signal::derive(move || {
                        t_string!(i18n, messages.page.loading).to_string()
                    })>
                        <MessagesStatusContent message=Signal::derive(move || {
                            t_string!(i18n, messages.page.loading).to_string()
                        }) />
                    </MessagesThreadFrame>
                }
            }>
                {move || {
                    let route_username = StoredValue::new(route_username.clone());
                    Suspend::new(async move {
                        match resolve_username(route_username.get_value()).await.ok() {
                            Some(user) => {
                                let other_user_id = user.uid;
                                let username = StoredValue::new(user.username);
                                let current_user_id = Signal::derive(move || {
                                    auth.user.with(|account| account.as_ref().map(|account| account.user.uid))
                                });
                                let auth_pending = auth.action.pending();

                                view! {
                                    <MessagesThreadFrame title=Signal::derive(move || username.get_value())>
                                        <ShowLet
                                            some=move || current_user_id.get()
                                            let:current_user_id
                                            fallback=move || {
                                                view! {
                                                    <MessagesStatusContent message=Signal::derive(move || {
                                                        if auth_pending.get() {
                                                            t_string!(i18n, messages.page.loading).to_string()
                                                        } else {
                                                            t_string!(i18n, messages.page.failed_conversations)
                                                                .to_string()
                                                        }
                                                    }) />
                                                }
                                            }
                                        >
                                            {move || {
                                                if current_user_id != other_user_id {
                                                    let destination = ChatDestination::User((
                                                        other_user_id,
                                                        username.get_value(),
                                                    ));
                                                    view! {
                                                        <DmChannelActions
                                                            other_id=other_user_id
                                                            username=Signal::derive(move || Some(username.get_value()))
                                                        />
                                                        <div class="overflow-hidden flex-1 min-h-0">
                                                            <ResolvedChatWindow destination />
                                                        </div>
                                                    }
                                                        .into_any()
                                                } else {
                                                    let dm_error = StoredValue::new(
                                                        "Direct messages to yourself are not supported"
                                                            .to_string(),
                                                    );
                                                    view! {
                                                        <MessagesStatusContent message=Signal::derive(move || {
                                                            dm_error.get_value()
                                                        }) />
                                                    }
                                                        .into_any()
                                                }
                                            }}
                                        </ShowLet>
                                    </MessagesThreadFrame>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <MessagesThreadFrame title=Signal::derive(move || {
                                        t_string!(i18n, messages.page.failed_conversations).to_string()
                                    })>
                                        <MessagesStatusContent message=Signal::derive(move || {
                                            t_string!(i18n, messages.page.failed_conversations).to_string()
                                        }) />
                                    </MessagesThreadFrame>
                                }
                                    .into_any()
                            }
                        }
                    })
                }}
            </Transition>
        </ShowLet>
        <Show when=move || route_username.get().is_none()>
            <MessagesThreadFrame title=Signal::derive(move || {
                t_string!(i18n, messages.page.failed_conversations).to_string()
            })>
                <MessagesStatusContent message=Signal::derive(move || {
                    t_string!(i18n, messages.page.failed_conversations).to_string()
                }) />
            </MessagesThreadFrame>
        </Show>
    }
}

#[component]
pub fn MessagesTournamentThread() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let params = use_params_map();
    let route_nanoid =
        Signal::derive(move || params.get().get("nanoid").map(|nanoid| nanoid.to_string()));
    let user_is_admin = Signal::derive(move || {
        auth.user
            .with(|user| user.as_ref().is_some_and(|account| account.user.admin))
    });

    view! {
        <ShowLet some=move || route_nanoid.get() let:nanoid>
            <Transition fallback=move || {
                view! {
                    <MessagesThreadFrame title=Signal::derive(move || {
                        t_string!(i18n, messages.page.loading).to_string()
                    })>
                        <MessagesStatusContent message=Signal::derive(move || {
                            t_string!(i18n, messages.page.loading).to_string()
                        }) />
                    </MessagesThreadFrame>
                }
            }>
                {move || {
                    let nanoid = StoredValue::new(nanoid.clone());
                    Suspend::new(async move {
                        match get_tournament_route_data(nanoid.get_value()).await.ok() {
                            Some(route_data) => {
                                let title = StoredValue::new(route_data.name);
                                let is_participant = route_data.is_participant;
                                let muted = route_data.muted;
                                let can_send = Signal::derive(move || {
                                    tournament_route_can_send(is_participant, user_is_admin.get())
                                });
                                let destination = StoredValue::new(ChatDestination::TournamentLobby(
                                    TournamentId(nanoid.get_value()),
                                ));

                                view! {
                                    <MessagesThreadFrame title=Signal::derive(move || title.get_value())>
                                        <TournamentRouteActions
                                            nanoid=nanoid.get_value()
                                            route_ready=Signal::derive(move || true)
                                            can_send
                                            muted=Signal::derive(move || muted)
                                        />
                                        <div class="overflow-hidden flex-1 min-h-0">
                                            <ResolvedChatWindow
                                                destination=destination.get_value()
                                                input_disabled=Signal::derive(move || !can_send.get())
                                            />
                                        </div>
                                    </MessagesThreadFrame>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <MessagesThreadFrame title=Signal::derive(move || {
                                        t_string!(i18n, messages.page.failed_conversations).to_string()
                                    })>
                                        <MessagesStatusContent message=Signal::derive(move || {
                                            t_string!(i18n, messages.page.failed_conversations).to_string()
                                        }) />
                                    </MessagesThreadFrame>
                                }
                                    .into_any()
                            }
                        }
                    })
                }}
            </Transition>
        </ShowLet>
        <Show when=move || route_nanoid.get().is_none()>
            <MessagesThreadFrame title=Signal::derive(move || {
                t_string!(i18n, messages.page.failed_conversations).to_string()
            })>
                <MessagesStatusContent message=Signal::derive(move || {
                    t_string!(i18n, messages.page.failed_conversations).to_string()
                }) />
            </MessagesThreadFrame>
        </Show>
    }
}

#[component]
pub fn MessagesGamePlayersThread() -> impl IntoView {
    view! { <MessagesGameThread thread=GameChatThread::Players /> }
}

#[component]
pub fn MessagesGameSpectatorsThread() -> impl IntoView {
    view! { <MessagesGameThread thread=GameChatThread::Spectators /> }
}

#[component]
fn MessagesGameThread(thread: GameChatThread) -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params_map();
    let route_game_id = Signal::derive(move || {
        params
            .get()
            .get("nanoid")
            .map(|nanoid| GameId(nanoid.to_string()))
    });

    view! {
        <ShowLet some=move || route_game_id.get() let:game_id>
            <Transition fallback=move || {
                view! {
                    <MessagesThreadFrame title=Signal::derive(move || {
                        if thread == GameChatThread::Players {
                            t_string!(i18n, messages.chat.players_chat).to_string()
                        } else {
                            t_string!(i18n, messages.chat.spectator_chat).to_string()
                        }
                    })>
                        <MessagesStatusContent message=Signal::derive(move || {
                            t_string!(i18n, messages.page.loading).to_string()
                        }) />
                    </MessagesThreadFrame>
                }
            }>
                {move || {
                    let game_id = StoredValue::new(game_id.clone());
                    Suspend::new(async move {
                        match get_game_chat_route_data(game_id.get_value()).await.ok() {
                            Some(route_data) => {
                                let title = StoredValue::new(if thread == GameChatThread::Players {
                                    t_string!(i18n, messages.chat.players_chat).to_string()
                                } else {
                                    t_string!(i18n, messages.chat.spectator_chat).to_string()
                                });
                                let denied_message = StoredValue::new(match thread {
                                    GameChatThread::Players if !route_data.is_player => {
                                        Some("Only players can view the players chat.".to_string())
                                    }
                                    GameChatThread::Spectators
                                        if route_data.is_player && !route_data.finished =>
                                    {
                                            Some(t_string!(i18n, messages.chat.spectator_unlock).to_string())
                                    }
                                    _ => None,
                                });
                                let game_id_string =
                                    StoredValue::new(game_id.with_value(|game_id| game_id.0.clone()));
                                let is_player = route_data.is_player;
                                let finished = route_data.finished;
                                let can_view_thread =
                                    denied_message.with_value(|message| message.is_none());
                                let chat_destination = StoredValue::new(match thread {
                                    GameChatThread::Players => {
                                        ChatDestination::GamePlayers(game_id.get_value())
                                    }
                                    GameChatThread::Spectators => {
                                        ChatDestination::GameSpectators(game_id.get_value())
                                    }
                                });

                                view! {
                                    <MessagesThreadFrame title=Signal::derive(move || title.get_value())>
                                        <GameChatHeader
                                            current_thread=thread
                                            game_id=Signal::derive(move || game_id_string.get_value())
                                            is_player=Signal::derive(move || is_player)
                                            finished=Signal::derive(move || finished)
                                        />
                                        <Show
                                            when=move || can_view_thread
                                            fallback=move || {
                                                view! {
                                                    <MessagesStatusContent message=Signal::derive(move || {
                                                        denied_message.get_value().unwrap_or_else(|| {
                                                            t_string!(i18n, messages.page.failed_game).to_string()
                                                        })
                                                    }) />
                                                }
                                            }
                                        >
                                            <div class="overflow-hidden flex-1 min-h-0">
                                                <ResolvedChatWindow destination=chat_destination.get_value() />
                                            </div>
                                        </Show>
                                    </MessagesThreadFrame>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <MessagesThreadFrame title=Signal::derive(move || {
                                        t_string!(i18n, messages.page.failed_game).to_string()
                                    })>
                                        <MessagesStatusContent message=Signal::derive(move || {
                                            t_string!(i18n, messages.page.failed_game).to_string()
                                        }) />
                                    </MessagesThreadFrame>
                                }
                                    .into_any()
                            }
                        }
                    })
                }}
            </Transition>
        </ShowLet>
        <Show when=move || route_game_id.get().is_none()>
            <MessagesThreadFrame title=Signal::derive(move || {
                t_string!(i18n, messages.page.failed_game).to_string()
            })>
                <MessagesStatusContent message=Signal::derive(move || {
                    t_string!(i18n, messages.page.failed_game).to_string()
                }) />
            </MessagesThreadFrame>
        </Show>
    }
}

#[component]
fn MessagesThreadFrame(title: Signal<String>, children: Children) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex overflow-hidden flex-col flex-1 min-h-0 bg-white border-r border-gray-200 shadow-inner sm:rounded-l-xl dark:bg-gray-900 dark:border-gray-700">
            <div class="flex gap-2 items-center py-3 px-2 bg-gray-50 border-b border-gray-200 sm:px-4 dark:border-gray-700 shrink-0 min-h-[2.75rem] dark:bg-gray-800/50">
                <A
                    href=messages_root_href()
                    prop:replace=true
                    scroll=false
                    attr:class="no-link-style flex flex-shrink-0 gap-1 justify-center items-center -ml-1 text-gray-600 rounded-lg transition-colors sm:hidden dark:text-gray-400 hover:text-gray-900 hover:bg-gray-200 min-h-[2.25rem] min-w-[2.25rem] dark:hover:bg-gray-700 dark:hover:text-gray-100"
                    attr:aria-label=move || {
                        t_string!(i18n, messages.page.back_to_conversations)
                    }
                >
                    <span class="text-lg" aria-hidden="true">
                        "←"
                    </span>
                    <span class="text-sm font-medium">
                        {t!(i18n, messages.page.conversations)}
                    </span>
                </A>
                <h2 class="flex-1 min-w-0 text-lg font-semibold text-gray-800 dark:text-gray-100 truncate">
                    {move || title.get()}
                </h2>
            </div>
            {children()}
        </div>
    }
}

#[component]
fn MessagesStatusContent(message: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-col flex-1 gap-2 justify-center items-center p-8 text-gray-500 dark:text-gray-400">
            <p class="max-w-xs text-sm font-medium text-center">
                {move || message.get()}
            </p>
        </div>
    }
}

#[component]
fn ChannelHeaderBar(children: Children) -> impl IntoView {
    view! {
        <div class="py-2 px-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50/80 shrink-0 dark:bg-gray-800/30">
            {children()}
        </div>
    }
}

#[component]
fn DmChannelActions(other_id: Uuid, username: Signal<Option<String>>) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let is_blocked =
        Signal::derive(move || chat.blocked_user_ids.with(|ids| ids.contains(&other_id)));
    let on_block_toggle_success = Callback::new(move |_| {
        refresh_open_dm_thread(
            chat,
            auth.user.with_untracked(|u| u.as_ref().map(|u| u.user.uid)),
            other_id,
        );
    });

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-wrap gap-2 items-center">
                <ShowLet some=move || username.get() let:username>
                    <A
                        href=format!("/@/{}", username)
                        attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                    >
                        {t!(i18n, messages.page.view_profile)}
                    </A>
                </ShowLet>
                <BlockToggleButton
                    blocked_user_id=other_id
                    is_blocked
                    on_success=on_block_toggle_success
                />
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn TournamentRouteActions(
    nanoid: String,
    route_ready: Signal<bool>,
    can_send: Signal<bool>,
    muted: Signal<bool>,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let nanoid = StoredValue::new(nanoid);
    let tournament_href = StoredValue::new(format!("/tournament/{}", nanoid.get_value()));
    let muted_override = RwSignal::new(None::<bool>);
    let mute_error = RwSignal::new(None::<String>);
    let resolved_muted =
        Signal::derive(move || muted_override.get().unwrap_or_else(|| muted.get()));
    let toggle_mute = Action::new(move |currently_muted: &bool| {
        let currently_muted = *currently_muted;
        async move {
            if currently_muted {
                unmute_tournament_chat(nanoid.get_value())
                    .await
                    .map(|_| false)
                    .map_err(|error| error.to_string())
            } else {
                mute_tournament_chat(nanoid.get_value())
                    .await
                    .map(|_| true)
                    .map_err(|error| error.to_string())
            }
        }
    });
    Effect::watch(
        toggle_mute.version(),
        move |_, _, _| {
            let Some(result) = toggle_mute.value().get_untracked() else {
                return;
            };
            match result {
                Ok(new_muted) => {
                    mute_error.set(None);
                    muted_override.set(Some(new_muted));
                    chat.invalidate_conversation_list();
                    chat.refresh_unread_counts();
                }
                Err(error) => mute_error.set(Some(error)),
            }
        },
        false,
    );

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-col gap-1">
                <div class="flex flex-wrap gap-2 items-center">
                    <A
                        href=tournament_href.get_value()
                        attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                    >
                        {t!(i18n, messages.page.view_tournament)}
                    </A>
                    <button
                        type="button"
                        disabled=move || toggle_mute.pending().get() || !route_ready.get()
                        class="text-sm font-medium text-gray-600 transition-colors dark:text-gray-400 disabled:text-gray-400 dark:hover:text-pillbug-teal/90 dark:disabled:text-gray-500 hover:text-pillbug-teal"
                        on:click=move |_| {
                            mute_error.set(None);
                            toggle_mute.dispatch(resolved_muted.get_untracked());
                        }
                    >
                        {move || {
                            if toggle_mute.pending().get() || !route_ready.get() {
                                t_string!(i18n, messages.page.loading)
                            } else if resolved_muted.get() {
                                t_string!(i18n, messages.page.unmute_tournament_chat)
                            } else {
                                t_string!(i18n, messages.page.mute_tournament_chat)
                            }
                        }}
                    </button>
                </div>
                <Show when=move || route_ready.get() && !can_send.get()>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                        {t!(i18n, messages.chat.tournament_read_restricted)}
                    </p>
                </Show>
                <Show when=move || mute_error.get().is_some()>
                    <p class="text-xs text-red-600 dark:text-red-400">
                        {move || mute_error.get().unwrap_or_default()}
                    </p>
                </Show>
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn GameChatToggle(game_id: String, current_thread: GameChatThread) -> impl IntoView {
    let i18n = use_i18n();
    let players_href = messages_game_href(&game_id, ChannelType::GamePlayers)
        .unwrap_or_else(|| messages_root_href().to_string());
    let spectators_href = messages_game_href(&game_id, ChannelType::GameSpectators)
        .unwrap_or_else(|| messages_root_href().to_string());
    let viewing_players = current_thread == GameChatThread::Players;

    view! {
        <div class="flex p-0.5 bg-gray-100 rounded-lg border border-gray-300 dark:bg-gray-800 dark:border-gray-600">
            <A
                href=players_href
                prop:replace=true
                scroll=false
                attr:class=move || {
                    format!(
                        "no-link-style flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors text-center {}",
                        if viewing_players {
                            "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                        } else {
                            "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
                        },
                    )
                }
            >
                {t!(i18n, messages.chat.players)}
            </A>
            <A
                href=spectators_href
                prop:replace=true
                scroll=false
                attr:class=move || {
                    format!(
                        "no-link-style flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors text-center {}",
                        if !viewing_players {
                            "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
                        } else {
                            "text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100"
                        },
                    )
                }
            >
                {t!(i18n, messages.chat.spectators)}
            </A>
        </div>
    }
}

#[component]
fn GameChatHeader(
    current_thread: GameChatThread,
    game_id: Signal<String>,
    is_player: Signal<bool>,
    finished: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let static_chat_label = Signal::derive(move || {
        if current_thread == GameChatThread::Players {
            t_string!(i18n, messages.chat.players_chat)
        } else {
            t_string!(i18n, messages.chat.spectator_chat)
        }
    });

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-col gap-2">
                <div class="flex flex-wrap gap-2 items-center">
                    <A
                        href=move || format!("/game/{}", game_id.get())
                        attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                    >
                        {t!(i18n, messages.page.view_game)}
                    </A>
                </div>
                <Show
                    when=move || is_player.get() && finished.get()
                    fallback=move || {
                        view! {
                            <div class="flex flex-col gap-1">
                                <span class="inline-flex items-center py-1 px-2.5 text-xs font-medium text-gray-700 bg-white rounded-full border border-gray-300 dark:text-gray-200 dark:bg-gray-800 dark:border-gray-600 w-fit">
                                    {move || static_chat_label.get()}
                                </span>
                                <Show when=move || is_player.get() && !finished.get()>
                                    <p class="text-xs text-gray-500 dark:text-gray-400">
                                        {t!(i18n, messages.chat.spectator_unlock)}
                                    </p>
                                </Show>
                            </div>
                        }
                    }
                >
                    <GameChatToggle game_id=game_id.get() current_thread />
                </Show>
            </div>
        </ChannelHeaderBar>
    }
}

/// Max height for each channel list section so none dominates; scrollable within.
const SECTION_LIST_MAX_H: &str = "max-h-48 min-h-0 overflow-y-auto";
/// Section header button: collapsible, sticky when sidebar scrolls, good touch target (>=44px).
const SECTION_HEADER_BTN: &str = "sticky top-0 z-10 w-full text-left flex items-center justify-between gap-2 px-2 py-2.5 \
    text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider \
    border-l-2 border-pillbug-teal/50 dark:border-pillbug-teal/40 \
    bg-white dark:bg-gray-900 hover:bg-gray-100 dark:hover:bg-gray-800/50 rounded-r transition-colors min-h-[2.75rem]";
const EMPTY_HINT_CLASS: &str = "px-2 py-1.5 text-sm text-gray-400 dark:text-gray-500 italic";
const CHANNEL_BTN_BASE: &str =
    "no-link-style w-full text-left px-3 py-2 rounded-lg flex justify-between items-center gap-2 \
    transition-colors duration-150 truncate text-sm min-h-[2.75rem]";
const CHANNEL_BTN_SELECTED: &str =
    "bg-pillbug-teal/25 dark:bg-pillbug-teal/35 text-gray-900 dark:text-gray-100 font-medium";
const CHANNEL_BTN_IDLE: &str =
    "hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300";

fn channel_button_class(is_selected: bool) -> String {
    format!(
        "{} {}",
        CHANNEL_BTN_BASE,
        if is_selected {
            CHANNEL_BTN_SELECTED
        } else {
            CHANNEL_BTN_IDLE
        }
    )
}

#[component]
fn ChannelLists(
    conv: MyConversations,
    me: Option<Uuid>,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let MyConversations {
        dms,
        tournaments,
        games,
    } = conv;

    view! {
        <DmChannelsSection dms=dms me=me current_route=current_route chat=chat />
        <TournamentChannelsSection
            tournaments=tournaments
            current_route=current_route
            chat=chat
        />
        <GameChannelsSection games=games current_route=current_route chat=chat />
        <GlobalChannelSection current_route=current_route />
    }
}

#[component]
fn MessagesChannelLink(
    href: String,
    is_selected: Signal<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <A
            href=href
            prop:replace=true
            scroll=false
            attr:class=move || channel_button_class(is_selected.get())
        >
            {children()}
        </A>
    }
}

#[component]
fn SectionHeaderButton(title: Signal<String>, open: RwSignal<bool>) -> impl IntoView {
    view! {
        <button type="button" class=SECTION_HEADER_BTN on:click=move |_| open.update(|o| *o = !*o)>
            <span>{move || title.get()}</span>
            <span class="opacity-70 text-[0.65rem]">
                {move || if open.get() { "▼" } else { "▶" }}
            </span>
        </button>
    }
}

#[component]
fn DmChannelsSection(
    dms: Vec<DmConversation>,
    me: Option<Uuid>,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let dms = StoredValue::new(dms);
    let is_empty = dms.with_value(|items| items.is_empty());

    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || t_string!(i18n, messages.sections.dms)).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    {is_empty.then(|| {
                        view! {
                            <p class=EMPTY_HINT_CLASS>{t!(i18n, messages.sections.no_dms)}</p>
                        }
                    })}
                    {move || {
                        dms.with_value(|items| {
                            items
                                .iter()
                                .cloned()
                                .map(|dm| {
                                    view! {
                                        <DmChannelItem
                                            dm=dm
                                            me=me
                                            current_route=current_route
                                            chat=chat
                                        />
                                    }
                                })
                                .collect_view()
                        })
                    }}
                </div>
            </Show>
        </section>
    }
}

#[component]
fn DmChannelItem(
    dm: DmConversation,
    me: Option<Uuid>,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let DmConversation {
        other_user_id,
        username,
        ..
    } = dm;
    let unread = Signal::derive(move || {
        me.map(|uid| chat.unread_count_for_dm(other_user_id, uid))
            .unwrap_or(0)
    });
    let username = StoredValue::new(username);
    let href = messages_dm_href(&username.get_value());
    let is_selected =
        Signal::derive(move || current_route.get() == MessagesThreadKey::Dm(username.get_value()));

    view! {
        <MessagesChannelLink href=href is_selected=is_selected>
            <span class="truncate">{username.get_value()}</span>
            <ChannelUnreadBadge unread=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn TournamentChannelsSection(
    tournaments: Vec<TournamentChannel>,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let tournaments = StoredValue::new(tournaments);
    let is_empty = tournaments.with_value(|items| items.is_empty());

    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || t_string!(i18n, messages.sections.tournaments)).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    {is_empty.then(|| {
                        view! {
                            <p class=EMPTY_HINT_CLASS>
                                {t!(i18n, messages.sections.no_tournament_chats)}
                            </p>
                        }
                    })}
                    {move || {
                        tournaments.with_value(|items| {
                            items
                                .iter()
                                .cloned()
                                .map(|tournament| {
                                    view! {
                                        <TournamentChannelItem
                                            tournament=tournament
                                            current_route=current_route
                                            chat=chat
                                        />
                                    }
                                })
                                .collect_view()
                        })
                    }}
                </div>
            </Show>
        </section>
    }
}

#[component]
fn TournamentChannelItem(
    tournament: TournamentChannel,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let TournamentChannel {
        nanoid,
        name,
        muted,
        ..
    } = tournament;
    let nanoid = StoredValue::new(nanoid);
    let tournament_id = TournamentId(nanoid.get_value());
    let unread = Signal::derive(move || chat.unread_count_for_tournament(&tournament_id));
    let href = messages_tournament_href(&nanoid.get_value());
    let is_selected = Signal::derive(move || {
        current_route.get() == MessagesThreadKey::Tournament(nanoid.get_value())
    });

    view! {
        <MessagesChannelLink href=href is_selected=is_selected>
            <span class="flex gap-1 items-center truncate">
                {name}
                {muted.then(|| {
                    view! {
                        <span
                            class="text-gray-400 uppercase dark:text-gray-500 shrink-0 text-[0.65rem]"
                            title=move || t_string!(i18n, messages.sections.muted)
                        >
                            {t!(i18n, messages.sections.muted)}
                        </span>
                    }
                })}
            </span>
            <ChannelUnreadBadge unread=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn GameChannelsSection(
    games: Vec<GameChannel>,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let games = StoredValue::new(games);
    let is_empty = games.with_value(|items| items.is_empty());

    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || t_string!(i18n, messages.sections.games)).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    {is_empty.then(|| {
                        view! {
                            <p class=EMPTY_HINT_CLASS>
                                {t!(i18n, messages.sections.no_game_chats)}
                            </p>
                        }
                    })}
                    {move || {
                        games.with_value(|items| {
                            items
                                .iter()
                                .cloned()
                                .map(|game| {
                                    view! {
                                        <GameChannelItem
                                            game=game
                                            current_route=current_route
                                            chat=chat
                                        />
                                    }
                                })
                                .collect_view()
                        })
                    }}
                </div>
            </Show>
        </section>
    }
}

#[component]
fn GameChannelItem(
    game: GameChannel,
    current_route: Signal<MessagesThreadKey>,
    chat: Chat,
) -> impl IntoView {
    let GameChannel {
        channel_type,
        channel_id,
        label,
        finished,
        ..
    } = game;
    let display_label = label
        .rsplit_once(" (")
        .map(|(base, _)| base.to_string())
        .unwrap_or_else(|| label.clone());
    let channel_id = StoredValue::new(channel_id);
    let display_label_with_nanoid =
        StoredValue::new(format!("{} ({})", display_label, channel_id.get_value()));
    let parsed_channel_type = channel_type.parse::<ChannelType>().ok();
    let game_id = GameId(channel_id.get_value());
    let unread = Signal::derive(move || chat.unread_count_for_game(&game_id));

    if let Some(channel_type) = parsed_channel_type {
        let is_selected = Signal::derive(move || {
            game_channel_matches_route(
                &channel_id.get_value(),
                channel_type,
                finished,
                &current_route.get(),
            )
        });
        let href = messages_game_href(&channel_id.get_value(), channel_type)
            .unwrap_or_else(|| messages_root_href().to_string());

        view! {
            <MessagesChannelLink href=href is_selected=is_selected>
                <span class="truncate" title=display_label_with_nanoid.get_value()>
                    {display_label_with_nanoid.get_value()}
                </span>
                <ChannelUnreadBadge unread=unread />
            </MessagesChannelLink>
        }
        .into_any()
    } else {
        view! {
            <div class=channel_button_class(false)>
                <span class="truncate" title=display_label_with_nanoid.get_value()>
                    {display_label_with_nanoid.get_value()}
                </span>
                <ChannelUnreadBadge unread=unread />
            </div>
        }
        .into_any()
    }
}

#[component]
fn GlobalChannelSection(current_route: Signal<MessagesThreadKey>) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let is_selected = Signal::derive(move || current_route.get() == MessagesThreadKey::Global);

    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <SectionHeaderButton
                title=Signal::derive(move || {
                    t_string!(i18n, messages.sections.recent_announcements)
                }).into()
                open=open
            />
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    <MessagesChannelLink
                        href=messages_global_href().to_string()
                        is_selected=is_selected
                    >
                        {t!(i18n, messages.sections.recent_announcements)}
                    </MessagesChannelLink>
                </div>
            </Show>
        </section>
    }
}

#[component]
fn ChannelUnreadBadge(unread: Signal<i64>) -> impl IntoView {
    view! {
        <Show when=move || unread.get() != 0>
            <span class="flex justify-center items-center px-1.5 h-5 text-xs font-medium leading-none text-white rounded-full dark:bg-red-500 shrink-0 min-w-5 bg-ladybug-red">
                {move || {
                    let count = unread.get();
                    if count > 99 { "99+".to_string() } else { count.to_string() }
                }}
            </span>
        </Show>
    }
}
