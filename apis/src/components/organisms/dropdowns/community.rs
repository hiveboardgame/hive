use crate::components::layouts::base_layout::{
    COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE, DROPDOWN_MENU_STYLE,
};
use crate::components::molecules::hamburger::Hamburger;
use crate::i18n::*;
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
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            content=move || name
            id="Community"
        >
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/top_players">
                {t!(i18n, header.community.top_players)}
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/resources">
                {t!(i18n, header.community.resources)}
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/faq">
                {t!(i18n, header.community.faq)}
            </a>
        </Hamburger>
    }
}
