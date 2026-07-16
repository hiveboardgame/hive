use super::{
    composer::{Composer, ComposerMode},
    history::{load_previous_callback, use_prepend_anchoring, use_thread_history, PendingPrepend},
    message_list::{
        scroll_to_latest,
        unread_divider_message_id,
        use_thread_ui_state,
        MessageList,
        MessageRow,
        ThreadBodyState,
    },
    read_eligibility::{
        is_element_in_scroll_view,
        use_bottom_visibility,
        use_thread_read_eligibility,
    },
};
use crate::{
    i18n::*,
    providers::{
        chat::{
            Chat,
            InitialHistoryStatus,
            OlderHistoryStatus,
            SubscriptionIssue,
            SubscriptionStatus,
        },
        game_state::GameStateSignal,
        AuthContext,
        AuthIdentity,
    },
};
use chrono::{DateTime, Duration, Utc};
use leptos::{either::Either, html, leptos_dom::helpers::request_animation_frame, prelude::*};
use leptos_router::hooks::use_params_map;
use shared_types::{ConversationKey, GameId, GameThread};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use uuid::Uuid;

const MESSAGE_GROUP_MAX_GAP: Duration = Duration::minutes(2);

#[component]
pub fn ResolvedChatWindow(
    conversation: ConversationKey,
    #[prop(optional, into)] composer_mode: Signal<ComposerMode>,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let conversation = StoredValue::new(conversation);
    // Identity replacement clears the conversation registry, so the fixed body
    // must resolve a fresh handle for the new session.
    view! {
        <For
            each=move || { Some((chat.session_epoch(), conversation.get_value())).into_iter() }
            key=|(session_epoch, conversation)| (*session_epoch, conversation.clone())
            children=move |(_, conversation)| {
                view! { <ResolvedChatWindowBody conversation composer_mode compact /> }
            }
        />
    }
}

