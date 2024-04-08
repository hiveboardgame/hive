use crate::{components::organisms::header::Redirect, providers::auth_context::AuthContext};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Login(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
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
                action=auth_context.login
                class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800 "
            >
                <label class="block font-bold mb-2" for="email">
                    E-Mail
                    <input
                        ref=my_input
                        class="shadow appearance-none border rounded w-full py-2 px-3 leading-tight focus:outline-none"
                        name="email"
                        id="email"
                        type="text"
                        autocomplete="email"
                        placeholder="E-mail"
                    />
                </label>
                <label class="block font-bold" for="password">
                    Password
                    <input
                        class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                        name="password"
                        id="password"
                        type="password"
                        autocomplete="current-password"
                        placeholder="********"
                    />
                </label>
                <input type="hidden" name="pathname" value=pathname().0/>
                <p class="h-5">
                    <Show when=move || {
                        auth_context.login.value().get().is_some_and(|v| v.is_err())
                    }>
                        <small class="text-ladybug-red">"Invalid email or password"</small>
                    </Show>
                </p>
                <input
                    class="bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer"
                    type="submit"
                    value="Sign In"
                />
            </ActionForm>
            <p class="text-center text-gray-500 text-xs">
                "Don't have an account?"
                <a
                    class="text-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300"
                    href="/register"
                >
                    Sign Up
                </a>
            </p>

        </div>
    }
}
