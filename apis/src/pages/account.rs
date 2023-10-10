use crate::functions::accounts::edit::EditAccount;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Account(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let account_action = create_server_action::<EditAccount>();
    view! {
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm action=account_action class="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
                <div class="mb-4">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="email">
                        New Email
                    </label>
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                        id="email"
                        name="new_email"
                        type="email"
                        placeholder="New email"
                    />
                </div>
                <div class="mb-6">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="old_password">
                        Current Password
                    </label>
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline"
                        id="old_password"
                        name="password"
                        type="password"
                        placeholder="Current password"
                    />
                </div>
                <div class="mb-6">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="new_password">
                        New Password
                    </label>
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline"
                        name="new_password"
                        id="new_password"
                        type="password"
                        placeholder="New password"
                    />
                </div>

                <div class="mb-6">
                    <label
                        class="block text-gray-700 text-sm font-bold mb-2"
                        for="confirm_password"
                    >
                        Confirm Password
                    </label>
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline"
                        id="confirm_password"
                        name="new_password_confirmation"
                        type="password"
                        placeholder="New password (again)"
                    />
                </div>

                <div class="flex items-center justify-between">
                    <input
                        type="submit"
                        class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                        value="Save"
                    />
                </div>
            </ActionForm>
        </div>
    }
}
