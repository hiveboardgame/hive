use crate::{
    components::{
        atoms::{block_toggle_button::BlockToggleButton, unread_badge::UnreadBadge},
        molecules::{
            empty_state::EmptyState,
            game_thread_toggle::{GameThreadToggle, GameThreadToggleSize},
        },
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
    prelude::*,
};
use leptos_icons::Icon;
use leptos_router::{
    components::{Outlet, Redirect, A},
    hooks::{use_location, use_navigate, use_params_map},
    NavigateOptions,
};
use shared_types::{
    ChatDestination,
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
use std::future::Future;
use uuid::Uuid;

// Messages hub: /message routes are the source of truth for the open thread.

#[derive(Clone)]
enum ResolveState<K, V> {
    Missing(Option<K>),
    Ready(K, V),
}

#[derive(Clone, PartialEq)]
struct VersionedRouteData<T> {
    session_epoch: u64,
    data: Option<T>,
}

#[derive(Clone, PartialEq)]
struct DmRouteData {
    other_user_id: Uuid,
    username: String,
    peer_deleted: bool,
}

#[derive(Clone, PartialEq)]
struct TournamentRouteData {
    name: String,
    muted: bool,
    access: TournamentChatCapabilities,
}

const MESSAGE_ROOT_PATH: &str = "/message";
const MESSAGE_GLOBAL_PATH: &str = "/message/global";
const SELF_DM_UNSUPPORTED_MESSAGE: &str = "Direct messages to yourself are not supported";
const DELETED_DM_PEER_MESSAGE: &str = "This account has been deleted.";
const PLAYERS_CHAT_ONLY_MESSAGE: &str = "Only players can view the players chat.";
const HEADER_ACTION_BUTTON_PRIMARY: &str =
    "no-link-style ui-button ui-button-secondary ui-button-sm";
const MESSAGES_SHELL_CLASS: &str = "fixed inset-x-0 bottom-0 top-10 z-0 flex overflow-hidden flex-col border-t border-black/10 bg-light dark:border-white/10 dark:bg-surface-muted sm:flex-row";
const MESSAGES_SIDEBAR_PANE_CLASS: &str = "flex min-h-0 w-full flex-shrink-0 flex-col overflow-hidden border-black/10 bg-light dark:border-white/10 dark:bg-surface-muted sm:w-72 sm:border-r";
const MESSAGES_THREAD_PANE_CLASS: &str =
    "flex min-h-0 flex-1 flex-col overflow-hidden bg-light dark:bg-surface-muted";
const MESSAGES_INDEX_PANE_CLASS: &str =
    "hidden min-h-0 flex-1 flex-col overflow-hidden bg-light dark:bg-surface-muted sm:flex";
const MESSAGES_PRIMARY_HEADER_CLASS: &str = "flex min-h-11 items-center justify-between gap-3 border-b border-black/10 bg-light px-3 py-2.5 dark:border-white/10 dark:bg-surface-muted xs:px-4 xs:py-3";
const MESSAGES_SUBHEADER_CLASS: &str = "flex shrink-0 items-center justify-between gap-3 border-b border-black/10 bg-light px-3 py-2.5 dark:border-white/10 dark:bg-surface-muted xs:px-4 xs:py-3";
const MESSAGES_CHAT_BODY_CLASS: &str =
    "overflow-hidden flex-1 min-h-0 bg-even-light/95 dark:bg-surface-panel";

fn normalized_message_path(path: &str) -> &str {
    match path.trim_end_matches('/') {
        "" => MESSAGE_ROOT_PATH,
        path => path,
    }
}

fn message_path_is(path: &str, href: &str) -> bool {
    normalized_message_path(path) == href
}

fn message_dm_href(username: &str) -> String {
    format!("/message/dm/{username}")
}

fn message_tournament_href(tournament_id: &TournamentId) -> String {
    format!("/message/tournament/{}", tournament_id.0)
}

fn message_game_href(game_id: &GameId, thread: GameThread) -> String {
    format!("/message/game/{}/{}", game_id.0, thread.slug())
}

fn message_path_matches_game(
    path: &str,
    game_id: &GameId,
    thread: GameThread,
    finished: bool,
) -> bool {
    if finished {
        message_path_is(path, &message_game_href(game_id, GameThread::Players))
            || message_path_is(path, &message_game_href(game_id, GameThread::Spectators))
    } else {
        message_path_is(path, &message_game_href(game_id, thread))
    }
}

fn route_param(name: &'static str) -> Signal<Option<String>> {
    let params = use_params_map();
    Signal::derive(move || params.get().get(name))
}

impl<K, V> ResolveState<K, V> {
    fn key(&self) -> Option<&K> {
        match self {
            Self::Missing(key) => key.as_ref(),
            Self::Ready(key, _) => Some(key),
        }
    }
}

impl<T> VersionedRouteData<T> {
    fn new(session_epoch: u64, data: Option<T>) -> Self {
        Self {
            session_epoch,
            data,
        }
    }
}

impl From<DmConversation> for DmRouteData {
    fn from(dm: DmConversation) -> Self {
        Self {
            other_user_id: dm.other_user_id,
            username: dm.username,
            peer_deleted: dm.peer_deleted,
        }
    }
}

impl From<(String, bool, TournamentChatCapabilities)> for TournamentRouteData {
    fn from((name, muted, access): (String, bool, TournamentChatCapabilities)) -> Self {
        Self {
            name,
            muted,
            access,
        }
    }
}

fn resolve_from_cache_or_fetch<K, V, D, CacheDependency, Lookup, Fetch, Fut>(
    current_key: Signal<Option<K>>,
    cache_dependency: CacheDependency,
    lookup: Lookup,
    fetch: Fetch,
) -> LocalResource<ResolveState<K, V>>
where
    K: Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    CacheDependency: Get<Value = D> + Send + Sync + 'static,
    Lookup: Fn(&K) -> Option<V> + Send + Sync + 'static,
    Fetch: Fn(K) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<V>> + 'static,
{
    let lookup = StoredValue::new(lookup);
    let fetch = StoredValue::new(fetch);
    LocalResource::new(move || {
        let current_key = current_key.get();
        let _cache_dependency = cache_dependency.get();
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

fn find_dm_conversation(hub_data: &MessagesHubData, username: &str) -> Option<DmConversation> {
    hub_data
        .dms
        .iter()
        .find(|dm| dm.username == username)
        .cloned()
}

fn dm_route_data_from_hub(hub_data: &MessagesHubData, username: &str) -> Option<DmRouteData> {
    find_dm_conversation(hub_data, username).map(DmRouteData::from)
}

fn dm_display_name(dm: &DmConversation) -> String {
    if dm.peer_deleted {
        "Deleted user".to_string()
    } else {
        dm.username.clone()
    }
}

fn find_tournament_channel(
    hub_data: &MessagesHubData,
    tournament_id: &TournamentId,
) -> Option<TournamentChannel> {
    hub_data
        .tournaments
        .iter()
        .find(|channel| &channel.tournament_id == tournament_id)
        .cloned()
}

fn tournament_route_data_from_hub(
    hub_data: &MessagesHubData,
    tournament_id: &TournamentId,
) -> Option<TournamentRouteData> {
    let muted = hub_data.muted_tournament_ids.contains(tournament_id);
    find_tournament_channel(hub_data, tournament_id).map(|channel| TournamentRouteData {
        name: channel.name,
        muted,
        access: channel.access,
    })
}

fn find_game_channel(hub_data: &MessagesHubData, game_id: &GameId) -> Option<GameChannel> {
    hub_data
        .games
        .iter()
        .find(|channel| channel.game_id == *game_id)
        .cloned()
}

fn game_route_data_from_hub(
    hub_data: &MessagesHubData,
    game_id: &GameId,
) -> Option<GameChatCapabilities> {
    find_game_channel(hub_data, game_id).map(|game| game.access)
}

fn dm_route_cache_dependency(
    chat: Chat,
    route_username: Signal<Option<String>>,
) -> Memo<VersionedRouteData<DmRouteData>> {
    Memo::new(move |_| {
        let selected = route_username.get().and_then(|username| {
            chat.messages_hub_data.with(|hub| {
                hub.as_ref()
                    .and_then(|hub| dm_route_data_from_hub(hub, &username))
            })
        });
        VersionedRouteData::new(chat.session_epoch(), selected)
    })
}

fn tournament_route_cache_dependency(
    chat: Chat,
    route_tournament_id: Signal<Option<TournamentId>>,
) -> Memo<VersionedRouteData<TournamentRouteData>> {
    Memo::new(move |_| {
        let selected = route_tournament_id.get().and_then(|tournament_id| {
            chat.messages_hub_data.with(|hub| {
                hub.as_ref()
                    .and_then(|hub| tournament_route_data_from_hub(hub, &tournament_id))
            })
        });
        VersionedRouteData::new(chat.session_epoch(), selected)
    })
}

fn game_route_cache_dependency(
    chat: Chat,
    route_game_thread: Signal<Option<(GameId, GameThread)>>,
) -> Memo<VersionedRouteData<GameChatCapabilities>> {
    Memo::new(move |_| {
        let selected = route_game_thread.get().and_then(|(game_id, _)| {
            chat.messages_hub_data.with(|hub| {
                hub.as_ref()
                    .and_then(|hub| game_route_data_from_hub(hub, &game_id))
            })
        });
        VersionedRouteData::new(chat.session_epoch(), selected)
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
        None => EitherOf3::A(
            view! { <MessagesStatusFrame title=loading_title message=loading_message /> },
        ),
        Some(state) if current_key.get().as_ref() != state.key() => EitherOf3::A(
            view! { <MessagesStatusFrame title=loading_title message=loading_message /> },
        ),
        Some(ResolveState::Missing(_)) => EitherOf3::B(
            view! { <MessagesStatusFrame title=missing_title message=missing_message /> },
        ),
        Some(ResolveState::Ready(key, value)) => {
            EitherOf3::C(render_ready.with_value(|render_ready| render_ready(key, value)))
        }
    }
}

#[component]
pub fn MessagesLayout() -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let current_path = Signal::derive(move || location.pathname.get());
    let mobile_list_visible =
        Signal::derive(move || message_path_is(&current_path.get(), MESSAGE_ROOT_PATH));
    let mobile_thread_visible =
        Signal::derive(move || !message_path_is(&current_path.get(), MESSAGE_ROOT_PATH));

    view! {
        <div class=MESSAGES_SHELL_CLASS>
            <aside class=move || {
                format!(
                    "{MESSAGES_SIDEBAR_PANE_CLASS} {} sm:!flex",
                    if mobile_list_visible.get() { "" } else { "hidden " },
                )
            }>
                <div class=MESSAGES_PRIMARY_HEADER_CLASS>
                    <h1 class="text-xl ui-page-title">{t!(i18n, messages.page.title)}</h1>
                </div>
                <div class="overflow-y-auto flex-1 p-2 pb-6 min-h-0 sm:pb-2">
                    <MessagesSidebar current_path />
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
fn MessagesSidebar(current_path: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let dms_title = Signal::derive(move || t_string!(i18n, messages.sections.dms).to_string());
    let dms_empty_label =
        Signal::derive(move || t_string!(i18n, messages.sections.no_dms).to_string());
    let tournaments_title =
        Signal::derive(move || t_string!(i18n, messages.sections.tournaments).to_string());
    let tournaments_empty_label =
        Signal::derive(move || t_string!(i18n, messages.sections.no_tournament_chats).to_string());
    let games_title = Signal::derive(move || t_string!(i18n, messages.sections.games).to_string());
    let games_empty_label =
        Signal::derive(move || t_string!(i18n, messages.sections.no_game_chats).to_string());
    view! {
        {move || match chat.messages_hub_data.get() {
            Some(hub) => {
                EitherOf3::A(
                    view! {
                        <ChannelListSection
                            title=dms_title
                            empty_label=dms_empty_label
                            items=hub.dms
                            render_item=move |dm| view! { <DmChannelItem dm current_path /> }
                        />
                        <ChannelListSection
                            title=tournaments_title
                            empty_label=tournaments_empty_label
                            items=hub.tournaments
                            render_item=move |tournament| {
                                view! { <TournamentChannelItem tournament current_path /> }
                            }
                        />
                        <ChannelListSection
                            title=games_title
                            empty_label=games_empty_label
                            items=hub.games
                            render_item=move |game| view! { <GameChannelItem game current_path /> }
                        />
                        <GlobalChannelSection current_path />
                    },
                )
            }
            None if chat.messages_hub_loading.get() => {
                EitherOf3::B(
                    view! {
                        <p class="p-3 animate-pulse ui-field-helper">
                            {t!(i18n, messages.page.loading)}
                        </p>
                    },
                )
            }
            None => {
                EitherOf3::C(
                    view! {
                        <p class="p-3 ui-field-error">
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
        <div class=MESSAGES_INDEX_PANE_CLASS>
            <div class="flex flex-1 justify-center items-center p-4">
                <EmptyState
                    title=move || t_string!(i18n, messages.page.select_conversation).to_string()
                    message=move || t_string!(i18n, messages.page.choose_conversation).to_string()
                    class="max-w-sm"
                />
            </div>
        </div>
    }
}

#[component]
pub fn MessagesGlobalThread() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let disabled = Signal::derive(move || {
        auth.user
            .with(|user| !user.as_ref().is_some_and(|account| account.user.admin))
    });
    view! {
        <MessagesThreadFrame title=Signal::derive(move || {
            t_string!(i18n, messages.sections.recent_announcements).to_string()
        })>
            <div class=MESSAGES_CHAT_BODY_CLASS>
                <ResolvedChatWindow destination=ChatDestination::Global input_disabled=disabled />
            </div>
        </MessagesThreadFrame>
    }
}

#[component]
pub fn MessagesDmThread() -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let route_username = route_param("username");
    let loading_message =
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string());
    let failed_message =
        Signal::derive(move || t_string!(i18n, messages.page.failed_conversations).to_string());
    let route_cache_dependency = dm_route_cache_dependency(chat, route_username);
    let resolved = resolve_from_cache_or_fetch(
        route_username,
        route_cache_dependency,
        move |username: &String| {
            chat.messages_hub_data.with_untracked(|hub| {
                hub.as_ref()
                    .and_then(|hub| dm_route_data_from_hub(hub, username))
            })
        },
        |username| async move {
            resolve_username(username)
                .await
                .ok()
                .map(|user| DmRouteData {
                    other_user_id: user.uid,
                    username: user.username,
                    peer_deleted: user.deleted,
                })
        },
    );

    resolved_route_shell(
        route_username,
        resolved,
        loading_message,
        loading_message,
        failed_message,
        failed_message,
        move |_, route_data| {
            view! {
                <MessagesResolvedDmView
                    loading_message
                    failed_message
                    other_user_id=route_data.other_user_id
                    username=route_data.username
                    peer_deleted=route_data.peer_deleted
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
    peer_deleted: bool,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let title = StoredValue::new(if peer_deleted {
        "Deleted user".to_string()
    } else {
        username.clone()
    });
    let username_store = StoredValue::new(username);
    let input_disabled = Signal::derive(move || peer_deleted);
    let destination =
        Signal::derive(move || ChatDestination::User((other_user_id, username_store.get_value())));
    let current_user_id = Signal::derive(move || {
        auth.user
            .with(|account| account.as_ref().map(|account| account.user.uid))
    });
    let unavailable_message = Signal::derive(move || {
        if auth.logged_in.get().is_none() {
            loading_message.get()
        } else {
            failed_message.get()
        }
    });
    let self_dm_error = Signal::derive(|| SELF_DM_UNSUPPORTED_MESSAGE.to_string());
    view! {
        <MessagesThreadFrame title=Signal::derive(move || {
            title.get_value()
        })>
            {move || match current_user_id.get() {
                Some(current_user_id) if current_user_id != other_user_id => {
                    EitherOf3::A(
                        view! {
                            <DmActions other_user_id username=username_store peer_deleted />
                            <div class=MESSAGES_CHAT_BODY_CLASS>
                                <ResolvedChatWindow destination input_disabled />
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
fn DmActions(
    other_user_id: Uuid,
    username: StoredValue<String>,
    peer_deleted: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let is_blocked = Signal::derive(move || {
        chat.blocked_user_ids
            .with(|ids| ids.contains(&other_user_id))
    });
    view! {
        <ChannelHeaderBar>
            {if peer_deleted {
                Either::Left(
                    view! {
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                            {DELETED_DM_PEER_MESSAGE}
                        </p>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <div class="flex flex-wrap gap-2 items-center">
                            <A
                                href=move || format!("/@/{}", username.get_value())
                                attr:class=HEADER_ACTION_BUTTON_PRIMARY
                            >
                                {t!(i18n, messages.page.view_profile)}
                            </A>
                            <BlockToggleButton blocked_user_id=other_user_id is_blocked />
                        </div>
                    },
                )
            }}
        </ChannelHeaderBar>
    }
}

#[component]
pub fn MessagesTournamentThread() -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let route_nanoid = route_param("nanoid");
    let route_tournament_id = Signal::derive(move || route_nanoid.get().map(TournamentId));
    let route_cache_dependency = tournament_route_cache_dependency(chat, route_tournament_id);
    let resolved = resolve_from_cache_or_fetch(
        route_tournament_id,
        route_cache_dependency,
        move |tournament_id: &TournamentId| chat_tournament_from_hub(chat, tournament_id),
        |tournament_id: TournamentId| async move {
            get_tournament_route_data(tournament_id.0)
                .await
                .ok()
                .map(TournamentRouteData::from)
        },
    );

    resolved_route_shell(
        route_tournament_id,
        resolved,
        Signal::derive(move || t_string!(i18n, messages.chat.tournament_title).to_string()),
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string()),
        Signal::derive(move || t_string!(i18n, messages.chat.tournament_title).to_string()),
        Signal::derive(move || t_string!(i18n, messages.page.failed_conversations).to_string()),
        |tournament_id, route_data| {
            view! {
                <MessagesResolvedTournamentView
                    tournament_id
                    title=route_data.name
                    muted=route_data.muted
                    access=route_data.access
                />
            }
        },
    )
}

fn chat_tournament_from_hub(
    chat: Chat,
    tournament_id: &TournamentId,
) -> Option<TournamentRouteData> {
    chat.messages_hub_data.with_untracked(|hub| {
        hub.as_ref()
            .and_then(|hub| tournament_route_data_from_hub(hub, tournament_id))
    })
}

#[component]
fn MessagesResolvedTournamentView(
    tournament_id: TournamentId,
    title: String,
    muted: bool,
    access: TournamentChatCapabilities,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let title = StoredValue::new(title);
    let tournament_id = StoredValue::new(tournament_id);
    Effect::new(move |_| {
        chat.set_tournament_muted(&tournament_id.get_value(), muted);
    });
    let destination =
        Signal::derive(move || ChatDestination::TournamentLobby(tournament_id.get_value()));
    view! {
        <MessagesThreadFrame title=Signal::derive(move || {
            title.get_value()
        })>
            {if access.can_read() {
                Either::Left(
                    view! {
                        <TournamentActions tournament_id=tournament_id.get_value() />
                        <div class=MESSAGES_CHAT_BODY_CLASS>
                            <ResolvedChatWindow destination />
                        </div>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <MessagesStatusContent message=Signal::derive(move || {
                            t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
                        }) />
                    },
                )
            }}
        </MessagesThreadFrame>
    }
}

#[component]
fn TournamentActions(tournament_id: TournamentId) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let tournament_id = StoredValue::new(tournament_id);
    let muted = chat.tournament_muted_signal(tournament_id.get_value());
    let error = RwSignal::new(None::<String>);
    let toggle = Action::new(move |currently_muted: &bool| {
        let currently_muted = *currently_muted;
        async move {
            if currently_muted {
                unmute_tournament_chat(tournament_id.get_value().0)
                    .await
                    .map(|_| false)
            } else {
                mute_tournament_chat(tournament_id.get_value().0)
                    .await
                    .map(|_| true)
            }
        }
    });
    Effect::watch(
        toggle.version(),
        move |_, _, _| {
            let Some(result) = toggle.value().get_untracked() else {
                return;
            };
            match result {
                Ok(new_muted) => {
                    error.set(None);
                    if chat
                        .set_tournament_muted_authoritative(&tournament_id.get_value(), new_muted)
                    {
                        chat.refresh_messages_hub_silent();
                    }
                }
                Err(err) => error.set(Some(err.to_string())),
            }
        },
        false,
    );
    let button_label = Signal::derive(move || {
        if toggle.pending().get() {
            t_string!(i18n, messages.page.loading)
        } else if muted.get() {
            t_string!(i18n, messages.page.unmute_tournament_chat)
        } else {
            t_string!(i18n, messages.page.mute_tournament_chat)
        }
    });
    view! {
        <ChannelHeaderBar>
            <div class="flex flex-wrap gap-2 items-center">
                <A
                    href=move || format!("/tournament/{}", tournament_id.get_value().0)
                    attr:class=HEADER_ACTION_BUTTON_PRIMARY
                >
                    {t!(i18n, messages.page.view_tournament)}
                </A>
                <button
                    type="button"
                    disabled=toggle.pending()
                    class=move || {
                        if muted.get() {
                            "ui-button ui-button-secondary ui-button-sm"
                        } else {
                            "ui-button ui-button-danger ui-button-sm"
                        }
                    }
                    on:click=move |_| {
                        error.set(None);
                        toggle.dispatch(muted.get_untracked());
                    }
                >
                    {button_label}
                </button>
                <ShowLet some=move || error.get() let:error>
                    <span class="ui-field-error">{error}</span>
                </ShowLet>
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
pub fn MessagesGameThread(thread: GameThread) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let route_nanoid = route_param("nanoid");
    let route_game_thread =
        Signal::derive(move || route_nanoid.get().map(|nanoid| (GameId(nanoid), thread)));
    let route_cache_dependency = game_route_cache_dependency(chat, route_game_thread);
    let resolved = resolve_from_cache_or_fetch(
        route_game_thread,
        route_cache_dependency,
        move |(game_id, _): &(GameId, GameThread)| {
            chat.messages_hub_data.with_untracked(|hub| {
                hub.as_ref()
                    .and_then(|hub| game_route_data_from_hub(hub, game_id))
            })
        },
        |(game_id, _)| async move { get_game_chat_route_data(game_id).await.ok() },
    );
    resolved_route_shell(
        route_game_thread,
        resolved,
        Signal::derive(move || match thread {
            GameThread::Players => t_string!(i18n, messages.chat.players_chat).to_string(),
            GameThread::Spectators => t_string!(i18n, messages.chat.spectator_chat).to_string(),
        }),
        Signal::derive(move || t_string!(i18n, messages.page.loading).to_string()),
        Signal::derive(move || match thread {
            GameThread::Players => t_string!(i18n, messages.chat.players_chat).to_string(),
            GameThread::Spectators => t_string!(i18n, messages.chat.spectator_chat).to_string(),
        }),
        Signal::derive(move || t_string!(i18n, messages.page.failed_game).to_string()),
        |(game_id, thread), access| {
            if thread == GameThread::Players
                && !access.can_read(GameThread::Players)
                && access.can_read(GameThread::Spectators)
            {
                Either::Left(view! {
                    <Redirect
                        path=message_game_href(&game_id, GameThread::Spectators)
                        options=NavigateOptions {
                            replace: true,
                            ..Default::default()
                        }
                    />
                })
            } else {
                Either::Right(view! { <MessagesResolvedGameView game_id thread access /> })
            }
        },
    )
}

#[component]
fn MessagesResolvedGameView(
    game_id: GameId,
    thread: GameThread,
    access: GameChatCapabilities,
) -> impl IntoView {
    let i18n = use_i18n();
    let game_id = StoredValue::new(game_id);
    let destination = Signal::derive(move || match thread {
        GameThread::Players => ChatDestination::GamePlayers(game_id.get_value()),
        GameThread::Spectators => ChatDestination::GameSpectators(game_id.get_value()),
    });
    let title = Signal::derive(move || match thread {
        GameThread::Players => t_string!(i18n, messages.chat.players_chat).to_string(),
        GameThread::Spectators => t_string!(i18n, messages.chat.spectator_chat).to_string(),
    });
    view! {
        <MessagesThreadFrame title>
            <GameActions game_id=game_id.get_value() thread access />
            {if access.can_read(thread) {
                Either::Left(
                    view! {
                        <div class=MESSAGES_CHAT_BODY_CLASS>
                            <ResolvedChatWindow destination />
                        </div>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <MessagesStatusContent message=Signal::derive(move || {
                            if thread == GameThread::Players {
                                PLAYERS_CHAT_ONLY_MESSAGE.to_string()
                            } else {
                                t_string!(i18n, messages.chat.spectator_unlock).to_string()
                            }
                        }) />
                    },
                )
            }}
        </MessagesThreadFrame>
    }
}

#[component]
fn GameActions(game_id: GameId, thread: GameThread, access: GameChatCapabilities) -> impl IntoView {
    let i18n = use_i18n();
    let navigate = use_navigate();
    let selected = RwSignal::new(thread);
    let game_id = StoredValue::new(game_id);
    let players_href =
        StoredValue::new(message_game_href(&game_id.get_value(), GameThread::Players));
    let spectators_href = StoredValue::new(message_game_href(
        &game_id.get_value(),
        GameThread::Spectators,
    ));
    let on_select = Callback::new(move |thread| {
        let href = match thread {
            GameThread::Players => players_href.get_value(),
            GameThread::Spectators => spectators_href.get_value(),
        };
        navigate(
            &href,
            NavigateOptions {
                replace: true,
                scroll: false,
                ..Default::default()
            },
        );
    });
    let spectator_unlock_needed =
        access.can_toggle_embedded_threads() && !access.can_read(GameThread::Spectators);
    view! {
        <ChannelHeaderBar>
            <div class="flex flex-col gap-2">
                <div class="flex flex-wrap gap-2 items-center">
                    <A
                        href=move || format!("/game/{}", game_id.get_value().0)
                        attr:class=HEADER_ACTION_BUTTON_PRIMARY
                    >
                        {t!(i18n, messages.page.view_game)}
                    </A>
                    <GameThreadToggle
                        selected
                        players_enabled=Signal::derive(move || access.can_read(GameThread::Players))
                        spectators_enabled=Signal::derive(move || {
                            access.can_read(GameThread::Spectators)
                        })
                        size=GameThreadToggleSize::Route
                        on_select
                    />
                </div>
                <Show when=move || spectator_unlock_needed>
                    <p class="text-xs text-gray-500 dark:text-gray-400">
                        {t!(i18n, messages.chat.spectator_unlock)}
                    </p>
                </Show>
            </div>
        </ChannelHeaderBar>
    }
}

#[component]
fn DmChannelItem(dm: DmConversation, current_path: Signal<String>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let href = message_dm_href(&dm.username);
    let selected_href = href.clone();
    let other_user_id = dm.other_user_id;
    let label = StoredValue::new(dm_display_name(&dm));
    let unread = Signal::derive(move || chat.unread_count_for_dm(other_user_id));
    view! {
        <MessagesChannelLink
            href
            is_selected=Signal::derive(move || message_path_is(&current_path.get(), &selected_href))
        >
            <span class="truncate">{label.get_value()}</span>
            <UnreadBadge count=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn TournamentChannelItem(
    tournament: TournamentChannel,
    current_path: Signal<String>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let tournament_id = StoredValue::new(tournament.tournament_id);
    let muted = chat.tournament_muted_signal(tournament_id.get_value());
    let unread =
        Signal::derive(move || chat.unread_count_for_tournament(&tournament_id.get_value()));
    let href = message_tournament_href(&tournament_id.get_value());
    let selected_href = href.clone();
    view! {
        <MessagesChannelLink
            href
            is_selected=Signal::derive(move || message_path_is(&current_path.get(), &selected_href))
        >
            <span class="flex gap-1 items-center truncate">
                {tournament.name} <Show when=muted>
                    <span
                        class="text-gray-400 uppercase dark:text-gray-500 shrink-0 text-[0.65rem]"
                        title=move || t_string!(i18n, messages.sections.muted)
                    >
                        {t!(i18n, messages.sections.muted)}
                    </span>
                </Show>
            </span>
            <UnreadBadge count=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn GameChannelItem(game: GameChannel, current_path: Signal<String>) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let GameChannel {
        game_id,
        label,
        access,
        ..
    } = game;
    let game_id = StoredValue::new(game_id);
    let href = message_game_href(&game_id.get_value(), GameThread::Players);
    let display_label_with_nanoid =
        StoredValue::new(format!("{} ({})", label, game_id.get_value().0));
    let unread = Signal::derive(move || chat.unread_count_for_game(&game_id.get_value()));
    let is_selected = Signal::derive(move || {
        message_path_matches_game(
            &current_path.get(),
            &game_id.get_value(),
            GameThread::Players,
            access.finished,
        )
    });
    view! {
        <MessagesChannelLink href is_selected>
            <span class="truncate" title=display_label_with_nanoid.get_value()>
                {display_label_with_nanoid.get_value()}
            </span>
            <UnreadBadge count=unread />
        </MessagesChannelLink>
    }
}

#[component]
fn GlobalChannelSection(current_path: Signal<String>) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(true);
    let is_selected =
        Signal::derive(move || message_path_is(&current_path.get(), MESSAGE_GLOBAL_PATH));

    view! {
        <ChannelSection
            title=Signal::derive(move || {
                t_string!(i18n, messages.sections.recent_announcements).to_string()
            })
            open=open
        >
            <MessagesChannelLink href=MESSAGE_GLOBAL_PATH.to_string() is_selected>
                <span class="truncate">{t!(i18n, messages.sections.recent_announcements)}</span>
            </MessagesChannelLink>
        </ChannelSection>
    }
}

#[component]
fn MessagesThreadFrame(title: Signal<String>, children: Children) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class=MESSAGES_THREAD_PANE_CLASS>
            <div class=MESSAGES_PRIMARY_HEADER_CLASS>
                <A
                    href=MESSAGE_ROOT_PATH
                    prop:replace=true
                    scroll=false
                    attr:class="no-link-style ui-button ui-button-secondary ui-button-sm flex-shrink-0 sm:hidden"
                    attr:aria-label=move || t_string!(i18n, messages.page.back_to_conversations)
                >
                    <Icon icon=icondata_lu::LuArrowLeft attr:class="size-4 shrink-0" />
                    <span>{t!(i18n, messages.page.conversations)}</span>
                </A>
                <h2 class="flex-1 min-w-0 text-lg font-bold text-gray-900 dark:text-gray-100 truncate">
                    {move || title.get()}
                </h2>
            </div>
            {children()}
        </div>
    }
}

#[component]
fn MessagesStatusFrame(
    #[prop(into)] title: Signal<String>,
    #[prop(into)] message: Signal<String>,
) -> impl IntoView {
    view! {
        <MessagesThreadFrame title>
            <MessagesStatusContent message />
        </MessagesThreadFrame>
    }
}

#[component]
fn MessagesStatusContent(#[prop(into)] message: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex flex-1 justify-center items-center p-4 bg-even-light/95 dark:bg-surface-panel">
            <EmptyState title=move || message.get() class="max-w-sm" />
        </div>
    }
}

#[component]
fn ChannelHeaderBar(children: Children) -> impl IntoView {
    view! { <div class=MESSAGES_SUBHEADER_CLASS>{children()}</div> }
}

const SECTION_LIST_MAX_H: &str = "overflow-y-auto min-h-0 max-h-48";
const SECTION_HEADER_BUTTON_CLASS: &str = "sticky top-0 z-10 flex justify-between items-center w-full text-xs text-left text-gray-600 uppercase rounded-none dark:text-gray-300 ui-disclosure-summary min-h-10";
const EMPTY_HINT_CLASS: &str = "py-1.5 px-2 text-sm italic text-gray-500 dark:text-gray-400";
const CHANNEL_BUTTON_BASE_CLASS: &str =
    "no-link-style flex min-h-10 w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm transition-colors duration-200";
const CHANNEL_BUTTON_SELECTED_CLASS: &str = "ui-segmented-active font-bold";
const CHANNEL_BUTTON_IDLE_CLASS: &str =
    "text-gray-700 hover:bg-blue-light/70 dark:text-gray-200 dark:hover:bg-pillbug-teal/15";

fn channel_button_class(is_selected: bool) -> String {
    format!(
        "{} {}",
        CHANNEL_BUTTON_BASE_CLASS,
        if is_selected {
            CHANNEL_BUTTON_SELECTED_CLASS
        } else {
            CHANNEL_BUTTON_IDLE_CLASS
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
            attr:aria-current=move || is_selected.get().then_some("page")
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
                class=SECTION_HEADER_BUTTON_CLASS
                attr:aria-expanded=move || open.get().to_string()
                on:click=move |_| open.update(|state| *state = !*state)
            >
                <span>{move || title.get()}</span>
                {move || {
                    if open.get() {
                        Either::Left(
                            view! { <Icon icon=icondata_lu::LuChevronDown attr:class="size-4" /> },
                        )
                    } else {
                        Either::Right(
                            view! { <Icon icon=icondata_lu::LuChevronRight attr:class="size-4" /> },
                        )
                    }
                }}
            </button>
            <Show when=open>
                <div class=SECTION_LIST_MAX_H>
                    {if is_empty {
                        Either::Left(
                            view! {
                                <ShowLet
                                    some=move || {
                                        let empty_label = empty_label.get();
                                        (!empty_label.is_empty()).then_some(empty_label)
                                    }
                                    let:empty_label
                                >
                                    <p class=EMPTY_HINT_CLASS>{empty_label}</p>
                                </ShowLet>
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
        <ChannelSection title open=open is_empty empty_label=empty_label>
            {move || {
                items.with_value(|items| items.iter().cloned().map(render_item).collect_view())
            }}
        </ChannelSection>
    }
}
