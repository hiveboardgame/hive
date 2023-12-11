use crate::providers::auth_context::AuthContext;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Register(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    view! {
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm
                action=auth_context.register
                class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800 "
            >
                // <div class="mb-4">
                <label class="block font-bold mb-2">
                    Username
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                        name="username"
                        type="text"
                        placeholder="Username"
                    />
                </label>
                // </div>
                // <div class="mb-4">
                <label class="block font-bold mb-2">
                    Email
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                        name="email"
                        type="text"
                        placeholder="Email(optional)"
                    />
                </label>
                // </div>
                // <div class="mb-6">
                <label class="block font-bold mb-2">
                    Password
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                        name="password"
                        type="password"
                        placeholder="hunter2"
                    />
                </label>
                // </div>
                // <div class="mb-6">
                <label class="block font-bold mb-2">
                    Confirm Password
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                        name="password_confirmation"
                        type="password"
                        placeholder="hunter2"
                    />
                </label>
                // </div>
                // <div class="flex items-center justify-between">
                <input
                    type="submit"
                    class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none"
                    value="Sign Up"
                />

            // </div>
            </ActionForm>

            <a
                class="inline-block align-baseline font-bold text-blue-500 hover:text-blue-800"
                href="/login"
            >
                Already have an account?
            </a>
        </div>
    }
}
