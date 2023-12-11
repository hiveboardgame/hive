use crate::providers::auth_context::AuthContext;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Logout(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    view! {
        <ActionForm
            action=auth_context.logout
            class=format!("w-full shadow-md rounded {extend_tw_classes}")
        >
            <input
                type="submit"
                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 m-1 rounded focus:outline-none focus:shadow-outline"
                value="Logout"
            />
        </ActionForm>
    }
}
