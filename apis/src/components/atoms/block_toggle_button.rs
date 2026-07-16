use crate::{
    components::molecules::modal::Modal,
    functions::blocks_mutes::set_user_blocked,
    i18n::*,
    providers::{
        chat::{Chat, ChatSessionToken},
        AuthIdentity,
    },
};
use leptos::{html::Dialog, prelude::*};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct BlockRequest {
    blocked: bool,
    session: ChatSessionToken,
}

#[component]
pub fn BlockToggleButton(blocked_user_id: Uuid) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let is_blocked = Signal::derive(move || chat.is_blocked_user(&blocked_user_id));
    let i18n = use_i18n();
    let error = RwSignal::new(None::<String>);
    let dialog_ref = NodeRef::<Dialog>::new();
    let action = Action::new(move |request: &BlockRequest| {
        let request = *request;
        async move {
            let result = set_user_blocked(blocked_user_id, request.blocked)
                .await
                .map_err(|error| error.to_string());
            (request, result)
        }
    });

    let pending = action.pending();
    let logged_in_and_not_self = move || {
        matches!(
            chat.identity(),
            Some(AuthIdentity::User(current_user_id)) if current_user_id != blocked_user_id
        )
    };

    Effect::watch(
        action.version(),
        move |_, _, _| {
            let Some((request, result)) = action.value().get_untracked() else {
                return;
            };
            if !chat.is_current(request.session) {
                return;
            }
            match result {
                Ok(is_now_blocked) => {
                    error.set(None);
                    chat.apply_blocked_user_update(blocked_user_id, is_now_blocked);
                }
                Err(err) => error.set(Some(err)),
            }
        },
        false,
    );
    let on_toggle_click = move |_| {
        if pending.get_untracked()
            || !matches!(chat.identity_untracked(), Some(AuthIdentity::User(_)))
        {
            return;
        }
        if is_blocked.get_untracked() {
            let Some(session) = chat.current_session_token() else {
                return;
            };
            error.set(None);
            action.dispatch(BlockRequest {
                blocked: false,
                session,
            });
        } else if let Some(dialog) = dialog_ref.get_untracked() {
            let _ = dialog.show_modal();
        }
    };
    let on_confirm_block = move |_| {
        if pending.get_untracked() {
            return;
        }
        let Some(session) = chat.current_session_token() else {
            return;
        };
        if let Some(dialog) = dialog_ref.get_untracked() {
            dialog.close();
        }
        error.set(None);
        action.dispatch(BlockRequest {
            blocked: true,
            session,
        });
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
            <Show
                when=move || chat.inbox_ready()
                fallback=move || {
                    view! {
                        <button
                            type="button"
                            disabled=true
                            aria-busy="true"
                            class="ui-button ui-button-secondary ui-button-sm"
                        >
                            {t!(i18n, messages.page.loading)}
                        </button>
                    }
                }
            >
                <button
                    type="button"
                    title=button_label
                    on:click=on_toggle_click
                    disabled=pending
                    aria-busy=move || pending.get().to_string()
                    class=move || {
                        format!(
                            "ui-button ui-button-sm {}",
                            if is_blocked.get() {
                                "ui-button-secondary"
                            } else {
                                "ui-button-danger"
                            },
                        )
                    }
                >
                    {button_label}
                </button>
                <ShowLet some=move || error.get() let:error>
                    <span class="ui-field-error">{error}</span>
                </ShowLet>
                <Modal dialog_el=dialog_ref aria_labelledby="block-dialog-title">
                    <div class="px-5 pb-5 space-y-4 max-w-md w-[90vw]">
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
                            <form method="dialog">
                                <button
                                    type="submit"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                >
                                    {t!(i18n, messages.block_dialog.cancel)}
                                </button>
                            </form>
                            <button
                                type="button"
                                on:click=on_confirm_block
                                disabled=pending
                                aria-busy=move || pending.get().to_string()
                                class="ui-button ui-button-danger ui-button-sm"
                            >
                                {t!(i18n, messages.block_dialog.confirm)}
                            </button>
                        </div>
                    </div>
                </Modal>
            </Show>
        </Show>
    }
}
