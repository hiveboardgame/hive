use crate::{functions::accounts::edit::EditTakeback, i18n::*, providers::AuthContext};
use leptos::prelude::*;
use shared_types::Takeback;

#[component]
pub fn TakebackConf() -> impl IntoView {
    let i18n = use_i18n();
    let action = ServerAction::<EditTakeback>::new();
    let action_value = action.value();
    let auth_context = expect_context::<AuthContext>();
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if action_value
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                auth_context.refresh_account();
            }
        },
        false,
    );
    view! {
        <ActionForm action=action attr:class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config. allow_takeback)}</p>
            <div class="ui-choice-group">
                <Button takeback=Takeback::Always />
                <Button takeback=Takeback::CasualOnly />
                <Button takeback=Takeback::Never />
            </div>
        </ActionForm>
    }
}

#[component]
fn Button(takeback: Takeback) -> impl IntoView {
    let i18n = use_i18n();
    let takeback = StoredValue::new(takeback);
    let auth_context = expect_context::<AuthContext>();
    let user = auth_context.user;
    let is_active = move || {
        user.with(|u| {
            u.as_ref()
                .is_some_and(|user| user.user.takeback == takeback.get_value())
        })
    };
    let value = match takeback.get_value() {
        Takeback::Always => "Always",
        Takeback::CasualOnly => "CasualOnly",
        Takeback::Never => "Never",
    };
    view! {
        <button
            class="ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            type="submit"
            name="takeback"
            value=value
        >
            {move || match takeback.get_value() {
                Takeback::Always => t_string!(i18n, user_config.allow_takeback_buttons.always),
                Takeback::CasualOnly => {
                    t_string!(i18n, user_config.allow_takeback_buttons.casual_only)
                }
                Takeback::Never => t_string!(i18n, user_config.allow_takeback_buttons.never),
            }}
        </button>
    }
}
