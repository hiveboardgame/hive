use crate::components::layouts::base_layout::{
    OrientationSignal, COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE,
};
use crate::components::molecules::{hamburger::Hamburger, ping::Ping};
use crate::components::organisms::chat::ChatWindow;
use crate::components::organisms::darkmode_toggle::DarkModeToggle;
use crate::components::organisms::header::set_redirect;
use crate::components::organisms::logout::Logout;
use crate::providers::chat::Chat;
use leptos::*;
use leptos_icons::*;
use shared_types::SimpleDestination;

const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2";

#[component]
pub fn UserDropdown(username: String) -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move || hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="bg-button-dawn dark:bg-button-twilight text-white rounded-md px-2 py-1 m-1 hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap"
            dropdown_style="mr-1 xs:mt-0 mt-1 flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md p-2 right-0 lg:right-10"
            content=username.clone()
        >
            <a
                class=COMMON_LINK_STYLE
                href=format!("/@/{}", username)

                on:click=move |_| onclick_close()
            >
                Profile
            </a>
            <a
                class=COMMON_LINK_STYLE
                href="/account"
                on:focus=move |_| set_redirect()
                on:click=move |_| onclick_close()
            >
                Edit Account
            </a>
            <a
                class=COMMON_LINK_STYLE
                href="/config"
                on:focus=move |_| set_redirect()
                on:click=move |_| onclick_close()
            >
                Config
            </a>
            <DarkModeToggle/>
            <Logout on:submit=move |_| onclick_close()/>
            <Ping/>
        </Hamburger>
    }
}

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

#[component]
pub fn LearnDropdown() -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            content="Learn"
        >
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tutorial">
                Tutorial
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/rules">
                Rules
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/strategy">
                Strategy
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/puzzles">
                Puzzles
            </a>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/analysis">
                Analysis
            </a>
        </Hamburger>
    }
}

#[component]
pub fn TournamentDropdown() -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            content="Tournament"
        >
            <button class=COMMON_LINK_STYLE on:click=onclick_close>
                Create Tournament
            </button>
            <a class=COMMON_LINK_STYLE on:click=onclick_close href="/tournaments">
                View Tournaments
            </a>
        </Hamburger>
    }
}

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

#[component]
pub fn ChatDropdown(destination: SimpleDestination) -> impl IntoView {
    let chat = expect_context::<Chat>();
    let destination = StoredValue::new(destination);
    let hamburger_show = expect_context::<OrientationSignal>().chat_dropdown_open;
    let chat_style = "absolute z-50 flex-col w-full h-[80dvh] max-w-screen bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md left-0 p-2";
    let button_color = move || {
        if hamburger_show() {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
        } else if chat.has_messages() {
            "bg-ladybug-red"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
        }
    };

    create_effect(move |_| {
        hamburger_show();
        chat.seen_messages()
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style=Signal::derive(move || {
                format!(
                    "{} transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 m-1 px-4 rounded",
                    button_color(),
                )
            })

            dropdown_style=chat_style
            content=view! { <Icon icon=icondata::BiChatRegular class="w-4 h-4"/> }
        >
            <ChatWindow destination=destination()/>
        </Hamburger>
    }
}
