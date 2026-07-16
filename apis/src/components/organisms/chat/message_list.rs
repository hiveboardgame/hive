use super::{
    composer::SendErrorMessage,
    history::LoadPreviousMessages,
    read_eligibility::is_scrolled_near_bottom,
};
use crate::{
    i18n::*,
    providers::chat::{
        Chat,
        ConversationHandle,
        InitialHistoryStatus,
        OlderHistoryStatus,
        OutgoingChat,
        OutgoingState,
    },
};
use chrono::Local;
use leptos::{
    either::{Either, EitherOf3, EitherOf4},
    html,
    leptos_dom::helpers::request_animation_frame,
    prelude::*,
};
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use shared_types::ChatMessage;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use uuid::Uuid;

pub(super) fn unread_divider_message_id(
    messages: &[Arc<ChatMessage>],
    unread_at_open: Option<i64>,
    current_user_id: Option<Uuid>,
) -> Option<i64> {
    if messages.is_empty() {
        return None;
    }

    let unread_count = unread_at_open.unwrap_or(0).max(0) as usize;
    if unread_count == 0 {
        return None;
    }

    let mut remaining = unread_count;
    for message in messages.iter().rev() {
        let message_id = message.id;
        if Some(message.user_id) == current_user_id {
            continue;
        }
        remaining = remaining.saturating_sub(1);
        if remaining == 0 {
            return Some(message_id);
        }
    }

    messages
        .get(messages.len().saturating_sub(unread_count))
        .map(|message| message.id)
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct MessageRow {
    pub(super) message: Arc<ChatMessage>,
    pub(super) show_header: bool,
    pub(super) is_current_user: bool,
    pub(super) show_unread_divider_before: bool,
}

#[derive(Copy, Clone)]
pub(super) struct ThreadUiState {
    pub(super) unread_at_open: RwSignal<Option<i64>>,
    unread_anchor_initialized: StoredValue<bool>,
    pub(super) show_jump_to_latest: RwSignal<bool>,
    expanded_hidden_messages: RwSignal<HashSet<i64>>,
    pub(super) incoming_announcement_revision: RwSignal<u64>,
    pub(super) incoming_announcement_below: RwSignal<bool>,
}

impl ThreadUiState {
    fn new() -> Self {
        Self {
            unread_at_open: RwSignal::new(None::<i64>),
            unread_anchor_initialized: StoredValue::new(false),
            show_jump_to_latest: RwSignal::new(false),
            expanded_hidden_messages: RwSignal::new(HashSet::new()),
            incoming_announcement_revision: RwSignal::new(0),
            incoming_announcement_below: RwSignal::new(false),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ThreadBodyState {
    Loading,
    Subscribing,
    ErrorOnly(String),
    Empty,
    Rows { banner_error: Option<String> },
}

pub(super) fn use_thread_ui_state(conversation: ConversationHandle) -> ThreadUiState {
    let thread_ui = ThreadUiState::new();
    let UseTimeoutFnReturn {
        start: start_unread_timeout,
        stop: stop_unread_timeout,
        ..
    } = use_timeout_fn(
        move |_: ()| {
            thread_ui.unread_at_open.set(None);
        },
        10_000.0,
    );

    Effect::watch(
        move || conversation.initial().get(),
        move |initial, previous, _| {
            let history_cycle_changed = previous.is_none_or(|previous_initial| {
                matches!(previous_initial, InitialHistoryStatus::Ready { .. })
                    != matches!(initial, InitialHistoryStatus::Ready { .. })
            });
            if !history_cycle_changed && thread_ui.unread_anchor_initialized.get_value() {
                return;
            }
            if history_cycle_changed {
                thread_ui.unread_anchor_initialized.set_value(false);
                stop_unread_timeout();
            }
            thread_ui.unread_at_open.set(None);
            thread_ui.show_jump_to_latest.set(false);
            thread_ui.expanded_hidden_messages.set(HashSet::new());
            thread_ui.incoming_announcement_revision.set(0);
            thread_ui.incoming_announcement_below.set(false);

            if let InitialHistoryStatus::Ready { unread_anchor, .. } = initial {
                if let Some(unread_anchor) = unread_anchor {
                    let unread = *unread_anchor;
                    if unread > 0 {
                        thread_ui.unread_at_open.set(Some(unread));
                        start_unread_timeout(());
                    }
                }
                thread_ui.unread_anchor_initialized.set_value(true);
            }
        },
        true,
    );

    thread_ui
}

pub(super) fn scroll_to_latest(
    messages_ref: NodeRef<html::Div>,
    thread_ui: ThreadUiState,
    mounted: Arc<AtomicBool>,
) -> Callback<(), ()> {
    Callback::new(move |_: ()| {
        if let Some(container) = messages_ref.get_untracked() {
            let mounted = Arc::clone(&mounted);
            request_animation_frame(move || {
                if !mounted.load(Ordering::Acquire) {
                    return;
                }
                container.set_scroll_top(container.scroll_height());
                thread_ui.show_jump_to_latest.set(false);
            });
        }
    })
}

#[component]
fn MessageRowView(
    row: MessageRow,
    bypass_block_filter: bool,
    expanded_hidden_messages: RwSignal<HashSet<i64>>,
    unread_at_open: RwSignal<Option<i64>>,
    first_unread_ref: NodeRef<html::Div>,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let MessageRow {
        message,
        show_header,
        is_current_user,
        show_unread_divider_before,
    } = row;
    let expanded_message_id = message.id;
    let blocked_user_id = message.user_id;
    let message = StoredValue::new(message);
    let sender_blocked =
        Signal::derive(move || !bypass_block_filter && chat.is_blocked_user(&blocked_user_id));
    let expanded_signal = Signal::derive(move || {
        expanded_hidden_messages.with(|set| set.contains(&expanded_message_id))
    });
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
        <div class=outer_class>
            {move || {
                if sender_blocked.get() && !expanded_signal.get() {
                    Either::Left(
                        view! {
                            <button
                                type="button"
                                class="mb-1 ui-chat-hidden-message"
                                on:click=move |_| {
                                    expanded_hidden_messages
                                        .update(|set| {
                                            set.insert(expanded_message_id);
                                        });
                                }
                            >
                                {t!(i18n, messages.chat.hidden_message)}
                            </button>
                        },
                    )
                } else {
                    Either::Right(
                        view! {
                            <Show when=move || show_header>
                                <div class="flex flex-wrap gap-x-2 items-baseline px-1 mb-1 max-w-[85%] sm:max-w-[75%]">
                                    <span class="text-sm font-bold text-gray-800 dark:text-gray-100 truncate">
                                        {message.with_value(|message| message.username.clone())}
                                    </span>
                                    <span class="text-xs text-gray-500 whitespace-nowrap dark:text-gray-400">
                                        {message
                                            .with_value(|message| {
                                                message
                                                    .timestamp
                                                    .with_timezone(&Local)
                                                    .format("%d/%m %H:%M")
                                                    .to_string()
                                            })}
                                        {move || {
                                            message
                                                .with_value(|message| {
                                                    message
                                                        .turn
                                                        .map(|turn| {
                                                            t_string!(i18n, messages.chat.turn, turn = turn)
                                                        })
                                                        .unwrap_or_default()
                                                })
                                        }}
                                    </span>
                                </div>
                            </Show>
                            <div class=bubble_class>
                                {message.with_value(|message| message.message.clone())}
                            </div>
                        },
                    )
                }
            }}
        </div>
    }
}

#[component]
fn OutgoingMessageRow(outgoing: OutgoingChat, conversation: ConversationHandle) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let client_id = outgoing.client_id();
    let body = outgoing.body().to_string();
    let state = outgoing.state().clone();
    let conversation = StoredValue::new(conversation);

    view! {
        <div class="flex flex-col items-end mb-1 w-full">
            <div class="border-dashed shadow-none ui-chat-bubble ui-chat-bubble-own">{body}</div>
            {match state {
                OutgoingState::Pending => {
                    EitherOf3::A(
                        view! {
                            <span class="px-1 mt-1 text-xs text-gray-600 dark:text-gray-300">
                                {t!(i18n, messages.chat.sending)}
                            </span>
                        },
                    )
                }
                OutgoingState::DeliveryUnknown { last_error } => {
                    EitherOf3::B(
                        view! {
                            <div
                                class="flex flex-wrap gap-2 justify-end items-center px-1 mt-1 text-xs text-gray-700 dark:text-gray-200"
                                role="status"
                            >
                                <span>
                                    {t!(i18n, messages.chat.delivery_unknown)}
                                    {last_error
                                        .map(|error| {
                                            view! { <span>" " <SendErrorMessage error /></span> }
                                        })}
                                </span>
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| {
                                        chat.retry_outgoing(&conversation.get_value(), client_id);
                                    }
                                >
                                    {t!(i18n, messages.chat.retry)}
                                </button>
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| {
                                        chat.dismiss_outgoing(&conversation.get_value(), client_id);
                                    }
                                >
                                    {t!(i18n, messages.chat.dismiss)}
                                </button>
                            </div>
                        },
                    )
                }
                OutgoingState::Failed { error } => {
                    EitherOf3::C(
                        view! {
                            <div
                                class="flex flex-wrap gap-2 justify-end items-center px-1 mt-1 text-xs dark:text-red-300 text-ladybug-red"
                                role="alert"
                            >
                                <SendErrorMessage error />
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| {
                                        chat.dismiss_outgoing(&conversation.get_value(), client_id);
                                    }
                                >
                                    {t!(i18n, messages.chat.dismiss)}
                                </button>
                            </div>
                        },
                    )
                }
            }}
        </div>
    }
}

#[component]
pub(super) fn MessageList(
    conversation: ConversationHandle,
    outgoing: ArcRwSignal<Vec<OutgoingChat>>,
    body_state: Signal<ThreadBodyState>,
    rows: Signal<Vec<MessageRow>>,
    rows_active: Signal<bool>,
    bypass_block_filter: bool,
    initial_history: Signal<InitialHistoryStatus>,
    older_history: Signal<OlderHistoryStatus>,
    retryable_thread_error: Signal<bool>,
    retry_thread_error_disabled: Signal<bool>,
    retry_thread_error: Callback<web_sys::MouseEvent>,
    load_previous: Callback<web_sys::MouseEvent>,
    messages_ref: NodeRef<html::Div>,
    first_unread_ref: NodeRef<html::Div>,
    bottom_ref: NodeRef<html::Div>,
    thread_ui: ThreadUiState,
    scroll_to_latest: Callback<(), ()>,
    compact: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let conversation = StoredValue::new(conversation);
    let outgoing = Signal::derive(move || outgoing.get());
    let message_list_class = if compact {
        "ui-chat-message-list p-2"
    } else {
        "ui-chat-message-list p-3 xs:p-4"
    };
    let conversation_column_class = "mx-auto flex min-h-full w-full max-w-6xl flex-col";
    let banner_error = Memo::new(move |_| {
        body_state.with(|state| match state {
            ThreadBodyState::Rows { banner_error, .. } => banner_error.clone(),
            _ => None,
        })
    });

    view! {
        <div class="relative flex-grow w-full min-w-full h-0 min-h-0">
            <div
                node_ref=messages_ref
                role="log"
                aria-live="off"
                aria-relevant="additions"
                aria-atomic="false"
                aria-label=move || t_string!(i18n, messages.chat.log_label).to_string()
                aria-busy=move || {
                    matches!(
                        body_state.get(),
                        ThreadBodyState::Loading | ThreadBodyState::Subscribing
                    )
                        .to_string()
                }
                on:scroll=move |_| {
                    if let Some(container) = messages_ref.get() {
                        let at_bottom = is_scrolled_near_bottom(&container);
                        if at_bottom {
                            thread_ui.show_jump_to_latest.set(false);
                        }
                    }
                }
                class=message_list_class
            >
                <Show
                    when=rows_active
                    fallback=move || match body_state.get() {
                        state @ (ThreadBodyState::Loading | ThreadBodyState::Subscribing) => {
                            EitherOf4::A(
                                view! {
                                    <div class="flex justify-center items-center h-full text-sm text-gray-500 dark:text-gray-400">
                                        {if matches!(&state, ThreadBodyState::Subscribing) {
                                            t_string!(i18n, messages.chat.subscribing).to_string()
                                        } else {
                                            t_string!(i18n, messages.chat.loading).to_string()
                                        }}
                                    </div>
                                },
                            )
                        }
                        ThreadBodyState::ErrorOnly(error) => {
                            EitherOf4::B(
                                view! {
                                    <div class="flex justify-center items-center h-full min-h-[8rem]">
                                        <div class="max-w-sm ui-empty-state" role="alert">
                                            <p class="text-sm font-medium">{error}</p>
                                            <Show when=retryable_thread_error>
                                                <button
                                                    type="button"
                                                    class="mt-3 ui-button ui-button-secondary ui-button-sm"
                                                    prop:disabled=retry_thread_error_disabled
                                                    on:click=move |event| retry_thread_error.run(event)
                                                >
                                                    {t!(i18n, messages.chat.retry)}
                                                </button>
                                            </Show>
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
                        ThreadBodyState::Rows { .. } => {
                            EitherOf4::D(view! { <div class="hidden"></div> })
                        }
                    }
                >
                    <div class=conversation_column_class>
                        <Show when=move || banner_error.get().is_some()>
                            <div class="mb-4 ui-warning-notice" role="alert">
                                <span>{move || banner_error.get().unwrap_or_default()}</span>
                                <Show when=retryable_thread_error>
                                    <button
                                        type="button"
                                        class="ml-3 ui-button ui-button-secondary ui-button-sm"
                                        prop:disabled=retry_thread_error_disabled
                                        on:click=move |event| retry_thread_error.run(event)
                                    >
                                        {t!(i18n, messages.chat.retry)}
                                    </button>
                                </Show>
                            </div>
                        </Show>
                        <LoadPreviousMessages initial_history older_history load_previous />
                        <For
                            each=rows
                            key=|row| (
                                row.message.id,
                                row.show_header,
                                row.is_current_user,
                                row.show_unread_divider_before,
                            )
                            let:row
                        >
                            <MessageRowView
                                row
                                bypass_block_filter
                                expanded_hidden_messages=thread_ui.expanded_hidden_messages
                                unread_at_open=thread_ui.unread_at_open
                                first_unread_ref
                            />
                        </For>
                        <For
                            each=outgoing
                            key=|outgoing| (outgoing.client_id(), outgoing.state().clone())
                            children=move |outgoing| {
                                view! {
                                    <OutgoingMessageRow
                                        outgoing
                                        conversation=conversation.get_value()
                                    />
                                }
                            }
                        />
                        <div node_ref=bottom_ref class="w-full h-px"></div>
                    </div>
                </Show>
            </div>
            <span class="sr-only" role="status" aria-live="polite" aria-atomic="true">
                <For
                    each=move || {
                        let revision = thread_ui.incoming_announcement_revision.get();
                        (revision > 0).then_some(revision).into_iter()
                    }
                    key=|revision| *revision
                    children=move |_| {
                        view! {
                            <span>
                                {move || {
                                    if thread_ui.incoming_announcement_below.get() {
                                        t_string!(i18n, messages.chat.new_messages_available)
                                            .to_string()
                                    } else {
                                        t_string!(i18n, messages.chat.new_chat_message).to_string()
                                    }
                                }}
                            </span>
                        }
                    }
                />
            </span>
            <Show when=thread_ui.show_jump_to_latest>
                <button
                    type="button"
                    class="absolute bottom-3 left-1/2 z-10 shadow-lg -translate-x-1/2 ui-button ui-button-primary ui-button-sm"
                    on:click=move |_| scroll_to_latest.run(())
                >
                    {t!(i18n, messages.chat.new_messages)}
                </button>
            </Show>
        </div>
    }
}
