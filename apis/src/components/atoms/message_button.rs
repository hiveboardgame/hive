use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::A;

#[component]
pub fn MessageButton(username: String, #[prop(optional)] compact: bool) -> impl IntoView {
    let i18n = use_i18n();
    let href = format!("/message/dm/{username}");

    if compact {
        view! {
            <A
                href=href
                attr:class="no-link-style ui-button ui-button-primary ui-button-icon"
                attr:title=move || t_string!(i18n, messages.page.message_button)
                attr:aria-label=move || t_string!(i18n, messages.page.message_button)
            >
                <Icon icon=icondata_bi::BiChatRegular attr:class="size-6" />
            </A>
        }
        .into_any()
    } else {
        view! {
            <A href=href attr:class="no-link-style ui-button ui-button-primary ui-button-sm">
                <Icon icon=icondata_bi::BiChatRegular attr:class="size-5 shrink-0" />
                {move || t_string!(i18n, messages.page.message_button)}
            </A>
        }
        .into_any()
    }
}
