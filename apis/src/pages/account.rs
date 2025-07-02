use crate::functions::accounts::edit::EditAccount;
use crate::functions::oauth::get_discord_handle;
use crate::providers::RefererContext;
use crate::{providers::ApiRequestsProvider, providers::AuthContext};
use leptos::form::ActionForm;
use leptos::*;
use leptos::{html, prelude::*};

#[component]
pub fn Account(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let account_action = ServerAction::<EditAccount>::new();
    let pathname = expect_context::<RefererContext>().pathname;
    let my_input = NodeRef::<html::Input>::new();

    Effect::new(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });

    let api = expect_context::<ApiRequestsProvider>();

    let oauth = move |_| {
        api.0.get().link_discord();
    };
    let auth_context = expect_context::<AuthContext>();
    let discord_name = Action::new(move |_: &()| async { get_discord_handle().await });
    Effect::new(move |_| {
        discord_name.dispatch(());
    });

    view! {
        <div class=format!("mx-auto max-w-md pt-20 {extend_tw_classes}")>
            // Discord Integration Section
            <div class="px-8 pt-6 pb-8 mb-6 rounded-lg shadow-lg bg-stone-300 dark:bg-slate-800 border border-stone-400 dark:border-slate-600">
                <div class="mb-6">
                    <h2 class="text-xl font-bold mb-4 text-center text-indigo-600 dark:text-indigo-400">
                        "🤖 Discord Integration"
                    </h2>
                    
                    // Busybee Messages Info
                    <div class="mb-6 p-4 bg-blue-50 dark:bg-blue-900/30 rounded-lg border border-blue-200 dark:border-blue-700">
                        <h3 class="font-semibold mb-2 text-blue-800 dark:text-blue-200">
                            "📬 Busybee Messages"
                        </h3>
                        <p class="text-sm text-blue-700 dark:text-blue-300 mb-3">
                            "Get instant notifications about tournament starts, game updates, and important events directly in Discord!"
                        </p>
                        <div class="text-xs text-blue-600 dark:text-blue-400 mb-3">
                            "⚠️ To receive busybee messages, you must:"
                        </div>
                        <ul class="text-xs text-blue-600 dark:text-blue-400 ml-4 space-y-1">
                            <li>"• Join our Discord server"</li>
                            <li>"• Link your Discord account below"</li>
                        </ul>
                    </div>

                    // Discord Server Invite
                    <div class="mb-6 text-center">
                        <div class="mb-3">
                            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                "Not in our Discord yet?"
                            </span>
                        </div>
                        <a
                            href="https://discord.gg/bCe4HC9G"
                            target="_blank"
                            rel="noopener noreferrer"
                            class="inline-flex items-center px-4 py-2 font-bold text-white bg-purple-600 hover:bg-purple-700 rounded-lg transition-all duration-300 transform cursor-pointer active:scale-95 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 shadow-lg"
                        >
                            <span class="mr-2 text-white">"💬"</span>
                            <span class="text-white">"Join HiveGame Discord"</span>
                            <span class="ml-2 text-white">"↗"</span>
                        </a>
                    </div>

                    // Current Discord Account
                    <div class="mb-4">
                        <label class="block mb-2 font-semibold text-gray-700 dark:text-gray-300">
                            "🔗 Linked Discord Account"
                        </label>
                        <div class="p-3 bg-gray-100 dark:bg-gray-700 rounded-lg border">
                            <Show 
                                when=move || auth_context.user.get().is_some()
                                fallback=|| view! {
                                    <span class="text-gray-500 dark:text-gray-400 italic">
                                        "No Discord account linked"
                                    </span>
                                }
                            >
                                {move || {
                                    match discord_name.value().get() {
                                        Some(Ok(name)) => view! {
                                            <span class="font-mono text-green-700 dark:text-green-400">
                                                {name.clone()}
                                            </span>
                                        }.into_any(),
                                        Some(Err(_)) => view! {
                                            <span class="text-red-500 dark:text-red-400 italic">
                                                "Error loading Discord name"
                                            </span>
                                        }.into_any(),
                                        None => view! {
                                            <span class="text-gray-500 dark:text-gray-400 italic">
                                                "Loading..."
                                            </span>
                                        }.into_any()
                                    }
                                }}
                            </Show>
                        </div>
                    </div>

                    // Link Discord Button
                    <div class="text-center">
                        <button
                            class="w-full px-4 py-3 font-bold text-white rounded-lg transition-all duration-300 transform cursor-pointer bg-purple-600 hover:bg-purple-700 active:scale-95 focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2"
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

            // Password Change Section
            <div class="px-8 pt-6 pb-8 mb-4 rounded-lg shadow-lg bg-stone-300 dark:bg-slate-800 border border-stone-400 dark:border-slate-600">
                <h2 class="text-xl font-bold mb-4 text-center text-gray-700 dark:text-gray-300">
                    "🔐 Change Password"
                </h2>
                <ActionForm
                    action=account_action
                >
                    <label class="hidden mb-2 font-bold" for="email">
                        New Email
                    </label>
                    <input
                        class="hidden px-3 py-2 leading-tight rounded border shadow appearance-none focus:outline-none"
                        id="email"
                        name="new_email"
                        type="email"
                        autocomplete="off"
                        placeholder="New email"
                    />
                    <label class="block mb-2 font-semibold text-gray-700 dark:text-gray-300" for="old_password">
                        Current Password
                    </label>
                    <input
                        node_ref=my_input
                        class="px-3 py-2 mb-3 w-full leading-tight rounded-lg border shadow appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        id="old_password"
                        name="password"
                        type="password"
                        autocomplete="current-password"
                        placeholder="Current password"
                    />
                    <label class="block mb-2 font-semibold text-gray-700 dark:text-gray-300" for="new_password">
                        New Password
                    </label>
                    <input
                        class="px-3 py-2 mb-3 w-full leading-tight rounded-lg border shadow appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        name="new_password"
                        id="new_password"
                        type="password"
                        autocomplete="new-password"
                        placeholder="New password"
                    />
                    <label class="block mb-2 font-semibold text-gray-700 dark:text-gray-300" for="confirm_password">
                        Confirm Password
                    </label>
                    <input
                        class="px-3 py-2 mb-4 w-full leading-tight rounded-lg border shadow appearance-none focus:outline-none focus:ring-2 focus:ring-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                        id="confirm_password"
                        name="new_password_confirmation"
                        type="password"
                        autocomplete="new-password"
                        placeholder="New password (again)"
                    />
                    <input type="hidden" name="pathname" value=pathname.get_value() />
                    <input
                        type="submit"
                        class="w-full px-4 py-3 font-bold text-white rounded-lg transition-all duration-300 transform cursor-pointer bg-green-600 hover:bg-green-700 active:scale-95 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2"
                        value="💾 Save Changes"
                    />
                </ActionForm>
            </div>
        </div>
    }
}
