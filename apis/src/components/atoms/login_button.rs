use crate::{common::login_redirect_url, i18n::*};
use leptos::prelude::*;

#[component]
pub fn LoginButton(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <a class=class href=login_redirect_url>

            {t!(use_i18n(), common.login)}
        </a>
    }
}
