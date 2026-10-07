use crate::{components::molecules::hamburger::Hamburger, i18n::*};
use leptos::prelude::*;

#[component]
pub fn TournamentDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let i18n = use_i18n();
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-dropdown-button"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-left ui-header-dropdown-menu"
            content=move || t!(i18n, common.tournaments)
            id="Tournaments"
        >
            <a class="ui-dropdown-link" on:click=onclick_close href="/tournaments">
                {t!(i18n, tournaments.view_list)}
            </a>
            <a class="ui-dropdown-link" on:click=onclick_close href="/tournaments/create">
                {t!(i18n, tournaments.create)}
            </a>
        </Hamburger>
    }
}
