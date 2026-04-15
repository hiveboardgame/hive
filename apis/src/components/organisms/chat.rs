use crate::{
    chat::{ChannelKey, SimpleDestination},
    components::update_from_event::update_from_input,
    i18n::*,
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use chrono::{Duration, Local};
use leptos::{html, leptos_dom::helpers::request_animation_frame, prelude::*, task::spawn_local};
use leptos_router::hooks::use_params_map;
use leptos_use::use_interval_fn;
use shared_types::{ChatDestination, ChatMessage, GameId};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Max time gap for grouping consecutive same-user messages. Messages further apart show the header again.
const MESSAGE_GROUP_MAX_GAP: Duration = Duration::minutes(2);
const SCROLL_BOTTOM_THRESHOLD_PX: i32 = 32;

fn is_scrolled_near_bottom(container: &web_sys::HtmlElement) -> bool {
    container.scroll_height() - container.client_height() - container.scroll_top()
        <= SCROLL_BOTTOM_THRESHOLD_PX
}

/// For each message, true if the "user · timestamp · turn" header should be shown (first of a consecutive same-user block).
/// Same-user messages are only grouped if they are within MESSAGE_GROUP_MAX_GAP (2 min) of each other.
pub(crate) fn messages_with_header_flags(messages: &[ChatMessage]) -> Vec<(ChatMessage, bool)> {
    messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let same_user = i > 0 && messages[i - 1].user_id == m.user_id;
            let gap_too_large = match (i.checked_sub(1).and_then(|j| messages.get(j)), m) {
                (Some(prev), curr) => match (prev.timestamp, curr.timestamp) {
                    (Some(prev_ts), Some(curr_ts)) => {
                        (curr_ts - prev_ts).abs() > MESSAGE_GROUP_MAX_GAP
                    }
                    _ => true,
                },
                _ => true,
            };
            let show_header = i == 0 || !same_user || gap_too_large;
            (m.clone(), show_header)
        })
        .collect()
}

