use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::components::A;
use uuid::Uuid;

pub fn dm_messages_href(other_user_id: Uuid, username: &str) -> String {
    format!(
        "/messages?dm={other_user_id}&username={}",
        urlencoding::encode(username)
    )
}

#[component]
pub fn MessageButton(
    other_user_id: Uuid,
    username: String,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    // TODO: Callers currently own "can this CTA be shown?" policy, and that policy is not
    // consistent yet: some places use cheap local checks while profile_view also calls
    // can_message_user(). Revisit whether that logic should be centralized here or in a
    // shared helper, but do not blindly generalize can_message_user() to high-cardinality
    // lists without caching or batching because it would turn large user renders into many
    // per-user DB-backed checks.
    let i18n = use_i18n();
    let href = StoredValue::new(dm_messages_href(other_user_id, &username));

    if compact {
        view! {
            <A
                href=move || href.get_value()
                attr:class="no-link-style inline-flex items-center justify-center size-8 rounded-lg text-white bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 transition-transform duration-300 [&_svg]:size-5 [&_svg]:shrink-0"
                attr:title=t_string!(i18n, messages.page.message_button)
            >
                <Icon
                    icon=icondata_hi::HiChatBubbleBottomCenterTextOutlineLg
                    attr:class="size-5"
                />
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
                    icon=icondata_hi::HiChatBubbleBottomCenterTextOutlineLg
                    attr:class="size-5 shrink-0"
                />
                {t!(i18n, messages.page.message_button)}
            </A>
        }
        .into_any()
    }
}
