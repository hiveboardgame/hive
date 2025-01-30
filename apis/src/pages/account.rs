use crate::functions;
use crate::functions::oauth::GetDiscordHandle;
use crate::{
    components::organisms::header::Redirect, functions::accounts::edit::EditAccount,
    providers::ApiRequests, providers::AuthContext,
};
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn Account(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let account_action = ServerAction::<EditAccount>::new();
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
            <div class="px-8 pt-6 pb-8 mb-4 rounded shadow-md bg-inherit bg-stone-300 dark:bg-slate-800">
                <div>
                    <div class="block mb-2 font-bold">Linked Discord account</div>
                    <div class="block mb-2 font-bold">
                        <Show when=move || {
                            matches!((auth_context.user)(), Some(Ok(Some(_account))))
                        }>{move || { discord_name.value().get() }}</Show>

                    </div>
                </div>
                <div>
                    <button
                        class="px-4 py-2 font-bold text-white rounded transition-transform duration-300 transform cursor-pointer bg-button-dawn dark:bg-button-twilight active:scale-95 hover:bg-pillbug-teal focus:outline-none"
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
                attr:class="px-8 pt-6 pb-8 mb-4 rounded shadow-md bg-inherit bg-stone-300 dark:bg-slate-800"
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
                <label class="block mb-2 font-bold" for="old_password">
                    Current Password
                </label>
                <input
                    node_ref=my_input
                    class="px-3 py-2 mb-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                    id="old_password"
                    name="password"
                    type="password"
                    autocomplete="current-password"
                    placeholder="Current password"
                />
                <label class="block mb-2 font-bold" for="new_password">
                    New Password
                </label>
                <input
                    class="px-3 py-2 mb-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                    name="new_password"
                    id="new_password"
                    type="password"
                    autocomplete="new-password"
                    placeholder="New password"
                />
                <label class="block mb-2 font-bold" for="confirm_password">
                    Confirm Password
                </label>
                <input
                    class="px-3 py-2 mb-3 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                    id="confirm_password"
                    name="new_password_confirmation"
                    type="password"
                    autocomplete="new-password"
                    placeholder="New password (again)"
                />
                <input type="hidden" name="pathname" value=pathname().0 />
                <input
                    type="submit"
                    class="px-4 py-2 font-bold text-white rounded transition-transform duration-300 transform cursor-pointer bg-button-dawn dark:bg-button-twilight active:scale-95 hover:bg-pillbug-teal focus:outline-none"
                    value="Save"
                />
            </ActionForm>
        </div>
    }
}