#[component]
fn ResolvedChatWindowBody(
    conversation: ConversationKey,
    composer_mode: Signal<ComposerMode>,
    compact: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let chat = expect_context::<Chat>();
    let auth = expect_context::<AuthContext>();
    let current_user_id =
        Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let bypass_block_filter = !conversation.applies_block_filter();
    let conversation_handle = chat.conversation(conversation.clone());
    let messages_ref = NodeRef::<html::Div>::new();
    let first_unread_ref = NodeRef::<html::Div>::new();
    let bottom_ref = NodeRef::<html::Div>::new();
    let mounted = Arc::new(AtomicBool::new(true));
    let mounted_for_cleanup = Arc::clone(&mounted);
    on_cleanup(move || mounted_for_cleanup.store(false, Ordering::Release));
    let subscription_view = chat.use_subscription(conversation.clone());
    let bottom_visible = use_bottom_visibility(bottom_ref);
    use_thread_read_eligibility(
        chat,
        conversation_handle.clone(),
        bottom_visible,
        messages_ref,
        bottom_ref,
        Arc::clone(&mounted),
    );
    use_thread_history(chat, conversation_handle.clone());
    let thread_ui = use_thread_ui_state(conversation_handle.clone());
    let pending_history_prepend = StoredValue::new(None::<PendingPrepend>);
    let initial_history: Signal<InitialHistoryStatus> = conversation_handle.initial().into();
    let older_history: Signal<OlderHistoryStatus> = conversation_handle.older().into();

    let error_conversation_key = conversation.clone();
    let visible_thread_error = Signal::derive(move || match subscription_view.get() {
        SubscriptionStatus::Retryable { issue, .. } | SubscriptionStatus::Failed { issue } => {
            Some(match issue {
                SubscriptionIssue::TimedOut | SubscriptionIssue::Unavailable => {
                    t_string!(i18n, messages.chat.subscription_failed).to_string()
                }
                SubscriptionIssue::AccessDenied => {
                    t_string!(i18n, messages.chat.access_denied).to_string()
                }
                SubscriptionIssue::RateLimited => {
                    t_string!(i18n, messages.chat.subscription_rate_limited).to_string()
                }
            })
        }
        SubscriptionStatus::Pending | SubscriptionStatus::Ready => match initial_history.get() {
            InitialHistoryStatus::AccessDenied => Some(match &error_conversation_key {
                ConversationKey::Direct(_) => {
                    t_string!(i18n, messages.page.failed_conversations).to_string()
                }
                ConversationKey::Tournament(_) => {
                    t_string!(i18n, messages.chat.tournament_read_restricted).to_string()
                }
                _ => t_string!(i18n, messages.chat.access_denied).to_string(),
            }),
            InitialHistoryStatus::Failed => {
                Some(t_string!(i18n, messages.chat.history_failed).to_string())
            }
            _ => match older_history.get() {
                OlderHistoryStatus::Failed => {
                    Some(t_string!(i18n, messages.chat.history_failed).to_string())
                }
                OlderHistoryStatus::Idle | OlderHistoryStatus::Loading(_) => None,
            },
        },
    });
    let retryable_thread_error = Signal::derive(move || {
        matches!(
            subscription_view.get(),
            SubscriptionStatus::Retryable { .. }
        ) || matches!(initial_history.get(), InitialHistoryStatus::Failed)
    });
    let retry_thread_error_disabled = Signal::derive(move || {
        matches!(
            subscription_view.get(),
            SubscriptionStatus::Retryable {
                can_retry_now: false,
                ..
            }
        )
    });
    let retry_key = conversation.clone();
    let retry_conversation = conversation_handle.clone();
    let retry_thread_error = Callback::new(move |_: web_sys::MouseEvent| {
        if matches!(
            subscription_view.get_untracked(),
            SubscriptionStatus::Retryable { .. }
        ) {
            chat.retry_subscription(retry_key.clone(), chat.session_epoch_untracked());
        } else {
            chat.retry_initial_history(retry_conversation.clone());
        }
    });
    let messages = conversation_handle.messages();
    let outgoing = conversation_handle.outgoing();
    let rows = Signal::derive(move || {
        if !matches!(initial_history.get(), InitialHistoryStatus::Ready { .. }) {
            return Vec::new();
        }
        let current_user_id = current_user_id.get();
        let unread_at_open = thread_ui.unread_at_open.get();
        messages.with(|messages| {
            let unread_divider_message_id =
                unread_divider_message_id(messages, unread_at_open, current_user_id);
            let mut previous_persisted_message = None::<(Uuid, DateTime<Utc>)>;
            messages
                .iter()
                .map(|message| {
                    let message_id = message.id;
                    let show_header = previous_persisted_message.is_none_or(
                        |(previous_user_id, previous_timestamp)| {
                            let same_user = previous_user_id == message.user_id;
                            let gap_too_large = (message.timestamp - previous_timestamp).abs()
                                > MESSAGE_GROUP_MAX_GAP;
                            !same_user || gap_too_large
                        },
                    );
                    previous_persisted_message = Some((message.user_id, message.timestamp));
                    let is_current_user = current_user_id == Some(message.user_id);
                    let show_unread_divider_before = unread_divider_message_id == Some(message_id);
                    MessageRow {
                        message: Arc::clone(message),
                        show_header,
                        is_current_user,
                        show_unread_divider_before,
                    }
                })
                .collect::<Vec<_>>()
        })
    });
    let body_state_outgoing = outgoing.clone();
    let body_state = Signal::derive(move || {
        if !bypass_block_filter && !chat.inbox_ready() {
            return ThreadBodyState::Loading;
        }
        let rows_empty = rows.with(Vec::is_empty);
        let outgoing_empty = body_state_outgoing.with(Vec::is_empty);
        let error = visible_thread_error.get();
        let subscription_view = subscription_view.get();
        if !outgoing_empty {
            ThreadBodyState::Rows {
                banner_error: error,
            }
        } else if matches!(subscription_view, SubscriptionStatus::Pending) {
            ThreadBodyState::Subscribing
        } else if matches!(
            subscription_view,
            SubscriptionStatus::Retryable { .. } | SubscriptionStatus::Failed { .. }
        ) && rows_empty
        {
            ThreadBodyState::ErrorOnly(error.unwrap_or_default())
        } else if matches!(
            initial_history.get(),
            InitialHistoryStatus::NotLoaded | InitialHistoryStatus::Loading(_)
        ) {
            ThreadBodyState::Loading
        } else if rows_empty {
            error.map_or(ThreadBodyState::Empty, ThreadBodyState::ErrorOnly)
        } else {
            ThreadBodyState::Rows {
                banner_error: error,
            }
        }
    });
    let rows_active = Signal::derive(move || {
        body_state.with(|state| matches!(state, ThreadBodyState::Rows { .. }))
    });
    let message_bounds_outgoing = outgoing.clone();
    let message_bounds = Signal::derive(move || {
        rows.with(|rows| {
            (
                rows.len() + message_bounds_outgoing.with(Vec::len),
                rows.first().map(|row| row.message.id),
                rows.last().map(|row| row.message.id),
                rows.last().map(|row| row.is_current_user),
                message_bounds_outgoing.with(|outgoing| {
                    outgoing
                        .iter()
                        .map(|outgoing| (outgoing.client_id(), outgoing.state().clone()))
                        .collect::<Vec<_>>()
                }),
            )
        })
    });
    let load_previous = load_previous_callback(
        chat,
        conversation_handle.clone(),
        messages_ref,
        pending_history_prepend,
    );
    use_prepend_anchoring(
        conversation_handle.clone(),
        messages_ref,
        pending_history_prepend,
        Arc::clone(&mounted),
    );
    let effect_mounted = Arc::clone(&mounted);
    Effect::watch(
        move || {
            let (
                count,
                oldest_message_id,
                latest_message_id,
                latest_is_current_user,
                outgoing_states,
            ) = message_bounds.get();
            (
                rows_active.get(),
                count,
                oldest_message_id,
                latest_message_id,
                latest_is_current_user,
                outgoing_states,
            )
        },
        move |(
            rows_are_active,
            count,
            oldest_key,
            latest_key,
            latest_is_current_user,
            outgoing_states,
        ),
              previous,
              _| {
            if !*rows_are_active {
                return;
            }
            let previous_count = previous.map(|(_, count, _, _, _, _)| *count).unwrap_or(0);
            let latest_changed = previous
                .is_none_or(|(_, _, _, previous_latest, _, _)| previous_latest != latest_key);
            let oldest_changed = previous
                .is_none_or(|(_, _, previous_oldest, _, _, _)| previous_oldest != oldest_key);
            let outgoing_changed = previous.is_some_and(|(_, _, _, _, _, previous_outgoing)| {
                previous_outgoing != outgoing_states
            });
            let rows_became_active =
                previous.is_none_or(|(previous_rows_active, _, _, _, _, _)| !*previous_rows_active);
            if !rows_became_active
                && *count == previous_count
                && !latest_changed
                && !oldest_changed
                && !outgoing_changed
            {
                return;
            }
            let prepended_history = *count > previous_count
                && oldest_changed
                && !latest_changed
                && pending_history_prepend.with_value(Option::is_some);
            if prepended_history {
                return;
            }

            let is_new_message = previous_count > 0 && (*count > previous_count || latest_changed);
            let outgoing_layout_changed =
                outgoing_changed && *count == previous_count && !latest_changed && !oldest_changed;
            let container = messages_ref.get_untracked();
            let should_auto_scroll = bottom_visible.get_untracked();
            let announce_incoming_message =
                is_new_message && latest_changed && *latest_is_current_user == Some(false);
            if announce_incoming_message {
                thread_ui
                    .incoming_announcement_below
                    .set(!should_auto_scroll);
                thread_ui
                    .incoming_announcement_revision
                    .update(|revision| *revision = revision.saturating_add(1));
            }
            let mounted = Arc::clone(&effect_mounted);

            request_animation_frame(move || {
                if !mounted.load(Ordering::Acquire) || !rows_active.get_untracked() {
                    return;
                }
                if outgoing_layout_changed {
                    if should_auto_scroll {
                        if let Some(container) = container.as_ref() {
                            container.set_scroll_top(container.scroll_height());
                        }
                        thread_ui.show_jump_to_latest.set(false);
                    } else {
                        thread_ui.show_jump_to_latest.set(true);
                    }
                } else if !is_new_message {
                    thread_ui.show_jump_to_latest.set(false);
                    let first_unread = first_unread_ref.get_untracked();
                    if let (Some(first_unread), Some(container)) =
                        (first_unread, container.as_ref())
                    {
                        if !is_element_in_scroll_view(container, &first_unread) {
                            first_unread.scroll_into_view_with_bool(true);
                        }
                    } else if let Some(container) = container.as_ref() {
                        container.set_scroll_top(container.scroll_height());
                    }
                } else if should_auto_scroll {
                    if let Some(container) = container.as_ref() {
                        container.set_scroll_top(container.scroll_height());
                    }
                    thread_ui.show_jump_to_latest.set(false);
                } else {
                    thread_ui.show_jump_to_latest.set(true);
                }
            });
        },
        true,
    );
    let scroll_to_latest = scroll_to_latest(messages_ref, thread_ui, Arc::clone(&mounted));

    view! {
        <div class="flex overflow-hidden flex-col flex-grow w-full min-w-full max-w-full h-full min-h-0">
            <MessageList
                conversation=conversation_handle.clone()
                outgoing=outgoing.clone()
                body_state
                rows
                rows_active
                bypass_block_filter
                initial_history
                older_history
                retryable_thread_error
                retry_thread_error_disabled
                retry_thread_error
                load_previous
                messages_ref
                first_unread_ref
                bottom_ref
                thread_ui
                scroll_to_latest
                compact
            />
            <Composer conversation=conversation_handle composer_mode compact visible_thread_error />
        </div>
    }
}

#[component]
pub fn GameChatWindow(#[prop(into)] selected_thread: Signal<GameThread>) -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params_map();
    let game_state = expect_context::<GameStateSignal>();
    let route_game_id = Signal::derive(move || {
        params
            .get()
            .get("nanoid")
            .map(|nanoid| GameId(nanoid.to_string()))
    });
    let conversation = Memo::new(move |_| {
        let game_id = route_game_id.get()?;
        let thread = selected_thread.get();
        game_state.signal.with(|state| {
            if state.game_id.as_ref() != Some(&game_id) {
                return None;
            }
            Some(match thread {
                GameThread::Players => ConversationKey::game_players(&game_id),
                GameThread::Spectators => ConversationKey::game_spectators(&game_id),
            })
        })
    });

    view! {
        {move || match conversation.get() {
            Some(conversation) => {
                Either::Left(view! { <ResolvedChatWindow conversation compact=true /> })
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
