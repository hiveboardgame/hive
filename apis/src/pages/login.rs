use crate::{components::organisms::header::Redirect, providers::AuthContext};
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
                class="px-8 pt-6 pb-8 mb-4 rounded shadow-md bg-stone-300 dark:bg-reserve-twilight"
            >
                <label class="block mb-2 font-bold" for="email">
                    E-Mail
                    <input
                        ref=my_input
                        class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="email"
                        id="email"
                        type="email"
                        inputmode="email"
                        autocomplete="email"
                        placeholder="E-mail"
                    />
                </label>
                <label class="block font-bold" for="password">
                    Password
                    <input
                        class="px-3 py-2 mb-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
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
                    class="px-4 py-2 font-bold text-white rounded transition-transform duration-300 transform cursor-pointer bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 focus:outline-none"
                    type="submit"
                    value="Sign In"
                />
            </ActionForm>
            <p class="text-xs text-center text-gray-500">
                "Don't have an account?"
                <a
                    class="text-blue-500 transition-transform duration-300 transform hover:underline"
                    href="/register"
                >
                    Sign Up
                </a>
            </p>

        </div>
    }
}
