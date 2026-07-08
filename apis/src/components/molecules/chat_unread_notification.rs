use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ChatUnreadNotification(
    #[prop(into)] unread_message_id: Signal<i64>,
    dismissed_unread_message_id: RwSignal<i64>,
    dropdown_open: RwSignal<bool>,
) -> impl IntoView {
    let visible = move || unread_message_id.get() > dismissed_unread_message_id.get();
    let open_messages = move |_| {
        dropdown_open.set(false);
    };
    let dismiss = move |event: web_sys::MouseEvent| {
        event.prevent_default();
        event.stop_propagation();
        dismissed_unread_message_id.set(unread_message_id.get_untracked());
    };

    view! {
        <Show when=visible>
            <div class="ui-notification-item">
                <div class="relative flex-1 min-w-0">
                    <div class="ui-notification-label">Messages</div>
                    <div class="ui-notification-title">You have new messages</div>
                    <a
                        class="absolute top-0 left-0 z-10 size-full"
                        href="/message"
                        aria-label="Open messages"
                        on:click=open_messages
                    ></a>
                </div>
                <button
                    type="button"
                    title="Dismiss"
                    aria-label="Dismiss message notification"
                    on:click=dismiss
                    class="z-20 ui-button ui-button-ghost ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                </button>
            </div>
        </Show>
    }
}
