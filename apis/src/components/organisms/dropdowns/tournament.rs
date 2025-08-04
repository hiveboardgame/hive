use crate::components::layouts::base_layout::{
    COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE, DROPDOWN_MENU_STYLE,
};
use crate::components::molecules::hamburger::Hamburger;
use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn TournamentDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let i18n = use_i18n();
    let name = t!(i18n, header.tournaments.title);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            content=move || name
            id="Tournaments"
        >
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments">
                {t!(i18n, header.tournaments.view)}
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments/create">
                {t!(i18n, header.tournaments.create)}
            </a>
        </Hamburger>
    }
}
