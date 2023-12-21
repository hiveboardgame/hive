use crate::{components::organisms::header::Redirect, functions::accounts::edit::EditAccount};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Account(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let account_action = create_server_action::<EditAccount>();
    let pathname =
        move || use_context::<Redirect>().unwrap_or(Redirect(RwSignal::new(String::from("/"))));
    view! {
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm
                action=account_action
                class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800"
            >
                <label class="block font-bold mb-2" for="email">
                    New Email
                </label>
                <input
                    class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                    id="email"
                    name="new_email"
                    type="email"
                    placeholder="New email"
                />
                <label class="block font-bold mb-2" for="old_password">
                    Current Password
                </label>
                <input
                    class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                    id="old_password"
                    name="password"
                    type="password"
                    placeholder="Current password"
                />
                <label class="block font-bold mb-2" for="new_password">
                    New Password
                </label>
                <input
                    class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                    name="new_password"
                    id="new_password"
                    type="password"
                    placeholder="New password"
                />
                <label class="block font-bold mb-2" for="confirm_password">
                    Confirm Password
                </label>
                <input
                    class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                    id="confirm_password"
                    name="new_password_confirmation"
                    type="password"
                    placeholder="New password (again)"
                />
                <input type="hidden" name="pathname" value=pathname().0/>
                <input
                    type="submit"
                    class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none"
                    value="Save"
                />
            </ActionForm>
        </div>
    }
}
