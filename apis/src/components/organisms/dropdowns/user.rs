use crate::components::layouts::base_layout::COMMON_LINK_STYLE;
use crate::components::molecules::{hamburger::Hamburger, ping::Ping};
use crate::components::organisms::darkmode_toggle::DarkModeToggle;
use crate::components::organisms::header::set_redirect;
use crate::components::organisms::logout::Logout;
use leptos::*;

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
            id = "Username"
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
