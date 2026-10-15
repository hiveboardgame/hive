use crate::{
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
        update_from_event::update_from_input,
    },
    functions::auth::reset_password::{verify_reset_token, ResetPassword},
    i18n::{t, t_string, use_i18n},
    security::csrf::ActionForm,
};
use leptos::{either::Either, leptos_dom::helpers::debounce, prelude::*};
use leptos_router::hooks::use_query_map;
use std::time::Duration;

#[component]
pub fn ResetPassword() -> impl IntoView {
    let i18n = use_i18n();
    let token = StoredValue::new(
        use_query_map()
            .get_untracked()
            .get("token")
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    let valid = OnceResource::new_blocking(verify_reset_token(token.get_value()));
    let action = ServerAction::<ResetPassword>::new();
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let password_invalid = move || {
        let new_pw = new_password();
        !(7 < new_pw.len() && new_pw == confirm_password())
    };
    let display_error = move || action.value().get().is_some_and(|result| result.is_err());
    view! {
        <PageShell variant=PageShellVariant::Form>
            <PageCard class="p-6 sm:p-8">
                <Suspense fallback=move || {
                    view! {
                        <small class="ui-page-subtitle">
                            {t!(i18n, common.auth.checking_your_reset_link)}
                        </small>
                    }
                }>
                    {move || {
                        valid
                            .get()
                            .map(move |result| match result {
                                Ok(true) => {
                                    Either::Left(
                                        view! {
                                            <ActionForm action=action attr:class="space-y-4">

                                                <input type="hidden" name="token" value=token.get_value() />
                                                <div class="flex flex-col gap-1">
                                                    <h1 class="ui-page-title">
                                                        {t!(i18n, common.auth.reset_your_password)}
                                                    </h1>
                                                </div>
                                                <label class="flex flex-col gap-1.5">
                                                    <span class="ui-field-label">
                                                        {t!(i18n, common.new_password)}
                                                    </span>
                                                    <input
                                                        on:input=debounce(
                                                            Duration::from_millis(250),
                                                            update_from_input(new_password),
                                                        )
                                                        class="ui-field-input"
                                                        name="new_password"
                                                        type="password"
                                                        prop:value=new_password
                                                        autocomplete="new-password"
                                                        placeholder=move || t_string!(i18n, common.new_password)
                                                        minlength="8"
                                                        maxlength="128"
                                                    />
                                                </label>
                                                <label class="flex flex-col gap-1.5">
                                                    <span class="ui-field-label">
                                                        {t!(i18n, common.confirm_password)}
                                                    </span>
                                                    <input
                                                        on:input=debounce(
                                                            Duration::from_millis(250),
                                                            update_from_input(confirm_password),
                                                        )
                                                        class="ui-field-input"
                                                        name="new_password_confirmation"
                                                        type="password"
                                                        prop:value=confirm_password
                                                        autocomplete="new-password"
                                                        placeholder=move || t_string!(i18n, common.confirm_password)
                                                        minlength="8"
                                                        maxlength="128"
                                                    />
                                                </label>
                                                <Show when=move || {
                                                    password_invalid() && !new_password().is_empty()
                                                }>
                                                    <small class="ui-field-error">
                                                        {t!(
                                                            i18n, common.auth.passwords_must_match_and_be_at_least_8_characters
                                                        )}
                                                    </small>
                                                </Show>
                                                <button
                                                    class="w-full ui-button ui-button-primary ui-button-md"
                                                    type="submit"
                                                    disabled=password_invalid
                                                >
                                                    {t!(i18n, common.auth.reset_password)}
                                                </button>
                                                <Show when=display_error>
                                                    <small class="ui-field-error">
                                                        {t!(
                                                            i18n, common.auth.couldn_t_reset_your_password_the_link_may_have
                                                        )}
                                                    </small>
                                                </Show>
                                            </ActionForm>
                                        },
                                    )
                                }
                                _ => {
                                    Either::Right(
                                        view! {
                                            <div class="space-y-4">
                                                <div class="flex flex-col gap-1">
                                                    <h1 class="ui-page-title">
                                                        {t!(i18n, common.auth.this_link_has_expired)}
                                                    </h1>
                                                    <p class="ui-page-subtitle">
                                                        {t!(
                                                            i18n, common.auth.this_password_reset_link_is_invalid_or_has_expired
                                                        )}
                                                    </p>
                                                </div>
                                                <a class="ui-text-link" href="/forgot-password">
                                                    {t!(i18n, common.auth.request_a_new_link)}
                                                </a>
                                            </div>
                                        },
                                    )
                                }
                            })
                    }}
                </Suspense>
            </PageCard>
        </PageShell>
    }
}
