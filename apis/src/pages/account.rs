use crate::{
    components::update_from_event::update_from_input,
    functions::accounts::{delete::DeleteAccount, edit::EditAccount},
    i18n::*,
    providers::{AuthContext, RefererContext},
    pwa,
};
use leptos::{form::ActionForm, leptos_dom::helpers::debounce, prelude::*, task::spawn_local};
use std::time::Duration;

#[component]
pub fn Account(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let i18n = use_i18n();
    let account_action = ServerAction::<EditAccount>::new();
    let delete_action = ServerAction::<DeleteAccount>::new();
    let auth_context = expect_context::<AuthContext>();
    let pathname = expect_context::<RefererContext>().pathname;
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let delete_password = RwSignal::new(String::new());

    let password_invalid = move || {
        let new_pw = new_password();
        let confirm_pw = confirm_password();
        !(new_pw.len() > 7 && new_pw == confirm_pw)
    };

    let form_invalid = move || current_password.with(|c| c.len() < 8) || password_invalid();

    let display_account_error = move || {
        account_action
            .value()
            .with(|a| a.as_ref().is_some_and(|v| v.is_err()))
    };
    let display_delete_error = move || {
        delete_action
            .value()
            .with(|a| a.as_ref().is_some_and(|v| v.is_err()))
    };
    let delete_form_invalid = move || delete_password.with(|p| p.len() < 8);
    Effect::watch(
        delete_action.version(),
        move |_, _, _| {
            if delete_action
                .value()
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                spawn_local(pwa::clear_local_subscription());
                auth_context.refresh(true);
            }
        },
        false,
    );

    view! {
        <div class=format!(
            "mx-auto w-full max-w-md px-4 pt-20 pb-20 sm:max-w-lg md:max-w-xl pt-20 pb-20 {extend_tw_classes}",
        )>
            <div class="px-8 pt-6 pb-8 mb-4 rounded-lg border shadow-lg bg-stone-300 border-stone-400 dark:bg-slate-800 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-gray-700 dark:text-gray-300">
                    "🔐 Change Password"
                </h2>
                <ActionForm action=account_action>
                    <label
                        class="block mb-2 font-semibold text-gray-700 dark:text-gray-300"
                        for="old_password"
                    >
                        Current Password
                    </label>
                    <input
                        on:input=debounce(
                            Duration::from_millis(350),
                            update_from_input(current_password),
                        )
                        class="py-2 px-3 mb-3 w-full leading-tight rounded-lg border shadow appearance-none dark:text-white dark:bg-gray-700 dark:border-gray-600 focus:ring-2 focus:ring-blue-500 focus:outline-none"
                        id="old_password"
                        name="password"
                        type="password"
                        prop:value=current_password
                        autocomplete="current-password"
                        placeholder="Current password"
                    />
                    <label
                        class="block mb-2 font-semibold text-gray-700 dark:text-gray-300"
                        for="new_password"
                    >
                        New Password
                    </label>
                    <input
                        on:input=debounce(
                            Duration::from_millis(350),
                            update_from_input(new_password),
                        )
                        class="py-2 px-3 mb-3 w-full leading-tight rounded-lg border shadow appearance-none dark:text-white dark:bg-gray-700 dark:border-gray-600 focus:ring-2 focus:ring-blue-500 focus:outline-none"
                        name="new_password"
                        id="new_password"
                        type="password"
                        prop:value=new_password
                        autocomplete="new-password"
                        placeholder="New password"
                        minlength="8"
                        maxlength="128"
                    />
                    <label
                        class="block mb-2 font-semibold text-gray-700 dark:text-gray-300"
                        for="confirm_password"
                    >
                        Confirm Password
                    </label>
                    <input
                        on:input=debounce(
                            Duration::from_millis(350),
                            update_from_input(confirm_password),
                        )
                        class="py-2 px-3 mb-3 w-full leading-tight rounded-lg border shadow appearance-none dark:text-white dark:bg-gray-700 dark:border-gray-600 focus:ring-2 focus:ring-blue-500 focus:outline-none"
                        id="confirm_password"
                        name="new_password_confirmation"
                        type="password"
                        prop:value=confirm_password
                        autocomplete="new-password"
                        placeholder="New password (again)"
                        minlength="8"
                        maxlength="128"
                    />
                    <Show when=move || password_invalid() && !new_password().is_empty()>
                        <small class="block mb-3 text-ladybug-red">
                            "Password must be at least 8 characters and match confirmation"
                        </small>
                    </Show>
                    <input type="hidden" name="pathname" value=pathname.get_value() />
                    <input
                        type="submit"
                        disabled=form_invalid
                        class="py-3 px-4 w-full font-bold text-white bg-green-600 rounded-lg transition-all duration-300 cursor-pointer hover:bg-green-700 focus:ring-2 focus:ring-green-500 focus:ring-offset-2 focus:outline-none active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed"
                        value="💾 Save Changes"
                    />
                    <Show when=display_account_error>
                        <small class="block mt-2 text-ladybug-red">
                            "Error updating password. Please check your current password and try again."
                        </small>
                    </Show>
                </ActionForm>
            </div>

            <div class="px-8 pt-6 pb-8 mb-4 rounded-lg border shadow-lg bg-stone-300 border-stone-400 dark:bg-slate-800 dark:border-slate-600">
                <h2 class="mb-4 text-xl font-bold text-center text-ladybug-red">
                    {t!(i18n, user_config.delete_account.title)}
                </h2>
                <p class="mb-4 text-sm text-gray-700 dark:text-gray-300">
                    {t!(i18n, user_config.delete_account.description)}
                </p>
                <ActionForm action=delete_action>
                    <label
                        class="block mb-2 font-semibold text-gray-700 dark:text-gray-300"
                        for="delete_password"
                    >
                        {t!(i18n, user_config.delete_account.password)}
                    </label>
                    <input
                        on:input=debounce(
                            Duration::from_millis(350),
                            update_from_input(delete_password),
                        )
                        class="py-2 px-3 mb-3 w-full leading-tight rounded-lg border shadow appearance-none dark:text-white dark:bg-gray-700 dark:border-gray-600 focus:ring-2 focus:outline-none focus:ring-ladybug-red"
                        id="delete_password"
                        name="password"
                        type="password"
                        prop:value=delete_password
                        autocomplete="current-password"
                        placeholder=move || { t_string!(i18n, user_config.delete_account.password) }
                    />
                    <input
                        type="submit"
                        disabled=delete_form_invalid
                        class="py-3 px-4 w-full font-bold text-white rounded-lg transition-all duration-300 cursor-pointer hover:bg-red-700 focus:ring-2 focus:ring-offset-2 focus:outline-none active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed bg-ladybug-red focus:ring-ladybug-red"
                        value=move || t_string!(i18n, user_config.delete_account.button)
                    />
                    <Show when=display_delete_error>
                        <small class="block mt-2 text-ladybug-red">
                            {t!(i18n, user_config.delete_account.error)}
                        </small>
                    </Show>
                </ActionForm>
            </div>
        </div>
    }
}
