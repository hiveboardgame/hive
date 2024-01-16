use crate::{components::organisms::header::Redirect, providers::auth_context::AuthContext};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Register(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let pathname =
        move || use_context::<Redirect>().unwrap_or(Redirect(RwSignal::new(String::from("/"))));
    let my_input = NodeRef::<html::Input>::new();
    create_effect(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });

    view! {
        <div class=format!("w-full max-w-xs mx-auto pt-20 {extend_tw_classes}")>
            <ActionForm
                action=auth_context.register
                class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800 "
            >
                <label class="block font-bold mb-2">
                    Username
                    <input
                        ref=my_input
                        class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                        name="username"
                        type="text"
                        autocomplete="off"
                        placeholder="Username"
                    />
                </label>
                <label class="font-bold mb-2 hidden">
                    Email
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                        name="email"
                        type="text"
                        autocomplete="off"
                        placeholder="Email(optional)"
                    />
                </label>
                <label class="block font-bold mb-2">
                    Password
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                        name="password"
                        type="password"
                        autocomplete="new-password"
                        placeholder="password"
                    />
                </label>
                <label class="block font-bold mb-2">
                    Confirm Password
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                        name="password_confirmation"
                        type="password"
                        autocomplete="new-password"
                        placeholder="password"
                    />
                </label>
                <input type="hidden" name="pathname" value=pathname().0/>
                <input
                    type="submit"
                    class="bg-blue-500 hover:bg-blue-700 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none"
                    value="Sign Up"
                />

            </ActionForm>

            <a
                class="inline-block align-baseline font-bold text-blue-500 hover:text-blue-800 transform transition-transform duration-300 active:scale-95"
                href="/login"
            >
                Already have an account?
            </a>
        </div>
    }
}
