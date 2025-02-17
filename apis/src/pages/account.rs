use crate::functions;
use crate::functions::oauth::GetDiscordHandle;
use crate::{
    components::molecules::banner::Banner,
    components::organisms::header::Redirect,
    functions::accounts::{discord_handle::DiscordHandle, edit::EditAccount},
    providers::ApiRequests,
    providers::AuthContext,
};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Account(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let account_action = create_server_action::<EditAccount>();
    let pathname =
        move || use_context::<Redirect>().unwrap_or(Redirect(RwSignal::new(String::from("/"))));
    let my_input = NodeRef::<html::Input>::new();

    create_effect(move |_| {
        let _ = my_input.get_untracked().map(|el| el.focus());
    });

    let oauth = move |_| {
        let api = ApiRequests::new();
        api.link_discord();
    };
    let auth_context = expect_context::<AuthContext>();
    let discord_name = create_server_action::<GetDiscordHandle>();
    discord_name.dispatch(functions::oauth::GetDiscordHandle {});

    view! {
        <div class=format!("mx-auto max-w-xs pt-20 {extend_tw_classes}")>
            <div class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800">
                <div>
                    <label class="block font-bold mb-2" for="old_password">
                        Linked Discord account
                    </label>
                    <Show when=move || {
                        matches!((auth_context.user)(), Some(Ok(Some(_account))))
                    }>{move || { discord_name.value().get() }}</Show>
                </div>
                <div class="pt6">
                    <button
                        class="bg-button-dawn dark:bg-button-twilight transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer"
                        on:click=oauth
                    >
                        Link Discord
                    </button>
                </div>
            </div>
        </div>

        <div class=format!("mx-auto max-w-xs {extend_tw_classes}")>
            <ActionForm
                action=account_action
                class="bg-inherit shadow-md rounded px-8 pt-6 pb-8 mb-4 bg-stone-300 dark:bg-slate-800"
            >
                <label class="hidden font-bold mb-2" for="email">
                    New Email
                </label>
                <input
                    class="hidden shadow appearance-none border rounded py-2 px-3 leading-tight focus:outline-none"
                    id="email"
                    name="new_email"
                    type="email"
                    autocomplete="off"
                    placeholder="New email"
                />
                <label class="block font-bold mb-2" for="old_password">
                    Current Password
                </label>
                <input
                    ref=my_input
                    class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                    id="old_password"
                    name="password"
                    type="password"
                    autocomplete="current-password"
                    placeholder="Current password"
                />
                <label class="block font-bold mb-2" for="new_password">
                    New Password
                </label>
                <input
                    class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                    name="new_password"
                    id="new_password"
                    type="password"
                    autocomplete="new-password"
                    placeholder="New password"
                />
                <label class="block font-bold mb-2" for="confirm_password">
                    Confirm Password
                </label>
                <input
                    class="shadow appearance-none border rounded w-full py-2 px-3 mb-3 leading-tight focus:outline-none"
                    id="confirm_password"
                    name="new_password_confirmation"
                    type="password"
                    autocomplete="new-password"
                    placeholder="New password (again)"
                />
                <input type="hidden" name="pathname" value=pathname().0 />
                <input
                    type="submit"
                    class="bg-button-dawn dark:bg-button-twilight transform transition-transform duration-300 active:scale-95 hover:bg-pillbug-teal text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer"
                    value="Save"
                />
            </ActionForm>
        </div>
    }
}
