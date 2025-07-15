use crate::providers::Config;
use leptos::prelude::*;

#[component]
pub fn PreSelectToggle(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    view! {
        <div class=format!("flex flex-wrap {extend_tw_classes}")>
            <PreSelectButton enabled=true />
            <PreSelectButton enabled=false />
        </div>
    }
}

#[component]
fn PreSelectButton(enabled: bool) -> impl IntoView {
    let Config(config, set_cookie) = expect_context();
    let is_active = move || {
        if config.with(|c| c.allow_preselect) == enabled {
            "bg-pillbug-teal"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        }
    };

    view! {
        <div class="inline-flex justify-center items-center m-1 text-base font-medium rounded-md border border-transparent shadow cursor-pointer">
            <button
                class=move || {
                    format!(
                        "w-full h-full text-white transform transition-transform duration-300 active:scale-95 font-bold py-2 px-4 rounded focus:outline-none cursor-pointer {}",
                        is_active(),
                    )
                }
                on:click=move |_| {
                    set_cookie
                        .update(|c| {
                            if let Some(cookie) = c {
                                cookie.allow_preselect = enabled;
                            }
                        });
                }
            >
                {if enabled { "Yes" } else { "No" }}
            </button>
        </div>
    }
}
