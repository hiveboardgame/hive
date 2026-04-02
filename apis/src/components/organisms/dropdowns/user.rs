use crate::{
    components::{
        layouts::base_layout::COMMON_LINK_STYLE,
        molecules::{hamburger::Hamburger, ping::Ping},
        organisms::{darkmode_toggle::DarkModeToggle, header::set_redirect, logout::Logout},
    },
    i18n::*,
    providers::{chat::Chat, AuthContext, RefererContext},
};
use leptos::prelude::*;
use leptos_router::components::A;

/// Unread count badge for Messages: compact pill, works in light/dark, clear contrast.
const MESSAGES_BADGE_CLASS: &str = "shrink-0 inline-flex items-center justify-center min-w-[1.25rem] h-5 px-1.5 text-[10px] font-semibold leading-none text-white bg-ladybug-red dark:bg-red-500 rounded-full border border-white/20 dark:border-white/10";

#[component]
fn UnreadMessagesBadge(chat: Chat) -> impl IntoView {
    let unread_count = Memo::new(move |_| chat.total_unread_count());
    view! {
        <Show when=move || unread_count.get().gt(&0)>
            <span class=MESSAGES_BADGE_CLASS>
                {move || {
                    let n = unread_count.get();
                    if n > 99 { "99+".to_string() } else { n.to_string() }
                }}
            </span>
        </Show>
    }
}

#[component]
pub fn UserDropdown(username: String) -> impl IntoView {
    let i18n = use_i18n();
    let pathname = expect_context::<RefererContext>().pathname;
    let auth_context = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move || hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="bg-button-dawn dark:bg-button-twilight text-white rounded-md px-2 py-1 m-1 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap"
            dropdown_style="mr-1 xs:mt-0 mt-1 flex flex-col items-stretch absolute w-max bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md p-2 right-0 z-50"
            content=view! {
                <span class="flex gap-1.5 items-center">
                    {username.clone()} <UnreadMessagesBadge chat />
                </span>
            }
            id="Username"
        >
            <A
                attr:class=COMMON_LINK_STYLE
                href=format!("/@/{}", username)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.profile)}
            </A>
            <A
                attr:class=format!("{} inline-flex items-center gap-1.5", COMMON_LINK_STYLE)
                href="/messages"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.messages)}
                <UnreadMessagesBadge chat />
            </A>
            <A
                attr:class=COMMON_LINK_STYLE
                href="/account"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.edit_account)}
            </A>
            <A
                attr:class=COMMON_LINK_STYLE
                href="/config"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.config)}
            </A>
            <Show when=move || auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))>
                <A attr:class=COMMON_LINK_STYLE href="/admin" on:click=move |_| onclick_close()>
                    Admin
                </A>
            </Show>
            <DarkModeToggle />
            <Logout on:submit=move |_| onclick_close() />
            <Ping />
        </Hamburger>
    }
}
