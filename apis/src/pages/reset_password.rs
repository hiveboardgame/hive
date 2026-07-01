use crate::{
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
        update_from_event::update_from_input,
    },
    functions::auth::reset_password::{verify_reset_token, ResetPassword},
};
use leptos::{either::Either, form::ActionForm, leptos_dom::helpers::debounce, prelude::*};
use leptos_router::hooks::use_query_map;
use std::time::Duration;

#[component]
pub fn ResetPassword() -> impl IntoView {
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
                    view! { <small class="ui-page-subtitle">"Checking your reset link…"</small> }
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
                                                    <h1 class="ui-page-title">"Reset your password"</h1>
                                                </div>
                                                <label class="flex flex-col gap-1.5">
                                                    <span class="ui-field-label">"New password"</span>
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
                                                        placeholder="New password"
                                                        minlength="8"
                                                        maxlength="128"
                                                    />
                                                </label>
                                                <label class="flex flex-col gap-1.5">
                                                    <span class="ui-field-label">"Confirm new password"</span>
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
                                                        placeholder="Confirm new password"
                                                        minlength="8"
                                                        maxlength="128"
                                                    />
                                                </label>
                                                <Show when=move || {
                                                    password_invalid() && !new_password().is_empty()
                                                }>
                                                    <small class="ui-field-error">
                                                        "Passwords must match and be at least 8 characters."
                                                    </small>
                                                </Show>
                                                <button
                                                    class="w-full ui-button ui-button-primary ui-button-md"
                                                    type="submit"
                                                    disabled=password_invalid
                                                >
                                                    "Reset password"
                                                </button>
                                                <Show when=display_error>
                                                    <small class="ui-field-error">
                                                        "Couldn't reset your password. The link may have expired — request a new one."
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
                                                    <h1 class="ui-page-title">"This link has expired"</h1>
                                                    <p class="ui-page-subtitle">
                                                        "This password reset link is invalid or has expired. Request a new one and we'll email you a fresh link."
                                                    </p>
                                                </div>
                                                <a class="ui-text-link" href="/forgot-password">
                                                    "Request a new link"
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
