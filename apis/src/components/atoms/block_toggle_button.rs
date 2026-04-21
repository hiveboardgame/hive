use crate::{
    functions::blocks_mutes::{add_block, remove_block},
    i18n::*,
    providers::{chat::Chat, AuthContext},
};
use leptos::prelude::*;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum BlockOperation {
    Block,
    Unblock,
}

#[component]
pub fn BlockToggleButton(
    blocked_user_id: Uuid,
    is_blocked: Signal<bool>,
    #[prop(optional)] on_success: Option<Callback<bool>>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let chat = use_context::<Chat>();
    let i18n = use_i18n();
    let on_success = StoredValue::new(on_success);
    let show_confirm = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let action = Action::new(move |operation: &BlockOperation| {
        let operation = *operation;
        async move {
            match operation {
                BlockOperation::Block => add_block(blocked_user_id).await.map(|_| true),
                BlockOperation::Unblock => remove_block(blocked_user_id).await.map(|_| false),
            }
        }
    });

    let pending = Signal::derive(move || action.pending().get());
    let logged_in_and_not_self = move || {
        auth_context.user.with(|a| {
            a.as_ref()
                .is_some_and(|current_user| current_user.user.uid != blocked_user_id)
        })
    };

    Effect::watch(
        action.version(),
        move |_, _, _| {
            let Some(result) = action.value().get_untracked() else {
                return;
            };
            match result {
                Ok(is_now_blocked) => {
                    error.set(None);
                    if let Some(chat) = chat {
                        chat.set_blocked_user(blocked_user_id, is_now_blocked);
                        chat.invalidate_block_list();
                        chat.refresh_messages_hub();
                        chat.refresh_unread_counts();
                    }
                    if let Some(cb) = on_success.get_value() {
                        cb.run(is_now_blocked);
                    }
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        },
        false,
    );

    let on_toggle_click = move |_| {
        if pending.get() {
            return;
        }
        if is_blocked.get() {
            error.set(None);
            action.dispatch(BlockOperation::Unblock);
        } else {
            show_confirm.set(true);
        }
    };

    let on_confirm_block = move |_| {
        if pending.get() {
            return;
        }
        show_confirm.set(false);
        error.set(None);
        action.dispatch(BlockOperation::Block);
    };

    let on_cancel_click = move |_| {
        show_confirm.set(false);
    };
    let button_label = Signal::derive(move || {
        if pending.get() {
            t_string!(i18n, messages.page.loading)
        } else if is_blocked.get() {
            t_string!(i18n, messages.block_dialog.unblock)
        } else {
            t_string!(i18n, messages.block_dialog.confirm)
        }
    });

    view! {
        <Show when=logged_in_and_not_self>
            <button
                type="button"
                title=move || button_label.get()
                on:click=on_toggle_click
                disabled=pending
                class=move || {
                    format!(
                        "inline-flex items-center justify-center px-3 py-1.5 text-sm font-semibold rounded-lg text-white whitespace-nowrap transition-colors duration-300 active:scale-95 disabled:opacity-50 disabled:cursor-not-allowed {}",
                        if is_blocked.get() {
                            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
                        } else {
                            "bg-ladybug-red dark:bg-ladybug-red hover:bg-red-500 dark:hover:bg-red-500"
                        }
                    )
                }
            >
                {move || button_label.get()}
            </button>
            <Show when=move || error.get().is_some()>
                <span class="text-xs text-red-500">{move || error.get().unwrap_or_default()}</span>
            </Show>
            <Show when=move || show_confirm.get() && !is_blocked.get()>
                <div
                    class="flex fixed inset-0 justify-center items-center p-4 z-[100] bg-black/50 dark:bg-black/60"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="block-dialog-title"
                >
                    <div class="p-5 w-full max-w-md bg-white rounded-xl border border-gray-200 shadow-xl dark:bg-gray-900 dark:border-gray-700">
                        <h2
                            id="block-dialog-title"
                            class="mb-2 text-lg font-semibold text-gray-900 dark:text-gray-100"
                        >
                            {t!(i18n, messages.block_dialog.title)}
                        </h2>
                        <p class="mb-4 text-sm text-gray-600 dark:text-gray-300">
                            {t!(i18n, messages.block_dialog.body)}
                        </p>
                        <div class="flex gap-2 justify-end">
                            <button
                                type="button"
                                on:click=on_cancel_click
                                class="py-1.5 px-3 text-sm font-medium text-gray-700 bg-gray-100 rounded-lg transition-colors dark:text-gray-300 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600"
                            >
                                {t!(i18n, messages.block_dialog.cancel)}
                            </button>
                            <button
                                type="button"
                                on:click=on_confirm_block
                                disabled=pending
                                class="py-1.5 px-3 text-sm font-medium text-white bg-red-600 rounded-lg transition-colors dark:bg-red-600 hover:bg-red-700 disabled:opacity-50 dark:hover:bg-red-700"
                            >
                                {t!(i18n, messages.block_dialog.confirm)}
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </Show>
    }
}
