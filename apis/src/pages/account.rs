use crate::{
    common::{
        password_error_message,
        password_requirements_message,
        use_return_path,
        validate_password,
    },
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::panel::Panel,
        update_from_event::update_from_input,
    },
    functions::accounts::{delete::DeleteAccount, edit::EditAccount},
    i18n::*,
    providers::AuthContext,
    pwa,
    security::csrf::ActionForm,
};
use leptos::{prelude::*, task::spawn_local};

#[component]
pub fn Account() -> impl IntoView {
    let i18n = use_i18n();
    let account_action = ServerAction::<EditAccount>::new();
    let delete_action = ServerAction::<DeleteAccount>::new();
    let auth_session = expect_context::<AuthContext>().session_actions();
    let pathname = use_return_path();
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());
    let delete_password = RwSignal::new(String::new());

    let password_validation =
        Memo::new(move |_| validate_password(&new_password(), &confirm_password()));
    let password_invalid = move || password_validation.get().is_err();
    let password_blurred = RwSignal::new(false);
    let password_error = move || {
        password_validation
            .get()
            .err()
            .filter(|error| error.is_ready_to_display(password_blurred(), &confirm_password()))
    };

    let form_invalid = move || current_password.with(String::is_empty) || password_invalid();

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
    let delete_form_invalid = move || delete_password.with(String::is_empty);
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
                <h1 class="ui-page-title">{t!(i18n, common.account.title)}</h1>
                <p class="ui-page-subtitle">
                    {t!(i18n, common.account.manage_password_changes_and_account_deletion)}
                </p>
            </div>

            <Panel
                title=move || t_string!(i18n, common.account.change_password)
                body_class="space-y-4"
            >
                <ActionForm action=account_action attr:class="space-y-4">

                    <label class="flex flex-col gap-1.5" for="old_password">
                        <span class="ui-field-label">{t!(i18n, common.current_password)}</span>
                        <input
                            on:input=update_from_input(current_password)
                            class="ui-field-input"
                            id="old_password"
                            name="password"
                            type="password"
                            prop:value=current_password
                            autocomplete="current-password"
                            placeholder=move || t_string!(i18n, common.current_password)
                        />
                    </label>
                    <label class="flex flex-col gap-1.5" for="new_password">
                        <span class="ui-field-label">{t!(i18n, common.new_password)}</span>
                        <input
                            on:input=update_from_input(new_password)
                            on:blur=move |_| password_blurred.set(true)
                            class="ui-field-input"
                            name="new_password"
                            id="new_password"
                            type="password"
                            prop:value=new_password
                            autocomplete="new-password"
                            placeholder=move || t_string!(i18n, common.new_password)
                        />
                        <small class="ui-field-helper">
                            {move || password_requirements_message(i18n)}
                        </small>
                    </label>
                    <label class="flex flex-col gap-1.5" for="confirm_password">
                        <span class="ui-field-label">{t!(i18n, common.confirm_password)}</span>
                        <input
                            on:input=update_from_input(confirm_password)
                            class="ui-field-input"
                            id="confirm_password"
                            name="new_password_confirmation"
                            type="password"
                            prop:value=confirm_password
                            autocomplete="new-password"
                            placeholder=move || {
                                t_string!(
                                    i18n, user_config.edit_account.confirm_password_placeholder
                                )
                            }
                        />
                    </label>
                    <Show when=move || password_error().is_some() && !new_password().is_empty()>
                        <small class="ui-field-error">
                            {move || {
                                password_error().map(|error| password_error_message(i18n, error))
                            }}
                        </small>
                    </Show>
                    <input type="hidden" name="pathname" value=pathname />
                    <button
                        type="submit"
                        disabled=form_invalid
                        class="w-full ui-button ui-button-success ui-button-md"
                    >
                        {t!(i18n, common.save)}
                    </button>
                    <Show when=display_account_error>
                        <small class="ui-field-error">
                            {t!(i18n, common.account.password_update_failed)}
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
                        <span class="ui-field-label">{t!(i18n, common.current_password)}</span>
                        <input
                            on:input=update_from_input(delete_password)
                            class="ui-field-input"
                            id="delete_password"
                            name="password"
                            type="password"
                            prop:value=delete_password
                            autocomplete="current-password"
                            placeholder=move || { t_string!(i18n, common.current_password) }
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
