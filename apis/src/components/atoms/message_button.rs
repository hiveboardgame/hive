use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::A;

#[component]
pub fn MessageButton(username: String, #[prop(optional)] compact: bool) -> impl IntoView {
    let i18n = use_i18n();
    let href = format!("/message/dm/{username}");
    let label = move || t_string!(i18n, messages.page.message_button).to_string();
    view! {
        <A
            href=href
            attr:class=if compact {
                "no-link-style ui-button ui-button-primary ui-button-icon"
            } else {
                "no-link-style ui-button ui-button-primary ui-button-sm"
            }
            attr:title=label
            attr:aria-label=label
        >
            <Icon
                icon=icondata_bi::BiChatRegular
                attr:class=if compact { "size-6" } else { "size-5 shrink-0" }
            />
            <Show when=move || !compact>{label}</Show>
        </A>
    }
}
