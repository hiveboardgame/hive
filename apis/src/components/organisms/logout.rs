use crate::i18n::*;
use crate::providers::{online_users::OnlineUsersSignal, AuthContext};
use leptos::form::ActionForm;
use leptos::prelude::*;

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let mut online_users = expect_context::<OnlineUsersSignal>();
    let i18n = use_i18n();
    view! {
        <div class=format!("m-1 {extend_tw_classes}")>
            <ActionForm action=auth_context.logout>

                <button
                    on:click=move |_| {
                        auth_context
                            .user
                            .with(|a| {
                                if let Some(account) = a {
                                    online_users.remove(account.user.username.clone());
                                }
                            });
                    }

                    class="flex place-content-start px-4 py-2 w-full h-full font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-ladybug-red active:scale-95"
                    type="submit"
                >
                    {t!(i18n, header.user_menu.logout)}
                </button>
            </ActionForm>
        </div>
    }
}
