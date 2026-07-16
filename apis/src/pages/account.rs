use crate::{
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::panel::Panel,
        update_from_event::update_from_input,
    },
    functions::accounts::{delete::DeleteAccount, edit::EditAccount},
    i18n::*,
    providers::{AuthContext, RefererContext},
    pwa,
};
use leptos::{form::ActionForm, leptos_dom::helpers::debounce, prelude::*, task::spawn_local};
use std::time::Duration;

#[component]
pub fn Account() -> impl IntoView {
    let i18n = use_i18n();
    let account_action = ServerAction::<EditAccount>::new();
    let delete_action = ServerAction::<DeleteAccount>::new();
    let auth_session = expect_context::<AuthContext>().session_actions();
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
        account_action.version(),
        move |_, _, _| {
            if let Some(Ok(account)) = account_action.value().get_untracked() {
                auth_session.accept_same_user_refresh(account);
            }
        },
        false,
    );
    Effect::watch(
        delete_action.version(),
        move |_, _, _| {
            if delete_action
                .value()
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                spawn_local(pwa::clear_local_subscription());
                auth_session.accept_anonymous();
            }
        },
        false,
    );

    view! {
        <PageShell variant=PageShellVariant::Form>
            <div class="flex flex-col gap-1">
                <h1 class="ui-page-title">"Account"</h1>
                <p class="ui-page-subtitle">"Manage password changes and account deletion."</p>
            </div>

            <Panel title="Change Password" body_class="space-y-4">
                <ActionForm action=account_action attr:class="space-y-4">
                    <label class="flex flex-col gap-1.5" for="old_password">
                        <span class="ui-field-label">"Current Password"</span>
                        <input
                            on:input=debounce(
                                Duration::from_millis(350),
                                update_from_input(current_password),
                            )
                            class="ui-field-input"
                            id="old_password"
                            name="password"
                            type="password"
                            prop:value=current_password
                            autocomplete="current-password"
                            placeholder="Current password"
                        />
                    </label>
                    <label class="flex flex-col gap-1.5" for="new_password">
                        <span class="ui-field-label">"New Password"</span>
                        <input
                            on:input=debounce(
                                Duration::from_millis(350),
                                update_from_input(new_password),
                            )
                            class="ui-field-input"
                            name="new_password"
                            id="new_password"
                            type="password"
                            prop:value=new_password
                            autocomplete="new-password"
                            placeholder="New password"
                            minlength="8"
                            maxlength="128"
                        />
                    </label>
                    <label class="flex flex-col gap-1.5" for="confirm_password">
                        <span class="ui-field-label">"Confirm Password"</span>
                        <input
                            on:input=debounce(
                                Duration::from_millis(350),
                                update_from_input(confirm_password),
                            )
                            class="ui-field-input"
                            id="confirm_password"
                            name="new_password_confirmation"
                            type="password"
                            prop:value=confirm_password
                            autocomplete="new-password"
                            placeholder="New password (again)"
                            minlength="8"
                            maxlength="128"
                        />
                    </label>
                    <Show when=move || password_invalid() && !new_password().is_empty()>
                        <small class="ui-field-error">
                            "Password must be at least 8 characters and match confirmation"
                        </small>
                    </Show>
                    <input type="hidden" name="pathname" value=pathname.get_value() />
                    <button
                        type="submit"
                        disabled=form_invalid
                        class="w-full ui-button ui-button-success ui-button-md"
                    >
                        "Save Changes"
                    </button>
                    <Show when=display_account_error>
                        <small class="ui-field-error">
                            "Error updating password. Please check your current password and try again."
                        </small>
                    </Show>
                </ActionForm>
            </Panel>

            <Panel
                title=move || { t_string!(i18n, user_config.delete_account.title) }
                body_class="space-y-4"
            >
                <p class="ui-danger-notice">{t!(i18n, user_config.delete_account.description)}</p>
                <ActionForm action=delete_action attr:class="space-y-4">
                    <label class="flex flex-col gap-1.5" for="delete_password">
                        <span class="ui-field-label">
                            {t!(i18n, user_config.delete_account.password)}
                        </span>
                        <input
                            on:input=debounce(
                                Duration::from_millis(350),
                                update_from_input(delete_password),
                            )
                            class="ui-field-input"
                            id="delete_password"
                            name="password"
                            type="password"
                            prop:value=delete_password
                            autocomplete="current-password"
                            placeholder=move || {
                                t_string!(i18n, user_config.delete_account.password)
                            }
                        />
                    </label>
                    <button
                        type="submit"
                        disabled=delete_form_invalid
                        class="w-full ui-button ui-button-danger ui-button-md"
                    >
                        {move || t_string!(i18n, user_config.delete_account.button)}
                    </button>
                    <Show when=display_delete_error>
                        <small class="ui-field-error">
                            {t!(i18n, user_config.delete_account.error)}
                        </small>
                    </Show>
                </ActionForm>
            </Panel>
        </PageShell>
    }
}
