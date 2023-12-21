use crate::{components::organisms::header::Redirect, providers::auth_context::AuthContext};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Login(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let pathname =
        move || use_context::<Redirect>().unwrap_or(Redirect(RwSignal::new(String::from("/"))));
    view! {
        <div class=format!("w-full max-w-xs mx-auto mt-20 {extend_tw_classes}")>
            <ActionForm
                action=auth_context.login
                class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800 "
            >
                <label class="block font-bold mb-2" for="username">
                    Username
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                        name="username"
                        id="username"
                        type="text"
                        placeholder="Username"
                    />
                </label>
                <label class="block font-bold mb-2" for="password">
                    Password
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                        name="password"
                        id="password"
                        type="password"
                        placeholder="hunter2"
                    />
                </label>
                <input type="hidden" name="pathname" value=pathname().0/>
                <input
                    class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none"
                    type="submit"
                    value="Sign In"
                />
            </ActionForm>
            <p class="text-center text-gray-500 text-xs">
                "Don't have an account?"
                <a class="text-blue-500 hover:text-blue-800" href="/register">
                    Sign Up
                </a>
            </p>
        </div>
    }
}
