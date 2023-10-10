use crate::components::organisms::hamburger::HamburgerDropdown;
use crate::providers::auth_context::AuthContext;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Logout(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = use_context::<AuthContext>().expect("Failed to get AuthContext");
    let visible = use_context::<RwSignal<HamburgerDropdown>>().expect("An open/closed context");
    let onsubmit = move |_| visible.update(|b| *b = HamburgerDropdown(false));
    view! {
        <ActionForm
            on:submit=onsubmit
            action=auth_context.logout
            class=format!("w-full shadow-md rounded {extend_tw_classes}")
        >
            <input
                type="submit"
                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                value="Logout"
            />
        </ActionForm>
    }
}
