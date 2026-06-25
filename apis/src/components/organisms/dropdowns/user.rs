use crate::{
    components::{
        molecules::{hamburger::Hamburger, ping::Ping},
        organisms::{
            darkmode_toggle::{DarkModeToggle, DarkModeToggleVariant},
            header::set_redirect,
            logout::Logout,
        },
    },
    i18n::*,
    providers::{AuthContext, RefererContext},
};
use leptos::prelude::*;

#[component]
pub fn UserDropdown(username: String) -> impl IntoView {
    let i18n = use_i18n();
    let pathname = expect_context::<RefererContext>().pathname;
    let auth_context = expect_context::<AuthContext>();
    let hamburger_show = RwSignal::new(false);
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
