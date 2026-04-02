use crate::providers::AuthContext;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_icons::*;
use uuid::Uuid;

use crate::functions::blocks_mutes::remove_block;

#[component]
pub fn UnblockButton(
    blocked_user_id: Uuid,
    #[prop(optional)] on_success: Option<Callback<()>>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let (pending, set_pending) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let logged_in_and_not_self = move || {
        auth_context.user.with(|a| {
            a.as_ref()
                .is_some_and(|current_user| current_user.user.uid != blocked_user_id)
        })
    };

    let on_click = move |_| {
        if pending.get() {
            return;
        }
        set_pending.set(true);
        set_error.set(None);
        let blocked = blocked_user_id;
        spawn_local(async move {
            let result = remove_block(blocked).await;
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

    view! {
        <Show when=logged_in_and_not_self>
            <button
                title="Unblock user"
                on:click=on_click
                disabled=pending
                class="p-1 mx-2 text-white rounded transition-transform duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 disabled:opacity-50"
            >
                <Icon icon=icondata_bi::BiCheckCircleRegular attr:class="size-6" />
            </button>
            <Show when=move || error.get().is_some()>
                <span class="text-xs text-red-500">{move || error.get().unwrap_or_default()}</span>
            </Show>
        </Show>
    }
}
