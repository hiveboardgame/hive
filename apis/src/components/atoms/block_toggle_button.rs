use crate::{
    functions::blocks_mutes::{add_block, remove_block},
    i18n::*,
    providers::{chat::Chat, AuthContext},
};
use leptos::{html, prelude::*};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum BlockOperation {
    Block,
    Unblock,
}

#[component]
pub fn BlockToggleButton(
    blocked_user_id: Uuid,
    #[prop(into)] is_blocked: Signal<bool>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let chat = use_context::<Chat>();
    let i18n = use_i18n();
    let show_confirm = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let dialog_ref = NodeRef::<html::Dialog>::new();
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
        auth_context.user.with(|user| {
            user.as_ref()
                .is_some_and(|account| account.user.uid != blocked_user_id)
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
                        if chat.set_blocked_user(blocked_user_id, is_now_blocked) {
                            chat.refresh_blocked_user_ids();
                        }
                    }
                }
                Err(err) => error.set(Some(err.to_string())),
            }
        },
        false,
    );
    Effect::watch(
        show_confirm,
        move |show, _, _| {
            let Some(dialog) = dialog_ref.get() else {
                return;
            };
            if *show {
                if !dialog.open() {
                    let _ = dialog.show_modal();
                }
            } else if dialog.open() {
                dialog.close();
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
                title=button_label
                on:click=on_toggle_click
                disabled=pending
                class=move || {
                    format!(
                        "ui-button ui-button-sm {}",
                        if is_blocked.get() { "ui-button-secondary" } else { "ui-button-danger" },
                    )
                }
            >
                {button_label}
            </button>
            <ShowLet some=move || error.get() let:error>
                <span class="ui-field-error">{error}</span>
            </ShowLet>
            <dialog
                node_ref=dialog_ref
                class="p-5 max-w-md ui-modal-panel w-[90vw]"
                aria-labelledby="block-dialog-title"
                on:cancel=move |event: web_sys::Event| {
                    event.prevent_default();
                    show_confirm.set(false);
                }
            >
                <div class="space-y-4">
                    <div>
                        <h2
                            id="block-dialog-title"
                            class="text-lg font-bold text-gray-900 dark:text-gray-100"
                        >
                            {t!(i18n, messages.block_dialog.title)}
                        </h2>
                        <p class="mt-2 text-sm text-gray-600 dark:text-gray-300">
                            {t!(i18n, messages.block_dialog.body)}
                        </p>
                    </div>
                    <div class="flex flex-wrap gap-2 justify-end">
                        <button
                            type="button"
                            on:click=on_cancel_click
                            class="ui-button ui-button-secondary ui-button-sm"
                        >
                            {t!(i18n, messages.block_dialog.cancel)}
                        </button>
                        <button
                            type="button"
                            on:click=on_confirm_block
                            disabled=pending
                            class="ui-button ui-button-danger ui-button-sm"
                        >
                            {t!(i18n, messages.block_dialog.confirm)}
                        </button>
                    </div>
                </div>
            </dialog>
        </Show>
    }
}
