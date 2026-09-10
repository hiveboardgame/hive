use crate::providers::RefererContext;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn LoginButton(#[prop(into)] class: String) -> impl IntoView {
    let referrer = expect_context::<RefererContext>().pathname;
    view! {
        <a class=class href="/login" on:focus=move |_| set_redirect(referrer)>
            // TODO: i18n once copy is approved.
            "Login"
        </a>
    }
}

pub fn set_redirect(referrer: StoredValue<String>) {
    referrer.set_value(use_location().pathname.get_untracked());
}
