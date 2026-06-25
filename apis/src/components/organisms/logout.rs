use crate::{
    i18n::*,
    providers::{online_users::OnlineUsersSignal, AuthContext},
    pwa,
};
use leptos::{form::ActionForm, prelude::*, task::spawn_local};

#[component]
pub fn Logout() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let mut online_users = expect_context::<OnlineUsersSignal>();
    let i18n = use_i18n();

    let push_endpoint = LocalResource::new(|| async move { pwa::current_endpoint().await });

    view! {
        <ActionForm action=auth_context.logout attr:class="w-full">
            <Show when=move || push_endpoint.get().flatten().is_some()>
                <input
                    type="hidden"
                    name="device_endpoint"
                    value=move || push_endpoint.get().flatten().unwrap_or_default()
                />
            </Show>
            <button
                on:click=move |_| {
                    auth_context
                        .user
                        .with(|a| {
                            if let Some(account) = a {
                                online_users.remove(account.user.username.clone());
                            }
                        });
                    spawn_local(async { pwa::clear_local_subscription().await });
                }

                class="ui-dropdown-link ui-dropdown-link-danger"
                type="submit"
            >
                {t!(i18n, header.user_menu.logout)}
            </button>
        </ActionForm>
    }
}
