use crate::components::layouts::base_layout::COMMON_LINK_STYLE;
use crate::components::molecules::hamburger::Hamburger;
use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::*;

const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2";

#[component]
pub fn MobileDropdown() -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let div_style = "flex flex-col font-bold m-1 dark:text-white";
    let i18n = use_i18n();
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="py-1 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block lg:hidden m-1"
            dropdown_style=DROPDOWN_MENU_STYLE
            content=view! { <Icon icon=icondata::ChMenuHamburger attr:class="w-6 h-6" /> }
            id="Mobile"
        >

            <div class=div_style>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/">
                    {t!(i18n, header.home)}
                </a>
                {t!(i18n, header.community.title)}
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/top_players">
                    {t!(i18n, header.community.top_players)}
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/resources">
                    {t!(i18n, header.community.resources)}
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/faq">
                    {t!(i18n, header.community.faq)}
                </a>
                {t!(i18n, header.learn.title)}
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
                {t!(i18n, header.tournaments.title)}
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments">
                    {t!(i18n, header.tournaments.view)}
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments/create">
                    {t!(i18n, header.tournaments.create)}
                </a>
                {t!(i18n, header.support)}
                <a
                    class=COMMON_LINK_STYLE
                    on:click=onclick_close
                    href="https://www.gen42.com/"
                    target="_blank"
                    rel="external"
                >
                    {t!(i18n, header.buy_game)}
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/donate">
                    {t!(i18n, header.donate)}
                </a>
            </div>

        </Hamburger>
    }
}
