use crate::providers::AuthContext;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::*;
use uuid::Uuid;

use crate::functions::blocks_mutes::add_block;

#[component]
pub fn BlockButton(
    blocked_user_id: Uuid,
    #[prop(optional)] on_success: Option<Callback<()>>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let (show_confirm, set_show_confirm) = signal(false);
    let (pending, set_pending) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let logged_in_and_not_self = move || {
        auth_context.user.with(|a| {
            a.as_ref()
                .is_some_and(|current_user| current_user.user.uid != blocked_user_id)
        })
    };

    let on_confirm_click = move |_| {
        if pending.get() {
            return;
        }
        set_show_confirm.set(false);
        set_pending.set(true);
        set_error.set(None);
        let blocked = blocked_user_id;
        spawn_local(async move {
            let result = add_block(blocked).await;
            set_pending.set(false);
            if let Err(e) = result {
                set_error.set(Some(e.to_string()));
            } else {
                let ctx = expect_context::<AuthContext>();
                ctx.refresh(false);
                if let Some(cb) = on_success {
                    cb.run(());
                }
            }
        });
    };

    let on_cancel_click = move |_| {
        set_show_confirm.set(false);
    };

    view! {
        <Show when=logged_in_and_not_self>
            <button
                type="button"
                title="Block user"
                on:click=move |_| set_show_confirm.set(true)
                disabled=pending
                class="p-1 mx-2 text-white rounded transition-transform duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 disabled:opacity-50"
            >
                <Icon icon=icondata_bi::BiBlockRegular attr:class="size-6" />
            </button>
            <Show when=move || error.get().is_some()>
                <span class="text-xs text-red-500">{move || error.get().unwrap_or_default()}</span>
            </Show>
            <Show when=move || show_confirm.get()>
                <div
                    class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-black/50 dark:bg-black/60"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="block-dialog-title"
                >
                    <div class="bg-white dark:bg-gray-900 rounded-xl shadow-xl max-w-md w-full p-5 border border-gray-200 dark:border-gray-700">
                        <h2 id="block-dialog-title" class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
                            "Block this user?"
                        </h2>
                        <p class="text-sm text-gray-600 dark:text-gray-300 mb-4">
                            "They will no longer be able to send you direct messages. "
                            "In game and tournament chats, their messages will be hidden by default (you can click to reveal). "
                            "You can unblock them later from their profile or from Messages."
                        </p>
                        <div class="flex justify-end gap-2">
                            <button
                                type="button"
                                on:click=on_cancel_click
                                class="px-3 py-1.5 text-sm font-medium rounded-lg text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
                            >
                                "Cancel"
                            </button>
                            <button
                                type="button"
                                on:click=on_confirm_click
                                class="px-3 py-1.5 text-sm font-medium rounded-lg text-white bg-red-600 hover:bg-red-700 dark:bg-red-600 dark:hover:bg-red-700 transition-colors"
                            >
                                "Block"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </Show>
    }
}
