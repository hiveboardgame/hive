use crate::{
    i18n::*,
    providers::chat::{Chat, ConversationHandle, InitialHistoryStatus, OlderHistoryStatus},
};
use leptos::{html, leptos_dom::helpers::request_animation_frame, prelude::*};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
pub(super) struct PendingPrepend {
    start_revision: u64,
    scroll_height: i32,
    scroll_top: i32,
}

pub(super) fn use_thread_history(chat: Chat, conversation: ConversationHandle) {
    let watched_conversation = conversation.clone();
    Effect::watch(
        move || {
            let session_epoch = chat.session_epoch();
            let identity_resolved = chat.identity().is_some();
            let subscription_ready =
                chat.subscription_ready_for_history(watched_conversation.key(), session_epoch);
            (
                session_epoch,
                identity_resolved,
                subscription_ready,
                watched_conversation.initial().get(),
            )
        },
        move |(session_epoch, identity_resolved, subscription_ready, initial), previous, _| {
            if !*identity_resolved || !*subscription_ready {
                return;
            }
            let should_request = matches!(initial, InitialHistoryStatus::NotLoaded)
                || matches!(
                    initial,
                    InitialHistoryStatus::AccessDenied | InitialHistoryStatus::Failed
                ) && previous.is_none_or(
                    |(
                        previous_session_epoch,
                        previous_identity_resolved,
                        previous_subscription_ready,
                        _,
                    )| {
                        previous_session_epoch != session_epoch
                            || !*previous_identity_resolved
                            || !*previous_subscription_ready
                    },
                );
            if should_request {
                chat.ensure_initial_history(conversation.clone());
            }
        },
        true,
    );
}

pub(super) fn load_previous_callback(
    chat: Chat,
    conversation: ConversationHandle,
    messages_ref: NodeRef<html::Div>,
    pending_history_prepend: StoredValue<Option<PendingPrepend>>,
) -> Callback<web_sys::MouseEvent> {
    Callback::new(move |_| {
        let Some(container) = messages_ref.get_untracked() else {
            return;
        };
        pending_history_prepend.set_value(Some(PendingPrepend {
            start_revision: conversation.prepend_revision().get_untracked(),
            scroll_height: container.scroll_height(),
            scroll_top: container.scroll_top(),
        }));
        if !chat.load_older_history(conversation.clone()) {
            pending_history_prepend.set_value(None);
        }
    })
}

pub(super) fn use_prepend_anchoring(
    conversation: ConversationHandle,
    messages_ref: NodeRef<html::Div>,
    pending_history_prepend: StoredValue<Option<PendingPrepend>>,
    mounted: Arc<AtomicBool>,
) {
    let watched_conversation = conversation.clone();
    Effect::watch(
        move || {
            (
                watched_conversation.prepend_revision().get(),
                watched_conversation.older().get(),
                watched_conversation.initial().get(),
            )
        },
        {
            let mounted = Arc::clone(&mounted);
            move |(revision, older, initial), _, _| {
                let Some(pending) = pending_history_prepend.get_value() else {
                    return;
                };
                if *revision <= pending.start_revision {
                    if matches!(older, OlderHistoryStatus::Failed)
                        || !matches!(initial, InitialHistoryStatus::Ready { .. })
                    {
                        pending_history_prepend.set_value(None);
                    }
                    return;
                }
                let mounted = Arc::clone(&mounted);
                request_animation_frame(move || {
                    if !mounted.load(Ordering::Acquire) {
                        return;
                    }
                    if let Some(container) = messages_ref.get_untracked() {
                        let added_height = container.scroll_height() - pending.scroll_height;
                        container.set_scroll_top(pending.scroll_top + added_height);
                    }
                    pending_history_prepend.set_value(None);
                });
            }
        },
        true,
    );
}

#[component]
pub(super) fn LoadPreviousMessages(
    initial_history: Signal<InitialHistoryStatus>,
    older_history: Signal<OlderHistoryStatus>,
    load_previous: Callback<web_sys::MouseEvent>,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Show when=move || {
            matches!(
                initial_history.get(),
                InitialHistoryStatus::Ready { next_before_message_id: Some(_), .. }
            )
        }>
            <div class="flex justify-center mb-4">
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    prop:disabled=move || {
                        matches!(older_history.get(), OlderHistoryStatus::Loading(_))
                    }
                    aria-busy=move || {
                        matches!(older_history.get(), OlderHistoryStatus::Loading(_)).to_string()
                    }
                    on:click=move |event| load_previous.run(event)
                >
                    {move || {
                        if matches!(older_history.get(), OlderHistoryStatus::Loading(_)) {
                            t_string!(i18n, messages.page.loading).to_string()
                        } else {
                            t_string!(i18n, messages.chat.load_previous_messages).to_string()
                        }
                    }}
                </button>
            </div>
        </Show>
    }
}
