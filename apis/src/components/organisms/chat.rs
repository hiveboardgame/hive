use crate::{
    chat::ConversationKey,
    i18n::*,
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
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
use leptos_use::use_interval_fn;
use shared_types::{ChatDestination, ChatHistoryResponse, ChatMessage, GameId, GameThread};
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
    expanded_hidden_messages: RwSignal<HashSet<MessageId>>,
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

#[derive(Clone, PartialEq)]
enum ThreadFooterState {
    Composer,
    Notice(String),
}

fn use_thread_history(
    chat: Chat,
    active_channel_key: Signal<ConversationKey>,
) -> ThreadHistoryState {
    let history = ThreadHistoryState::new();

    Effect::watch(
        move || active_channel_key.get(),
        move |channel_key, previous, _| {
            if previous.is_some_and(|previous_key| previous_key == channel_key) {
                return;
            }

            if chat.has_cached_history(channel_key) {
                history.clear(channel_key);
                return;
            }

            let key = channel_key.clone();
            history.begin_loading(&key);
            spawn_local(async move {
                match chat.fetch_channel_history(&key).await {
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

fn use_thread_ui_state(chat: Chat, active_channel_key: Signal<ConversationKey>) -> ThreadUiState {
    let thread_ui = ThreadUiState::new();

    Effect::watch(
        move || active_channel_key.get(),
        move |channel_key, previous, _| {
            if let Some(previous_key) = previous.cloned() {
                chat.clear_channel_visible(&previous_key);
            }

            thread_ui.unread_at_open.set(None);
            thread_ui.show_jump_to_latest.set(false);
            thread_ui.is_scrolled_to_bottom.set(true);
            thread_ui.expanded_hidden_messages.set(HashSet::new());

            let unread = chat.unread_count_for_channel_untracked(channel_key);
            if unread > 0 {
                thread_ui.unread_at_open.set(Some(unread));
            }

            chat.set_channel_visible(channel_key);
            chat.open_channel(channel_key);
        },
        true,
    );

    on_cleanup(move || {
        chat.clear_channel_visible(&active_channel_key.get_untracked());
    });

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
    /// When false, only the message bubble is shown (for consecutive messages from the same user).
    show_header: bool,
    /// When true, show "Message hidden. Click to expand." for blocked senders.
    sender_blocked: Signal<bool>,
    /// Parent-owned expanded state so it survives re-renders.
    expanded_signal: Signal<bool>,
    /// Callback when user clicks "Click to expand". Parent should add this message to its expanded set.
    on_click_expand: Callback<(), ()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let ChatMessage {
        username,
        message: body,
        timestamp,
        turn,
        ..
    } = message;

    let is_expanded = move || expanded_signal.get();
    let on_expand = move |_| on_click_expand.run(());

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
            <Show when=move || sender_blocked.get() && !is_expanded()>
                <button
                    type="button"
                    class="py-2 px-3 mb-1 text-sm text-left text-gray-500 bg-gray-100 rounded-lg border border-gray-200 transition-colors dark:text-gray-400 dark:bg-gray-800 dark:border-gray-600 hover:bg-gray-200 max-w-[85%] sm:max-w-[75%] dark:hover:bg-gray-700"
                    on:click=on_expand
                >
                    {t!(i18n, messages.chat.hidden_message)}
                </button>
            </Show>
            <Show when=move || {
                !sender_blocked.get() || is_expanded()
            }>
                {if show_header {
                    Either::Left(
                        view! {
                            <div class="flex flex-wrap gap-y-0.5 gap-x-2 items-baseline px-1 mb-1 max-w-[85%] sm:max-w-[75%]">
                                <span class="text-sm font-semibold text-gray-800 dark:text-gray-200 truncate">
                                    {username.get_value()}
                                </span>
                                <span class="text-xs text-gray-500 whitespace-nowrap dark:text-gray-400">
                                    {user_local_time.get_value()}
                                    {move || {
                                        turn.map(|turn| {
                                                t_string!(i18n, messages.chat.turn, turn = turn)
                                            })
                                            .unwrap_or_default()
                                    }}
                                </span>
                            </div>
                        },
                    )
                } else {
                    Either::Right(())
                }} <div class=bubble_class>{body.get_value()}</div>
            </Show>
        </div>
    }
}

#[component]
fn MessageRowView(
    row: MessageRow,
    expanded_hidden_messages: RwSignal<HashSet<MessageId>>,
    unread_at_open: RwSignal<Option<i64>>,
    first_unread_ref: NodeRef<html::Div>,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let MessageRow {
        id,
        message,
        show_header,
        is_current_user,
        show_unread_divider_before,
        ..
    } = row;
    let expanded_key = id.clone();
    let expand_callback_key = id.clone();
    let blocked_user_id = message.user_id;
    let sender_blocked =
        Signal::derive(move || chat.blocked_user_ids.with(|blocked| blocked.contains(&blocked_user_id)));
    let expanded_signal =
        Signal::derive(move || expanded_hidden_messages.with(|set| set.contains(&expanded_key)));
    let on_expand = Callback::new(move |()| {
        expanded_hidden_messages.update(|set| {
            set.insert(expand_callback_key.clone());
        });
    });

    view! {
        <Show when=move || show_unread_divider_before && unread_at_open.get().is_some()>
            <div
                node_ref=first_unread_ref
                class="flex relative justify-center items-center my-4 text-xs text-gray-500 dark:text-gray-400"
            >
                <div class="absolute inset-x-0 border-b border-gray-300 dark:border-gray-600"></div>
                <span class="relative z-10 px-2 bg-white dark:bg-gray-900">
                    {t!(i18n, messages.chat.new_messages)}
                </span>
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
fn MessageRows(
    rows: Vec<MessageRow>,
    banner_error: Option<String>,
    expanded_hidden_messages: RwSignal<HashSet<MessageId>>,
    unread_at_open: RwSignal<Option<i64>>,
    first_unread_ref: NodeRef<html::Div>,
) -> impl IntoView {
    let rows = StoredValue::new(rows);

    view! {
        {if let Some(error) = banner_error {
            Either::Left(
                view! {
                    <div class="py-2 px-3 mb-4 text-sm text-amber-900 bg-amber-100 rounded-lg border border-amber-200 dark:text-amber-100 dark:border-amber-800 dark:bg-amber-950/40">
                        {error}
                    </div>
                },
            )
        } else {
            Either::Right(())
        }}
        <For each=move || rows.get_value() key=|row| row.id.clone() let:row>
            <MessageRowView
                row
                expanded_hidden_messages
                unread_at_open
                first_unread_ref
            />
        </For>
    }
}

#[component]
pub fn ChatInput(
    #[prop(into)] destination: Signal<ChatDestination>,
    #[prop(into)] disabled: Signal<bool>,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let game_state = use_context::<GameStateSignal>();
    let active_channel_key =
        Signal::derive(move || ConversationKey::from_destination(&destination.get()));
    let turn = move || game_state.map(|gs| gs.signal.with(|state| state.state.turn));
    let send = move || {
        if disabled.get() {
            return;
        }
        let channel_key = active_channel_key.get();
        let message = chat.draft_message(&channel_key);
        if !message.is_empty() {
            chat.send(&message, destination.get(), turn());
            chat.clear_draft_message(&channel_key);
        };
    };
    let placeholder = move || {
        if disabled.get() {
            t_string!(i18n, messages.chat.admin_only)
        } else {
            match destination.get() {
                ChatDestination::GamePlayers(_) => {
                    t_string!(i18n, messages.chat.with_opponent)
                }
                ChatDestination::GameSpectators(_) => {
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
            prop:disabled=disabled
            class="py-3 px-4 w-full placeholder-gray-500 text-black bg-white rounded-xl border border-gray-300 shadow-inner transition-shadow dark:placeholder-gray-400 dark:text-white dark:bg-gray-800 dark:border-gray-600 focus:ring-2 focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed box-border shrink-0 focus:ring-pillbug-teal/50 focus:border-pillbug-teal"
            prop:value=move || chat.draft_message(&active_channel_key.get())
            prop:placeholder=placeholder
            on:input=move |evt| {
                chat.set_draft_message(&active_channel_key.get_untracked(), event_target_value(&evt));
            }
            on:keydown=move |evt| {
                if evt.key() == "Enter" && !disabled.get() {
                    evt.prevent_default();
                    send();
                }
            }

            maxlength="1000"
        />
    }
}

#[component]
pub fn ResolvedChatWindow(
    #[prop(into)] destination: Signal<ChatDestination>,
    #[prop(optional, into)] input_disabled: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let auth_context = expect_context::<AuthContext>();
    let current_user_id = Signal::derive(move || {
        auth_context
            .user
            .with(|account| account.as_ref().map(|account| account.user.uid))
    });
    let active_channel_key =
        Signal::derive(move || ConversationKey::from_destination(&destination.get()));

    let div = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();
    let history = use_thread_history(chat, active_channel_key);
    let thread_ui = use_thread_ui_state(chat, active_channel_key);
    let visible_history_error =
        Signal::derive(move || history.error_for(&active_channel_key.get()));
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
    let visible_send_error =
        Signal::derive(move || chat.chat_send_error(&active_channel_key.get()));

    let merged_messages = Memo::new(move |_| chat.cached_messages(&active_channel_key.get()));

    let render_rows = Memo::new(move |_| {
        let messages = merged_messages.get();
        if messages.is_empty() {
            return Vec::new();
        }

        let split_idx = unread_divider_split_idx(messages.len(), thread_ui.unread_at_open.get());
        let current_user_id = current_user_id.get();

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
    let thread_body_state = Memo::new(move |_| {
        let channel_key = active_channel_key.get();
        let rows = render_rows.get();
        let display_error = visible_thread_error.get();

        if history.is_loading(&channel_key) && rows.is_empty() {
            ThreadBodyState::Loading
        } else if rows.is_empty() {
            display_error.map_or(ThreadBodyState::Empty, ThreadBodyState::ErrorOnly)
        } else {
            ThreadBodyState::Rows {
                rows,
                banner_error: display_error,
            }
        }
    });
    let thread_footer_state = Memo::new(move |_| {
        if visible_history_error.get() == Some(ThreadHistoryError::AccessDenied) {
            ThreadFooterState::Notice(visible_thread_error.get().unwrap_or_default())
        } else {
            ThreadFooterState::Composer
        }
    });

    Effect::watch(
        move || (active_channel_key.get(), merged_messages.get().len()),
        move |(channel_key, count), prev, _| {
            let previous_count = prev.map(|(_, count)| *count).unwrap_or(0);
            if *count == previous_count {
                return;
            }

            let channel_changed =
                prev.is_none_or(|(prev_channel_key, _)| prev_channel_key != channel_key);
            let is_new_message = !channel_changed && previous_count > 0 && *count > previous_count;
            let container = div.get_untracked();
            let target = first_unread_ref.get_untracked();
            let should_auto_scroll = thread_ui.is_scrolled_to_bottom.get_untracked();
            request_animation_frame(move || {
                if channel_changed || !is_new_message {
                    thread_ui.show_jump_to_latest.set(false);
                    if let Some(t) = target {
                        t.scroll_into_view_with_bool(true);
                    } else if let Some(c) = container.as_ref() {
                        c.set_scroll_top(c.scroll_height());
                    }
                    if let Some(c) = container.as_ref() {
                        let at_bottom = is_scrolled_near_bottom(c);
                        thread_ui.is_scrolled_to_bottom.set(at_bottom);
                        if at_bottom {
                            thread_ui.show_jump_to_latest.set(false);
                        }
                    }
                } else if should_auto_scroll {
                    if let Some(c) = container.as_ref() {
                        c.set_scroll_top(c.scroll_height());
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
        if let Some(container) = div.get_untracked() {
            request_animation_frame(move || {
                container.set_scroll_top(container.scroll_height());
                thread_ui.is_scrolled_to_bottom.set(true);
                thread_ui.show_jump_to_latest.set(false);
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
                            thread_ui.is_scrolled_to_bottom.set(at_bottom);
                            if at_bottom {
                                thread_ui.show_jump_to_latest.set(false);
                            }
                        }
                    }
                    class="overflow-y-auto flex-grow p-4 w-full min-w-full h-full min-h-0"
                >
                    {move || match thread_body_state.get() {
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
                                    <div class="flex flex-col gap-2 justify-center items-center h-full text-gray-500 dark:text-gray-400 min-h-[8rem]">
                                        <p class="text-sm font-medium">{error}</p>
                                    </div>
                                },
                            )
                        }
                        ThreadBodyState::Empty => {
                            EitherOf4::C(
                                view! {
                                    <div class="flex flex-col gap-2 justify-center items-center h-full text-gray-500 dark:text-gray-400 min-h-[8rem]">
                                        <span class="text-3xl opacity-40">"✉️"</span>
                                        <p class="text-sm font-medium">
                                            {t!(i18n, messages.chat.empty_title)}
                                        </p>
                                        <p class="text-xs">{t!(i18n, messages.chat.empty_body)}</p>
                                    </div>
                                },
                            )
                        }
                        ThreadBodyState::Rows { rows, banner_error } => {
                            EitherOf4::D(
                                view! {
                                    <MessageRows
                                        rows
                                        banner_error=banner_error
                                        expanded_hidden_messages=thread_ui.expanded_hidden_messages
                                        unread_at_open=thread_ui.unread_at_open
                                        first_unread_ref
                                    />
                                },
                            )
                        }
                    }}
                </div>
                <Show when=thread_ui.show_jump_to_latest>
                    <button
                        type="button"
                        class="absolute bottom-4 left-1/2 z-10 py-2 px-3 text-sm font-medium text-white rounded-full border shadow-lg transition-colors -translate-x-1/2 bg-pillbug-teal border-pillbug-teal/80 hover:bg-pillbug-teal/90"
                        on:click=scroll_to_latest
                    >
                        {t!(i18n, messages.chat.new_messages)}
                        " ↓"
                    </button>
                </Show>
            </div>
            <div class="border-t border-gray-200 dark:border-gray-700 shrink-0">
                {move || match thread_footer_state.get() {
                    ThreadFooterState::Composer => {
                        Either::Left(
                            view! {
                                <div class="flex flex-col gap-2 p-4">
                                    {move || {
                                        visible_send_error
                                            .get()
                                            .map(|error| {
                                                view! {
                                                    <div class="py-2 px-3 text-sm text-red-700 bg-red-50 rounded-lg border border-red-200 dark:text-red-200 dark:border-red-800 dark:bg-red-950/40">
                                                        {error}
                                                    </div>
                                                }
                                            })
                                    }} <ChatInput destination disabled=input_disabled />
                                </div>
                            },
                        )
                    }
                    ThreadFooterState::Notice(error) => {
                        Either::Right(
                            view! {
                                <div class="py-3 px-4 text-sm text-amber-900 bg-amber-100 dark:text-amber-100 dark:bg-amber-950/40">
                                    {error}
                                </div>
                            },
                        )
                    }
                }}
            </div>
        </div>
    }
}

#[component]
pub fn GameChatWindow(
    /// When Some, use the caller-selected thread. When None, derive Players/Spectators from game state.
    explicit_thread: Signal<Option<GameThread>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params_map();
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let route_game_id = Signal::derive(move || {
        params
            .get()
            .get("nanoid")
            .map(|nanoid| GameId(nanoid.to_string()))
    });
    let current_user_id = Signal::derive(move || {
        auth_context
            .user
            .with(|user| user.as_ref().map(|user| user.user.uid))
    });
    let resolved_destination = Signal::derive(move || {
        let route_game_id = route_game_id.get()?;
        let explicit_thread = explicit_thread.get();
        game_state.signal.with(|state| {
            if state.game_id.as_ref() != Some(&route_game_id) {
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
                GameThread::Players => ChatDestination::GamePlayers(route_game_id.clone()),
                GameThread::Spectators => ChatDestination::GameSpectators(route_game_id.clone()),
            })
        })
    });
    view! {
        {move || match resolved_destination.get() {
            Some(destination) => Either::Left(view! { <ResolvedChatWindow destination /> }),
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
