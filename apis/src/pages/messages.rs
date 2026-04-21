//! Messages hub: /message — DMs, Tournaments, Games, and recent announcements.
//! Canonical routes under /message are the source of truth for the open thread.

use crate::{
    chat::ConversationKey,
    components::{
        atoms::{block_toggle_button::BlockToggleButton, unread_badge::UnreadBadge},
        organisms::chat::ResolvedChatWindow,
    },
    functions::{
        blocks_mutes::{mute_tournament_chat, unmute_tournament_chat},
        chat::{get_game_chat_route_data, get_tournament_route_data},
        users::resolve_username,
    },
    i18n::*,
    providers::{chat::Chat, AuthContext},
};
use leptos::{
    either::{Either, EitherOf3},
    logging::log,
    prelude::*,
    task::spawn_local,
};
use leptos_router::{
    components::{A, Outlet, Redirect},
    hooks::{use_location, use_params_map},
    NavigateOptions,
};
use shared_types::{
    ChatDestination,
    ChatHistoryResponse,
    DmConversation,
    GameChannel,
    GameChatCapabilities,
    GameId,
    GameThread,
    MessagesHubData,
    TournamentChannel,
    TournamentChatCapabilities,
    TournamentId,
};
use uuid::Uuid;

// 1. Shared domain types and truly global helpers

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

const SELF_DM_UNSUPPORTED_MESSAGE: &str = "Direct messages to yourself are not supported";
const PLAYERS_CHAT_ONLY_MESSAGE: &str = "Only players can view the players chat.";
const HEADER_ACTION_BUTTON_PRIMARY: &str =
    "no-link-style inline-flex items-center justify-center gap-1.5 px-3 py-1.5 text-sm font-medium \
    text-pillbug-teal rounded-lg border border-pillbug-teal/30 bg-pillbug-teal/10 shadow-sm \
    transition-colors hover:bg-pillbug-teal/15 dark:border-pillbug-teal/40 dark:bg-pillbug-teal/20 \
    dark:text-pillbug-teal dark:hover:bg-pillbug-teal/25";
const GAME_CHAT_TOGGLE_CONTAINER_CLASS: &str =
    "flex p-0.5 bg-gray-100 rounded-lg border border-gray-300 dark:bg-gray-800 dark:border-gray-600";

fn game_chat_toggle_segment_class(selected: bool, disabled: bool) -> String {
    let state_classes = if selected {
        "bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-sm"
    } else if disabled {
        "text-gray-400 dark:text-gray-500 cursor-not-allowed"
    } else {
        "text-gray-600 dark:text-gray-400 hover:text-gray-900 hover:bg-white/70 dark:hover:text-gray-100 dark:hover:bg-gray-700/70"
    };

    format!(
        "no-link-style flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors text-center {}",
        state_classes,
    )
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

impl<K, V> ResolveState<K, V> {
    fn key(&self) -> Option<&K> {
        match self {
            Self::Missing(key) => key.as_ref(),
            Self::Ready(key, _) => Some(key),
        }
    }
}

fn route_param(name: &'static str) -> Signal<Option<String>> {
    let params = use_params_map();
    Signal::derive(move || params.get().get(name))
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
                fetch
                    .with_value(|fetch| fetch(key.clone()))
                    .await
                    .map_or(ResolveState::Missing(Some(key.clone())), |value| {
                        ResolveState::Ready(key, value)
                    })
            }
        }
    })
}

