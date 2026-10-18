use crate::{
    common::{
        auth_page_url,
        password_error_message,
        password_requirements_message,
        use_return_path,
        validate_password,
    },
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
        update_from_event::update_from_input,
    },
    functions::{auth::register::Register, users::username_taken},
    i18n::*,
    providers::AuthContext,
    security::csrf::ActionForm,
};
use leptos::{html, leptos_dom::helpers::debounce, prelude::*};
use shared_types::{validate_username, UsernameError, MAX_USERNAME_LENGTH, MIN_USERNAME_LENGTH};
use std::time::Duration;
use web_sys::Event;

#[component]
pub fn Register() -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let username_taken = Action::new(|user: &String| {
        let user = user.clone();
        async move {
            let taken = username_taken(user.clone()).await;
            (user, taken)
        }
    });
    let pathname = use_return_path();
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    let agree = RwSignal::new(false);
    let username = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let pw = RwSignal::new(String::new());
    let pw_confirm = RwSignal::new(String::new());
    let is_invalid_email = RwSignal::new(false);
    let username_blurred = RwSignal::new(false);
    let email_blurred = RwSignal::new(false);
    let username_validation = Memo::new(move |_| username.with(|name| validate_username(name)));
    let password_validation = Memo::new(move |_| validate_password(&pw(), &pw_confirm()));
    let pw_invalid = move || password_validation.get().is_err();
    let password_blurred = RwSignal::new(false);
    let password_error = move || {
        password_validation
            .get()
            .err()
            .filter(|error| error.is_ready_to_display(password_blurred(), &pw_confirm()))
    };

    let mut check_username = debounce(Duration::from_millis(300), move |name: String| {
        if username_validation.get_untracked().is_ok() {
            username_taken.dispatch(name);
        }
    });
    let on_username_input = move |evt: Event| {
        let name = event_target_value(&evt);
        username.set(name.clone());
        check_username(name);
    };
    let username_exists = move || {
        username_taken.value().with(|value| {
            value.as_ref().is_some_and(|(checked_name, result)| {
                username.with(|name| name == checked_name) && matches!(result, Ok(true))
            })
        })
    };
    let username_check_finished = move || {
        username_taken.value().with(|value| {
            value
                .as_ref()
                .is_some_and(|(checked_name, _)| username.with(|name| name == checked_name))
        })
    };
    let username_error = move || {
        if !username_blurred() || username.with(String::is_empty) {
            return None;
        }
        match username_validation.get() {
            Err(UsernameError::InvalidCharacters) => {
                Some(t_string!(i18n, user_config.create_account.username.error.invalid).to_string())
            }
            Err(UsernameError::TooShort | UsernameError::TooLong) => Some(
                t_string!(
                    i18n,
                    user_config.create_account.username.error.length,
                    min = MIN_USERNAME_LENGTH,
                    max = MAX_USERNAME_LENGTH
                )
                .to_string(),
            ),
            Err(UsernameError::Reserved | UsernameError::DeletedPrefix) => Some(
                t_string!(i18n, user_config.create_account.username.error.reserved).to_string(),
            ),
            Ok(()) if username_exists() => {
                Some(t_string!(i18n, user_config.create_account.username.error.taken).to_string())
            }
            Ok(()) => None,
        }
    };
    let conditionally_disable = move || {
        !agree()
            || username_validation.get().is_err()
            || username_exists()
            || !username_check_finished()
            || pw_invalid()
            || is_invalid_email()
    };
    let register = ServerAction::<Register>::new();
    let register_value = register.value();
    let email_ref = NodeRef::<html::Input>::new();
    let display_register_error = move || register.value().get().is_some_and(|v| v.is_err());
    Effect::watch(
        register.version(),
        move |_, _, _| {
            if let Some(Ok(account)) = register_value.get_untracked() {
                auth_context.accept_user(account);
            }
        },
        false,
    );
    view! {
        <PageShell variant=PageShellVariant::Form>
            <PageCard class="p-6 sm:p-8">
                <ActionForm action=register attr:class="space-y-4">

                    <div class="flex flex-col gap-1">
                        <h1 class="ui-page-title">
                            {t!(i18n, user_config.create_account.signup_button)}
                        </h1>
                        <p class="ui-page-subtitle">
                            {t!(i18n, common.auth.create_an_account_to_play_and_join_tournaments)}
                        </p>
                    </div>

                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">{t!(i18n, common.username)}</span>
                        <input
                            on:input=on_username_input
                            on:blur=move |_| username_blurred.set(true)
                            node_ref=my_input
                            class="ui-field-input"
                            name="username"
                            type="text"
                            prop:value=username
                            autocomplete="username"
                            placeholder=move || { t_string!(i18n, common.username) }
                            minlength=MIN_USERNAME_LENGTH.to_string()
                            maxlength=MAX_USERNAME_LENGTH.to_string()
                        />
                        {move || {
                            username_error()
                                .map(|message| {
                                    view! { <small class="ui-field-error">{message}</small> }
                                })
                        }}
                        <small class="ui-field-helper">
                            {t!(i18n, user_config.create_account.username.description)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">{t!(i18n, common.email)}</span>
                        <input
                            node_ref=email_ref
                            on:blur=move |_| email_blurred.set(true)
                            on:input=move |evt| {
                                if email_ref.get().is_some_and(|e| !e.check_validity()) {
                                    is_invalid_email.set(true);
                                } else {
                                    is_invalid_email.set(false);
                                }
                                email.update(|v| v.clone_from(&event_target_value(&evt)));
                            }
                            class="ui-field-input"
                            name="email"
                            type="email"
                            inputmode="email"
                            prop:value=email
                            autocomplete="email"
                            on:invalid=move |_| is_invalid_email.set(true)
                            placeholder=move || {
                                t_string!(i18n, user_config.create_account.email.description)
                            }
                        />
                        <Show when=move || email_blurred() && is_invalid_email()>
                            <small class="ui-field-error">
                                {t!(i18n, user_config.create_account.email.error.invalid)}
                            </small>
                        </Show>
                        <small class="ui-field-helper">
                            {t!(i18n, user_config.create_account.email.description)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">{t!(i18n, common.password)}</span>
                        <input
                            on:input=update_from_input(pw)
                            on:blur=move |_| password_blurred.set(true)
                            class="ui-field-input"
                            name="password"
                            type="password"
                            prop:value=pw
                            autocomplete="new-password"
                            placeholder=move || t_string!(i18n, common.password)
                        />
                        <small class="ui-field-helper">
                            {move || password_requirements_message(i18n)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">{t!(i18n, common.confirm_password)}</span>
                        <input
                            on:input=update_from_input(pw_confirm)
                            class="ui-field-input"
                            name="password_confirmation"
                            type="password"
                            prop:value=pw_confirm
                            autocomplete="new-password"
                            placeholder=move || t_string!(i18n, common.password)
                        />
                    </label>
                    <Show when=move || password_error().is_some() && !pw().is_empty()>
                        <small class="ui-field-error">
                            {move || {
                                password_error().map(|error| password_error_message(i18n, error))
                            }}
                        </small>
                    </Show>
                    <input type="hidden" name="pathname" value=pathname />
                    <div class="flex gap-2 items-start">
                        <input
                            id="agree-checkbox"
                            on:change=move |_| agree.update(|b| *b = !*b)
                            type="checkbox"
                            class="rounded focus:ring-2 size-4 border-black/20 bg-even-light text-pillbug-teal dark:border-white/20 dark:bg-surface-field focus:ring-pillbug-teal/40"
                            prop:checked=agree
                        />
                        <label
                            for="agree-checkbox"
                            class="text-sm font-medium text-gray-900 dark:text-gray-300"
                        >
                            {t!(i18n, user_config.create_account.be_nice_checkbox)}
                        </label>
                    </div>
                    <p class="text-xs text-gray-600 dark:text-gray-400">
                        {t!(
                            i18n, user_config.create_account.legal_notice,
                            < terms_link > =
                            <a class="ui-text-link" href="/terms" target="_blank" />,
                            < privacy_link > =
                            <a class="ui-text-link" href="/privacy" target="_blank" />
                        )}
                    </p>
                    <button
                        type="submit"
                        disabled=conditionally_disable
                        class="w-full ui-button ui-button-primary ui-button-md"
                    >
                        {move || t_string!(i18n, user_config.create_account.signup_button)}
                    </button>
                    <Show when=display_register_error>
                        <small class="ui-field-error">
                            {t!(i18n, user_config.create_account.registration_error)}
                        </small>
                    </Show>
                </ActionForm>
            </PageCard>

            <p class="text-xs text-center text-gray-500 dark:text-gray-400">
                {t!(
                    i18n, user_config.create_account.existing_account_prompt,
                    < login_link > =
                    <a class="ui-text-link" href=move || auth_page_url("/login", &pathname.get())/>
                )}
            </p>
        </PageShell>
    }
}
