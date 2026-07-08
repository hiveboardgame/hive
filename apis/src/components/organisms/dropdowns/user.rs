use crate::{
    components::{
        atoms::unread_badge::UnreadBadge,
        molecules::{hamburger::Hamburger, ping::Ping},
        organisms::{
            darkmode_toggle::{DarkModeToggle, DarkModeToggleVariant},
            header::set_redirect,
            logout::Logout,
        },
    },
    i18n::*,
    providers::{chat::Chat, AuthContext, RefererContext},
};
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_params_map};
use shared_types::GameId;

#[component]
pub fn UserDropdown(username: String) -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let params = use_params_map();
    let pathname = expect_context::<RefererContext>().pathname;
    let auth_context = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let current_game_id = Signal::derive(move || {
        location
            .pathname
            .with(|path| path.starts_with("/game/"))
            .then(|| {
                params
                    .get()
                    .get("nanoid")
                    .map(|nanoid| GameId(nanoid.to_string()))
            })
            .flatten()
    });
    let unread_count = Signal::derive(move || {
        let current_game_id = current_game_id.get();
        chat.total_unread_count_excluding_game(current_game_id.as_ref())
    });
    let onclick_close = move || hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-user-button"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-right ui-header-dropdown-menu"
            content=username.clone()
            id="Username"
        >
            <Ping />
            <a
                class="ui-dropdown-link"
                href=format!("/@/{}", username)

                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.profile)}
            </a>
            <a
                class="ui-dropdown-link"
                href="/message"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                <span>{t!(i18n, header.user_menu.messages)}</span>
                <span class="ml-auto">
                    <UnreadBadge
                        count=unread_count
                        aria_label=Signal::derive(move || {
                            format!("{} unread messages", unread_count.get())
                        })
                    />
                </span>
            </a>
            <a
                class="ui-dropdown-link"
                href="/account"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.edit_account)}
            </a>
            <a
                class="ui-dropdown-link"
                href="/config"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.config)}
            </a>
            <a
                class="ui-dropdown-link"
                href="/notifications"
                on:focus=move |_| set_redirect(pathname)
                on:click=move |_| onclick_close()
            >
                {t!(i18n, header.user_menu.notifications)}
            </a>
            <Show when=move || auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))>
                <a
                    class="ui-dropdown-link"
                    href="/admin"

                    on:click=move |_| onclick_close()
                >
                    Admin
                </a>
            </Show>
            <DarkModeToggle variant=DarkModeToggleVariant::Dropdown />
            <Logout on:submit=move |_| onclick_close() />
        </Hamburger>
    }
}