fn resolved_route_shell<K, V, F, IV>(
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
    F: Fn(K, V) -> IV + Send + Sync + 'static,
    IV: IntoView + 'static,
{
    let render_ready = StoredValue::new(render_ready);
    move || match state.get() {
        None => EitherOf3::A(view! { <MessagesStatusFrame title=loading_title message=loading_message /> }),
        Some(state) if current_key.get().as_ref() != state.key() => EitherOf3::A(view! { <MessagesStatusFrame title=loading_title message=loading_message /> }),
        Some(ResolveState::Missing(_)) => EitherOf3::B(view! { <MessagesStatusFrame title=missing_title message=missing_message /> }),
        Some(ResolveState::Ready(key, value)) => {
            EitherOf3::C(render_ready.with_value(|render_ready| render_ready(key, value)))
        }
    }
}

fn find_dm_conversation(hub_data: &MessagesHubData, username: &str) -> Option<DmConversation> {
    hub_data
        .dms
        .iter()
        .find(|dm| dm.username == username)
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

fn find_game_channel(hub_data: &MessagesHubData, game_id: &GameId) -> Option<GameChannel> {
    hub_data
        .games
        .iter()
        .find(|channel| channel.game_id == *game_id)
        .cloned()
}

fn refresh_open_dm_thread(chat: Chat, other_id: Uuid) {
    let key = ConversationKey::direct(other_id);
    spawn_local(async move {
        match chat.fetch_channel_history(&key).await {
            Ok(ChatHistoryResponse::Messages(messages)) => chat.replace_history(&key, messages),
            Ok(ChatHistoryResponse::AccessDenied) => {
                log!("Failed to refresh DM history for {other_id}: access denied")
            }
            Err(error) => log!("Failed to refresh DM history for {other_id}: {error}"),
        }
    });
}

// 2. Top-level page shell

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
        {move || match chat.messages_hub_data.get() {
            Some(hub_data) => {
                let MessagesHubData { dms, tournaments, games, .. } = hub_data;
                EitherOf3::A(

                    view! {
                        <ChannelListSection
                            title=Signal::derive(move || {
                                t_string!(i18n, messages.sections.dms).to_string()
                            })
                            empty_label=Signal::derive(move || {
                                t_string!(i18n, messages.sections.no_dms).to_string()
                            })
                            items=dms
                            render_item=move |dm| {
                                view! { <DmChannelItem dm current_route=current_route /> }
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
                                view! {
                                    <TournamentChannelItem tournament current_route=current_route />
                                }
                            }
                        />
                        <ChannelListSection
                            title=Signal::derive(move || {
                                t_string!(i18n, messages.sections.games).to_string()
                            })
                            empty_label=Signal::derive(move || {
                                t_string!(i18n, messages.sections.no_game_chats).to_string()
                            })
                            items=games
                            render_item=move |game| {
                                view! { <GameChannelItem game current_route=current_route /> }
                            }
                        />
                        <GlobalChannelSection current_route=current_route />
                    },
                )
            }
            None if chat.messages_hub_loading.get() => {
                EitherOf3::B(
                    view! {
                        <p class="p-3 text-sm text-gray-500 animate-pulse dark:text-gray-400">
                            {t!(i18n, messages.page.loading)}
                        </p>
                    },
                )
            }
            None => {
                EitherOf3::C(
                    view! {
                        <p class="p-3 text-sm text-red-600 dark:text-red-400">
                            {t!(i18n, messages.page.failed_conversations)}
                        </p>
                    },
                )
            }
        }}
    }
}

#[component]
pub fn MessagesIndex() -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class="hidden overflow-hidden flex-col flex-1 min-h-0 bg-white border-r border-gray-200 shadow-inner sm:flex sm:rounded-l-xl dark:bg-gray-900 dark:border-gray-700">
            <div class="flex flex-col flex-1 gap-2 justify-center items-center p-8 text-gray-500 dark:text-gray-400">
                <span class="text-4xl opacity-50">"💬"</span>
                <p class="font-medium text-center">{t!(i18n, messages.page.select_conversation)}</p>
                <p class="max-w-xs text-sm text-center">
                    {t!(i18n, messages.page.choose_conversation)}
                </p>
            </div>
        </div>
    }
}

