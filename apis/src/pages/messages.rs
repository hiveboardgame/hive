//! Messages hub: /message — DMs, Tournaments, Games, and recent announcements.
//! Canonical routes under /message are the source of truth for the open thread.

use crate::{
    chat::ConversationKey,
    components::{
        atoms::block_toggle_button::BlockToggleButton,
        organisms::chat::ResolvedChatWindow,
    },
    functions::{
        blocks_mutes::{mute_tournament_chat, unmute_tournament_chat},
        chat::{
            get_game_chat_route_data,
            get_tournament_route_data,
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
use shared_types::{
    ChatDestination,
    DmConversation,
    GameChannel,
    GameId,
    GameThread,
    MessagesHubData,
    TournamentChannel,
    TournamentId,
};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq)]
enum MessageRoute {
    Root,
    Global,
    Dm { username: String },
    Tournament { nanoid: String },
    Game { id: GameId, thread: GameThread },
}

#[derive(Clone)]
enum ResolveState<K, V> {
    Missing(Option<K>),
    Ready(K, V),
}

impl<K, V> ResolveState<K, V> {
    fn key(&self) -> Option<&K> {
        match self {
            Self::Missing(key) => key.as_ref(),
            Self::Ready(key, _) => Some(key),
        }
    }
}

impl MessageRoute {
    fn parse(path: &str) -> Self {
        let path = path.trim_end_matches('/');

        match path {
            "/message" | "" => Self::Root,
            "/message/global" => Self::Global,
            _ if path.starts_with("/message/dm/") => Self::Dm {
                username: path["/message/dm/".len()..].to_string(),
            },
            _ if path.starts_with("/message/tournament/") => Self::Tournament {
                nanoid: path["/message/tournament/".len()..].to_string(),
            },
            _ if path.starts_with("/message/game/") => {
                let rest = &path["/message/game/".len()..];
                let mut parts = rest.split('/');
                match (parts.next(), parts.next(), parts.next()) {
                    (Some(id), Some(thread), None) => {
                        if let Some(thread) = GameThread::parse_slug(thread) {
                            Self::Game {
                                id: GameId(id.to_string()),
                                thread,
                            }
                        } else {
                            Self::Root
                        }
                    }
                    _ => Self::Root,
                }
            }
            _ => Self::Root,
        }
    }

    fn href(&self) -> String {
        match self {
            Self::Root => "/message".to_string(),
            Self::Global => "/message/global".to_string(),
            Self::Dm { username } => format!("/message/dm/{username}"),
            Self::Tournament { nanoid } => format!("/message/tournament/{nanoid}"),
            Self::Game { id, thread } => format!("/message/game/{}/{}", id.0, thread.slug()),
        }
    }

    fn matches_dm(&self, username: &str) -> bool {
        matches!(self, Self::Dm { username: current } if current == username)
    }

    fn matches_tournament(&self, nanoid: &str) -> bool {
        matches!(self, Self::Tournament { nanoid: current } if current == nanoid)
    }

    fn matches_game(&self, id: &GameId, thread: GameThread, finished: bool) -> bool {
        match self {
            Self::Game {
                id: current_id,
                thread: current_thread,
            } => current_id == id && (finished || *current_thread == thread),
            _ => false,
        }
    }
}

fn resolve_from_hub_or_fetch<K, V, Lookup, Fetch, Fut>(
    current_key: Signal<Option<K>>,
    lookup: Lookup,
    fetch: Fetch,
) -> LocalResource<ResolveState<K, V>>
where
    K: Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    Lookup: Fn(&K) -> Option<V> + Send + Sync + 'static,
    Fetch: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Option<V>> + 'static,
{
    let lookup = StoredValue::new(lookup);
    let fetch = StoredValue::new(fetch);
    LocalResource::new(move || {
        let current_key = current_key.get();
        async move {
            let Some(key) = current_key else {
                return ResolveState::Missing(None);
            };

            if let Some(value) = lookup.with_value(|lookup| lookup(&key)) {
                ResolveState::Ready(key, value)
            } else {
                fetch.with_value(|fetch| fetch(key.clone()))
                    .await
                    .map_or(ResolveState::Missing(Some(key.clone())), |value| {
                        ResolveState::Ready(key, value)
                    })
            }
        }
    })
}

fn resolved_route_shell<K, V, F>(
    current_key: Signal<Option<K>>,
    state: LocalResource<ResolveState<K, V>>,
    loading_title: Signal<String>,
    loading_message: Signal<String>,
    missing_title: Signal<String>,
    missing_message: Signal<String>,
    render_ready: F,
) -> impl IntoView
where
    K: Clone + PartialEq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    F: Fn(K, V) -> AnyView + Send + Sync + 'static,
{
    let render_ready = StoredValue::new(render_ready);
    move || match state.get() {
        None => {
            view! { <MessagesStatusFrame title=loading_title message=loading_message /> }.into_any()
        }
        Some(state) if current_key.get().as_ref() != state.key() => {
            view! { <MessagesStatusFrame title=loading_title message=loading_message /> }.into_any()
        }
        Some(ResolveState::Missing(_)) => {
            view! { <MessagesStatusFrame title=missing_title message=missing_message /> }.into_any()
        }
        Some(ResolveState::Ready(key, value)) => {
            render_ready.with_value(|render_ready| render_ready(key, value))
        }
    }
}

fn route_param(name: &'static str) -> Signal<Option<String>> {
    let params = use_params_map();
    Signal::derive(move || params.get().get(name))
}

fn refresh_open_dm_thread(chat: Chat, other_id: Uuid) {
    let key = ConversationKey::direct(other_id);
    spawn_local(async move {
        match chat.fetch_channel_history(&key).await {
            Ok(messages) => chat.replace_history(&key, messages),
            Err(error) => log!("Failed to refresh DM history for {other_id}: {error}"),
        }
    });
}

fn find_dm_conversation(hub_data: &MessagesHubData, username: &str) -> Option<DmConversation> {
    hub_data
        .dms
        .iter()
        .find(|dm| dm.username == username)
        .cloned()
}

fn find_game_channel(hub_data: &MessagesHubData, game_id: &GameId) -> Option<GameChannel> {
    hub_data
        .games
        .iter()
        .find(|channel| channel.game_id == *game_id)
        .cloned()
}

fn find_tournament_channel(
    hub_data: &MessagesHubData,
    tournament_id: &TournamentId,
) -> Option<TournamentChannel> {
    hub_data
        .tournaments
        .iter()
        .find(|channel| channel.nanoid == tournament_id.0)
        .cloned()
}

const SELF_DM_UNSUPPORTED_MESSAGE: &str = "Direct messages to yourself are not supported";
const PLAYERS_CHAT_ONLY_MESSAGE: &str = "Only players can view the players chat.";

#[component]
fn MessagesStatusFrame(title: Signal<String>, message: Signal<String>) -> impl IntoView {
    view! {
        <MessagesThreadFrame title>
            <MessagesStatusContent message />
        </MessagesThreadFrame>
    }
}

fn game_thread_title_signal(
    thread: GameThread,
    players_title: Signal<String>,
    spectators_title: Signal<String>,
) -> Signal<String> {
    Signal::derive(move || match thread {
        GameThread::Players => players_title.get(),
        GameThread::Spectators => spectators_title.get(),
    })
}

#[component]
fn MessagesResolvedDmView(
    loading_message: Signal<String>,
    failed_message: Signal<String>,
    other_user_id: Uuid,
    username: String,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let username = StoredValue::new(username);
    let current_user_id = Signal::derive(move || {
        auth.user
            .with(|account| account.as_ref().map(|account| account.user.uid))
    });
    let auth_pending = auth.action.pending();
    let destination =
        Signal::derive(move || ChatDestination::User((other_user_id, username.get_value())));
    let username = StoredValue::new(username.get_value());
    let unavailable_message = Signal::derive(move || {
        if auth_pending.get() {
            loading_message.get()
        } else {
            failed_message.get()
        }
    });
    let self_dm_error = Signal::derive(|| SELF_DM_UNSUPPORTED_MESSAGE.to_string());

    view! {
        <MessagesThreadFrame title=Signal::derive(move || username.get_value())>
            <ShowLet
                some=move || current_user_id.get()
                let:current_user_id
                fallback=move || {
                    view! {
                        <MessagesStatusContent message=unavailable_message />
                    }
                }
            >
                <Show
                    when=move || current_user_id != other_user_id
                    fallback=move || {
                        view! {
                            <MessagesStatusContent message=self_dm_error />
                        }
                    }
                >
                    <DmChannelActions other_id=other_user_id username />
                    <div class="overflow-hidden flex-1 min-h-0">
                        <ResolvedChatWindow destination />
                    </div>
                </Show>
            </ShowLet>
        </MessagesThreadFrame>
    }
}

#[component]
fn MessagesResolvedTournamentView(
    restricted_message: Signal<String>,
    tournament_id: TournamentId,
    title: String,
    muted: bool,
    can_chat: bool,
) -> impl IntoView {
    let title = StoredValue::new(title);
    let tournament_id = StoredValue::new(tournament_id);
    let destination =
        Signal::derive(move || ChatDestination::TournamentLobby(tournament_id.get_value()));

    view! {
        <MessagesThreadFrame title=Signal::derive(move || title.get_value())>
            <TournamentRouteActions tournament_id=tournament_id.get_value() muted />
            <Show
                when=move || can_chat
                fallback=move || {
                    view! {
                        <MessagesStatusContent message=restricted_message />
                    }
                }
            >
                <div class="overflow-hidden flex-1 min-h-0">
                    <ResolvedChatWindow destination />
                </div>
            </Show>
        </MessagesThreadFrame>
    }
}

#[component]
fn MessagesResolvedGameView(
    failed_message: Signal<String>,
    game_id: GameId,
    thread: GameThread,
    title: Signal<String>,
    spectator_unlock_message: Signal<String>,
    is_player: bool,
    finished: bool,
) -> impl IntoView {
    let game_id = StoredValue::new(game_id);
    let denied_message = Signal::derive(move || match thread {
        GameThread::Players if !is_player => Some(PLAYERS_CHAT_ONLY_MESSAGE.to_string()),
        GameThread::Spectators if is_player && !finished => {
            Some(spectator_unlock_message.get())
        }
        _ => None,
    });
    let can_view_thread = Signal::derive(move || denied_message.get().is_none());
    let chat_destination = Signal::derive(move || match thread {
        GameThread::Players => ChatDestination::GamePlayers(game_id.get_value()),
        GameThread::Spectators => ChatDestination::GameSpectators(game_id.get_value()),
    });
    let status_message =
        Signal::derive(move || denied_message.get().unwrap_or_else(|| failed_message.get()));

    view! {
        <MessagesThreadFrame title>
            <GameChatHeader current_thread=thread game_id=game_id.get_value() is_player finished />
            <Show
                when=move || can_view_thread.get()
                fallback=move || {
                    view! {
                        <MessagesStatusContent message=status_message />
                    }
                }
            >
                <div class="overflow-hidden flex-1 min-h-0">
                    <ResolvedChatWindow destination=chat_destination />
                </div>
            </Show>
        </MessagesThreadFrame>
    }
}

#[component]
pub fn MessagesLayout() -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let current_route = Signal::derive(move || MessageRoute::parse(&location.pathname.get()));
    let mobile_list_visible =
        Signal::derive(move || matches!(current_route.get(), MessageRoute::Root));
    let mobile_thread_visible =
        Signal::derive(move || !matches!(current_route.get(), MessageRoute::Root));

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
                    <MessagesSidebar current_route />
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
fn MessagesSidebar(current_route: Signal<MessageRoute>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();

    view! {
        <ShowLet
            some=move || chat.messages_hub_data.get()
            let:hub_data
            fallback=move || {
                view! {
                    <Show
                        when=move || chat.messages_hub_loading.get()
                        fallback=move || {
                            view! {
                                <p class="p-3 text-sm text-red-600 dark:text-red-400">
                                    {t!(i18n, messages.page.failed_conversations)}
                                </p>
                            }
                        }
                    >
                        <p class="p-3 text-sm text-gray-500 animate-pulse dark:text-gray-400">
                            {t!(i18n, messages.page.loading)}
                        </p>
                    </Show>
                }
            }
        >
            {
                let MessagesHubData {
                    dms,
                    tournaments,
                    games,
                    ..
                } = hub_data;

                view! {
                    <ChannelListSection
                        title=Signal::derive(move || t_string!(i18n, messages.sections.dms).to_string())
                        empty_label=Signal::derive(move || {
                            t_string!(i18n, messages.sections.no_dms).to_string()
                        })
                        items=dms
                        render_item=move |dm| {
                            view! { <DmChannelItem dm current_route=current_route /> }.into_any()
                        }
                    />
                    <ChannelListSection
                        title=Signal::derive(move || {
                            t_string!(i18n, messages.sections.tournaments).to_string()
                        })
                        empty_label=Signal::derive(move || {
                            t_string!(i18n, messages.sections.no_tournament_chats).to_string()
                        })
                        items=tournaments
                        render_item=move |tournament| {
                            view! { <TournamentChannelItem tournament current_route=current_route /> }
                                .into_any()
                        }
                    />
                    <ChannelListSection
                        title=Signal::derive(move || t_string!(i18n, messages.sections.games).to_string())
                        empty_label=Signal::derive(move || {
                            t_string!(i18n, messages.sections.no_game_chats).to_string()
                        })
                        items=games
                        render_item=move |game| {
                            view! { <GameChannelItem game current_route=current_route /> }.into_any()
                        }
                    />
                    <GlobalChannelSection current_route=current_route />
                }
            }
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
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let route_username = route_param("username");
    let loading_message =
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string());
    let failed_message =
        Signal::derive(move || t_string!(i18n, messages.page.failed_conversations).to_string());
    let route_resolution = resolve_from_hub_or_fetch(
        route_username,
        move |route_username| {
            chat.messages_hub_data.with_untracked(|hub| {
                hub.as_ref()
                    .and_then(|hub| find_dm_conversation(hub, route_username))
                    .map(|dm| (dm.other_user_id, dm.username))
            })
        },
        move |route_username| async move {
            resolve_username(route_username)
                .await
                .ok()
                .map(|user| (user.uid, user.username))
        },
    );

    resolved_route_shell(
        route_username,
        route_resolution,
        loading_message,
        loading_message,
        failed_message,
        failed_message,
        move |_resolved_username, resolved_user| {
            view! {
                <MessagesResolvedDmView
                    loading_message
                    failed_message
                    other_user_id=resolved_user.0
                    username=resolved_user.1
                />
            }
            .into_any()
        },
    )
}

#[component]
pub fn MessagesTournamentThread() -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let route_nanoid = route_param("nanoid");
    let route_tournament_id = Signal::derive(move || route_nanoid.get().map(TournamentId));
    let loading_message =
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string());
    let failed_message =
        Signal::derive(move || t_string!(i18n, messages.page.failed_conversations).to_string());
    let restricted_message = Signal::derive(move || {
        t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
    });
    let route_resolution = resolve_from_hub_or_fetch(
        route_tournament_id,
        move |route_tournament_id| {
            chat.messages_hub_data.with_untracked(|hub| {
                hub.as_ref()
                    .and_then(|hub| find_tournament_channel(hub, route_tournament_id))
                    .map(|channel| (channel.name, channel.muted, channel.can_chat))
            })
        },
        move |route_tournament_id| async move {
            get_tournament_route_data(route_tournament_id.0).await.ok()
        },
    );

    resolved_route_shell(
        route_tournament_id,
        route_resolution,
        loading_message,
        loading_message,
        failed_message,
        failed_message,
        move |tournament_id, resolved_tournament| {
            view! {
                <MessagesResolvedTournamentView
                    restricted_message
                    tournament_id
                    title=resolved_tournament.0
                    muted=resolved_tournament.1
                    can_chat=resolved_tournament.2
                />
            }
            .into_any()
        },
    )
}

