use crate::{
    common::{password_error_message, password_requirements_message, validate_password},
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
        update_from_event::update_from_input,
    },
    functions::auth::reset_password::{verify_reset_token, ResetPassword},
    i18n::{t, t_string, use_i18n},
    security::csrf::ActionForm,
};
use leptos::{either::Either, prelude::*};
use leptos_router::hooks::use_query_map;

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
    let password_validation =
        Memo::new(move |_| validate_password(&new_password(), &confirm_password()));
    let password_invalid = move || password_validation.get().is_err();
    let password_blurred = RwSignal::new(false);
    let password_error = move || {
        password_validation
            .get()
            .err()
            .filter(|error| error.is_ready_to_display(password_blurred(), &confirm_password()))
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
                                                        on:input=update_from_input(new_password)
                                                        on:blur=move |_| password_blurred.set(true)
                                                        class="ui-field-input"
                                                        name="new_password"
                                                        type="password"
                                                        prop:value=new_password
                                                        autocomplete="new-password"
                                                        placeholder=move || t_string!(i18n, common.new_password)
                                                    />
                                                    <small class="ui-field-helper">
                                                        {move || password_requirements_message(i18n)}
                                                    </small>
                                                </label>
                                                <label class="flex flex-col gap-1.5">
                                                    <span class="ui-field-label">
                                                        {t!(i18n, common.confirm_password)}
                                                    </span>
                                                    <input
                                                        on:input=update_from_input(confirm_password)
                                                        class="ui-field-input"
                                                        name="new_password_confirmation"
                                                        type="password"
                                                        prop:value=confirm_password
                                                        autocomplete="new-password"
                                                        placeholder=move || t_string!(i18n, common.confirm_password)
                                                    />
                                                </label>
                                                <Show when=move || {
                                                    password_error().is_some() && !new_password().is_empty()
                                                }>
                                                    <small class="ui-field-error">
                                                        {move || {
                                                            password_error()
                                                                .map(|error| password_error_message(i18n, error))
                                                        }}
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
