use crate::components::layouts::base_layout::{COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE};
use crate::components::molecules::hamburger::Hamburger;
use crate::i18n::*;
use leptos::prelude::*;

const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2";

// Commented out very incomplete locales (< 30% translated)
const ALL_LOCALES: [Locale; 10] = [
    Locale::ca,
    //    Locale::cs,
    Locale::de,
    Locale::en,
    Locale::es,
    Locale::fr,
    Locale::hu,
    Locale::it,
    //    Locale::ja,
    //    Locale::nl,
    Locale::pt,
    Locale::ro,
    Locale::ru,
    //    Locale::sv,
];

#[component]
pub fn LocaleDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |locale| {
        use_i18n().set_locale(locale);
        hamburger_show.update(|b| *b = false);
    };
    let i18n = use_i18n();
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            id="locale_dropdown"
            content=move || i18n.get_locale().to_string()
        >
            <For each=move || ALL_LOCALES key=move |locale| (locale.to_string()) let:locale>
                <a class=COMMON_LINK_STYLE on:click=move |_| onclick_close(locale)>
                    {locale.to_string()}
                </a>
            </For>
        </Hamburger>
    }
}
