use crate::{
    functions::auth::login::Login,
    i18n::*,
    providers::{AuthContext, RefererContext},
};
use leptos::{form::ActionForm, html, prelude::*};
use leptos_router::hooks::use_navigate;

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
    // Capture the navigate closure in the synchronous component body,
    // where the <Router> owner is live. Calling use_navigate() inside the
    // async Effect::watch callback panics on CSR because the callback
    // fires outside that owner.
    let navigate = use_navigate();
    // Watch login.value() rather than login.version(): version fires on
    // dispatch (value still None) and again on completion, while value
    // transitions None -> Some(Ok(_)) exactly once on success and we can
    // react to that single transition without racing on which signal
    // updates first inside Leptos' batched microtask.
    Effect::watch(
        move || login.value().get(),
        move |result, _, _| {
            let Some(result) = result.as_ref() else {
                return;
            };
            let Ok(response) = result else {
                // Server returned an error (bad credentials, etc.). Surface
                // it via the existing Show-block; nothing to do here.
                return;
            };
            // Capture the bearer token before auth refresh fires —
            // get_account() needs it on the next request in cross-origin
            // (HiveGame mobile) contexts.
            crate::client::set_token(Some(response.token.clone()));
            auth_context.refresh(true);
            // SSR follows the server-issued redirect for us; CSR (HiveGame
            // mobile) doesn't, so push the route client-side too. Idempotent on SSR
            // since we land on the same path either way.
            let target = pathname.get_value();
            let target = if target.is_empty() || target == "/login" {
                "/".to_string()
            } else {
                target
            };
            navigate(&target, Default::default());
        },
        false,
    );
    view! {
        <div class=format!("w-full max-w-xs mx-auto pt-page {extend_tw_classes}")>
            <ActionForm
                action=login
                attr:class="px-8 pt-6 pb-8 mb-4 rounded shadow-md bg-stone-300 dark:bg-reserve-twilight"
            >
                <label class="block mb-2 font-bold" for="email">
                    {t!(i18n, user_config.login.email)}
                    <input
                        node_ref=my_input
                        class="py-2 px-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
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
                        class="py-2 px-3 mb-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
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
                    class="py-2 px-4 font-bold text-white rounded transition-transform duration-300 cursor-pointer focus:outline-none active:scale-95 bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal"
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
