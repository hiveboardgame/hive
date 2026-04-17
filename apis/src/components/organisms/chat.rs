use crate::{
    chat::ChannelKey,
    components::update_from_event::update_from_input,
    i18n::*,
    providers::{chat::Chat, game_state::GameStateSignal, AuthContext},
};
use chrono::{Duration, Local};
use leptos::{html, leptos_dom::helpers::request_animation_frame, prelude::*, task::spawn_local};
use leptos_router::hooks::use_params_map;
use leptos_use::use_interval_fn;
use shared_types::{ChatDestination, ChatMessage, GameId};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
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
    destination: impl Fn() -> ChatDestination + 'static + Send + Sync,
    disabled: impl Fn() -> bool + 'static + Send + Sync,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let i18n = use_i18n();
    let game_state = use_context::<GameStateSignal>();
    let turn = move || game_state.map(|gs| gs.signal.with(|state| state.state.turn));
    let destination = Arc::new(destination);
    let disabled = Arc::new(disabled);
    let send_destination = Arc::clone(&destination);
    let send_disabled = Arc::clone(&disabled);
    let send = move || {
        if send_disabled() {
            return;
        }
        let message = chat.typed_message.get();
        if !message.is_empty() {
            chat.send(&message, send_destination(), turn());
            chat.typed_message.set(String::new());
        };
    };
    let placeholder_destination = Arc::clone(&destination);
    let placeholder_disabled = Arc::clone(&disabled);
    let placeholder = move || {
        if placeholder_disabled() {
            t_string!(i18n, messages.chat.admin_only)
        } else {
            match placeholder_destination() {
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
    let input_disabled = Arc::clone(&disabled);
    let keydown_disabled = Arc::clone(&disabled);
    view! {
        <input
            node_ref=my_input
            type="text"
            disabled=move || input_disabled()
            class="py-3 px-4 w-full placeholder-gray-500 text-black bg-white rounded-xl border border-gray-300 shadow-inner transition-shadow dark:placeholder-gray-400 dark:text-white dark:bg-gray-800 dark:border-gray-600 focus:ring-2 focus:outline-none disabled:opacity-50 disabled:cursor-not-allowed box-border shrink-0 focus:ring-pillbug-teal/50 focus:border-pillbug-teal"
            prop:value=chat.typed_message
            prop:placeholder=placeholder
            on:input=update_from_input(chat.typed_message)
            on:keydown=move |evt| {
                if evt.key() == "Enter" && !keydown_disabled() {
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
    #[prop(optional)] thread_error: Option<String>,
    #[prop(optional)] hide_input: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let auth_context = expect_context::<AuthContext>();
    let destination = Memo::new(move |_| destination.get());
    let current_user_id = Memo::new(move |_| {
        auth_context
            .user
            .with(|account| account.as_ref().map(|account| account.user.uid))
    });
    let active_channel_key = Memo::new(move |_| {
        ChannelKey::from_destination(&destination.get(), current_user_id.get())
    });
    let preload_channel_keys =
        Memo::new(move |_| active_channel_key.get().into_iter().collect::<Vec<_>>());
    let is_shared_channel = Memo::new(move |_| !matches!(destination.get(), ChatDestination::User(_)));

    let div = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();
    let unread_at_open = RwSignal::new(None::<i64>);
    let history_loading_channels = RwSignal::new(HashSet::<ChannelKey>::new());
    let history_errors = RwSignal::new(HashMap::<ChannelKey, String>::new());
    let show_jump_to_latest = RwSignal::new(false);
    let is_scrolled_to_bottom = RwSignal::new(true);
    let expanded_hidden_messages = RwSignal::new(HashSet::<MessageId>::new());

    Effect::watch(
        move || preload_channel_keys.get(),
        move |channel_keys, prev_channel_keys, _| {
            if prev_channel_keys.is_some_and(|prev| prev == channel_keys) {
                return;
            }

            for key in channel_keys.iter().cloned() {
                let _ = history_loading_channels.try_update(|loading| {
                    loading.insert(key.clone());
                });
                let _ = history_errors.try_update(|errors| {
                    errors.remove(&key);
                });
                spawn_local(async move {
                    let result = chat.fetch_channel_history(&key).await;
                    let _ = history_loading_channels.try_update(|loading| {
                        loading.remove(&key);
                    });
                    match result {
                        Ok(messages) => {
                            let _ = history_errors.try_update(|errors| {
                                errors.remove(&key);
                            });
                            chat.inject_history(&key, messages);
                        }
                        Err(error) => {
                            let _ = history_errors.try_update(|errors| {
                                errors.insert(key.clone(), error);
                            });
                        }
                    }
                });
            }
        },
        true,
    );

    Effect::watch(
        move || active_channel_key.get(),
        move |channel_key, previous, _| {
            if let Some(previous_key) = previous.cloned().flatten() {
                chat.clear_channel_visible(&previous_key);
            }

            unread_at_open.set(None);
            show_jump_to_latest.set(false);
            is_scrolled_to_bottom.set(true);
            expanded_hidden_messages.set(HashSet::new());

            let Some(channel_key) = channel_key.as_ref() else {
                return;
            };

            let unread_key = channel_key.clone();
            let unread = untrack(move || chat.unread_count_for_channel(&unread_key));
            if unread > 0 {
                unread_at_open.set(Some(unread));
            }

            chat.set_channel_visible(channel_key);
            chat.open_channel(channel_key);
        },
        true,
    );
    on_cleanup(move || {
        if let Some(channel_key) = active_channel_key.try_get_untracked().flatten() {
            chat.clear_channel_visible(&channel_key);
        }
    });

    use_interval_fn(
        move || {
            if unread_at_open.get_untracked().is_some() {
                unread_at_open.set(None);
            }
        },
        10_000,
    );

    let active_history_loading = Memo::new(move |_| {
        active_channel_key.get().as_ref().is_some_and(|channel_key| {
            history_loading_channels.with(|loading| loading.contains(channel_key))
        })
    });
    let visible_history_error = Memo::new(move |_| {
        active_channel_key.get().and_then(|channel_key| {
            history_errors.with(|errors| errors.get(&channel_key).cloned())
        })
    });
    let visible_send_error = Memo::new(move |_| {
        active_channel_key
            .get()
            .and_then(|channel_key| chat.chat_send_error(&channel_key))
    });

    let thread_error_for_display = thread_error.clone();
    let visible_thread_error = Memo::new(move |_| {
        let destination = destination.get();
        thread_error_for_display.clone().or_else(|| {
            visible_history_error.get().map(|error| {
                if error.contains("Access denied") {
                    match destination {
                        ChatDestination::User(_) => {
                            t_string!(i18n, messages.page.failed_conversations).to_string()
                        }
                        ChatDestination::TournamentLobby(_) => {
                            t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
                        }
                        _ => error,
                    }
                } else {
                    error
                }
            })
        })
    });
    let thread_has_static_error = thread_error.is_some();
    let composer_hidden = move || {
        hide_input
            || thread_has_static_error
            || visible_history_error
                .get()
                .is_some_and(|error| error.contains("Access denied"))
    };

    let merged_messages = Memo::new(move |_| {
        let mut messages = match destination.get() {
            ChatDestination::TournamentLobby(tournament_id) => chat
                .tournament_lobby_messages
                .with(|threads| threads.get(&tournament_id).cloned().unwrap_or_default()),
            ChatDestination::GamePlayers(game_id) => chat
                .games_private_messages
                .with(|threads| threads.get(&game_id).cloned().unwrap_or_default()),
            ChatDestination::GameSpectators(game_id) => chat
                .games_public_messages
                .with(|threads| threads.get(&game_id).cloned().unwrap_or_default()),
            ChatDestination::User((other_user_id, _)) => chat
                .users_messages
                .with(|threads| threads.get(&other_user_id).cloned().unwrap_or_default()),
            ChatDestination::Global => chat.global_messages.get(),
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
            let should_auto_scroll = is_scrolled_to_bottom.get_untracked();
            request_animation_frame(move || {
                if channel_changed || !is_new_message {
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
                                            let blocked_set = Arc::new(blocked_user_ids);
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
                                                        let sender_blocked = is_shared_channel.get()
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
                    when=move || !composer_hidden()
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
                    <div class="flex flex-col gap-2 p-4">
                        <ShowLet some=move || visible_send_error.get() let:error>
                            <div class="py-2 px-3 text-sm text-red-700 bg-red-50 rounded-lg border border-red-200 dark:text-red-200 dark:bg-red-950/40 dark:border-red-800">
                                {error}
                            </div>
                        </ShowLet>
                        <ChatInput
                            destination=move || destination.get()
                            disabled=move || input_disabled.get()
                        />
                    </div>
                </Show>
            </div>
        </div>
    }
}

#[component]
pub fn GameChatWindow(
    /// When Some, use the caller-selected thread. When None, derive Players/Spectators from game state.
    explicit_thread: Signal<Option<GameChatThread>>,
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
                    GameChatThread::Players
                } else {
                    GameChatThread::Spectators
                }
            });
            Some(match thread {
                GameChatThread::Players => ChatDestination::GamePlayers(route_game_id.clone()),
                GameChatThread::Spectators => {
                    ChatDestination::GameSpectators(route_game_id.clone())
                }
            })
        })
    });
    let active_destination = Signal::derive(move || {
        resolved_destination
            .get()
            .expect("game chat destination is only read once it has resolved")
    });

    view! {
        <Show
            when=move || resolved_destination.get().is_some()
            fallback=move || {
                view! {
                    <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                        {t!(i18n, messages.chat.loading)}
                    </div>
                }
            }
        >
            <ResolvedChatWindow destination=active_destination />
        </Show>
    }
}
