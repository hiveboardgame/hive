use std::sync::Arc;

use crate::i18n::*;
use crate::providers::{ApiRequestsProvider, AuthContext};
use leptos::prelude::*;
use shared_types::Takeback;

#[component]
pub fn TakebackConf() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config. allow_takeback)}</p>
        <div class="flex">
            <Button takeback=Takeback::Always />
            <Button takeback=Takeback::CasualOnly />
            <Button takeback=Takeback::Never />
        </div>
    }
}

#[component]
fn Button(takeback: Takeback) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0.get_value();
    let i18n = use_i18n();
    let takeback = StoredValue::new(takeback);
    let auth_context = expect_context::<AuthContext>();
    let user = move || match auth_context.user.get() {
        Some(Ok(user)) => Some(user),
        _ => None,
    };
    let is_active = move || {
        if user().is_some_and(|user| user.user.takeback == takeback.get_value()) {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
        }
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

                on:click=move |_| {
                    api.set_server_user_conf(takeback.get_value());
                }
            >

                {match takeback.get_value() {
                    Takeback::Always => {
                        t!(i18n, user_config.allow_takeback_buttons.always).into_any()
                    }
                    Takeback::CasualOnly => {
                        t!(i18n, user_config.allow_takeback_buttons.casual_only).into_any()
                    }
                    Takeback::Never => {
                        t!(i18n, user_config.allow_takeback_buttons.never).into_any()
                    }
                }}

            </button>
        </div>
    }
}
