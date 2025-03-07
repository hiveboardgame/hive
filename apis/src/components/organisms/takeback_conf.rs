use crate::functions::accounts::edit::edit_config;
use crate::i18n::*;
use crate::providers::AuthContext;
use leptos::either::EitherOf3;
use leptos::prelude::*;
use shared_types::Takeback;

#[component]
pub fn TakebackConf() -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let action = Action::new(move |takeback: &Takeback| {
        let takeback = takeback.clone();
        let auth_context = auth_context.clone();
        async move { 
            if auth_context.user.get().is_some() {    
                let result = edit_config(takeback).await;
                if result.is_ok() {
                    auth_context.refresh();
                }
                
            }
            
         }
     });
    view! {
        <p class="m-1 text-black dark:text-white">{t!(i18n, user_config. allow_takeback)}</p>
        <div class="flex">
            <Button takeback=Takeback::Always action=action />
            <Button takeback=Takeback::CasualOnly action=action />
            <Button takeback=Takeback::Never action=action />
        </div>
    }
}

#[component]
fn Button(takeback: Takeback, action: Action<Takeback,()>) -> impl IntoView {
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
                    action.dispatch(takeback.get_value());
                }
            >

                {match takeback.get_value() {
                    Takeback::Always => {
                        EitherOf3::A(t!(i18n, user_config.allow_takeback_buttons.always))
                    }
                    Takeback::CasualOnly => {
                        EitherOf3::B(t!(i18n, user_config.allow_takeback_buttons.casual_only))
                    }
                    Takeback::Never => {
                        EitherOf3::C(t!(i18n, user_config.allow_takeback_buttons.never))
                    }
                }}

            </button>
        </div>
    }
}
