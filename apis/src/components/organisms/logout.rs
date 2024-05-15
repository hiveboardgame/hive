use crate::providers::{AuthContext, online_users::OnlineUsersSignal};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    view! {
        <ActionForm action=auth_context.logout class=format!("flex m-1 {extend_tw_classes}")>

            <button
                on:click=move |_| {
                    let mut online_users = expect_context::<OnlineUsersSignal>();
                    if let Some(Ok(Some(user))) = (auth_context.user)() {
                        online_users.remove(user.username);
                    }
                }

                class="flex place-content-start px-4 py-2 w-full h-full font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-ladybug-red active:scale-95"
                type="submit"
            >
                Logout
            </button>
        </ActionForm>
    }
}
