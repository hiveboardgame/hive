use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::A;

fn dm_messages_href(username: &str) -> String {
    format!("/message/dm/{username}")
}

#[component]
pub fn MessageButton(username: String, #[prop(optional)] compact: bool) -> impl IntoView {
    let i18n = use_i18n();
    let href = StoredValue::new(dm_messages_href(&username));

    if compact {
        view! {
            <A
                href=move || href.get_value()
                attr:class="no-link-style inline-flex items-center justify-center p-1 mx-2 rounded text-white bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 transition-transform duration-300 [&_svg]:size-6 [&_svg]:shrink-0"
                attr:title=move || t_string!(i18n, messages.page.message_button)
            >
                <Icon icon=icondata_bi::BiChatRegular attr:class="size-6 stroke-white" />
            </A>
        }
        .into_any()
    } else {
        view! {
            <A
                href=move || href.get_value()
                attr:class="no-link-style inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-lg bg-pillbug-teal text-white hover:bg-pillbug-teal/90 dark:bg-pillbug-teal dark:text-white dark:hover:bg-pillbug-teal/90 transition-colors [&_svg]:text-inherit"
            >
                <Icon
                    icon=icondata_bi::BiChatRegular
                    attr:class="size-5 shrink-0 stroke-white"
                />
                {move || t_string!(i18n, messages.page.message_button)}
            </A>
        }
        .into_any()
    }
}
