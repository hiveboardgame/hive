use crate::functions::accounts::edit::EditTakeback;
use crate::i18n::*;
use crate::providers::AuthContext;
use leptos::prelude::*;
use shared_types::Takeback;

#[component]
pub fn TakebackConf() -> impl IntoView {
    let i18n = use_i18n();
    let action = ServerAction::<EditTakeback>::new();
    let auth_context = expect_context::<AuthContext>();
    Effect::watch(
        action.version(),
        move |_, _, _| auth_context.refresh(false),
        false,
    );
    view! {
        <ActionForm action=action>
            <p class="m-1 text-black dark:text-white">{t!(i18n, user_config. allow_takeback)}</p>
            <div class="flex flex-wrap">
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
        if user.with(|u| {
            u.as_ref()
                .is_some_and(|user| user.user.takeback == takeback.get_value())
        }) {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    };
    let value = match takeback.get_value() {
        Takeback::Always => "Always",
        Takeback::CasualOnly => "CasualOnly",
        Takeback::Never => "Never",
    };
    view! {
        <div class="inline-flex justify-center items-center m-1 text-base font-medium rounded-md border border-transparent shadow cursor-pointer">
            <button
                class=move || {
                    format!(
                        "w-full h-full transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }
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
        </div>
    }
}
