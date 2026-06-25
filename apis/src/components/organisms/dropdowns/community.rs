use crate::{components::molecules::hamburger::Hamburger, i18n::*};
use leptos::prelude::*;

#[component]
pub fn CommunityDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let i18n = use_i18n();
    let name = t!(i18n, header.community.title);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-dropdown-button"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-left ui-header-dropdown-menu"
            content=move || name
            id="Community"
        >
            <a class="ui-dropdown-link" on:click=onclick_close href="/top_players">
                {t!(i18n, header.community.top_players)}
            </a>
            <a class="ui-dropdown-link" on:click=onclick_close href="/resources">
                {t!(i18n, header.community.resources)}
            </a>
            <a class="ui-dropdown-link" on:click=onclick_close href="/faq">
                {t!(i18n, header.community.faq)}
            </a>
        </Hamburger>
    }
}
