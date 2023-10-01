use crate::functions::auth::logout::Logout;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn LogOut(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let logout_action = create_server_action::<Logout>();
    view! {
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm action=logout_action class="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
                <input
                    type="submit"
                    class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                    value="Logout"
                />
            </ActionForm>
        </div>
    }
}
