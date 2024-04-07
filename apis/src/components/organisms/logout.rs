use crate::providers::{auth_context::AuthContext, users::UserSignal};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    view! {
        <ActionForm action=auth_context.logout class=format!("flex m-1 {extend_tw_classes}")>

            <button
                on:click=move |_| {
                    let mut online_users = expect_context::<UserSignal>();
                    if let Some(Ok(Some(user))) = (auth_context.user)() {
                        online_users.remove(user.username);
                    }
                }

                class="w-full h-full flex place-content-start bg-ant-blue hover:bg-ladybug-red transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded"
                type="submit"
            >
                Logout
            </button>
        </ActionForm>
    }
}
