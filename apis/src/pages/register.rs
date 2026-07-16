use crate::{
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
        update_from_event::update_from_input,
    },
    functions::{auth::register::Register, users::username_taken},
    i18n::*,
    providers::{AuthContext, RefererContext},
};
use leptos::{form::ActionForm, html, leptos_dom::helpers::debounce, prelude::*};
use std::time::Duration;
use web_sys::Event;

const VALID_USERNAME_CHARS: &str = "-_";

#[component]
pub fn Register() -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let username_taken = Action::new(|user: &String| {
        let user = user.clone();
        async move { username_taken(user).await }
    });
    let pathname = expect_context::<RefererContext>().pathname;
    let my_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });
    let agree = RwSignal::new(false);
    let username = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let pw = RwSignal::new(String::new());
    let pw_confirm = RwSignal::new(String::new());
    let has_invalid_char = RwSignal::new(false);
    let is_invalid_email = RwSignal::new(false);
    let pw_invalid = move || {
        let pw = pw();
        !(7 < pw.len() && pw == pw_confirm())
    };

    let validate_username = move |evt: Event| {
        let potential_username = event_target_value(&evt);
        if !potential_username.is_empty() {
            debounce(Duration::from_millis(300), move |evt: Event| {
                let potential_username = event_target_value(&evt);
                if !potential_username.chars().all(valid_username_char) {
                    has_invalid_char.set(true);
                } else {
                    has_invalid_char.set(false);
                    username.update(|v| v.clone_from(&potential_username));
                    if potential_username.len() > 1 {
                        username_taken.dispatch(potential_username);
                    }
                }
            })(evt);
        } else {
            username.set(String::new());
            has_invalid_char.set(false);
        }
    };
    let username_exists = move || {
        let username = username();
        shared_types::RESERVED_USERNAMES.contains(&username.to_lowercase().as_str())
            || username_taken.value().get().is_some_and(|v| {
                if let Ok(value) = v {
                    username.len() > 1 && value
                } else {
                    false
                }
            })
    };
    let conditionally_disable =
        move || !agree() || username_exists() || pw_invalid() || is_invalid_email();
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
                            "Create an account to play and join tournaments."
                        </p>
                    </div>

                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, user_config.create_account.username.title)}
                        </span>
                        <input
                            on:input=validate_username
                            node_ref=my_input
                            class="ui-field-input"
                            name="username"
                            type="text"
                            prop:value=username
                            autocomplete="username"
                            placeholder=move || {
                                t_string!(i18n, user_config.create_account.username.title)
                            }
                            minlength="2"
                            maxlength="20"
                        />
                        <Show when=username_exists>
                            <small class="ui-field-error">
                                {t!(i18n, user_config.create_account.username.error.taken)}
                            </small>
                        </Show>
                        <Show when=has_invalid_char>
                            <small class="ui-field-error">
                                {t!(i18n, user_config.create_account.username.error.invalid)}
                            </small>
                        </Show>
                        <small class="ui-field-helper">
                            {t!(i18n, user_config.create_account.username.description)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">"Email"</span>
                        <input
                            node_ref=email_ref
                            on:input=debounce(
                                Duration::from_millis(350),
                                move |evt| {
                                    if email_ref.get().is_some_and(|e| !e.check_validity()) {
                                        is_invalid_email.set(true);
                                    } else {
                                        is_invalid_email.set(false);
                                    }
                                    email.update(|v| v.clone_from(&event_target_value(&evt)));
                                },
                            )
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
                        <Show when=is_invalid_email>
                            <small class="ui-field-error">
                                {t!(i18n, user_config.create_account.email.error.invalid)}
                            </small>
                        </Show>
                        <small class="ui-field-helper">
                            {t!(i18n, user_config.create_account.email.description)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, user_config.create_account.password)}
                        </span>
                        <input
                            on:input=debounce(Duration::from_millis(350), update_from_input(pw))
                            class="ui-field-input"
                            name="password"
                            type="password"
                            prop:value=pw
                            autocomplete="new-password"
                            placeholder=move || t_string!(i18n, user_config.create_account.password)
                            minlength="8"
                            maxlength="128"
                        />
                        <small class="ui-field-helper">
                            {t!(i18n, user_config.create_account.password_requirements)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, user_config.create_account.confirm_password)}
                        </span>
                        <input
                            on:input=debounce(
                                Duration::from_millis(350),
                                update_from_input(pw_confirm),
                            )
                            class="ui-field-input"
                            name="password_confirmation"
                            type="password"
                            prop:value=pw_confirm
                            autocomplete="new-password"
                            placeholder=move || t_string!(i18n, user_config.create_account.password)
                            minlength="8"
                            maxlength="128"
                        />
                    </label>
                    <Show when=move || pw_invalid() && (!pw().is_empty())>
                        <small class="ui-field-error">
                            {t!(i18n, user_config.create_account.password_error)}
                        </small>
                    </Show>
                    <input type="hidden" name="pathname" value=pathname.get_value() />
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
                    <a class="ui-text-link" href="/login"/>
                )}
            </p>
        </PageShell>
    }
}

fn valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || VALID_USERNAME_CHARS.contains(c)
}
