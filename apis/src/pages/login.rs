use crate::{
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
    },
    functions::auth::login::Login,
    i18n::*,
    providers::{AuthContext, RefererContext},
};
use leptos::{form::ActionForm, html, prelude::*};

#[component]
pub fn Login() -> impl IntoView {
    let i18n = use_i18n();
    let pathname = expect_context::<RefererContext>().pathname;
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    let auth_context = expect_context::<AuthContext>();
    let login = ServerAction::<Login>::new();
    let login_value = login.value();
    Effect::watch(
        login.version(),
        move |_, _, _| {
            if login_value
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                auth_context.refresh(true);
            }
        },
        false,
    );
    view! {
        <PageShell variant=PageShellVariant::Form>
            <PageCard class="p-6 sm:p-8">
                <ActionForm action=login attr:class="space-y-4">
                    <div class="flex flex-col gap-1">
                        <h1 class="ui-page-title">{t!(i18n, user_config.login.login_button)}</h1>
                        <p class="ui-page-subtitle">"Sign in to continue playing."</p>
                    </div>

                    <label class="flex flex-col gap-1.5" for="email">
                        <span class="ui-field-label">{t!(i18n, user_config.login.email)}</span>
                        <input
                            node_ref=my_input
                            class="ui-field-input"
                            name="email"
                            id="email"
                            type="text"
                            autocomplete="email"
                            placeholder=move || t_string!(i18n, user_config.login.email)
                        />
                    </label>
                    <label class="flex flex-col gap-1.5" for="password">
                        <span class="ui-field-label">{t!(i18n, user_config.login.password)}</span>
                        <input
                            class="ui-field-input"
                            name="password"
                            id="password"
                            type="password"
                            autocomplete="current-password"
                            placeholder="********"
                        />
                    </label>
                    <input type="hidden" name="pathname" value=pathname.get_value() />
                    <p class="min-h-5">
                        <Show when=move || { login.value().get().is_some_and(|v| v.is_err()) }>
                            <small class="ui-field-error">
                                {t!(i18n, user_config.login.invalid_login)}
                            </small>
                        </Show>
                    </p>
                    <button class="w-full ui-button ui-button-primary ui-button-md" type="submit">
                        {move || t_string!(i18n, user_config.login.login_button)}
                    </button>
                </ActionForm>
            </PageCard>
            <p class="text-xs text-center">
                <a class="ui-text-link" href="/forgot-password">
                    "Forgot your password?"
                </a>
            </p>
            <p class="text-xs text-center text-gray-500 dark:text-gray-400">
                {t!(
                    i18n, user_config.login.no_account_prompt,
                    < register_link > =
                    <a class="ui-text-link" href="/register"/>
                )}
            </p>
        </PageShell>
    }
}
