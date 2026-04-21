use crate::{
    components::{
        atoms::unread_badge::UnreadBadge,
        layouts::base_layout::COMMON_LINK_STYLE,
        molecules::{hamburger::Hamburger, ping::Ping},
        organisms::{darkmode_toggle::DarkModeToggle, header::set_redirect, logout::Logout},
    },
    i18n::*,
    providers::{chat::Chat, AuthContext, RefererContext},
};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_location};
use shared_types::GameId;

fn current_game_id_from_path(pathname: &str) -> Option<GameId> {
    let mut segments = pathname.trim_matches('/').split('/');
    match (segments.next(), segments.next()) {
        (Some("game"), Some(game_id)) if !game_id.is_empty() => Some(GameId(game_id.to_string())),
        _ => None,
    }
}

#[component]
pub fn UserDropdown(username: String) -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let pathname = expect_context::<RefererContext>().pathname;
    let auth_context = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let current_game_id =
        Signal::derive(move || current_game_id_from_path(&location.pathname.get()));
    let unread_count = Signal::derive(move || {
        let suppressed_game_id = current_game_id.get();
        chat.total_unread_count_excluding_game(suppressed_game_id.as_ref())
    });
    let button_style = Signal::derive(move || {
        let color = if unread_count.get() > 0 {
            "bg-ladybug-red hover:bg-red-600 dark:bg-red-600 dark:hover:bg-red-500"
        } else {
            "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"
        };
        format!(
            "{color} text-white rounded-md px-2 py-1 m-1 transform transition-transform duration-300 active:scale-95 whitespace-nowrap"
        )
    });
    let onclick_close = move || hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style
            dropdown_style="mr-1 xs:mt-0 mt-1 flex flex-col items-stretch absolute w-max bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md p-2 right-0 z-50"
            content=view! { <span>{ username.clone()}</span> }
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
                href="/message"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.messages)}
                <UnreadBadge count=unread_count />
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
