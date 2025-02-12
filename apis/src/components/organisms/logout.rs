use crate::i18n::*;
use crate::providers::{online_users::OnlineUsersSignal, AuthContext};
use leptos::form::ActionForm;
use leptos::prelude::*;

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let i18n = use_i18n();
    view! {
        <ActionForm action=auth_context.logout prop:class=format!("flex m-1 {extend_tw_classes}")>

            <button
                on:click=move |_| {
                    let mut online_users = expect_context::<OnlineUsersSignal>();
                    if let Some(Ok(user)) = auth_context.user.get() {
                        online_users.remove(user.username);
                    }
                }

                class="flex place-content-start px-4 py-2 w-full h-full font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-ladybug-red active:scale-95"
                type="submit"
            >
                {t!(i18n, header.user_menu.logout)}
            </button>
        </ActionForm>
    }
}
