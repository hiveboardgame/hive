use crate::common::login_redirect_url;
use leptos::prelude::*;

#[component]
pub fn LoginButton(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <a class=class href=move || login_redirect_url()>
            // TODO: i18n once copy is approved.
            "Login"
        </a>
    }
}
