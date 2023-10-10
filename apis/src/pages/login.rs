use crate::providers::auth_context::AuthContext;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Login(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = use_context::<AuthContext>().expect("Failed to get AuthContext");
    view! {
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm action=auth_context.login class="bg-white shadow-md rounded px-8 pt-6 pb-8 mb-4">
                <div class="mb-4">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="username">
                        Username
                        <input
                            class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                            name="username"
                            id="username"
                            type="text"
                            placeholder="Username"
                        />
                    </label>
                </div>
                <div class="mb-6">
                    <label class="block text-gray-700 text-sm font-bold mb-2" for="password">
                        Password
                        <input
                            class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline"
                            name="password"
                            id="password"
                            type="password"
                            placeholder="hunter2"
                        />
                    </label>
                </div>
                <div class="flex items-center justify-between">
                    <input
                        class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                        type="submit"
                        value="Sign In"
                    />
                </div>
            </ActionForm>
            <p class="text-center text-gray-500 text-xs">
                "Don't have an account?"
                <a class="text-blue-500 hover:text-blue-800" href="/sign_up">
                    Sign Up
                </a>
            </p>
        </div>
    }
}
