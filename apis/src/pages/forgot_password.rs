use crate::{
    common::{auth_page_url, use_return_path},
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
    },
    functions::auth::forgot_password::ForgotPassword,
    i18n::{t, t_string, use_i18n},
    security::csrf::ActionForm,
};
use leptos::prelude::*;

#[component]
pub fn ForgotPassword() -> impl IntoView {
    let i18n = use_i18n();
    let pathname = use_return_path();
    let action = ServerAction::<ForgotPassword>::new();
    let submitted = move || action.value().get().is_some_and(|result| result.is_ok());
    view! {
        <PageShell variant=PageShellVariant::Form>
            <PageCard class="p-6 sm:p-8">
                <Show
                    when=submitted
                    fallback=move || {
                        view! {
                            <ActionForm action=action attr:class="space-y-4">

                                <div class="flex flex-col gap-1">
                                    <h1 class="ui-page-title">
                                        {t!(i18n, common.auth.forgot_your_password)}
                                    </h1>
                                    <p class="ui-page-subtitle">
                                        {t!(
                                            i18n, common.auth.enter_your_account_email_and_we_ll_send_you
                                        )}
                                    </p>
                                </div>
                                <label class="flex flex-col gap-1.5">
                                    <span class="ui-field-label">{t!(i18n, common.email)}</span>
                                    <input
                                        class="ui-field-input"
                                        name="email"
                                        type="email"
                                        inputmode="email"
                                        autocomplete="email"
                                        placeholder=move || t_string!(i18n, common.email)
                                    />
                                </label>
                                <button
                                    class="w-full ui-button ui-button-primary ui-button-md"
                                    type="submit"
                                >
                                    {t!(i18n, common.auth.send_reset_link)}
                                </button>
                            </ActionForm>
                        }
                    }
                >
                    <div class="space-y-4">
                        <div class="flex flex-col gap-1">
                            <h1 class="ui-page-title">{t!(i18n, common.auth.check_your_inbox)}</h1>
                            <p class="ui-page-subtitle">
                                {t!(
                                    i18n, common.auth.if_an_account_exists_for_that_email_a_password
                                )}
                            </p>
                        </div>
                        <a
                            class="ui-text-link"
                            href=move || auth_page_url("/login", &pathname.get())
                        >
                            {t!(i18n, common.auth.back_to_login)}
                        </a>
                    </div>
                </Show>
            </PageCard>
        </PageShell>
    }
}
