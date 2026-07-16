use crate::{components::molecules::hamburger::Hamburger, i18n::*};
use leptos::prelude::*;

#[component]
pub fn LearnDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let i18n = use_i18n();
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-dropdown-button"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-left ui-header-dropdown-menu"
            content=move || t!(i18n, header.learn.title)
            id="Learn"
        >
            <a class="ui-dropdown-link" on:click=onclick_close href="/analysis">
                {t!(i18n, header.learn.analysis)}
            </a>
            <a class="ui-dropdown-link" on:click=onclick_close href="/archive">
                {t!(i18n, header.learn.archive)}
            </a>
            <a
                class="ui-dropdown-link"
                on:click=onclick_close
                href="https://hivegame.com/download/rules.pdf"
                target="_blank"
            >
                {t!(i18n, header.learn.rules)}
            </a>
            <a class="ui-dropdown-link" on:click=onclick_close href="/rules_summary">
                {t!(i18n, header.learn.rules_summary)}
            </a>

        </Hamburger>
    }
}