// 3. Route components in route order

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
        },
    )
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
        <MessagesThreadFrame title=Signal::derive(move || {
            username.get_value()
        })>
            {move || match current_user_id.get() {
                Some(current_user_id) if current_user_id != other_user_id => {
                    EitherOf3::A(
                        view! {
                            <DmChannelActions other_id=other_user_id username />
                            <div class="overflow-hidden flex-1 min-h-0">
                                <ResolvedChatWindow destination />
                            </div>
                        },
                    )
                }
                Some(_) => EitherOf3::B(view! { <MessagesStatusContent message=self_dm_error /> }),
                None => {
                    EitherOf3::C(view! { <MessagesStatusContent message=unavailable_message /> })
                }
            }}
        </MessagesThreadFrame>
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
                    attr:class=HEADER_ACTION_BUTTON_PRIMARY
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
                    .map(|channel| (channel.name, channel.muted, channel.access))
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
            let (title, muted, access) = resolved_tournament;
            view! { <MessagesResolvedTournamentView restricted_message tournament_id title muted access /> }
        },
    )
}

#[component]
fn MessagesResolvedTournamentView(
    restricted_message: Signal<String>,
    tournament_id: TournamentId,
    title: String,
    muted: bool,
    access: TournamentChatCapabilities,
) -> impl IntoView {
    let title = StoredValue::new(title);
    let tournament_id = StoredValue::new(tournament_id);
    let destination =
        Signal::derive(move || ChatDestination::TournamentLobby(tournament_id.get_value()));

    view! {
        <MessagesThreadFrame title=Signal::derive(move || title.get_value())>
            <TournamentRouteActions tournament_id=tournament_id.get_value() muted />
            {if access.can_read() {
                Either::Left(
                    view! {
                        <div class="overflow-hidden flex-1 min-h-0">
                            <ResolvedChatWindow destination />
                        </div>
                    },
                )
            } else {
                Either::Right(view! { <MessagesStatusContent message=restricted_message /> })
            }}
        </MessagesThreadFrame>
    }
}

#[component]
fn TournamentRouteActions(tournament_id: TournamentId, muted: bool) -> impl IntoView {
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
    let mute_button_label = Signal::derive(move || {
        if toggle_mute.pending().get() {
            t_string!(i18n, messages.page.loading)
        } else if resolved_muted.get() {
            t_string!(i18n, messages.page.unmute_tournament_chat)
        } else {
            t_string!(i18n, messages.page.mute_tournament_chat)
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
                        attr:class=HEADER_ACTION_BUTTON_PRIMARY
                    >
                        {t!(i18n, messages.page.view_tournament)}
                    </A>
                    <button
                        type="button"
                        disabled=toggle_mute.pending()
                        title=move || mute_button_label.get()
                        class=move || {
                            format!(
                                "inline-flex items-center justify-center px-3 py-1.5 text-sm font-semibold rounded-lg text-white whitespace-nowrap transition-colors duration-300 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed {}",
                                if resolved_muted.get() {
                                    "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
                                } else {
                                    "bg-ladybug-red dark:bg-ladybug-red hover:bg-red-500 dark:hover:bg-red-500"
                                }
                            )
                        }
                        on:click=move |_| {
                            mute_error.set(None);
                            toggle_mute.dispatch(resolved_muted.get_untracked());
                        }
                    >
                        {move || mute_button_label.get()}
                    </button>
                </div>
                {move || {
                    mute_error
                        .get()
                        .map(|error| {
                            view! { <p class="text-xs text-red-600 dark:text-red-400">{error}</p> }
                        })
                }}
            </div>
        </ChannelHeaderBar>
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
pub fn MessagesGameThread(thread: GameThread) -> impl IntoView {
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
                    .map(|channel| GameChatCapabilities::new(channel.is_player, channel.finished))
            })
        },
        move |route_game_id| async move { get_game_chat_route_data(route_game_id).await.ok() },
    );

    resolved_route_shell(
        route_game_id,
        route_resolution,
        title,
        loading_message,
        failed_message,
        failed_message,
        move |game_id, resolved_game| {
            if thread == GameThread::Players
                && !resolved_game.can_read(GameThread::Players)
                && resolved_game.can_read(GameThread::Spectators)
            {
                let spectators_href = MessageRoute::Game {
                    id: game_id.clone(),
                    thread: GameThread::Spectators,
                }
                .href();

                Either::Left(view! {
                    <Redirect
                        path=spectators_href
                        options=NavigateOptions {
                            replace: true,
                            ..Default::default()
                        }
                    />
                })
            } else {
                Either::Right(view! {
                    <MessagesResolvedGameView
                        failed_message
                        game_id
                        thread
                        title
                        spectator_unlock_message
                        access=resolved_game
                    />
                })
            }
        },
    )
}

