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
use leptos_icons::Icon;
use leptos_router::hooks::use_location;
use shared_types::GameId;

#[component]
pub fn UserDropdown(
    username: String,
    current_game_id: Signal<Option<GameId>>,
    update_available: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let pathname = expect_context::<RefererContext>().pathname;
    let auth_context = expect_context::<AuthContext>();
    let chat = expect_context::<Chat>();
    let hamburger_show = RwSignal::new(false);
    let location = use_location();
    let in_analysis = move || {
        location
            .pathname
            .with(|path| path == "/analysis" || path.starts_with("/analysis/"))
    };
    let unread_count = Memo::new(move |_| {
        let current_game_id = current_game_id.get();
        chat.total_unread_count_excluding_game(current_game_id.as_ref())
    });
    let onclick_close = move || hamburger_show.update(|b| *b = false);
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-user-button relative"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-right ui-header-dropdown-menu"
            content=view! {
                <span>{username.clone()}</span>
                <Show when=update_available>
                    <span
                        class="absolute right-1 top-1.5 rounded-full border ring-1 ring-white size-2 border-amber-950 bg-orange-twilight dark:ring-surface-panel"
                        aria-hidden="true"
                    ></span>
                    <span class="sr-only">", " {t!(i18n, header.user_menu.update_available)}</span>
                </Show>
            }
            id="Username"
        >
            <Ping />
            <Show when=update_available>
                <div class="py-3 px-4 min-w-60 max-w-[calc(100vw-1rem)] border-y border-black/10 dark:border-white/10">
                    <div class="flex gap-2 items-center text-sm font-semibold dark:text-blue-300 text-button-dawn">
                        <Icon icon=icondata_bi::BiRefreshRegular attr:class="size-4 shrink-0" />
                        <span>{t!(i18n, header.user_menu.update_available)}</span>
                    </div>
                    <p class="mt-1.5 mb-2 text-xs leading-relaxed text-gray-600 dark:text-gray-300 max-w-60">
                        {move || {
                            if in_analysis() {
                                t_string!(i18n, header.user_menu.save_analysis_before_refresh)
                            } else {
                                t_string!(i18n, header.user_menu.refresh_when_ready)
                            }
                        }}
                    </p>
                    <button
                        type="button"
                        class="py-1.5 px-3 text-xs font-semibold rounded border dark:text-blue-300 dark:border-blue-300 min-h-10 border-button-dawn text-button-dawn dark:hover:bg-blue-dark dark:focus-visible:outline-blue-300 hover:bg-blue-light/30 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-button-dawn"
                        on:click=move |_| {
                            let _ = window().location().reload();
                        }
                    >
                        {t!(i18n, header.user_menu.refresh_now)}
                    </button>
                </div>
            </Show>
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
                            t_string!(
                                i18n,
                                messages.chat.unread_badge,
                                count = unread_count.get(),
                                conversation = t_string!(i18n, header.user_menu.messages).to_string()
                            )
                                .to_string()
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
