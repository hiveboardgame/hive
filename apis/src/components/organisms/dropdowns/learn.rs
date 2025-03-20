use crate::components::layouts::base_layout::{COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE};
use crate::components::molecules::hamburger::Hamburger;
use crate::i18n::*;
use leptos::prelude::*;
const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2";

#[component]
pub fn LearnDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let i18n = use_i18n();
    let name = t!(i18n, header.learn.title);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            content=move || name
            id="Learn"
        >
            <a
                class=COMMON_LINK_STYLE
                on:click=onclick_close
                href="https://hivegame.com/download/rules.pdf"
                target="_blank"
            >
                {t!(i18n, header.learn.rules)}
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/rules_summary">
                {t!(i18n, header.learn.rules_summary)}
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/analysis">
                {t!(i18n, header.learn.analysis)}
            </a>

        </Hamburger>
    }
}
