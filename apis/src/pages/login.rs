use crate::functions::auth::login::Login;
use crate::i18n::*;
use crate::providers::{AuthContext, RefererContext};
use leptos::prelude::*;
use leptos::{form::ActionForm, html};

#[component]
pub fn Login(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let i18n = use_i18n();
    let pathname = expect_context::<RefererContext>().pathname;
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    let auth_context = expect_context::<AuthContext>();
    let login = ServerAction::<Login>::new();
    Effect::watch(
        login.version(),
        move |_, _, _| auth_context.refresh(true),
        false,
    );
    view! {
        <div class=format!("w-full max-w-xs mx-auto pt-20 {extend_tw_classes}")>
            <ActionForm
                action=login
                attr:class="px-8 pt-6 pb-8 mb-4 rounded shadow-md bg-stone-300 dark:bg-reserve-twilight"
            >
                <label class="block mb-2 font-bold" for="email">
                    {t!(i18n, user_config.login.email)}
                    <input
                        node_ref=my_input
                        class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="email"
                        id="email"
                        type="text"
                        autocomplete="email"
                        placeholder=move || t_string!(i18n, user_config.login.email)
                    />
                </label>
                <label class="block font-bold" for="password">
                    {t!(i18n, user_config.login.password)}
                    <input
                        class="px-3 py-2 mb-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="password"
                        id="password"
                        type="password"
                        autocomplete="current-password"
                        placeholder="********"
                    />
                </label>
                <input type="hidden" name="pathname" value=pathname.get_value() />
                <p class="h-5">
                    <Show when=move || { login.value().get().is_some_and(|v| v.is_err()) }>
                        <small class="text-ladybug-red">
                            {t!(i18n, user_config.login.invalid_login)}
                        </small>
                    </Show>
                </p>
                <input
                    class="px-4 py-2 font-bold text-white rounded transition-transform duration-300 cursor-pointer bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 focus:outline-none"
                    type="submit"
                    value=move || t_string!(i18n, user_config.login.login_button)
                />
            </ActionForm>
            <p class="text-xs text-center text-gray-500">
                {t!(
                    i18n, user_config.login.no_account_prompt,
                    < register_link > =
                    <a class="text-blue-500 transition-transform duration-300 hover:underline" href="/register"/>
                )}
            </p>
        </div>
    }
}