#[component]
pub fn MessagesGamePlayersThread() -> impl IntoView {
    view! { <MessagesGameThread thread=GameThread::Players /> }
}

#[component]
pub fn MessagesGameSpectatorsThread() -> impl IntoView {
    view! { <MessagesGameThread thread=GameThread::Spectators /> }
}

#[component]
fn MessagesGameThread(thread: GameThread) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let route_game_nanoid = route_param("nanoid");
    let route_game_id = Signal::derive(move || route_game_nanoid.get().map(GameId));
    let loading_message =
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string());
    let failed_message =
        Signal::derive(move || t_string!(i18n, messages.page.failed_game).to_string());
    let players_title =
        Signal::derive(move || t_string!(i18n, messages.chat.players_chat).to_string());
    let spectators_title =
        Signal::derive(move || t_string!(i18n, messages.chat.spectator_chat).to_string());
    let spectator_unlock_message =
        Signal::derive(move || t_string!(i18n, messages.chat.spectator_unlock).to_string());
    let title = game_thread_title_signal(thread, players_title, spectators_title);
    let route_resolution = resolve_from_hub_or_fetch(
        route_game_id,
        move |route_game_id| {
            chat.messages_hub_data.with_untracked(|hub| {
                hub.as_ref()
                    .and_then(|hub| find_game_channel(hub, route_game_id))
                    .map(|channel| (channel.is_player, channel.finished))
            })
        },
        move |route_game_id| async move {
            get_game_chat_route_data(route_game_id).await.ok()
        },
    );

    resolved_route_shell(
        route_game_id,
        route_resolution,
        title,
        loading_message,
        failed_message,
        failed_message,
        move |game_id, resolved_game| {
            view! {
                <MessagesResolvedGameView
                    failed_message
                    game_id
                    thread
                    title
                    spectator_unlock_message
                    is_player=resolved_game.0
                    finished=resolved_game.1
                />
            }
            .into_any()
        },
    )
}

