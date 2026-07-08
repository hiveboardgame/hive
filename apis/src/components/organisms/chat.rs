use crate::{
    i18n::*,
    providers::{
        chat::Chat,
        game_state::GameStateSignal,
        websocket::{ConnectionReadyState, WebsocketContext},
        ApiRequestsProvider,
        AuthContext,
    },
};
use chrono::{Duration, Local};
use leptos::{
    either::{Either, EitherOf4},
    html,
    leptos_dom::helpers::request_animation_frame,
    prelude::*,
    task::spawn_local,
};
use leptos_router::hooks::use_params_map;
use leptos_use::{
    use_document_visibility,
    use_intersection_observer_with_options,
    use_interval_fn,
    UseIntersectionObserverOptions,
};
use shared_types::{
    normalize_chat_message,
    ChatDestination,
    ChatHistoryResponse,
    ChatMessage,
    ConversationKey,
    GameId,
    GameThread,
    SimpleDestination,
    MAX_CHAT_MESSAGE_LENGTH,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

const MESSAGE_GROUP_MAX_GAP: Duration = Duration::minutes(2);
const SCROLL_BOTTOM_THRESHOLD_PX: i32 = 32;

fn is_scrolled_near_bottom(container: &web_sys::HtmlElement) -> bool {
    container.scroll_height() - container.client_height() - container.scroll_top()
        <= SCROLL_BOTTOM_THRESHOLD_PX
}

pub(crate) fn messages_with_header_flags(messages: &[ChatMessage]) -> Vec<(ChatMessage, bool)> {
    messages
        .iter()
        .enumerate()
        .map(|(idx, message)| {
            let show_header = if idx == 0 {
                true
            } else {
                let previous = &messages[idx - 1];
                let same_user = previous.user_id == message.user_id;
                let gap_too_large = match (previous.timestamp, message.timestamp) {
                    (Some(previous), Some(current)) => {
                        (current - previous).abs() > MESSAGE_GROUP_MAX_GAP
                    }
                    _ => true,
                };
                !same_user || gap_too_large
            };
            (message.clone(), show_header)
        })
        .collect()
}

fn unread_divider_split_idx(
    messages: &[ChatMessage],
    unread_at_open: Option<i64>,
    current_user_id: Option<Uuid>,
) -> Option<usize> {
    if messages.is_empty() {
        return None;
    }

    let unread_count = unread_at_open.unwrap_or(0).max(0) as usize;
    if unread_count == 0 {
        return None;
    }

    let mut remaining = unread_count;
    for (idx, message) in messages.iter().enumerate().rev() {
        if Some(message.user_id) == current_user_id {
            continue;
        }
        remaining = remaining.saturating_sub(1);
        if remaining == 0 {
            return Some(idx);
        }
    }

    Some(messages.len().saturating_sub(unread_count))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MessageKey {
    id: Option<i64>,
    timestamp_millis: i64,
    user_id: Uuid,
    turn: Option<usize>,
    message: String,
}

#[derive(Clone, PartialEq)]
struct MessageRow {
    key: MessageKey,
    message: ChatMessage,
    show_header: bool,
    is_current_user: bool,
    show_unread_divider_before: bool,
}

fn message_key(message: &ChatMessage) -> MessageKey {
    MessageKey {
        id: message.id,
        timestamp_millis: message
            .timestamp
            .map(|timestamp| timestamp.timestamp_millis())
            .unwrap_or(0),
        user_id: message.user_id,
        turn: message.turn,
        message: message.message.clone(),
    }
}

#[derive(Copy, Clone)]
struct ThreadHistoryState {
    loading_channels: RwSignal<HashSet<ConversationKey>>,
    errors: RwSignal<HashMap<ConversationKey, ThreadHistoryError>>,
}

#[derive(Clone, PartialEq, Eq)]
enum ThreadHistoryError {
    AccessDenied,
    Unexpected(String),
}

impl ThreadHistoryState {
    fn new() -> Self {
        Self {
            loading_channels: RwSignal::new(HashSet::new()),
            errors: RwSignal::new(HashMap::new()),
        }
    }

    fn begin_loading(&self, key: &ConversationKey) {
        self.loading_channels.update(|loading| {
            loading.insert(key.clone());
        });
        self.errors.update(|errors| {
            errors.remove(key);
        });
    }

    fn clear(&self, key: &ConversationKey) {
        self.loading_channels.update(|loading| {
            loading.remove(key);
        });
        self.errors.update(|errors| {
            errors.remove(key);
        });
    }

    fn set_error(&self, key: &ConversationKey, error: ThreadHistoryError) {
        self.loading_channels.update(|loading| {
            loading.remove(key);
        });
        self.errors.update(|errors| {
            errors.insert(key.clone(), error);
        });
    }

    fn is_loading(&self, key: &ConversationKey) -> bool {
        self.loading_channels.with(|loading| loading.contains(key))
    }

    fn error_for(&self, key: &ConversationKey) -> Option<ThreadHistoryError> {
        self.errors.with(|errors| errors.get(key).cloned())
    }
}

#[derive(Copy, Clone)]
struct ThreadUiState {
    unread_at_open: RwSignal<Option<i64>>,
    show_jump_to_latest: RwSignal<bool>,
    is_scrolled_to_bottom: RwSignal<bool>,
    expanded_hidden_messages: RwSignal<HashSet<MessageKey>>,
}

impl ThreadUiState {
    fn new() -> Self {
        Self {
            unread_at_open: RwSignal::new(None::<i64>),
            show_jump_to_latest: RwSignal::new(false),
            is_scrolled_to_bottom: RwSignal::new(true),
            expanded_hidden_messages: RwSignal::new(HashSet::new()),
        }
    }
}

#[derive(Clone, PartialEq)]
enum ThreadBodyState {
    Loading,
    ErrorOnly(String),
    Empty,
    Rows {
        rows: Vec<MessageRow>,
        banner_error: Option<String>,
    },
}

fn use_thread_history(
    chat: Chat,
    active_channel_key: Signal<ConversationKey>,
) -> ThreadHistoryState {
    let history = ThreadHistoryState::new();

    Effect::watch(
        move || {
            let channel_key = active_channel_key.get();
            let session_epoch = chat.session_epoch();
            let subscription_ready = !needs_explicit_chat_subscription(&channel_key)
                || chat.has_confirmed_chat_subscription(&channel_key, session_epoch);
            (
                channel_key,
                session_epoch,
                chat.history_epoch(),
                subscription_ready,
            )
        },
        move |(channel_key, session_epoch, history_epoch, subscription_ready), previous, _| {
            if previous.is_some_and(
                |(previous_key, previous_epoch, previous_history_epoch, previous_ready)| {
                    previous_key == channel_key
                        && previous_epoch == session_epoch
                        && previous_history_epoch == history_epoch
                        && previous_ready == subscription_ready
                },
            ) {
                return;
            }
            if !*subscription_ready {
                if !chat.has_cached_history(channel_key) {
                    history.begin_loading(channel_key);
                }
                return;
            }
            if chat.has_cached_history(channel_key) {
                history.clear(channel_key);
                return;
            }

            let key = channel_key.clone();
            let request_session_epoch = *session_epoch;
            let request_history_epoch = *history_epoch;
            let request_user_id = chat.current_user_id_untracked();
            history.begin_loading(&key);
            spawn_local(async move {
                let result = chat.fetch_channel_history(&key).await;
                if chat.session_epoch_untracked() != request_session_epoch
                    || chat.history_epoch_untracked() != request_history_epoch
                    || chat.current_user_id_untracked() != request_user_id
                {
                    return;
                }
                match result {
                    Ok(ChatHistoryResponse::Messages(messages)) => {
                        history.clear(&key);
                        chat.inject_history(&key, messages);
                    }
                    Ok(ChatHistoryResponse::AccessDenied) => {
                        history.set_error(&key, ThreadHistoryError::AccessDenied);
                    }
                    Err(error) => {
                        history.set_error(&key, ThreadHistoryError::Unexpected(error));
                    }
                }
            });
        },
        true,
    );

    history
}

fn needs_explicit_chat_subscription(key: &ConversationKey) -> bool {
    matches!(
        key,
        ConversationKey::Game {
            thread: GameThread::Spectators,
            ..
        }
    )
}

fn use_explicit_chat_subscription(chat: Chat, active_channel_key: Signal<ConversationKey>) {
    let api = expect_context::<ApiRequestsProvider>();
    let websocket = expect_context::<WebsocketContext>();
    let subscribed_key = RwSignal::new(None::<(ConversationKey, u64)>);
    let api_for_effect = api.clone();

    Effect::watch(
        move || {
            (
                active_channel_key.get(),
                chat.session_epoch(),
                websocket.ready_state.get(),
            )
        },
        move |(channel_key, session_epoch, ready_state), _previous, _| {
            let session_epoch = *session_epoch;
            if *ready_state != ConnectionReadyState::Open {
                chat.clear_confirmed_chat_subscriptions();
                subscribed_key.set(None);
                return;
            }

            if subscribed_key.with_untracked(|current| {
                current
                    .as_ref()
                    .is_some_and(|(key, epoch)| key == channel_key && *epoch == session_epoch)
            }) {
                return;
            }

            if let Some((previous_key, previous_epoch)) = subscribed_key.get_untracked() {
                chat.clear_confirmed_chat_subscription(&previous_key, previous_epoch);
                api_for_effect
                    .0
                    .get_untracked()
                    .unsubscribe_chat(previous_key);
            }

            if needs_explicit_chat_subscription(channel_key)
                && api_for_effect
                    .0
                    .get_untracked()
                    .subscribe_chat(channel_key.clone())
            {
                chat.set_pending_chat_subscription(channel_key.clone(), session_epoch);
                subscribed_key.set(Some((channel_key.clone(), session_epoch)));
            } else {
                subscribed_key.set(None);
            }
        },
        true,
    );

    on_cleanup(move || {
        if let Some((key, session_epoch)) = subscribed_key.get_untracked() {
            chat.clear_confirmed_chat_subscription(&key, session_epoch);
            api.0.get_untracked().unsubscribe_chat(key);
        }
    });
}

fn use_thread_read_eligibility(
    chat: Chat,
    active_channel_key: Signal<ConversationKey>,
    bottom_visible: RwSignal<bool>,
) {
    let document_visibility = use_document_visibility();
    let registered_key = RwSignal::new(None::<(ConversationKey, u64)>);

    Effect::watch(
        move || {
            (
                active_channel_key.get(),
                chat.session_epoch(),
                document_visibility.get() == web_sys::VisibilityState::Visible,
                bottom_visible.get(),
            )
        },
        move |(channel_key, session_epoch, document_visible, bottom_visible), _previous, _| {
            let session_epoch = *session_epoch;
            let read_eligible = *document_visible && *bottom_visible;
            let current = registered_key.get_untracked();

            if !read_eligible {
                if let Some((registered, _)) = current {
                    chat.clear_channel_visible(&registered);
                    registered_key.set(None);
                }
                return;
            }

            if current.as_ref().is_some_and(|(registered, epoch)| {
                registered == channel_key && *epoch == session_epoch
            }) {
                return;
            }

            if let Some((registered, _)) = current {
                chat.clear_channel_visible(&registered);
            }
            chat.set_channel_visible(channel_key);
            registered_key.set(Some((channel_key.clone(), session_epoch)));
        },
        true,
    );

    on_cleanup(move || {
        if let Some((registered, _)) = registered_key.get_untracked() {
            chat.clear_channel_visible(&registered);
        }
    });
}

fn use_thread_ui_state(chat: Chat, active_channel_key: Signal<ConversationKey>) -> ThreadUiState {
    let thread_ui = ThreadUiState::new();

    Effect::watch(
        move || (active_channel_key.get(), chat.session_epoch()),
        move |(channel_key, _session_epoch), _previous, _| {
            thread_ui.unread_at_open.set(None);
            thread_ui.show_jump_to_latest.set(false);
            thread_ui.is_scrolled_to_bottom.set(true);
            thread_ui.expanded_hidden_messages.set(HashSet::new());

            let unread = chat.unread_count_for_channel_untracked(channel_key);
            if unread > 0 {
                thread_ui.unread_at_open.set(Some(unread));
            }
        },
        true,
    );

    use_interval_fn(
        move || {
            if thread_ui.unread_at_open.get_untracked().is_some() {
                thread_ui.unread_at_open.set(None);
            }
        },
        10_000,
    );

    thread_ui
}

#[component]
fn Message(
    message: ChatMessage,
    is_current_user: bool,
    show_header: bool,
    sender_blocked: Signal<bool>,
    expanded_signal: Signal<bool>,
    on_click_expand: Callback<(), ()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let ChatMessage {
        username,
        message,
        timestamp,
        turn,
        ..
    } = message;
    let username = StoredValue::new(username);
    let message = StoredValue::new(message);
    let user_local_time = timestamp
        .map(|time| time.with_timezone(&Local).format("%d/%m %H:%M").to_string())
        .unwrap_or_default();
    let user_local_time = StoredValue::new(user_local_time);
    let turn = StoredValue::new(turn);
    let margin = if show_header { "mb-3" } else { "mb-1" };
    let outer_class = if is_current_user {
        format!("flex flex-col items-end {margin} w-full")
    } else {
        format!("flex flex-col items-start {margin} w-full")
    };
    let bubble_class = if is_current_user {
        "ui-chat-bubble ui-chat-bubble-own"
    } else {
        "ui-chat-bubble ui-chat-bubble-other"
    };

    view! {
        <div class=outer_class>
            <Show when=move || sender_blocked.get() && !expanded_signal.get()>
                <button
                    type="button"
                    class="mb-1 ui-chat-hidden-message"
                    on:click=move |_| on_click_expand.run(())
                >
                    {t!(i18n, messages.chat.hidden_message)}
                </button>
            </Show>
            <Show when=move || !sender_blocked.get() || expanded_signal.get()>
                <Show when=move || show_header>
                    <div class="flex flex-wrap gap-x-2 items-baseline px-1 mb-1 max-w-[85%] sm:max-w-[75%]">
                        <span class="text-sm font-bold text-gray-800 dark:text-gray-100 truncate">
                            {username.get_value()}
                        </span>
                        <span class="text-xs text-gray-500 whitespace-nowrap dark:text-gray-400">
                            {user_local_time.get_value()}
                            {move || {
                                turn.get_value()
                                    .map(|turn| t_string!(i18n, messages.chat.turn, turn = turn))
                                    .unwrap_or_default()
                            }}
                        </span>
                    </div>
                </Show>
                <div class=bubble_class>{message.get_value()}</div>
            </Show>
        </div>
    }
}

#[component]
fn MessageRowView(
    row: MessageRow,
    expanded_hidden_messages: RwSignal<HashSet<MessageKey>>,
    unread_at_open: RwSignal<Option<i64>>,
    first_unread_ref: NodeRef<html::Div>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let MessageRow {
        key,
        message,
        show_header,
        is_current_user,
        show_unread_divider_before,
    } = row;
    let expanded_key = StoredValue::new(key);
    let blocked_user_id = message.user_id;
    let sender_blocked = Signal::derive(move || {
        chat.blocked_user_ids
            .with(|ids| ids.contains(&blocked_user_id))
    });
    let expanded_signal = Signal::derive(move || {
        expanded_hidden_messages.with(|set| set.contains(&expanded_key.get_value()))
    });
    let on_expand = Callback::new(move |()| {
        expanded_hidden_messages.update(|set| {
            set.insert(expanded_key.get_value());
        });
    });

    view! {
        <Show when=move || show_unread_divider_before && unread_at_open.get().is_some()>
            <div
                node_ref=first_unread_ref
                class="flex gap-2 items-center my-3 text-xs font-medium text-gray-600 dark:text-gray-300"
            >
                <span class="flex-1 h-px bg-black/10 dark:bg-white/20"></span>
                <span class="shrink-0">{t!(i18n, messages.chat.new_messages)}</span>
                <span class="flex-1 h-px bg-black/10 dark:bg-white/20"></span>
            </div>
        </Show>
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

#[component]
pub fn ChatInput(
    #[prop(into)] destination: Signal<ChatDestination>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let game_state = use_context::<GameStateSignal>();
    let active_key = Signal::derive(move || ConversationKey::from_destination(&destination.get()));
    let turn = move || {
        let game_id = match destination.get() {
            ChatDestination::GamePlayers(game_id) | ChatDestination::GameSpectators(game_id) => {
                game_id
            }
            _ => return None,
        };
        game_state.and_then(|state| {
            state.signal.with(|state| {
                (state.game_id.as_ref() == Some(&game_id)).then_some(state.state.turn)
            })
        })
    };
    let send = move || {
        if disabled.get() {
            return;
        }
        let key = active_key.get();
        let message = chat.draft_message(&key);
        let message = normalize_chat_message(&message);
        if !message.trim().is_empty() && chat.send(&message, destination.get(), turn()) {
            chat.clear_draft_message(&key);
        }
    };
    let placeholder = move || {
        if disabled.get() {
            t_string!(i18n, messages.chat.admin_only).to_string()
        } else {
            match destination.get() {
                ChatDestination::GamePlayers(_) => {
                    t_string!(i18n, messages.chat.with_opponent).to_string()
                }
                ChatDestination::GameSpectators(_) => {
                    t_string!(i18n, messages.chat.with_spectators).to_string()
                }
                _ => t_string!(i18n, messages.chat.placeholder).to_string(),
            }
        }
    };
    let input_ref = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = input_ref.get_untracked().map(|input| input.focus());
    });

    view! {
        <input
            node_ref=input_ref
            type="text"
            prop:disabled=disabled
            class="overscroll-contain py-3 disabled:opacity-50 disabled:cursor-not-allowed ui-field-input box-border shrink-0 touch-pan-x"
            prop:value=move || chat.draft_message(&active_key.get())
            prop:placeholder=placeholder
            on:input=move |event| {
                chat.set_draft_message(&active_key.get_untracked(), event_target_value(&event));
            }
            on:keydown=move |event| {
                if event.key() == "Enter" && !disabled.get() {
                    event.prevent_default();
                    send();
                }
            }
            maxlength=MAX_CHAT_MESSAGE_LENGTH.to_string()
        />
    }
}

#[component]
pub fn ResolvedChatWindow(
    #[prop(into)] destination: Signal<ChatDestination>,
    #[prop(optional, into)] input_disabled: Signal<bool>,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let auth = expect_context::<AuthContext>();
    let current_user_id = Signal::derive(move || {
        auth.user
            .with(|account| account.as_ref().map(|account| account.user.uid))
    });
    let active_key = Signal::derive(move || ConversationKey::from_destination(&destination.get()));
    let messages_ref = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();
    let bottom_ref = NodeRef::<html::Div>::new();
    let bottom_visible = RwSignal::new(false);
    use_explicit_chat_subscription(chat, active_key);
    _ = use_intersection_observer_with_options(
        bottom_ref,
        move |entries, _| {
            bottom_visible.set(entries.first().is_some_and(|entry| entry.is_intersecting()));
        },
        UseIntersectionObserverOptions::default().thresholds(vec![0.95]),
    );
    Effect::watch(
        move || active_key.get(),
        move |_, _, _| {
            bottom_visible.set(false);
        },
        true,
    );
    use_thread_read_eligibility(chat, active_key, bottom_visible);
    let history = use_thread_history(chat, active_key);
    let thread_ui = use_thread_ui_state(chat, active_key);

    let visible_history_error = Signal::derive(move || history.error_for(&active_key.get()));
    let visible_thread_error = Signal::derive(move || {
        visible_history_error.get().map(|error| match error {
            ThreadHistoryError::AccessDenied => match destination.get() {
                ChatDestination::User(_) => {
                    t_string!(i18n, messages.page.failed_conversations).to_string()
                }
                ChatDestination::TournamentLobby(_) => {
                    t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
                }
                _ => "Access denied".to_string(),
            },
            ThreadHistoryError::Unexpected(error) => error,
        })
    });
    let visible_send_error = Signal::derive(move || chat.chat_send_error(&active_key.get()));
    let messages = Memo::new(move |_| chat.cached_messages(&active_key.get()));
    let rows = Memo::new(move |_| {
        let messages = messages.get();
        if messages.is_empty() {
            return Vec::new();
        }
        let current_user_id = current_user_id.get();
        let split_idx =
            unread_divider_split_idx(&messages, thread_ui.unread_at_open.get(), current_user_id);
        messages_with_header_flags(&messages)
            .into_iter()
            .enumerate()
            .map(|(idx, (message, show_header))| {
                let user_id = message.user_id;
                MessageRow {
                    key: message_key(&message),
                    message,
                    show_header,
                    is_current_user: current_user_id == Some(user_id),
                    show_unread_divider_before: split_idx == Some(idx),
                }
            })
            .collect::<Vec<_>>()
    });
    let body_state = Memo::new(move |_| {
        let key = active_key.get();
        let rows = rows.get();
        let error = visible_thread_error.get();
        if history.is_loading(&key) && rows.is_empty() {
            ThreadBodyState::Loading
        } else if rows.is_empty() {
            error.map_or(ThreadBodyState::Empty, ThreadBodyState::ErrorOnly)
        } else {
            ThreadBodyState::Rows {
                rows,
                banner_error: error,
            }
        }
    });

    Effect::watch(
        move || {
            let channel_key = active_key.get();
            let messages = messages.get();
            let latest_key = messages.last().map(message_key);
            (channel_key, messages.len(), latest_key)
        },
        move |(channel_key, count, latest_key), previous, _| {
            let previous_count = previous.map(|(_, count, _)| *count).unwrap_or(0);
            let latest_changed =
                previous.is_none_or(|(_, _, previous_latest)| previous_latest != latest_key);
            let channel_changed =
                previous.is_none_or(|(previous_key, _, _)| previous_key != channel_key);
            if !channel_changed && *count == previous_count && !latest_changed {
                return;
            }

            let is_new_message = !channel_changed
                && previous_count > 0
                && (*count > previous_count || latest_changed);
            let container = messages_ref.get_untracked();
            let first_unread = first_unread_ref.get_untracked();
            let should_auto_scroll = thread_ui.is_scrolled_to_bottom.get_untracked();

            request_animation_frame(move || {
                if channel_changed || !is_new_message {
                    thread_ui.show_jump_to_latest.set(false);
                    if let Some(first_unread) = first_unread {
                        first_unread.scroll_into_view_with_bool(true);
                    } else if let Some(container) = container.as_ref() {
                        container.set_scroll_top(container.scroll_height());
                    }
                    if let Some(container) = container.as_ref() {
                        let at_bottom = is_scrolled_near_bottom(container);
                        thread_ui.is_scrolled_to_bottom.set(at_bottom);
                        if at_bottom {
                            thread_ui.show_jump_to_latest.set(false);
                        }
                    }
                } else if should_auto_scroll {
                    if let Some(container) = container.as_ref() {
                        container.set_scroll_top(container.scroll_height());
                    }
                    thread_ui.is_scrolled_to_bottom.set(true);
                    thread_ui.show_jump_to_latest.set(false);
                } else {
                    thread_ui.show_jump_to_latest.set(true);
                }
            });
        },
        true,
    );

    let scroll_to_latest = move |_| {
        if let Some(container) = messages_ref.get_untracked() {
            request_animation_frame(move || {
                container.set_scroll_top(container.scroll_height());
                thread_ui.is_scrolled_to_bottom.set(true);
                thread_ui.show_jump_to_latest.set(false);
            });
        }
    };

    let message_list_class = if compact {
        "ui-chat-message-list p-2"
    } else {
        "ui-chat-message-list p-3 xs:p-4"
    };
    let composer_class = if compact {
        "ui-chat-composer p-2"
    } else {
        "ui-chat-composer p-3"
    };
    let conversation_column_class = "mx-auto flex min-h-full w-full max-w-6xl flex-col";
    let composer_inner_class = "mx-auto w-full max-w-6xl";

    view! {
        <div class="flex overflow-hidden flex-col flex-grow w-full min-w-full max-w-full h-full min-h-0">
            <div class="relative flex-grow w-full min-w-full h-0 min-h-0">
                <div
                    node_ref=messages_ref
                    on:scroll=move |_| {
                        if let Some(container) = messages_ref.get() {
                            let at_bottom = is_scrolled_near_bottom(&container);
                            thread_ui.is_scrolled_to_bottom.set(at_bottom);
                            if at_bottom {
                                thread_ui.show_jump_to_latest.set(false);
                            }
                        }
                    }
                    class=message_list_class
                >
                    {move || match body_state.get() {
                        ThreadBodyState::Loading => {
                            EitherOf4::A(
                                view! {
                                    <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                                        {t!(i18n, messages.chat.loading)}
                                    </div>
                                },
                            )
                        }
                        ThreadBodyState::ErrorOnly(error) => {
                            EitherOf4::B(
                                view! {
                                    <div class="flex justify-center items-center h-full min-h-[8rem]">
                                        <div class="max-w-sm ui-empty-state">
                                            <p class="text-sm font-medium">{error}</p>
                                        </div>
                                    </div>
                                },
                            )
                        }
                        ThreadBodyState::Empty => {
                            EitherOf4::C(
                                view! {
                                    <div class="flex justify-center items-center h-full min-h-[8rem]">
                                        <div class="max-w-sm ui-empty-state">
                                            <p class="text-sm font-bold text-gray-800 dark:text-gray-100">
                                                {t!(i18n, messages.chat.empty_title)}
                                            </p>
                                            <p class="mt-1 text-xs">
                                                {t!(i18n, messages.chat.empty_body)}
                                            </p>
                                        </div>
                                    </div>
                                },
                            )
                        }
                        ThreadBodyState::Rows { rows, banner_error } => {
                            let rows = StoredValue::new(rows);
                            EitherOf4::D(
                                view! {
                                    <div class=conversation_column_class>
                                        {banner_error
                                            .map(|error| {
                                                view! { <div class="mb-4 ui-warning-notice">{error}</div> }
                                            })}
                                        <For
                                            each=move || rows.get_value()
                                            key=|row| row.key.clone()
                                            let:row
                                        >
                                            <MessageRowView
                                                row
                                                expanded_hidden_messages=thread_ui.expanded_hidden_messages
                                                unread_at_open=thread_ui.unread_at_open
                                                first_unread_ref
                                            />
                                        </For> <div node_ref=bottom_ref class="w-full h-px"></div>
                                    </div>
                                },
                            )
                        }
                    }}
                </div>
                <Show when=thread_ui.show_jump_to_latest>
                    <button
                        type="button"
                        class="absolute bottom-3 left-1/2 z-10 shadow-lg -translate-x-1/2 ui-button ui-button-primary ui-button-sm"
                        on:click=scroll_to_latest
                    >
                        {t!(i18n, messages.chat.new_messages)}
                    </button>
                </Show>
            </div>
            <div class=composer_class>
                <div class=composer_inner_class>
                    <Show
                        when=move || {
                            visible_history_error.get() != Some(ThreadHistoryError::AccessDenied)
                        }
                        fallback=move || {
                            view! {
                                <Show when=move || visible_thread_error.get().is_some()>
                                    <div class="ui-warning-notice">
                                        {move || visible_thread_error.get().unwrap_or_default()}
                                    </div>
                                </Show>
                            }
                        }
                    >
                        <div class="flex flex-col gap-2">
                            <Show when=move || visible_send_error.get().is_some()>
                                <div class="ui-danger-notice">
                                    {move || visible_send_error.get().unwrap_or_default()}
                                </div>
                            </Show>
                            <ChatInput destination disabled=input_disabled />
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn GameChatWindow(#[prop(into)] explicit_thread: Signal<Option<GameThread>>) -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params_map();
    let game_state = expect_context::<GameStateSignal>();
    let auth = expect_context::<AuthContext>();
    let route_game_id = Signal::derive(move || {
        params
            .get()
            .get("nanoid")
            .map(|nanoid| GameId(nanoid.to_string()))
    });
    let current_user_id = Signal::derive(move || {
        auth.user
            .with(|account| account.as_ref().map(|account| account.user.uid))
    });
    let destination = Signal::derive(move || {
        let game_id = route_game_id.get()?;
        let explicit_thread = explicit_thread.get();
        game_state.signal.with(|state| {
            if state.game_id.as_ref() != Some(&game_id) {
                return None;
            }
            if explicit_thread.is_none() && (state.white_id.is_none() || state.black_id.is_none()) {
                return None;
            }
            let thread = explicit_thread.unwrap_or_else(|| {
                if state.uid_is_player(current_user_id.get()) {
                    GameThread::Players
                } else {
                    GameThread::Spectators
                }
            });
            Some(match thread {
                GameThread::Players => ChatDestination::GamePlayers(game_id),
                GameThread::Spectators => ChatDestination::GameSpectators(game_id),
            })
        })
    });

    view! {
        {move || match destination.get() {
            Some(destination) => {
                Either::Left(view! { <ResolvedChatWindow destination compact=true /> })
            }
            None => {
                Either::Right(
                    view! {
                        <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                            {t!(i18n, messages.chat.loading)}
                        </div>
                    },
                )
            }
        }}
    }
}

#[component]
pub fn ChatWindow(
    destination: SimpleDestination,
    #[prop(optional)] correspondant_id: Option<Uuid>,
    #[prop(optional)] correspondant_username: String,
) -> impl IntoView {
    let params = use_params_map();
    let destination = Signal::derive(move || match destination.clone() {
        SimpleDestination::Game => params
            .get()
            .get("nanoid")
            .map(|nanoid| ChatDestination::GameSpectators(GameId(nanoid.to_string())))
            .unwrap_or(ChatDestination::Global),
        SimpleDestination::User => ChatDestination::User((
            correspondant_id.unwrap_or_else(Uuid::new_v4),
            correspondant_username.clone(),
        )),
        SimpleDestination::Tournament(tournament_id) => {
            ChatDestination::TournamentLobby(tournament_id)
        }
        SimpleDestination::Global => ChatDestination::Global,
    });
    view! { <ResolvedChatWindow destination /> }
}