#[component]
fn MessagesResolvedGameView(
    failed_message: Signal<String>,
    game_id: GameId,
    thread: GameThread,
    title: Signal<String>,
    spectator_unlock_message: Signal<String>,
    access: GameChatCapabilities,
) -> impl IntoView {
    let game_id = StoredValue::new(game_id);
    let denied_message = Signal::derive(move || match thread {
        GameThread::Players if !access.can_read(GameThread::Players) => {
            Some(PLAYERS_CHAT_ONLY_MESSAGE.to_string())
        }
        GameThread::Spectators if !access.can_read(GameThread::Spectators) => {
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
            <GameChatHeader current_thread=thread game_id=game_id.get_value() access />
            <Show
                when=can_view_thread
                fallback=move || {
                    view! { <MessagesStatusContent message=status_message /> }
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
fn GameChatHeader(
    current_thread: GameThread,
    game_id: GameId,
    access: GameChatCapabilities,
) -> impl IntoView {
    let i18n = use_i18n();
    let spectator_unlock_needed =
        access.can_toggle_embedded_threads() && !access.can_read(GameThread::Spectators);

    view! {
        <ChannelHeaderBar>
            <div class="flex flex-col gap-2">
                <div class="flex flex-wrap gap-2 items-center">
                    <A
                        href=format!("/game/{game_id}")
                        attr:class=HEADER_ACTION_BUTTON_PRIMARY
                    >
                        {t!(i18n, messages.page.view_game)}
                    </A>
                </div>
                <div class="flex flex-col gap-1.5">
                    <GameChatToggle game_id=game_id.clone() current_thread access />
                    {spectator_unlock_needed
                        .then(|| {
                            view! {
                                <p class="text-xs text-gray-500 dark:text-gray-400">
                                    {t!(i18n, messages.chat.spectator_unlock)}
                                </p>
                            }
                        })}
                </div>
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn GameChatToggle(
    game_id: GameId,
    current_thread: GameThread,
    access: GameChatCapabilities,
) -> impl IntoView {
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
    let can_read_players = access.can_read(GameThread::Players);
    let can_read_spectators = access.can_read(GameThread::Spectators);
    let viewing_players = current_thread == GameThread::Players;
    let viewing_spectators = current_thread == GameThread::Spectators;

    view! {
        <div class=GAME_CHAT_TOGGLE_CONTAINER_CLASS>
            {if can_read_players {
                Either::Left(
                    view! {
                        <A
                            href=players_href
                            prop:replace=true
                            scroll=false
                            attr:class=game_chat_toggle_segment_class(viewing_players, false)
                        >
                            {t!(i18n, messages.chat.players)}
                        </A>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <span class=game_chat_toggle_segment_class(viewing_players, true)>
                            {t!(i18n, messages.chat.players)}
                        </span>
                    },
                )
            }}
            {if can_read_spectators {
                Either::Left(
                    view! {
                        <A
                            href=spectators_href
                            prop:replace=true
                            scroll=false
                            attr:class=game_chat_toggle_segment_class(viewing_spectators, false)
                        >
                            {t!(i18n, messages.chat.spectators)}
                        </A>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <span class=game_chat_toggle_segment_class(viewing_spectators, true)>
                            {t!(i18n, messages.chat.spectators)}
                        </span>
                    },
                )
            }}
        </div>
    }
}

// 4. Sidebar item cluster in the same visual order the sidebar renders

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
fn DmChannelItem(dm: DmConversation, current_route: Signal<MessageRoute>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let DmConversation {
        other_user_id,
        username,
        ..
    } = dm;
    let unread = Signal::derive(move || chat.unread_count_for_dm(other_user_id));
    let username = StoredValue::new(username);
    let href = {
        MessageRoute::Dm {
            username: username.get_value(),
        }
        .href()
    };
    let is_selected = Signal::derive(move || current_route.get().matches_dm(&username.get_value()));

    view! {
        <MessagesChannelLink href is_selected=is_selected>
            <span class="truncate">{username.get_value()}</span>
            <UnreadBadge count=unread />
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
    let href = {
        MessageRoute::Tournament {
            nanoid: nanoid.get_value(),
        }
        .href()
    };
    let is_selected =
        Signal::derive(move || current_route.get().matches_tournament(&nanoid.get_value()));

    view! {
        <MessagesChannelLink href is_selected=is_selected>
            <span class="flex gap-1 items-center truncate">
                {name}
                {muted
                    .then(|| {
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
            <UnreadBadge count=unread />
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
    let href = MessageRoute::Game {
        id: game_id.get_value(),
        thread,
    }
    .href();
    let display_label_with_nanoid =
        StoredValue::new(format!("{} ({})", label, game_id.get_value().0));
    let unread = Signal::derive(move || chat.unread_count_for_game(&game_id.get_value()));
    let is_selected = Signal::derive(move || {
        current_route
            .get()
            .matches_game(&game_id.get_value(), thread, finished)
    });

    view! {
        <MessagesChannelLink href is_selected=is_selected>
            <span class="truncate" title=display_label_with_nanoid.get_value()>
                {display_label_with_nanoid.get_value()}
            </span>
            <UnreadBadge count=unread />
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
                t_string!(i18n, messages.sections.recent_announcements).to_string()
            })
            open=open
        >
            <MessagesChannelLink href=MessageRoute::Global.href() is_selected=is_selected>
                {t!(i18n, messages.sections.recent_announcements)}
            </MessagesChannelLink>
        </ChannelSection>
    }
}

// 5. Shared leaf UI primitives last

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
                    attr:class="no-link-style inline-flex flex-shrink-0 gap-1.5 justify-center items-center px-3 py-1.5 text-sm font-medium text-gray-700 rounded-lg border border-gray-300 bg-white shadow-sm transition-colors sm:hidden dark:border-gray-600 dark:bg-gray-800 dark:text-gray-200 hover:bg-gray-100 hover:text-gray-900 dark:hover:bg-gray-700 dark:hover:text-gray-100"
                    attr:aria-label=move || { t_string!(i18n, messages.page.back_to_conversations) }
                >
                    <span class="text-base" aria-hidden="true">
                        "←"
                    </span>
                    <span>{t!(i18n, messages.page.conversations)}</span>
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
fn MessagesStatusFrame(title: Signal<String>, message: Signal<String>) -> impl IntoView {
    view! {
        <MessagesThreadFrame title>
            <MessagesStatusContent message />
        </MessagesThreadFrame>
    }
}

#[component]
fn MessagesStatusContent(message: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-col flex-1 gap-2 justify-center items-center p-8 text-gray-500 dark:text-gray-400">
            <p class="max-w-xs text-sm font-medium text-center">{move || message.get()}</p>
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
            <Show when=open>
                <div class=SECTION_LIST_MAX_H>
                    {if is_empty {
                        Either::Left(
                            view! {
                                <Show when=move || !empty_label.get().is_empty()>
                                    <p class=EMPTY_HINT_CLASS>{move || empty_label.get()}</p>
                                </Show>
                            },
                        )
                    } else {
                        Either::Right(
                            view! { {move || children.with_value(|children| children())} },
                        )
                    }}
                </div>
            </Show>
        </section>
    }
}

#[component]
fn ChannelListSection<T, F, IV>(
    title: Signal<String>,
    empty_label: Signal<String>,
    items: Vec<T>,
    render_item: F,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
    F: Fn(T) -> IV + Copy + Send + Sync + 'static,
    IV: IntoView + 'static,
{
    let open = RwSignal::new(true);
    let items = StoredValue::new(items);
    let is_empty = items.with_value(|items| items.is_empty());

    view! {
        <ChannelSection title open=open is_empty=is_empty empty_label=empty_label>
            {move || {
                items.with_value(|items| { items.iter().cloned().map(render_item).collect_view() })
            }}
        </ChannelSection>
    }
}
