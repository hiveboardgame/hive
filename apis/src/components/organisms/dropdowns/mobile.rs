use crate::components::layouts::base_layout::COMMON_LINK_STYLE;
use crate::components::molecules::hamburger::Hamburger;
use leptos::*;
use leptos_icons::*;

const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2";

#[component]
pub fn MobileDropdown() -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let div_style = "flex flex-col font-bold m-1 dark:text-white";

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="py-1 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block lg:hidden m-1"
            dropdown_style=DROPDOWN_MENU_STYLE
            content=view! { <Icon icon=icondata::ChMenuHamburger class="w-6 h-6"/> }
        >

            <div class=div_style>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/">
                    Home
                </a>
                Community:
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/top_players">
                    Top Players
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/resources">
                    Resources
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/faq">
                    FAQ
                </a>
                Learn:
                <a
                    class=COMMON_LINK_STYLE
                    on:click=onclick_close
                    href="https://hivegame.com/download/rules.pdf"
                    target="_blank"
                >
                    Rules
                </a>
                <a
                    class=COMMON_LINK_STYLE
                    on:click=onclick_close
                    href="/analysis"
                >
                    Analysis
                </a>
                Tournament:
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments">
                    View Tournaments
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments/create">
                    Create tournament
                </a>
                Support:
                <a
                    class=COMMON_LINK_STYLE
                    on:click=onclick_close
                    href="https://www.gen42.com/"
                    target="_blank"
                    rel="external"
                >
                    Buy Game
                </a>
                <a class=COMMON_LINK_STYLE on:click=onclick_close href="/donate">
                    Donate
                </a>
            </div>

        </Hamburger>
    }
}