#[component]
fn MessagesThreadFrame(title: Signal<String>, children: Children) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="flex overflow-hidden flex-col flex-1 min-h-0 bg-white border-r border-gray-200 shadow-inner sm:rounded-l-xl dark:bg-gray-900 dark:border-gray-700">
            <div class="flex gap-2 items-center py-3 px-2 bg-gray-50 border-b border-gray-200 sm:px-4 dark:border-gray-700 shrink-0 min-h-[2.75rem] dark:bg-gray-800/50">
                <A
                    href=MessageRoute::Root.href()
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
fn DmChannelActions(other_id: Uuid, username: StoredValue<String>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let is_blocked =
        Signal::derive(move || chat.blocked_user_ids.with(|ids| ids.contains(&other_id)));
    let on_block_toggle_success = Callback::new(move |_| {
        refresh_open_dm_thread(chat, other_id);
    });

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-wrap gap-2 items-center">
                <A
                    href=move || format!("/@/{}", username.get_value())
                    attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                >
                    {t!(i18n, messages.page.view_profile)}
                </A>
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
    tournament_id: TournamentId,
    muted: bool,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let tournament_id = StoredValue::new(tournament_id);
    let tournament_href = StoredValue::new(format!("/tournament/{}", tournament_id.get_value()));
    let muted_override = RwSignal::new(None::<bool>);
    let mute_error = RwSignal::new(None::<String>);
    let resolved_muted = Signal::derive(move || muted_override.get().unwrap_or(muted));
    let toggle_mute = Action::new(move |currently_muted: &bool| {
        let currently_muted = *currently_muted;
        async move {
            if currently_muted {
                unmute_tournament_chat(tournament_id.get_value().0)
                    .await
                    .map(|_| false)
                    .map_err(|error| error.to_string())
            } else {
                mute_tournament_chat(tournament_id.get_value().0)
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
                    chat.set_tournament_muted(&tournament_id.get_value().0, new_muted);
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
                        disabled=move || toggle_mute.pending().get()
                        class="text-sm font-medium text-gray-600 transition-colors dark:text-gray-400 disabled:text-gray-400 dark:hover:text-pillbug-teal/90 dark:disabled:text-gray-500 hover:text-pillbug-teal"
                        on:click=move |_| {
                            mute_error.set(None);
                            toggle_mute.dispatch(resolved_muted.get_untracked());
                        }
                    >
                        {move || {
                            if toggle_mute.pending().get() {
                                t_string!(i18n, messages.page.loading)
                            } else if resolved_muted.get() {
                                t_string!(i18n, messages.page.unmute_tournament_chat)
                            } else {
                                t_string!(i18n, messages.page.mute_tournament_chat)
                            }
                        }}
                    </button>
                </div>
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
fn GameChatToggle(game_id: GameId, current_thread: GameThread) -> impl IntoView {
    let i18n = use_i18n();
    let players_href = MessageRoute::Game {
        id: game_id.clone(),
        thread: GameThread::Players,
    }
    .href();
    let spectators_href = MessageRoute::Game {
        id: game_id.clone(),
        thread: GameThread::Spectators,
    }
    .href();
    let viewing_players = current_thread == GameThread::Players;

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
    current_thread: GameThread,
    game_id: GameId,
    is_player: bool,
    finished: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let static_chat_label = Signal::derive(move || {
        if current_thread == GameThread::Players {
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
                        href=format!("/game/{game_id}")
                        attr:class="inline-flex items-center text-sm font-medium text-pillbug-teal hover:text-pillbug-teal/80 dark:text-pillbug-teal dark:hover:text-pillbug-teal/90 transition-colors"
                    >
                        {t!(i18n, messages.page.view_game)}
                    </A>
                </div>
                <Show
                    when=move || is_player && finished
                    fallback=move || {
                        view! {
                            <div class="flex flex-col gap-1">
                                <span class="inline-flex items-center py-1 px-2.5 text-xs font-medium text-gray-700 bg-white rounded-full border border-gray-300 dark:text-gray-200 dark:bg-gray-800 dark:border-gray-600 w-fit">
                                    {move || static_chat_label.get()}
                                </span>
                                <Show when=move || is_player && !finished>
                                    <p class="text-xs text-gray-500 dark:text-gray-400">
                                        {t!(i18n, messages.chat.spectator_unlock)}
                                    </p>
                                </Show>
                            </div>
                        }
                    }
                >
                    <GameChatToggle game_id=game_id.clone() current_thread />
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
fn ChannelSection(
    title: Signal<String>,
    open: RwSignal<bool>,
    #[prop(optional)] is_empty: bool,
    #[prop(optional)] empty_label: Option<Signal<String>>,
    children: ChildrenFn,
) -> impl IntoView {
    let empty_label = empty_label.unwrap_or_else(|| Signal::derive(String::new));
    let children = StoredValue::new(children);

    view! {
        <section class="flex flex-col mb-2 min-h-0">
            <button
                type="button"
                class=SECTION_HEADER_BTN
                on:click=move |_| open.update(|state| *state = !*state)
            >
                <span>{move || title.get()}</span>
                <span class="opacity-70 text-[0.65rem]">
                    {move || if open.get() { "▼" } else { "▶" }}
                </span>
            </button>
            <Show when=move || open.get()>
                <div class=SECTION_LIST_MAX_H>
                    <Show
                        when=move || !is_empty
                        fallback=move || {
                            view! {
                                <Show when=move || !empty_label.get().is_empty()>
                                    <p class=EMPTY_HINT_CLASS>{move || empty_label.get()}</p>
                                </Show>
                            }
                        }
                    >
                        {move || children.with_value(|children| children())}
                    </Show>
                </div>
            </Show>
        </section>
    }
}

#[component]
fn ChannelListSection<T, F>(
    title: Signal<String>,
    empty_label: Signal<String>,
    items: Vec<T>,
    render_item: F,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
    F: Fn(T) -> AnyView + Copy + Send + Sync + 'static,
{
    let open = RwSignal::new(true);
    let items = StoredValue::new(items);
    let is_empty = items.with_value(|items| items.is_empty());

    view! {
        <ChannelSection
            title
            open=open
            is_empty=is_empty
            empty_label=empty_label
        >
            {move || {
                items.with_value(|items| {
                    items
                        .iter()
                        .cloned()
                        .map(render_item)
                        .collect_view()
                })
            }}
        </ChannelSection>
    }
}

#[component]
fn DmChannelItem(dm: DmConversation, current_route: Signal<MessageRoute>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let DmConversation {
        other_user_id,
        username,
        ..
    } = dm;
    let unread = Signal::derive(move || chat.unread_count_for_dm(other_user_id));
    let username = StoredValue::new(username);
    let href = Signal::derive(move || {
        MessageRoute::Dm {
            username: username.get_value(),
        }
        .href()
    });
    let is_selected = Signal::derive(move || current_route.get().matches_dm(&username.get_value()));

    view! {
        <MessagesChannelLink href=href.get() is_selected=is_selected>
            <span class="truncate">{username.get_value()}</span>
            <ChannelUnreadBadge unread=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn TournamentChannelItem(
    tournament: TournamentChannel,
    current_route: Signal<MessageRoute>,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
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
    let href = Signal::derive(move || {
        MessageRoute::Tournament {
            nanoid: nanoid.get_value(),
        }
        .href()
    });
    let is_selected =
        Signal::derive(move || current_route.get().matches_tournament(&nanoid.get_value()));

    view! {
        <MessagesChannelLink href=href.get() is_selected=is_selected>
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
fn GameChannelItem(game: GameChannel, current_route: Signal<MessageRoute>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let GameChannel {
        game_id,
        thread,
        label,
        finished,
        ..
    } = game;
    let game_id = StoredValue::new(game_id);
    let route = StoredValue::new(MessageRoute::Game {
        id: game_id.get_value(),
        thread,
    });
    let display_label_with_nanoid =
        StoredValue::new(format!("{} ({})", label, game_id.get_value().0));
    let unread = Signal::derive(move || chat.unread_count_for_game(&game_id.get_value()));
    let is_selected = Signal::derive(move || {
        current_route
            .get()
            .matches_game(&game_id.get_value(), thread, finished)
    });

    view! {
        <MessagesChannelLink href=route.get_value().href() is_selected=is_selected>
            <span class="truncate" title=display_label_with_nanoid.get_value()>
                {display_label_with_nanoid.get_value()}
            </span>
            <ChannelUnreadBadge unread=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn GlobalChannelSection(current_route: Signal<MessageRoute>) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let is_selected = Signal::derive(move || current_route.get() == MessageRoute::Global);

    view! {
        <ChannelSection
            title=Signal::derive(move || {
                t_string!(i18n, messages.sections.recent_announcements)
            }).into()
            open=open
        >
            <MessagesChannelLink href=MessageRoute::Global.href() is_selected=is_selected>
                {t!(i18n, messages.sections.recent_announcements)}
            </MessagesChannelLink>
        </ChannelSection>
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
