use crate::components::layouts::base_layout::{
    COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE,
};
use crate::components::molecules::hamburger::Hamburger;
use leptos::*;

const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2";

#[component]
pub fn CommunityDropdown() -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            content="Community"
        >
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/top_players">
                Top Players
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/resources">
                Resources
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/faq">
                FAQ
            </a>
        </Hamburger>
    }
}
