use crate::{
    components::molecules::hamburger::Hamburger,
    i18n::*,
    providers::{AuthContext, AuthIdentity},
};
use leptos::prelude::*;

#[component]
pub fn TournamentDropdown() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let logged_in = move || matches!(auth_context.identity.get(), Some(AuthIdentity::User(_)));
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
                {t!(i18n, tournaments.menu.browse)}
            </a>
            <Show when=logged_in>
                <a class="ui-dropdown-link" on:click=onclick_close href="/@/me/tournaments">
                    {t!(i18n, tournaments.menu.mine)}
                </a>
            </Show>
            <a class="ui-dropdown-link" on:click=onclick_close href="/tournaments/create">
                {t!(i18n, tournaments.menu.create)}
            </a>
        </Hamburger>
    }
}
