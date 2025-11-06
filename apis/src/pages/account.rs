use crate::components::update_from_event::update_from_input;
use crate::functions::accounts::edit::EditAccount;
use crate::functions::oauth::get_discord_handle;
use crate::providers::RefererContext;
use crate::websocket::new_style::client::ClientApi;
use leptos::form::ActionForm;
use leptos::leptos_dom::helpers::debounce;
use leptos::prelude::*;
use std::time::Duration;

#[component]
pub fn Account(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let account_action = ServerAction::<EditAccount>::new();
    let pathname = expect_context::<RefererContext>().pathname;
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());

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
    let api = expect_context::<ClientApi>();

    let oauth = move |_: leptos::ev::MouseEvent| {
        api.link_discord();
    };
    let discord_name = Action::new(move |_: &()| async { get_discord_handle().await });
    Effect::new(move |_| {
        discord_name.dispatch(());
    });

    view! {
        <div class=format!(
            "mx-auto w-full max-w-md px-4 pt-20 pb-20 sm:max-w-lg md:max-w-xl pt-20 pb-20 {extend_tw_classes}",
        )>
            <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                <div class="mb-6">
                    <h2 class="mb-4 text-xl font-bold text-center text-indigo-600 dark:text-indigo-400">
                        "🤖 Discord Integration"
                    </h2>

                    <div class="p-4 mb-6 bg-blue-50 rounded-lg border border-blue-200 dark:bg-blue-900/30 dark:border-blue-700">
                        <h3 class="mb-2 font-semibold text-blue-800 dark:text-blue-200">
                            "📬 Busybee Messages"
                        </h3>
                        <p class="mb-3 text-sm text-blue-700 dark:text-blue-300">
                            "Get instant notifications about tournament starts, game updates, and important events directly in Discord!"
                        </p>
                        <div class="mb-3 text-xs text-blue-600 dark:text-blue-400">
                            "⚠️ To receive busybee messages, you must:"
                        </div>
                        <ul class="ml-4 space-y-1 text-xs text-blue-600 dark:text-blue-400">
                            <li>"• Join our Discord server"</li>
                            <li>"• Link your Discord account below"</li>
                        </ul>
                    </div>

                    <div class="mb-6 text-center">
                        <div class="mb-3">
                            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                "Not in our Discord yet?"
                            </span>
                        </div>
                        <a
                            href="https://discord.gg/7EwNTJnfab"
                            target="_blank"
                            rel="noopener noreferrer"
                            class="inline-flex items-center px-4 py-2 font-bold text-white bg-purple-600 rounded-lg shadow-lg transition-all duration-300 cursor-pointer no-link-style hover:bg-purple-700 active:scale-95 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2"
                        >
                            <span class="mr-2 text-white">"💬"</span>
                            <span class="text-white">"Join HiveGame Discord"</span>
                            <span class="ml-2 text-white">"↗"</span>
                        </a>
                    </div>

                    <div class="mb-4">
                        <label class="block mb-2 font-semibold text-gray-700 dark:text-gray-300">
                            "🔗 Linked Discord Account"
                        </label>
                        <div class="p-3 bg-gray-100 rounded-lg border dark:bg-gray-700">
                            {move || {
                                match discord_name.value().get() {
                                    Some(Ok(name)) => {
                                        view! {
                                            <span class="font-mono text-green-700 dark:text-green-400">
                                                {name.clone()}
                                            </span>
                                        }
                                            .into_any()
                                    }
                                    Some(Err(_)) => {
                                        view! {
                                            <span class="italic text-red-500 dark:text-red-400">
                                                "Error loading Discord name"
                                            </span>
                                        }
                                            .into_any()
                                    }
                                    None => {
                                        view! {
                                            <span class="italic text-gray-500 dark:text-gray-400">
                                                "No Discord account linked"
                                            </span>
                                        }
                                            .into_any()
                                    }
                                }
                            }}
                        </div>
                    </div>

                    <div class="text-center">
                        <button
                            class="px-4 py-3 w-full font-bold text-white bg-purple-600 rounded-lg transition-all duration-300 cursor-pointer hover:bg-purple-700 active:scale-95 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2"
                            on:click=oauth
                        >
                            <span class="mr-2">"🔗"</span>
                            "Link Discord Account"
                        </button>
                        <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                            "This will redirect you to Discord for authorization"
                        </div>
                    </div>
                </div>
            </div>

            <div class="px-8 pt-6 pb-8 mb-4 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
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
                        class="px-3 py-2 mb-3 w-full leading-tight rounded-lg border shadow appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
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
                        class="px-3 py-2 mb-3 w-full leading-tight rounded-lg border shadow appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
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
                        class="px-3 py-2 mb-3 w-full leading-tight rounded-lg border shadow appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
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
                        class="px-4 py-3 w-full font-bold text-white bg-green-600 rounded-lg transition-all duration-300 cursor-pointer hover:bg-green-700 active:scale-95 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 disabled:opacity-25 disabled:cursor-not-allowed"
                        value="💾 Save Changes"
                    />
                    <Show when=display_account_error>
                        <small class="block mt-2 text-ladybug-red">
                            "Error updating password. Please check your current password and try again."
                        </small>
                    </Show>
                </ActionForm>
            </div>
        </div>
    }
}