fn unread_divider_split_idx(message_count: usize, unread_at_open: Option<i64>) -> Option<usize> {
    if message_count == 0 {
        return None;
    }

    let unread_count = unread_at_open.unwrap_or(0).max(0) as usize;
    if unread_count == 0 {
        None
    } else {
        Some(message_count.saturating_sub(unread_count))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MessageId {
    timestamp_millis: i64,
    user_id: Uuid,
    turn: Option<usize>,
    message: String,
}

#[derive(Clone, PartialEq)]
struct MessageRow {
    id: MessageId,
    message: ChatMessage,
    show_header: bool,
    is_current_user: bool,
    show_unread_divider_before: bool,
}

fn message_id(message: &ChatMessage) -> MessageId {
    MessageId {
        timestamp_millis: message.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0),
        user_id: message.user_id,
        turn: message.turn,
        message: message.message.clone(),
    }
}

#[derive(Clone, PartialEq, Eq)]
struct ResolvedDmThread {
    destination: ChatDestination,
    key: ChannelKey,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GameChatData {
    pub game_id: GameId,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub finished: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameChatThread {
    Players,
    Spectators,
}

#[component]
pub fn Message(
    message: ChatMessage,
    #[prop(optional)] is_current_user: bool,
    /// When false, only the message bubble is shown (for consecutive messages from the same user).
    #[prop(optional, default = true)]
    show_header: bool,
    /// When true, show "Message hidden. Click to expand." (Discord-style for blocked users in shared channels).
    #[prop(optional)]
    sender_blocked: bool,
    /// When sender_blocked, parent-owned expanded state so it survives re-renders. If None, component uses internal state.
    #[prop(optional)]
    expanded_signal: Option<Signal<bool>>,
    /// Callback when user clicks "Click to expand". Parent should add this message to its expanded set.
    #[prop(optional)]
    on_click_expand: Option<Callback<(), ()>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let ChatMessage {
        username,
        message: body,
        timestamp,
        turn,
        ..
    } = message;

    let expanded_inner = RwSignal::new(false);
    let is_expanded = move || {
        expanded_signal
            .as_ref()
            .map(|signal| signal.get())
            .unwrap_or_else(|| expanded_inner.get())
    };

    let on_expand = move |_| {
        if let Some(cb) = on_click_expand.as_ref() {
            cb.run(());
        } else {
            expanded_inner.set(true);
        }
    };

    let margin = if show_header { "mb-3" } else { "mb-1" };
    let outer_class = if is_current_user {
        format!("flex flex-col items-end {margin} w-full")
    } else {
        format!("flex flex-col items-start {margin} w-full")
    };

    let bubble_class = if is_current_user {
        "px-3 py-2 rounded-2xl rounded-br-md max-w-[85%] sm:max-w-[75%] \
         bg-pillbug-teal/90 dark:bg-pillbug-teal/80 text-white text-sm break-words shadow-sm"
    } else {
        "px-3 py-2 rounded-2xl rounded-bl-md max-w-[85%] sm:max-w-[75%] \
         bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 text-sm break-words shadow-sm"
    };

    let user_local_time = timestamp
        .map(|t| t.with_timezone(&Local).format("%d/%m %H:%M").to_string())
        .unwrap_or_default();
    let username = StoredValue::new(username);
    let body = StoredValue::new(body);
    let user_local_time = StoredValue::new(user_local_time);

    view! {
        <div class=outer_class>
            <Show when=move || sender_blocked && !is_expanded()>
                <button
                    type="button"
                    class="py-2 px-3 mb-1 text-sm text-left text-gray-500 bg-gray-100 rounded-lg border border-gray-200 transition-colors dark:text-gray-400 dark:bg-gray-800 dark:border-gray-600 hover:bg-gray-200 max-w-[85%] sm:max-w-[75%] dark:hover:bg-gray-700"
                    on:click=on_expand
                >
                    {t!(i18n, messages.chat.hidden_message)}
                </button>
            </Show>
            <Show when=move || !sender_blocked || is_expanded()>
                <Show when=move || show_header>
                    <div class="flex flex-wrap gap-y-0.5 gap-x-2 items-baseline px-1 mb-1 max-w-[85%] sm:max-w-[75%]">
                        <span class="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                            {username.get_value()}
                        </span>
                        <span class="text-xs text-gray-500 whitespace-nowrap dark:text-gray-400">
                            {user_local_time.get_value()}
                            {move || {
                                turn
                                    .map(|turn| t_string!(i18n, messages.chat.turn, turn = turn))
                                    .unwrap_or_default()
                            }}
                        </span>
                    </div>
                </Show>
                <div class=bubble_class>{body.get_value()}</div>
            </Show>
        </div>
    }
}

#[component]
pub fn ChatInput(
    destination: Signal<ChatDestination>,
    disabled: impl Fn() -> bool + 'static + Send + Sync,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let game_state = use_context::<GameStateSignal>();
    let turn = move || game_state.map(|gs| gs.signal.with(|state| state.state.turn));
    let is_disabled = Signal::derive(disabled);
    let is_disabled_send = is_disabled;
    let is_disabled_placeholder = is_disabled;
    let is_disabled_keydown = is_disabled;
    let send = move || {
        if is_disabled_send.get() {
            return;
        }
        let message = chat.typed_message.get();
        if !message.is_empty() {
            chat.send(&message, destination(), turn());
            chat.typed_message.set(String::new());
        };
    };
    let placeholder = move || {
        if is_disabled_placeholder.get() {
            t_string!(i18n, messages.chat.admin_only)
        } else {
            match destination() {
                ChatDestination::GamePlayers(_, _, _) => {
                    t_string!(i18n, messages.chat.with_opponent)
                }
                ChatDestination::GameSpectators(_, _, _) => {
                    t_string!(i18n, messages.chat.with_spectators)
                }
                _ => t_string!(i18n, messages.chat.placeholder),
            }
        }
    };
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    view! {
        <input
            node_ref=my_input
            type="text"
            disabled=move || is_disabled.get()
            class="py-3 px-4 w-full placeholder-gray-500 text-black bg-white rounded-xl border border-gray-300 shadow-inner transition-shadow dark:placeholder-gray-400 dark:text-white dark:bg-gray-800 dark:border-gray-600 focus:ring-2 focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed box-border shrink-0 focus:ring-pillbug-teal/50 focus:border-pillbug-teal"
            prop:value=chat.typed_message
            prop:placeholder=placeholder
            on:input=update_from_input(chat.typed_message)
            on:keydown=move |evt| {
                if evt.key() == "Enter" && !is_disabled_keydown.get() {
                    evt.prevent_default();
                    send();
                }
            }

            maxlength="1000"
        />
    }
}

#[component]
pub fn ChatComposer(
    destination: Signal<ChatDestination>,
    disabled: impl Fn() -> bool + 'static + Send + Sync,
) -> impl IntoView {
    view! {
        <div class="p-4 bg-white rounded-xl border border-gray-200 shadow-sm dark:bg-gray-900 dark:border-gray-700">
            <ChatInput destination disabled />
        </div>
    }
}

/// When provided for SimpleDestination::Game, overrides which channel to show: true = Players, false = Spectators.
/// Used on the game page so players can toggle. When None, destination is derived from uid_is_player.
#[component]
pub fn ChatWindow(
    destination: SimpleDestination,
    #[prop(optional)] correspondant_id: Option<Uuid>,
    #[prop(optional)] correspondant_username: String,
    /// When Some and true, input is disabled (e.g. tournament read-only for non-participants).
    #[prop(optional)]
    input_disabled: Option<Signal<bool>>,
    /// Optional explicit game metadata for non-game pages that still render a game chat thread.
    #[prop(optional)]
    game_data: Option<Signal<Option<GameChatData>>>,
    /// When Some, force a specific game chat thread; when None, fall back to uid_is_player.
    #[prop(optional)]
    explicit_game_thread: Option<Signal<Option<GameChatThread>>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params_map();
    let route_game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let chat = expect_context::<Chat>();
    let auth_context = expect_context::<AuthContext>();
    let game_state = use_context::<GameStateSignal>();
    let me_uid = Memo::new(move |_| auth_context.user.with(|a| a.as_ref().map(|a| a.user.uid)));
    let uid = auth_context
        .user
        .with_untracked(|a| a.as_ref().map(|user| user.user.uid));
    let destination_is_user = matches!(&destination, SimpleDestination::User);
    let white_id = move || {
        game_state
            .as_ref()
            .and_then(|gs| gs.signal.with(|state| state.white_id))
    };
    let black_id = move || {
        game_state
            .as_ref()
            .and_then(|gs| gs.signal.with(|state| state.black_id))
    };
    let game_finished = move || {
        game_state.as_ref().is_some_and(|gs| {
            gs.signal
                .with(|state| state.game_response.as_ref().is_some_and(|gr| gr.finished))
        })
    };
    let uid_is_player = move || {
        game_state
            .as_ref()
            .is_some_and(|gs| gs.signal.with(|state| state.uid_is_player(uid)))
    };

    let destination_kind = StoredValue::new(destination);
    let dm_peer_username = StoredValue::new(correspondant_username);
    let resolved_dm_thread = Memo::new(move |_| {
        let other_user_id = correspondant_id?;
        let current_user_id = me_uid.get()?;
        if current_user_id == other_user_id {
            return None;
        }

        Some(ResolvedDmThread {
            destination: ChatDestination::User((other_user_id, dm_peer_username.get_value())),
            key: ChannelKey::direct(current_user_id, other_user_id),
        })
    });
    let dm_thread_error = Memo::new(move |_| {
        if !destination_is_user {
            return None;
        }

        match (correspondant_id, me_uid.get()) {
            (Some(other_user_id), Some(current_user_id)) if current_user_id == other_user_id => {
                Some("Direct messages to yourself are not supported".to_string())
            }
            (Some(_), Some(_)) => None,
            _ => Some(t_string!(i18n, messages.page.failed_conversations).to_string()),
        }
    });
    let div = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();
    let unread_at_open = RwSignal::new(None::<i64>);
    let history_loading_channels = RwSignal::new(HashSet::<ChannelKey>::new());
    let history_errors = RwSignal::new(HashMap::<ChannelKey, String>::new());
    let show_jump_to_latest = RwSignal::new(false);
    let is_scrolled_to_bottom = RwSignal::new(true);
    let expanded_hidden_messages = RwSignal::new(HashSet::<MessageId>::new());

    let route_game_data = Memo::new(move |_| match (white_id(), black_id()) {
        (Some(white_id), Some(black_id)) => Some(GameChatData {
            game_id: route_game_id(),
            white_id,
            black_id,
            finished: game_finished(),
        }),
        _ => None,
    });
    let current_game_data = Memo::new(move |_| {
        game_data
            .as_ref()
            .and_then(|signal| signal.get())
            .or_else(|| route_game_data.get())
    });
    let game_state_ready = Memo::new(move |_| current_game_data.get().is_some());
    let selected_game_thread = Memo::new(move |_| {
        explicit_game_thread
            .as_ref()
            .and_then(|signal| signal.get())
    });
    let has_explicit_game_thread = Memo::new(move |_| selected_game_thread.get().is_some());
    let show_players_channel = Memo::new(move |_| {
        match selected_game_thread.get() {
            Some(GameChatThread::Players) => true,
            Some(GameChatThread::Spectators) => false,
            None => uid_is_player(),
        }
    });
    let actual_destination = Memo::new(move |_| match destination_kind.get_value() {
        SimpleDestination::Game => match current_game_data.get() {
            Some(GameChatData {
                game_id,
                white_id,
                black_id,
                finished: _finished,
            }) => {
                if show_players_channel.get() {
                    Some(ChatDestination::GamePlayers(game_id, white_id, black_id))
                } else {
                    Some(ChatDestination::GameSpectators(game_id, white_id, black_id))
                }
            }
            _ => None,
        },
        SimpleDestination::User => resolved_dm_thread.get().map(|thread| thread.destination),
        SimpleDestination::Global => Some(ChatDestination::Global),
        SimpleDestination::Tournament(tournament_id) => {
            Some(ChatDestination::TournamentLobby(tournament_id))
        }
    });
    let actual_destination_signal =
        Signal::derive(move || actual_destination.get().unwrap_or(ChatDestination::Global));
    let actual_channel_key = Memo::new(move |_| match destination_kind.get_value() {
        SimpleDestination::User => resolved_dm_thread.get().map(|thread| thread.key),
        _ => actual_destination
            .get()
            .as_ref()
            .and_then(|destination| ChannelKey::from_destination(destination, me_uid.get())),
    });
    let visible_history_error = Memo::new(move |_| {
        if destination_is_user {
            resolved_dm_thread
                .get()
                .and_then(|thread| history_errors.with(|errors| errors.get(&thread.key).cloned()))
        } else {
            actual_channel_key
                .get()
                .and_then(|key| history_errors.with(|errors| errors.get(&key).cloned()))
        }
    });
    let visible_thread_error = Memo::new(move |_| {
        if destination_is_user {
            dm_thread_error.get().or_else(|| {
                visible_history_error.get().map(|error| {
                    if error.contains("Access denied") {
                        t_string!(i18n, messages.page.failed_conversations).to_string()
                    } else {
                        error
                    }
                })
            })
        } else {
            visible_history_error.get().map(|error| {
                if error.contains("Access denied")
                    && matches!(
                        destination_kind.get_value(),
                        SimpleDestination::Tournament(_)
                    )
                {
                    t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
                } else {
                    error
                }
            })
        }
    });
    let chat_input_hidden = Memo::new(move |_| {
        dm_thread_error.get().is_some()
            || visible_history_error
                .get()
                .is_some_and(|error| error.contains("Access denied"))
    });
    let active_history_loading = Memo::new(move |_| {
        actual_channel_key
            .get()
            .is_some_and(|key| history_loading_channels.with(|loading| loading.contains(&key)))
    });
    let show_loading = Memo::new(move |_| {
        matches!(destination_kind.get_value(), SimpleDestination::Game) && !game_state_ready.get()
    });

    // Fetch chat history when the thread changes.
    Effect::watch(
        move || match destination_kind.get_value() {
            SimpleDestination::User => resolved_dm_thread
                .get()
                .map(|thread| vec![thread.key])
                .unwrap_or_default(),
            SimpleDestination::Global => vec![ChannelKey::global()],
            SimpleDestination::Tournament(tid) => {
                vec![ChannelKey::tournament(&tid)]
            }
            SimpleDestination::Game => match current_game_data.get() {
                Some(GameChatData {
                    game_id,
                    white_id: _white_id,
                    black_id: _black_id,
                    finished: true,
                }) => {
                    // Finished games eagerly preload both threads so players can toggle without
                    // a second round-trip. That is intentionally broader than the minimum fetch
                    // set for now because the UI responsiveness is worth the extra DB pressure.
                    vec![
                        ChannelKey::game_players(&game_id),
                        ChannelKey::game_spectators(&game_id),
                    ]
                }
                Some(GameChatData {
                    game_id,
                    white_id: _white_id,
                    black_id: _black_id,
                    finished: _finished,
                }) => {
                    let current_channel = if show_players_channel.get() {
                        ChannelKey::game_players(&game_id)
                    } else {
                        ChannelKey::game_spectators(&game_id)
                    };
                    vec![current_channel]
                }
                None => {
                    vec![]
                }
            },
        },
        move |channels, prev_channels, _| {
            if prev_channels.is_some_and(|prev| prev == channels) {
                return;
            }
            for key in channels {
                let chat = chat;
                let key = key.clone();
                history_loading_channels.update(|loading| {
                    loading.insert(key.clone());
                });
                history_errors.update(|errors| {
                    errors.remove(&key);
                });
                spawn_local(async move {
                    let result = chat.fetch_channel_history(&key).await;
                    history_loading_channels.update(|loading| {
                        loading.remove(&key);
                    });
                    match result {
                        Ok(messages) => {
                            history_errors.update(|errors| {
                                errors.remove(&key);
                            });
                            chat.inject_history(&key, messages);
                        }
                        Err(error) => {
                            history_errors.update(|errors| {
                                errors.insert(key.clone(), error);
                            });
                        }
                    }
                });
            }
        },
        true,
    );

    // Mark channel as read when destination changes and track which channel is visible.
    // Capture unread count before marking so we can show divider and scroll to first unread.
    Effect::watch(
        move || actual_channel_key.get(),
        move |channel_key, previous, _| {
            if let Some(previous_key) = previous.cloned().flatten() {
                chat.clear_channel_visible(&previous_key);
            }

            unread_at_open.set(None);
            show_jump_to_latest.set(false);
            is_scrolled_to_bottom.set(true);
            let Some(channel_key) = channel_key.clone() else {
                return;
            };
            let unread_key = channel_key.clone();
            let unread = untrack(move || chat.unread_count_for_channel(&unread_key));
            if unread > 0 {
                unread_at_open.set(Some(unread));
            }

            chat.set_channel_visible(&channel_key);
            chat.open_channel(&channel_key);
        },
        true,
    );
    on_cleanup(move || {
        if let Some(channel_key) = actual_channel_key.get_untracked() {
            chat.clear_channel_visible(&channel_key);
        }
    });

    // Clear divider after 10 seconds
    use_interval_fn(
        move || {
            if unread_at_open.get_untracked().is_some() {
                unread_at_open.set(None);
            }
        },
        10_000,
    );

    // Scroll: on destination change / initial load → first unread (if any) or bottom.
    // For new live messages in an already-open thread, only auto-scroll if the viewer was already near bottom.
    // Otherwise keep their scroll position and show a jump-to-latest affordance.
    Effect::watch(
        move || {
            let dest = actual_destination.get();
            let count = match &dest {
                Some(ChatDestination::TournamentLobby(tid)) => (chat.tournament_lobby_messages)()
                    .get(tid)
                    .map(|v| v.len())
                    .unwrap_or(0),
                Some(ChatDestination::GamePlayers(gid, ..)) => {
                    let private_count = (chat.games_private_messages)()
                        .get(gid)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let public_count = (chat.games_public_messages)()
                        .get(gid)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let finished = current_game_data.get().is_some_and(|game| game.finished);
                    let single_channel = has_explicit_game_thread.get();
                    if finished && !single_channel {
                        private_count + public_count
                    } else {
                        private_count
                    }
                }
                Some(ChatDestination::GameSpectators(gid, ..)) => {
                    let private_count = (chat.games_private_messages)()
                        .get(gid)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let public_count = (chat.games_public_messages)()
                        .get(gid)
                        .map(|v| v.len())
                        .unwrap_or(0);
                    let finished = current_game_data.get().is_some_and(|game| game.finished);
                    let single_channel = has_explicit_game_thread.get();
                    if finished && !single_channel {
                        private_count + public_count
                    } else {
                        public_count
                    }
                }
                Some(ChatDestination::User((id, _))) => (chat.users_messages)()
                    .get(id)
                    .map(|v| v.len())
                    .unwrap_or(0),
                Some(ChatDestination::Global) => chat.global_messages.get().len(),
                None => 0,
            };
            (dest, count)
        },
        move |(dest, count), prev, _| {
            let (run, dest_changed, is_new_message) = match prev {
                None => (true, true, false),
                Some((prev_dest, prev_count)) => {
                    let dest_changed = dest != prev_dest;
                    let count_increased = count > prev_count;
                    let is_new_message = count_increased && dest == prev_dest && *prev_count > 0;
                    (
                        dest_changed || count_increased,
                        dest_changed,
                        is_new_message,
                    )
                }
            };
            if !run {
                return;
            }
            let container = div.get_untracked();
            let target = first_unread_ref.get_untracked();
            let should_auto_scroll = is_scrolled_to_bottom.get_untracked();
            request_animation_frame(move || {
                if dest_changed || !is_new_message {
                    show_jump_to_latest.set(false);
                    if let Some(t) = target {
                        t.scroll_into_view_with_bool(true);
                    } else if let Some(c) = container.as_ref() {
                        c.set_scroll_top(c.scroll_height());
                    }
                    if let Some(c) = container.as_ref() {
                        let at_bottom = is_scrolled_near_bottom(c);
                        is_scrolled_to_bottom.set(at_bottom);
                        if at_bottom {
                            show_jump_to_latest.set(false);
                        }
                    }
                } else if should_auto_scroll {
                    if let Some(c) = container.as_ref() {
                        c.set_scroll_top(c.scroll_height());
                    }
                    is_scrolled_to_bottom.set(true);
                    show_jump_to_latest.set(false);
                } else {
                    show_jump_to_latest.set(true);
                }
            });
        },
        true,
    );

    let merged_messages = Memo::new(move |_| {
        let mut messages = match actual_destination.get() {
            Some(ChatDestination::TournamentLobby(tournament)) => chat
                .tournament_lobby_messages
                .with(|threads| threads.get(&tournament).cloned().unwrap_or_default()),
            Some(ChatDestination::GamePlayers(game_id, ..)) => {
                let finished = current_game_data.get().is_some_and(|game| game.finished);
                let single_channel = has_explicit_game_thread.get();
                if finished && !single_channel {
                    let mut merged = chat.games_private_messages.with(|threads| {
                        threads.get(&game_id).cloned().unwrap_or_default()
                    });
                    chat.games_public_messages.with(|threads| {
                        if let Some(public_messages) = threads.get(&game_id) {
                            merged.extend(public_messages.iter().cloned());
                        }
                    });
                    merged
                } else {
                    chat.games_private_messages.with(|threads| {
                        threads.get(&game_id).cloned().unwrap_or_default()
                    })
                }
            }
            Some(ChatDestination::GameSpectators(game_id, ..)) => {
                let finished = current_game_data.get().is_some_and(|game| game.finished);
                let single_channel = has_explicit_game_thread.get();
                if finished && !single_channel {
                    let mut merged = chat.games_private_messages.with(|threads| {
                        threads.get(&game_id).cloned().unwrap_or_default()
                    });
                    chat.games_public_messages.with(|threads| {
                        if let Some(public_messages) = threads.get(&game_id) {
                            merged.extend(public_messages.iter().cloned());
                        }
                    });
                    merged
                } else {
                    chat.games_public_messages.with(|threads| {
                        threads.get(&game_id).cloned().unwrap_or_default()
                    })
                }
            }
            Some(ChatDestination::User((correspondant_id, _username))) => chat
                .users_messages
                .with(|threads| threads.get(&correspondant_id).cloned().unwrap_or_default()),
            Some(ChatDestination::Global) => chat.global_messages.get(),
            None => Vec::new(),
        };
        messages.sort_by_key(|m| m.timestamp.map(|t| t.timestamp_millis()).unwrap_or(0));
        messages
    });

    let render_rows = Memo::new(move |_| {
        let messages = merged_messages.get();
        if messages.is_empty() {
            return Vec::new();
        }

        let split_idx = unread_divider_split_idx(messages.len(), unread_at_open.get());
        let current_user_id = me_uid.get();

        messages_with_header_flags(&messages)
            .into_iter()
            .enumerate()
            .map(|(idx, (message, show_header))| {
                let user_id = message.user_id;
                MessageRow {
                    id: message_id(&message),
                    message,
                    show_header,
                    is_current_user: current_user_id.is_some_and(|me| me == user_id),
                    show_unread_divider_before: split_idx == Some(idx),
                }
            })
            .collect::<Vec<_>>()
    });
    let scroll_to_latest = move |_| {
        if let Some(container) = div.get_untracked() {
            request_animation_frame(move || {
                container.set_scroll_top(container.scroll_height());
                is_scrolled_to_bottom.set(true);
                show_jump_to_latest.set(false);
            });
        }
    };

    view! {
        <div
            id="ignoreChat"
            class="flex overflow-hidden flex-col flex-grow justify-between w-full min-w-full max-w-full h-full min-h-0"
        >
            <div class="relative flex-grow w-full min-w-full h-0 min-h-0">
                <div
                    node_ref=div
                    on:scroll=move |_| {
                        if let Some(container) = div.get() {
                            let at_bottom = is_scrolled_near_bottom(&container);
                            is_scrolled_to_bottom.set(at_bottom);
                            if at_bottom {
                                show_jump_to_latest.set(false);
                            }
                        }
                    }
                    class="overflow-y-auto flex-grow p-4 w-full min-w-full h-full min-h-0"
                >
                    <Show
                        when=move || show_loading.get()
                        fallback=move || {
                            view! {
                                <Show
                                    when=move || !(active_history_loading.get() && render_rows.get().is_empty())
                                    fallback=move || {
                                        view! {
                                            <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                                                {t!(i18n, messages.chat.loading)}
                                            </div>
                                        }
                                    }
                                >
                                    <Show
                                        when=move || {
                                            let rows = render_rows.get();
                                            !rows.is_empty() || visible_thread_error.get().is_none()
                                        }
                                        fallback=move || {
                                            view! {
                                                <div class="flex flex-col gap-2 justify-center items-center h-full text-gray-500 dark:text-gray-400 min-h-[8rem]">
                                                    <p class="text-sm font-medium">
                                                        {move || visible_thread_error.get().unwrap_or_default()}
                                                    </p>
                                                </div>
                                            }
                                        }
                                    >
                                        <Transition fallback=move || {
                                            view! {
                                                <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                                                    {t!(i18n, messages.chat.loading)}
                                                </div>
                                            }
                                        }>
                                            <ShowLet
                                                some=move || {
                                                    let rows = render_rows.get();
                                                    if rows.is_empty() { None } else { Some(rows) }
                                                }
                                                let:rows
                                                fallback=move || {
                                                    view! {
                                                        <div class="flex flex-col gap-2 justify-center items-center h-full text-gray-500 dark:text-gray-400 min-h-[8rem]">
                                                            <span class="text-3xl opacity-40">"✉️"</span>
                                                            <p class="text-sm font-medium">
                                                                {t!(i18n, messages.chat.empty_title)}
                                                            </p>
                                                            <p class="text-xs">{t!(i18n, messages.chat.empty_body)}</p>
                                                        </div>
                                                    }
                                                }
                                            >
                                                <ShowLet
                                                    some=move || Some(chat.blocked_user_ids.get())
                                                    let:blocked_user_ids
                                                >
                                                    {
                                                        let blocked_set =
                                                            std::sync::Arc::new(blocked_user_ids);
                                                        let is_shared_channel = matches!(
                                                            actual_destination.get(),
                                                            Some(ChatDestination::GamePlayers(_, _, _))
                                                            | Some(ChatDestination::GameSpectators(_, _, _))
                                                            | Some(ChatDestination::TournamentLobby(_))
                                                            | Some(ChatDestination::Global)
                                                        );
                                                        let rows_for_render = StoredValue::new(rows.clone());
                                                        view! {
                                                            <ShowLet
                                                                some=move || visible_thread_error.get()
                                                                let:error
                                                            >
                                                                <div class="py-2 px-3 mb-4 text-sm text-amber-900 bg-amber-100 rounded-lg border border-amber-200 dark:text-amber-100 dark:bg-amber-950/40 dark:border-amber-800">
                                                                    {error}
                                                                </div>
                                                            </ShowLet>
                                                            <For
                                                                each=move || rows_for_render.get_value()
                                                                key=|row| row.id.clone()
                                                                let:row
                                                            >
                                                                {
                                                                    let MessageRow {
                                                                        id,
                                                                        message,
                                                                        show_header,
                                                                        is_current_user,
                                                                        show_unread_divider_before,
                                                                        ..
                                                                    } = row;
                                                                    let expanded_set = expanded_hidden_messages;
                                                                    let expanded_key = id.clone();
                                                                    let expand_callback_key = id.clone();
                                                                    let blocked_for_row = blocked_set.clone();
                                                                    let sender_blocked = is_shared_channel
                                                                        && blocked_for_row.contains(&message.user_id);
                                                                    let expanded_signal = Signal::derive(move || {
                                                                        expanded_set.with(|set| set.contains(&expanded_key))
                                                                    });
                                                                    let on_expand = Callback::new(move |()| {
                                                                        expanded_set.update(|set| {
                                                                            set.insert(expand_callback_key.clone());
                                                                        });
                                                                    });
                                                                    view! {
                                                                        <ShowLet
                                                                            some=move || {
                                                                                if show_unread_divider_before {
                                                                                    unread_at_open.get()
                                                                                } else {
                                                                                    None
                                                                                }
                                                                            }
                                                                            let:_n
                                                                        >
                                                                            <div
                                                                                node_ref=first_unread_ref
                                                                                class="flex relative justify-center items-center my-4 text-xs text-gray-500 dark:text-gray-400"
                                                                            >
                                                                                <div class="absolute inset-x-0 border-b border-gray-300 dark:border-gray-600"></div>
                                                                                <span class="relative z-10 px-2 bg-white dark:bg-gray-900">
                                                                                    {t!(i18n, messages.chat.new_messages)}
                                                                                </span>
                                                                            </div>
                                                                        </ShowLet>
                                                                        <Message
                                                                            message
                                                                            is_current_user
                                                                            show_header
                                                                            sender_blocked
                                                                            expanded_signal
                                                                            on_click_expand=on_expand
                                                                        />
                                                                    }
                                                                }
                                                            </For>
                                                        }
                                                    }
                                                </ShowLet>
                                            </ShowLet>
                                        </Transition>
                                    </Show>
                                </Show>
                            }
                        }
                    >
                        <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                            {t!(i18n, messages.chat.loading)}
                        </div>
                    </Show>
                </div>
                <Show when=move || show_jump_to_latest.get()>
                    <button
                        type="button"
                        class="absolute bottom-4 left-1/2 z-10 px-3 py-2 text-sm font-medium text-white rounded-full border shadow-lg transition-colors -translate-x-1/2 bg-pillbug-teal border-pillbug-teal/80 hover:bg-pillbug-teal/90"
                        on:click=scroll_to_latest
                    >
                        {t!(i18n, messages.chat.new_messages)} " ↓"
                    </button>
                </Show>
            </div>
            <div class="border-t border-gray-200 dark:border-gray-700 shrink-0">
                <Show
                    when=move || {
                        !show_loading.get()
                            && actual_destination.get().is_some()
                            && !chat_input_hidden.get()
                    }
                    fallback=move || {
                        view! {
                            <Show
                                when=move || show_loading.get()
                                fallback=move || {
                                    view! {
                                        <Show
                                            when=move || visible_thread_error.get().is_some()
                                            fallback=move || view! { <div class="py-3 px-4"></div> }
                                        >
                                            <div class="py-3 px-4 text-sm text-amber-900 dark:text-amber-100 bg-amber-100 dark:bg-amber-950/40">
                                                {move || visible_thread_error.get().unwrap_or_default()}
                                            </div>
                                        </Show>
                                    }
                                }
                            >
                                <div class="py-3 px-4 text-sm text-gray-500 dark:text-gray-400">
                                    {t!(i18n, messages.chat.loading)}
                                </div>
                            </Show>
                        }
                    }
                >
                    <ChatInput
                        destination=actual_destination_signal
                        disabled=move || {
                            let extra = input_disabled.as_ref().map(|s| s.get()).unwrap_or(false);
                            if extra {
                                true
                            } else {
                                match actual_destination.get() {
                                    Some(ChatDestination::Global) => {
                                        !auth_context
                                            .user
                                            .with_untracked(|u| {
                                                u.as_ref().is_some_and(|a| a.user.admin)
                                            })
                                    }
                                    None => true,
                                    _ => false,
                                }
                            }
                        }
                    />
                </Show>
            </div>
        </div>
    }
}
