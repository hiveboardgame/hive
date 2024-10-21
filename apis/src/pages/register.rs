use crate::functions;
use crate::functions::users::get::UsernameTaken;
use crate::i18n::*;
use crate::{
    components::{organisms::header::Redirect, update_from_event::update_from_input},
    providers::AuthContext,
};
use lazy_static::lazy_static;
use leptos::leptos_dom::helpers::debounce;
use leptos::*;
use leptos_router::ActionForm;
use regex::Regex;
use std::time::Duration;
use web_sys::Event;

const BANNED_USERNAMES: [&str; 3] = ["black", "white", "admin"];
const VALID_USERNAME_CHARS: &str = "-_";

lazy_static! {
    static ref EMAIL_RE: Regex =
        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
}

#[component]
pub fn Register(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let login_link = |children| {
        view! {
            <a
                class="text-blue-500 transition-transform duration-300 transform hover:underline"
                href="/login"
            >
                {children}
            </a>
        }
    };
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let username_taken = create_server_action::<UsernameTaken>();
    let pathname =
        move || use_context::<Redirect>().unwrap_or(Redirect(RwSignal::new(String::from("/"))));
    let my_input = NodeRef::<html::Input>::new();
    create_effect(move |_| {
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
                        username_taken.dispatch(functions::users::get::UsernameTaken {
                            username: potential_username,
                        })
                    }
                }
            })(evt);
        } else {
            batch(move || {
                username.set(String::new());
                has_invalid_char.set(false);
            })
        }
    };
    let username_exists = move || {
        let username = username();
        BANNED_USERNAMES.contains(&username.to_lowercase().as_str())
            || username_taken.value().get().is_some_and(|v| {
                if let Ok(value) = v {
                    username.len() > 1 && value
                } else {
                    false
                }
            })
    };
    let conditionally_disable =
        move || batch(move || !agree() || username_exists() || pw_invalid() || is_invalid_email());
    let display_register_error = move || {
        auth_context
            .register
            .value()
            .get()
            .is_some_and(|v| v.is_err())
    };
    view! {
        <div class=format!("w-full max-w-xs mx-auto pt-20 {extend_tw_classes}")>
            <ActionForm
                action=auth_context.register
                class="px-8 pt-6 pb-8 mb-4 rounded shadow-md bg-inherit bg-stone-300 dark:bg-slate-800"
            >
                <label class="block mb-2">
                    <p class="font-bold">{t!(i18n, user_config.create_account.username.title)}</p>
                    <input
                        on:input=validate_username
                        ref=my_input
                        class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="username"
                        type="text"
                        prop:value=username
                        autocomplete="username"
                        placeholder=t!(i18n, user_config.create_account.username.title)
                        attr:minlength="2"
                        attr:maxlength="20"
                    />

                    <Show when=username_exists>
                        <small class="text-ladybug-red">
                            {t!(i18n, user_config.create_account.username.error.taken)}
                        </small>
                    </Show>
                    <Show when=has_invalid_char>
                        <small class="text-ladybug-red">
                            {t!(i18n, user_config.create_account.username.error.invalid)}
                        </small>
                    </Show>
                    <br/>
                    <small>{t!(i18n, user_config.create_account.username.description)}</small>
                </label>
                <label class="mb-2">
                    <p class="font-bold">Email</p>
                    <input
                        on:input=debounce(Duration::from_millis(350), update_from_input(email))

                        on:change=move |evt| {
                            if invalid_email(&event_target_value(&evt)) {
                                is_invalid_email.set(true)
                            } else {
                                is_invalid_email.set(false)
                            }
                        }

                        class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="email"
                        type="email"
                        inputmode="email"
                        prop:value=email
                        autocomplete="email"
                        placeholder=t!(i18n, user_config.create_account.email.description)
                    />

                    <Show when=is_invalid_email>
                        <small class="text-ladybug-red">
                            {t!(i18n, user_config.create_account.email.error.invalid)}
                        </small>
                    </Show>
                    <br/>
                    <small>{t!(i18n, user_config.create_account.email.description)}</small>
                </label>
                <label>
                    <p class="font-bold">{t!(i18n, user_config.create_account.password)}</p>
                    <input
                        on:input=debounce(Duration::from_millis(350), update_from_input(pw))

                        class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="password"
                        type="password"
                        prop:value=pw
                        autocomplete="new-password"
                        placeholder=t!(i18n, user_config.create_account.password)
                        attr:minlength="8"
                        attr:maxlength="128"
                    />
                </label>
                <small>{t!(i18n, user_config.create_account.password_requirements)}</small>
                <label>
                    <p class="font-bold">{t!(i18n, user_config.create_account.confirm_password)}</p>
                    <input
                        on:input=debounce(Duration::from_millis(350), update_from_input(pw_confirm))
                        class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                        name="password_confirmation"
                        type="password"
                        prop:value=pw_confirm
                        autocomplete="new-password"
                        placeholder=t!(i18n, user_config.create_account.password)
                        attr:minlength="8"
                        attr:maxlength="128"
                    />
                </label>

                <Show when=move || pw_invalid() && (!pw().is_empty())>
                    <small class="text-ladybug-red">
                        {t!(i18n, user_config.create_account.password_error)}
                    </small>
                </Show>

                <input type="hidden" name="pathname" value=pathname().0/>
                <div class="flex items-center mb-2">
                    <input
                        on:change=move |_| agree.update(|b| *b = !*b)
                        type="checkbox"
                        class="w-4 h-4 text-blue-600 bg-gray-100 rounded border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                        prop:value=agree
                    />
                    <label
                        for="agree-checkbox"
                        class="ml-2 text-sm font-medium text-gray-900 dark:text-gray-300"
                    >
                        {t!(i18n, user_config.create_account.be_nice_checkbox)}
                    </label>
                </div>
                <input
                    type="submit"
                    disabled=conditionally_disable
                    class="px-4 py-2 font-bold text-white rounded transition-transform duration-300 transform cursor-pointer bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 focus:outline-none disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                    value=t!(i18n, user_config.create_account.signup_button)
                />
                <Show when=display_register_error>
                    <small class="text-ladybug-red">
                        {t!(i18n, user_config.create_account.registration_error)}
                    </small>
                </Show>

            </ActionForm>

            <p class="text-xs text-center text-gray-500">
                {t!(i18n, user_config.create_account.existing_account_prompt, < login_link >)}
            </p>
        </div>
    }
}

fn valid_username_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || VALID_USERNAME_CHARS.contains(c)
}

fn invalid_email(email: &str) -> bool {
    !EMAIL_RE.is_match(email)
}
