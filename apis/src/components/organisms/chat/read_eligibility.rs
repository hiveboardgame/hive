use crate::providers::chat::{Chat, ConversationHandle, InitialHistoryStatus};
use leptos::{html, leptos_dom::helpers::request_animation_frame, prelude::*};
use leptos_use::{
    use_document_visibility,
    use_intersection_observer_with_options,
    UseIntersectionObserverOptions,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

const SCROLL_BOTTOM_THRESHOLD_PX: i32 = 32;

pub(super) fn is_scrolled_near_bottom(container: &web_sys::HtmlElement) -> bool {
    container.scroll_height() - container.client_height() - container.scroll_top()
        <= SCROLL_BOTTOM_THRESHOLD_PX
}

pub(super) fn is_element_in_scroll_view(
    container: &web_sys::HtmlElement,
    element: &web_sys::HtmlElement,
) -> bool {
    let container_bounds = container.get_bounding_client_rect();
    let element_bounds = element.get_bounding_client_rect();
    element_bounds.bottom() > container_bounds.top()
        && element_bounds.top() < container_bounds.bottom()
}

pub(super) fn use_bottom_visibility(bottom_ref: NodeRef<html::Div>) -> RwSignal<bool> {
    let bottom_visible = RwSignal::new(false);
    _ = use_intersection_observer_with_options(
        bottom_ref,
        move |entries, _| {
            bottom_visible.set(entries.first().is_some_and(|entry| entry.is_intersecting()));
        },
        UseIntersectionObserverOptions::default().thresholds(vec![0.95]),
    );
    bottom_visible
}

pub(super) fn use_thread_read_eligibility(
    chat: Chat,
    conversation: ConversationHandle,
    bottom_visible: RwSignal<bool>,
    messages_ref: NodeRef<html::Div>,
    bottom_ref: NodeRef<html::Div>,
    mounted: Arc<AtomicBool>,
) {
    let document_visibility = use_document_visibility();
    let registered_owner = StoredValue::new(None::<(u64, u64)>);
    let watched_conversation = conversation.clone();

    Effect::watch(
        move || {
            let history_ready = matches!(
                watched_conversation.initial().get(),
                InitialHistoryStatus::Ready { .. }
            );
            (
                chat.session_epoch(),
                document_visibility.get() == web_sys::VisibilityState::Visible,
                bottom_visible.get(),
                if history_ready {
                    watched_conversation
                        .messages()
                        .with(|messages| messages.last().map(|message| message.id))
                        .unwrap_or(0)
                } else {
                    0
                },
            )
        },
        move |(session_epoch, document_visible, bottom_intersecting, latest_message_id),
              _previous,
              _| {
            let session_epoch = *session_epoch;
            let current = registered_owner.get_value();

            if !*document_visible {
                if let Some((_, owner_id)) = current {
                    chat.clear_channel_visible(owner_id);
                    registered_owner.set_value(None);
                }
                return;
            }

            if !current
                .as_ref()
                .is_some_and(|(epoch, _)| *epoch == session_epoch)
            {
                if let Some((_, owner_id)) = current {
                    chat.clear_channel_visible(owner_id);
                }
                let owner_id = chat.set_channel_visible(conversation.key());
                registered_owner.set_value(Some((session_epoch, owner_id)));
            }

            if !*bottom_intersecting || *latest_message_id <= 0 {
                return;
            }
            let conversation = conversation.clone();
            let mounted = Arc::clone(&mounted);
            request_animation_frame(move || {
                if !mounted.load(Ordering::Acquire) {
                    return;
                }
                let bottom_is_really_visible = document_visibility.get_untracked()
                    == web_sys::VisibilityState::Visible
                    && matches!(
                        conversation.initial().get_untracked(),
                        InitialHistoryStatus::Ready { .. }
                    )
                    && bottom_visible.get_untracked()
                    && messages_ref
                        .get_untracked()
                        .zip(bottom_ref.get_untracked())
                        .is_some_and(|(container, bottom)| {
                            is_element_in_scroll_view(&container, &bottom)
                        });
                if bottom_is_really_visible {
                    let latest_message_id = chat.latest_cached_message_id_untracked(&conversation);
                    if latest_message_id > 0 {
                        chat.mark_thread_caught_up(conversation.key(), latest_message_id);
                    }
                }
            });
        },
        true,
    );

    on_cleanup(move || {
        if let Some((_, owner_id)) = registered_owner.get_value() {
            chat.clear_channel_visible(owner_id);
        }
    });
}
