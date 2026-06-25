use crate::providers::Config;
use leptos::prelude::*;

#[component]
pub fn PreSelectToggle() -> impl IntoView {
    view! {
        <div class="ui-choice-group">
            <PreSelectButton enabled=true />
            <PreSelectButton enabled=false />
        </div>
    }
}

#[component]
fn PreSelectButton(enabled: bool) -> impl IntoView {
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config.with(|c| c.allow_preselect) == enabled;

    view! {
        <button
            class="ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
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
    }
}
